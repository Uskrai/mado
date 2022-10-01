use anyhow::Context;
use deno_core::{
    v8::{self, Function, Global, HandleScope, Local, Object, Value},
    OpState,
};

use mado_core::{ChapterImageInfo, ChapterTask, Error, MadoModule, MangaAndChaptersInfo, Uuid};
use serde::de::DeserializeOwned;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use crate::{error::Error as DenoError, task::DenoChapterTask, ResultJson};

#[derive(Debug)]
pub struct DenoMadoModule {
    name: String,
    uuid: Uuid,
    domain: Url,
    sender: mpsc::Sender<ModuleMessage>,
    client: mado_core::Client,
}

impl DenoMadoModule {
    pub fn new(
        name: String,
        uuid: Uuid,
        domain: Url,
        client: mado_core::Client,
        sender: mpsc::Sender<ModuleMessage>,
    ) -> Self {
        Self {
            name,
            uuid,
            domain,
            sender,
            client,
        }
    }

    async fn send_message<R>(
        &self,
        produce: impl FnOnce(oneshot::Sender<Result<R, Error>>) -> ModuleMessage,
    ) -> Result<R, Error> {
        let (cx, rx) = oneshot::channel();

        self.sender
            .send(produce(cx))
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        rx.await.context("cannot await request")?
    }

    async fn close(self) -> Result<(), Error> {
        self.send_message(ModuleMessage::Close).await
    }
}

#[async_trait::async_trait]
impl MadoModule for DenoMadoModule {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn client(&self) -> &mado_core::Client {
        &self.client
    }

    fn domain(&self) -> &Url {
        &self.domain
    }

    async fn get_info(&self, url: mado_core::url::Url) -> Result<MangaAndChaptersInfo, Error> {
        self.send_message(|cx| ModuleMessage::GetInfo(url, cx))
            .await
    }

    async fn get_chapter_images(&self, id: &str, task: Box<dyn ChapterTask>) -> Result<(), Error> {
        self.send_message(|cx| ModuleMessage::GetChapterImages(id.to_string(), task, cx))
            .await
    }

    async fn download_image(
        &self,
        image: ChapterImageInfo,
    ) -> Result<mado_core::RequestBuilder, mado_core::Error> {
        self.send_message(|cx| ModuleMessage::DownloadImage(image, cx))
            .await
    }
}

pub enum ModuleMessage {
    GetInfo(Url, oneshot::Sender<Result<MangaAndChaptersInfo, Error>>),
    GetChapterImages(
        String,
        Box<dyn ChapterTask>,
        oneshot::Sender<Result<(), Error>>,
    ),
    DownloadImage(
        ChapterImageInfo,
        oneshot::Sender<Result<mado_core::RequestBuilder, Error>>,
    ),
    Close(oneshot::Sender<Result<(), Error>>),
}

pub struct ModuleLoop {
    receiver: mpsc::Receiver<ModuleMessage>,
    runtime: crate::Runtime,
    object: Global<Object>,
    client: mado_core::http::Client,
}

struct FunctionCaller<'a> {
    recv: Local<'a, Object>,
    function: Local<'a, Function>,
}

impl<'a> FunctionCaller<'a> {
    fn call<'s>(
        &self,
        scope: &mut HandleScope<'s>,
        args: &[Local<Value>],
    ) -> Option<Local<'s, Value>> {
        self.function.call(scope, self.recv.into(), args)
    }
}

impl ModuleLoop {
    pub fn new(
        receiver: mpsc::Receiver<ModuleMessage>,
        runtime: crate::Runtime,
        object: Global<Object>,
        client: mado_core::http::Client,
    ) -> Self {
        Self {
            receiver,
            runtime,
            object,
            client,
        }
    }

