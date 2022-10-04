use std::{cell::RefCell, rc::Rc, sync::Arc};

use anyhow::Context;
use deno_core::{
    futures::StreamExt,
    parking_lot::Mutex,
    v8::{self, Function, Global, HandleScope, Local, Object, Value},
    Extension, ExtensionBuilder, OpState,
};

use mado_core::{ChapterImageInfo, ChapterTask, Error, MadoModule, MangaAndChaptersInfo, Uuid};
use serde::de::DeserializeOwned;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use crate::{
    error::Error as DenoError,
    task::{DenoChapterTask, JsChapterTask},
    try_json, ResultJson, ToResultJson,
};

#[derive(Debug)]
pub struct DenoMadoModule {
    name: String,
    uuid: Uuid,
    domain: Url,
    sender: mpsc::Sender<ModuleMessage>,
    client: mado_core::Client,
}

impl deno_core::Resource for DenoMadoModule {}

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
    handler: ModuleMessageHandler,
}

#[derive(Clone)]
struct ModuleMessageHandler {
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
            handler: ModuleMessageHandler {
                runtime,
                object,
                client,
            },
        }
    }

    pub async fn start(mut self) {
        let stream = futures::stream::poll_fn(move |cx| self.receiver.poll_recv(cx));

        stream
            .for_each_concurrent(None, |msg| async {
                self.handler.handle_msg(msg).await;
            })
            .await;
    }
}

