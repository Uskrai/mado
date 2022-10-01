use std::{cell::RefCell, path::Path, rc::Rc, sync::Arc};

use anyhow::Context;
use deno_core::{
    v8::{self, Global, Local},
    JsRuntime, RuntimeOptions,
};
use event_listener::{Event, EventListener};
use mado_core::Uuid;
use tokio::sync::mpsc;

use crate::{DenoMadoModule, ModuleLoop};

pub struct ModuleLoader {
    runtime: Runtime,
    max_module: i32,
}
impl ModuleLoader {
    pub fn new(options: RuntimeOptions) -> Self {
        Self {
            runtime: Runtime::new(options),
            max_module: 0,
        }
    }

    pub fn from_runtime(runtime: Runtime) -> Self {
        Self {
            runtime,
            max_module: 0,
        }
    }

    pub fn max_module(&self) -> i32 {
        self.max_module
    }

    pub async fn load_file(&mut self, path: &Path) -> Result<i32, anyhow::Error> {
        let path = path.canonicalize().unwrap();
        let url = url::Url::parse(&format!("file://{}", path.to_string_lossy()))?;

        let module = self
            .runtime
            .js
            .borrow_mut()
            .load_side_module(&url, None)
            .await?;

        if module > self.max_module {
            let _receiver = self.runtime.js.borrow_mut().mod_evaluate(module);
            self.runtime.js.borrow_mut().run_event_loop(false).await?;
            _receiver.await??;
            self.max_module = module;
        }

        Ok(module)
    }

    pub async fn init_module(
        &mut self,
        module: i32,
    ) -> Result<Vec<Result<(DenoMadoModule, ModuleLoop), ModuleLoadError>>, ModuleLoadError> {
        let (array, length) = {
            let namespace = self.runtime.js.borrow_mut().get_module_namespace(module);
            self.runtime.with_scope(|scope| {
                let name = v8::String::new(scope, "initMadoModule").unwrap();
                let null = v8::null(scope);

                namespace
                    .map(|it| Local::new(scope, it))
                    .and_then(|it| {
                        it.get(scope, name.into())
                            .context("initMadoModule doesn't exists")
                    })
                    .and_then(|it| {
                        Local::<v8::Function>::try_from(it)
                            .context("initMadoModule isn't a function")
                    })
                    .and_then(|it| {
                        it.call(scope, null.into(), &[])
                            .context("initMadoModule return None")
                    })
                    .and_then(|it| {
                        Local::<v8::Array>::try_from(it)
                            .context("initMadoModule doesn't return an array")
                    })
                    .map(|it| (it, it.length()))
                    .map(|it| (Global::new(scope, it.0), it.1))
                    .map_err(ModuleLoadError::NotModule)
            })?
        };

        let mut vec = vec![];

        for index in 0..length {
            let value = {
                self.runtime.with_scope(|scope| {
                    let array = array.open(scope);
                    let value = array
                        .get_index(scope, index)
                        .and_then(|it| Local::<v8::Object>::try_from(it).ok())
                        .map(|it| Global::new(scope, it));

                    value
                })
            };

            if let Some(value) = value {
                let it = self.load_object(value);
                vec.push(it);
            }
        }

        Ok(vec)
    }

    pub fn load_object(
        &mut self,
        object: Global<v8::Object>,
    ) -> Result<(DenoMadoModule, ModuleLoop), ModuleLoadError> {
        #[derive(serde::Deserialize)]
        struct ClientSerde {
            rid: u32,
        }

        #[derive(serde::Deserialize)]
        struct ObjectSerde {
            name: String,
            domain: url::Url,
            uuid: Uuid,
            client: ClientSerde,
        }

        let value = self.runtime.with_scope(|scope| {
            let object = v8::Local::new(scope, object.clone());
            crate::from_v8::<ObjectSerde>(scope, object.into()).map_err(ModuleLoadError::SerdeError)
        })?;

        let client = self.runtime.with_state(|state| {
            state
                // .borrow_mut()
                .resource_table
                .get::<crate::http::Client>(value.client.rid)
                .map_err(ModuleLoadError::WrongTypeError)
                .map(|it| it.client.clone())
        })?;

        let (cx, rx) = mpsc::channel(5);

        let sender = crate::DenoMadoModule::new(
            value.name,
            value.uuid,
            value.domain,
            client.clone().into(),
            cx,
        );

        let looper = crate::ModuleLoop::new(rx, self.runtime.clone(), object, client);

        Ok((sender, looper))
    }

    pub fn into_runtime(self) -> Runtime {
        self.runtime
    }
}