    pub async fn start(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            let future = async {
                match msg {
                    ModuleMessage::GetInfo(url, cx) => self.get_info(url, cx).await,
                    ModuleMessage::GetChapterImages(id, task, cx) => {
                        self.get_chapter_image(id, task, cx).await
                    }
                    ModuleMessage::DownloadImage(info, cx) => self.download_image(info, cx).await,
                    ModuleMessage::Close(cx) => {
                        cx.send(Ok(())).unwrap();
                    }
                };
            };

            future.await;
        }
    }

    fn with_scope<F, R>(&self, fun: F) -> Result<R, DenoError>
    where
        F: FnOnce(&mut HandleScope) -> Result<R, DenoError>,
    {
        self.runtime.clone().with_scope(fun)
    }

    fn with_state<F, R>(&self, fun: F) -> Result<R, DenoError>
    where
        F: FnOnce(&mut OpState) -> Result<R, DenoError>,
    {
        self.runtime.clone().with_state(fun)
    }

    fn with_scope_state<F, R>(&self, fun: F) -> Result<R, DenoError>
    where
        F: FnOnce(&mut HandleScope, &mut OpState) -> Result<R, DenoError>,
    {
        self.runtime.clone().with_scope_state(fun)
    }

    async fn call_async_function<F, Resource, Ref>(
        &self,
        name: &str,
        resource: Resource,
        args: F,
    ) -> Result<Global<Value>, DenoError>
    where
        Resource: FnOnce(&mut OpState) -> Ref,
        Ref: AsRef<[u32]>,
        F: for<'b> FnOnce(&mut HandleScope<'b>, &[u32], FunctionCaller) -> Option<Local<'b, Value>>,
    {
        let it = {
            let resource = self.with_state(|state| Ok(resource(state)))?;

            let value = |scope: &mut HandleScope| -> Result<Global<Value>, DenoError> {
                let v8_name = v8::String::new(scope, name).unwrap();
                let it = self
                    .object
                    .open(scope)
                    .get(scope, v8_name.into())
                    .with_context(|| format!("no value named {}", name))?;

                let function = Local::<Function>::try_from(it)
                    .with_context(|| format!("{} is not function", name))?;
                let recv = Local::new(scope, self.object.clone());
                let it = args(scope, resource.as_ref(), FunctionCaller { recv, function })
                    .with_context(|| format!("{} return None", name))?;

                Ok(Global::new(scope, it))
            };

            let value = self.with_scope(value);

            self.with_state(|op_state| {
                for it in resource.as_ref() {
                    let _ = op_state.resource_table.close(*it);
                }
                Ok(())
            })?;

            value
        }?;

        self.runtime.resolve_value(it).await.map_err(Into::into)
    }

    fn serialize_result<T>(&self, result: Global<Value>) -> Result<T, DenoError>
    where
        T: DeserializeOwned + serde::Serialize,
    {
        self.with_scope_state(|scope, state| {
            let it = Local::new(scope, result);

            match crate::from_v8(scope, it)? {
                ResultJson::Ok(it) => Ok(it),
                ResultJson::Err(err) => Err(err.take(state)),
            }
        })
    }

    async fn call_async_serialize<T, F, Resource, A>(
        &self,
        name: &str,
        resource: Resource,
        args: F,
    ) -> Result<T, DenoError>
    where
        F: for<'b> FnOnce(&mut HandleScope<'b>, &[u32], FunctionCaller) -> Option<Local<'b, Value>>,
        Resource: FnOnce(&mut OpState) -> A,
        A: AsRef<[u32]>,
        T: DeserializeOwned + serde::Serialize,
    {
        self.call_async_function(name, resource, args)
            .await
            .and_then(|it| self.serialize_result(it))
    }

    pub async fn get_info(
        &self,
        url: Url,
        cx: oneshot::Sender<Result<MangaAndChaptersInfo, Error>>,
    ) {
        let it = self
            .call_async_serialize(
                "getInfo",
                |_| [],
                |scope, _, call| {
                    let args = &[v8::String::new(scope, url.as_str()).unwrap().into()];
                    call.call(scope, args)
                },
            )
            .await;

        let _ = cx.send(it.map_err(Into::into));
    }

    pub async fn get_chapter_image(
        &self,
        id: String,
        task: Box<dyn ChapterTask>,
        cx: oneshot::Sender<Result<(), Error>>,
    ) {
        let it = self
            .call_async_serialize(
                "getChapterImageRust",
                |state| [DenoChapterTask::new_to_state(task, state)],
                |scope, state, call| {
                    let args = &[
                        v8::String::new(scope, &id).unwrap().into(),
                        v8::BigInt::new_from_u64(scope, state[0].into()).into(),
                    ];

                    call.call(scope, args)
                },
            )
            .await;

        let _ = cx.send(it.map_err(Into::into));
    }

    pub async fn download_image(
        &self,
        info: ChapterImageInfo,
        cx: oneshot::Sender<Result<mado_core::RequestBuilder, Error>>,
    ) {
        let it: Result<crate::http::RequestBuilder, _> = self
            .call_async_serialize(
                "downloadImage",
                |_| &[],
                |scope, _, call| {
                    let args = &[serde_v8::to_v8(scope, info).unwrap()];
                    call.call(scope, args)
                },
            )
            .await;

        let it = it.map(|it| it.to_request(&self.client));

        let _ = cx.send(it.map(Into::into).map_err(Into::into));
    }
}