impl ModuleMessageHandler {
    async fn handle_msg(&self, msg: ModuleMessage) {
        match msg {
            ModuleMessage::GetInfo(url, cx) => self.get_info(url, cx).await,
            ModuleMessage::GetChapterImages(id, task, cx) => {
                self.get_chapter_image(id, task, cx).await
            }
            ModuleMessage::DownloadImage(info, cx) => self.download_image(info, cx).await,
            ModuleMessage::Close(cx) => self.close(cx).await,
        };
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
        let value = match value {
            Ok(val) => self.runtime.resolve_value(val).await.map_err(Into::into),
            Err(err) => Err(err),
        };

        self.with_state(|op_state| {
            for it in resource.as_ref() {
                let _ = op_state.resource_table.close(*it);
            }
            Ok(())
        })?;

        value
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

    async fn call_async_void<F, Resource, A>(
        &self,
        name: &str,
        resource: Resource,
        args: F,
    ) -> Result<(), DenoError>
    where
        F: for<'b> FnOnce(&mut HandleScope<'b>, &[u32], FunctionCaller) -> Option<Local<'b, Value>>,
        Resource: FnOnce(&mut OpState) -> A,
        A: AsRef<[u32]>,
    {
        let it: Result<Option<()>, _> = self.call_async_serialize(name, resource, args).await;

        it.map(|_| ())
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
            .call_async_void(
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

    pub async fn close(&self, cx: oneshot::Sender<Result<(), Error>>) {
        let it: Result<_, _> = self
            .call_async_function(
                "close",
                |_| &[],
                |scope, _, call| {
                    let args = &[];
                    call.call(scope, args)
                },
            )
            .await;

        let _ = cx.send(it.map(|_| ()).map_err(Into::into));
    }
}

#[deno_core::op(v8)]
fn op_mado_module_new(
    scope: &mut v8::HandleScope,
    state: &mut OpState,
    value: serde_v8::Value,
) -> ResultJson<u32> {
    let runtime = state.borrow::<crate::Runtime>().clone();

    let object = value
        .v8_value
        .to_object(scope)
        .ok_or_else(|| anyhow::anyhow!("argument should be object"))
        .map_err(DenoError::from)
        .to_result_json(state);

    let object = try_json!(object);

    let object = v8::Global::new(scope, object);

    let (module, looper) = try_json!(runtime
        .load_object_with_scope_state(scope, state, object)
        .map_err(DenoError::from)
        .to_result_json(state));

    let module = state.resource_table.add(module);
    crate::spawn_local(looper.start());

    ResultJson::Ok(module)
}

fn get_module(state: Rc<RefCell<OpState>>, rid: u32) -> ResultJson<Rc<DenoMadoModule>> {
    let state = &mut state.borrow_mut();

    state
        .resource_table
        .get(rid)
        .map_err(|_| DenoError::resource_error(rid, "Module already closed"))
        .to_result_json(state)
}

#[deno_core::op]
async fn op_mado_module_get_info(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    url: Url,
) -> ResultJson<MangaAndChaptersInfo> {
    let module = crate::try_json!(get_module(state.clone(), rid));

    module
        .get_info(url)
        .await
        .map_err(DenoError::from)
        .to_result_json_borrow(state)
}

#[deno_core::op]
async fn op_mado_module_get_chapter_images(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    id: String,
    task_rid: u32,
) -> ResultJson<()> {
    let module: Rc<DenoMadoModule> = crate::try_json!(get_module(state.clone(), rid));

    let task = try_json!(crate::task::get_chapter_task(
        &mut state.borrow_mut(),
        task_rid
    ));
    state.borrow_mut().resource_table.replace(
        task_rid,
        DenoChapterTask::new(crate::task::ChapterTaskType::Js(JsChapterTask::default())),
    );

    let task = try_json!(std::rc::Rc::try_unwrap(task)
        .map_err(|_| DenoError::resource_error(task_rid, "Cannot unwrap ChapterTask"))
        .to_result_json_borrow(state.clone()));

    #[derive(Clone)]
    pub struct ArcChapterTask {
        pub task: Arc<Mutex<JsChapterTask>>,
    }

    impl ChapterTask for ArcChapterTask {
        fn add(&mut self, image: ChapterImageInfo) {
            self.task.lock().add(image);
        }
    }

    match task.into_inner_type() {
        crate::task::ChapterTaskType::Trait(_) => {
            unreachable!("this should not be used in production")
        }
        crate::task::ChapterTaskType::Js(task) => {
            let task = ArcChapterTask {
                task: Arc::new(Mutex::new(task)),
            };

            let task_ = task.clone();
            let it = || async {
                module
                    .get_chapter_images(&id, Box::new(task_))
                    .await
                    .map_err(Into::into)
            };

            let it: Result<(), DenoError> = it().await;
            try_json!(it.to_result_json_borrow(state.clone()));

            let task = try_json!(Arc::try_unwrap(task.task)
                .map_err(|_| DenoError::resource_error(task_rid, "Cannot unwrap ArcChapterTask"))
                .to_result_json_borrow(state.clone()));

            state.borrow_mut().resource_table.replace(
                task_rid,
                DenoChapterTask::new(crate::task::ChapterTaskType::Js(task.into_inner())),
            );

            Ok(()).into()
        }
    }
}

#[deno_core::op]
async fn op_mado_module_close(state: Rc<RefCell<OpState>>, rid: u32) -> ResultJson<()> {
    let it: Rc<DenoMadoModule> = {
        let state = &mut state.borrow_mut();

        try_json!(state
            .resource_table
            .take(rid)
            .map_err(|_| DenoError::ResourceError(rid, "Module Already Closed".to_string()))
            .to_result_json(state))
    };

    let it = match Rc::try_unwrap(it) {
        Ok(it) => it,
        Err(_) => return ResultJson::Ok(()),
    };

    it.close()
        .await
        .map_err(DenoError::from)
        .to_result_json_borrow(state)
}

pub struct MadoCoreRequestBuilderResource(mado_core::RequestBuilder);
impl deno_core::Resource for MadoCoreRequestBuilderResource {}

#[deno_core::op]
async fn op_mado_module_download_image(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    chapter: ChapterImageInfo,
) -> ResultJson<u32> {
    let module = try_json!(get_module(state.clone(), rid));

    let it = try_json!(module
        .download_image(chapter)
        .await
        .map_err(DenoError::from)
        .to_result_json_borrow(state.clone())
        .into());

    let it = state
        .borrow_mut()
        .resource_table
        .add(MadoCoreRequestBuilderResource(it));

    ResultJson::Ok(it)
}

pub fn init() -> Extension {
    ExtensionBuilder::default()
        .ops(vec![
            op_mado_module_new::decl(),
            op_mado_module_get_info::decl(),
            op_mado_module_get_chapter_images::decl(),
            op_mado_module_download_image::decl(),
            op_mado_module_close::decl(),
        ])
        .build()
}

#[cfg(test)]
mod tests {}