/// wrapper for Rc<RefCell<JsRuntime>>
#[derive(Clone)]
pub struct Runtime {
    js: Rc<RefCell<JsRuntime>>,
    event: Arc<event_listener::Event>,
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleLoadError {
    #[error("{0}")]
    NotModule(anyhow::Error),
    #[error("{0}")]
    SerdeError(anyhow::Error),

    #[error("{0}")]
    WrongTypeError(anyhow::Error),
}

impl Runtime {
    pub fn new(options: RuntimeOptions) -> Self {
        let event = Arc::new(event_listener::Event::new());

        Self {
            js: Rc::new(RefCell::new(JsRuntime::new(options))),
            event,
        }
    }

    pub fn with_scope<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut v8::HandleScope) -> R,
    {
        let mut runtime = self.js.borrow_mut();
        let scope = &mut runtime.handle_scope();

        fun(scope)
    }

    pub fn with_state<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut deno_core::OpState) -> R,
    {
        let mut runtime = self.js.borrow_mut();
        let ops = runtime.op_state();
        let ops = &mut ops.borrow_mut();
        // let scope = &mut runtime.handle_scope();

        fun(ops)
    }

    pub fn with_scope_state<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut v8::HandleScope, &mut deno_core::OpState) -> R,
    {
        let mut runtime = self.js.borrow_mut();
        let ops = runtime.op_state();
        let ops = &mut ops.borrow_mut();
        let scope = &mut runtime.handle_scope();

        fun(scope, ops)
    }

    pub async fn resolve_value(
        &self,
        value: v8::Global<v8::Value>,
    ) -> Result<Global<v8::Value>, anyhow::Error> {
        ValueResolver::new(self, value).await
    }

    // pub async fn resolve_value_catch(&self, value: v8::Global<v8::Value>) {
    //     self.with_scope(|scope| {
    //         let it = value.open(scope);
    //         let function = v8::Function::new(scope, fn_catch_callback);
    //
    //         if let Ok(function)
    //     });
    // }

    pub fn js(&self) -> &RefCell<JsRuntime> {
        self.js.as_ref()
    }
}

// https://github.com/denoland/deno/issues/13458
struct ValueResolver {
    value: v8::Global<v8::Value>,
    listener: Option<EventListener>,
    event: Arc<Event>,
    prev_err: Option<anyhow::Error>,
    js: Rc<RefCell<JsRuntime>>,
}

impl ValueResolver {
    pub fn new(runtime: &Runtime, value: v8::Global<v8::Value>) -> Self {
        Self {
            value,
            listener: Some(runtime.event.listen()),
            event: runtime.event.clone(),
            prev_err: None,
            js: runtime.js.clone(),
        }
    }
}

impl std::future::Future for ValueResolver {
    type Output = Result<Global<v8::Value>, anyhow::Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let event = self.event.clone();
        let this = &mut *self;

        {
            // keep listening even after its ready.
            let mut poll = std::task::Poll::Ready(());

            while poll.is_ready() {
                let listen = this
                    // .get_mut()
                    .listener
                    .get_or_insert_with(|| this.event.listen());
                futures::pin_mut!(listen);
                poll = listen.poll(cx);

                if poll.is_ready() {
                    this.listener = None;
                }
            }
        };

        // use event.notify as waker instead of passed cx
        let waker = waker_fn::waker_fn(move || {
            event.notify(1);
        });
        let cx = &mut std::task::Context::from_waker(&waker);

        let mut js = this.js.borrow_mut();

        let poll = js.poll_value(&this.value, cx);

        // notify event when poll is ready so another poller will wait instead.
        if poll.is_ready() {
            this.event.notify(1);
        }

        let result = futures::ready!(poll);

        match result {
            Ok(ok) => std::task::Poll::Ready(Ok(ok)),
            Err(err) => {
                let it = err.downcast::<deno_core::error::JsError>();

                match it {
                    Ok(err) => std::task::Poll::Ready(Err(err.into())),
                    // probs related:
                    // https://github.com/denoland/deno/issues/15176
                    Err(err) => {
                        //
                        // match this.prev_err {
                        //     Some(_) => std::task::Poll::Ready(Err(err)),
                        //     None => {
                        cx.waker().wake_by_ref();
                        this.prev_err = Some(err);

                        dbg!(&this.prev_err);
                        std::task::Poll::Pending
                    }
                }
            }
        }
    }
}

// fn fn_catch_callback(
//     scope: &mut v8::HandleScope,
//     args: v8::FunctionCallbackArguments,
//     mut rv: v8::ReturnValue,
// ) {
//     let errors = args.get(0);
//     rv.set(errors);
// }
