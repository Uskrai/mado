use std::{cell::RefCell, collections::HashMap, num::NonZeroI32, path::Path, rc::Rc, sync::Arc};

use anyhow::Context;
use deno_core::{
    v8::{self, Global, Local},
    JsRuntime, RuntimeOptions,
};
use event_listener::{Event, EventListener};
use futures::FutureExt;
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

    #[tracing::instrument(skip(self))]
    pub async fn load_file(&mut self, path: &Path) -> Result<i32, anyhow::Error> {
        let path = path.canonicalize().unwrap();
        let path = format!("file://{}", path.to_string_lossy());
        tracing::trace!("loading {}", path);
        let url = url::Url::parse(&path)?;

        let module = self
            .runtime
            .js
            .borrow_mut()
            .load_side_module(&url, None)
            .await?;

        if module > self.max_module() {
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
        self.runtime.load_object(object)
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

impl Default for Runtime {
    fn default() -> Self {
        let options = deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extensions: crate::extensions(),
            ..Default::default()
        };

        Self::new(options)
    }
}

impl Runtime {
    pub fn new(options: RuntimeOptions) -> Self {
        let event = Arc::new(event_listener::Event::new());
        let mut js = JsRuntime::new(options);

        js.handle_scope().set_promise_hook(promise_hook);

        let this = Self {
            js: Rc::new(RefCell::new(js)),
            event,
        };

        this.js
            .borrow_mut()
            .op_state()
            .borrow_mut()
            .put(this.clone());

        this
    }

    fn with_runtime<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&Self, &mut JsRuntime) -> R,
    {
        let runtime = &mut *self.js.borrow_mut();

        fun(self, runtime)
    }

    pub fn with_scope<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&mut v8::HandleScope) -> R,
    {
        self.with_runtime_scope(|_, scope| fun(scope))
    }

    pub fn with_runtime_scope<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&Runtime, &mut v8::HandleScope) -> R,
    {
        self.with_runtime(|this, runtime| {
            let scope = &mut runtime.handle_scope();
            fun(this, scope)
        })
    }

    pub fn with_state<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&mut deno_core::OpState) -> R,
    {
        self.with_runtime(|_, runtime| {
            let ops = runtime.op_state();
            let ops = &mut ops.borrow_mut();
            fun(ops)
        })
    }

    pub fn with_scope_state<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&mut v8::HandleScope, &mut deno_core::OpState) -> R,
    {
        self.with_runtime_scope_state(|_, scope, state| fun(scope, state))
    }

    pub fn with_runtime_scope_state<F, R>(&mut self, fun: F) -> R
    where
        F: FnOnce(&Runtime, &mut v8::HandleScope, &mut deno_core::OpState) -> R,
    {
        self.with_runtime(|this, runtime| {
            let ops = runtime.op_state();
            let ops = &mut ops.borrow_mut();
            let scope = &mut runtime.handle_scope();

            fun(this, scope, ops)
        })
    }

    pub fn load_object(
        &mut self,
        object: Global<v8::Object>,
    ) -> Result<(DenoMadoModule, ModuleLoop), ModuleLoadError> {
        self.with_runtime_scope_state(|this, scope, state| {
            this.load_object_with_scope_state(scope, state, object)
        })
    }

    pub fn load_object_with_scope_state(
        &self,
        scope: &mut v8::HandleScope,
        state: &mut deno_core::OpState,
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

        let value = {
            let object = v8::Local::new(scope, object.clone());
            crate::from_v8::<ObjectSerde>(scope, object.into()).map_err(ModuleLoadError::SerdeError)
        }?;

        let client = {
            state
                .resource_table
                .get::<crate::http::Client>(value.client.rid)
                .map_err(ModuleLoadError::WrongTypeError)
                .map(|it| it.client.clone())
        }?;

        let (cx, rx) = mpsc::channel(5);

        let sender = crate::DenoMadoModule::new(
            value.name,
            value.uuid,
            value.domain,
            client.clone().into(),
            cx,
        );

        let looper = crate::ModuleLoop::new(rx, self.clone(), object, client);

        Ok((sender, looper))
    }

    pub async fn resolve_value(
        &self,
        value: v8::Global<v8::Value>,
    ) -> Result<Global<v8::Value>, anyhow::Error> {
        let value = self.clone().with_scope(|scope| {
            let value = v8::Local::new(scope, value);

            let value = match v8::Local::<v8::Promise>::try_from(value) {
                Ok(promise) => {
                    let function = build_function(scope, fn_catch_callback);

                    let promise = promise.catch(scope, function).unwrap();
                    v8::Local::<v8::Value>::try_from(promise).unwrap()
                }
                Err(_) => value,
            };

            PROMISE_SPAN.with(|slot| {
                slot.borrow_mut().insert(
                    value.get_hash(),
                    TracingSpan::Span(tracing::Span::current()),
                );
            });

            v8::Global::new(scope, value)
        });

        ValueResolver::new(self, value).await
    }

    pub async fn with_event_loop<T>(&mut self, fut: impl std::future::Future<Output = T>) -> T {
        futures::pin_mut!(fut);

        loop {
            let it = futures::future::poll_fn(|cx| {
                self.with_runtime(|_, js| js.poll_event_loop(cx, false))
            });

            tokio::select! {
                _ = it => {}
                result = &mut fut => {
                    return result;
                }
            };
        }
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
    timer: async_io::Timer,
}

impl ValueResolver {
    pub fn new(runtime: &Runtime, value: v8::Global<v8::Value>) -> Self {
        Self {
            value,
            listener: Some(runtime.event.listen()),
            event: runtime.event.clone(),
            prev_err: None,
            js: runtime.js.clone(),
            timer: async_io::Timer::interval(std::time::Duration::from_millis(250)),
        }
    }
}

impl std::future::Future for ValueResolver {
    type Output = Result<Global<v8::Value>, anyhow::Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let _ = self.timer.poll_unpin(cx);

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
        let root_cx = cx;
        let cx = &mut std::task::Context::from_waker(&waker);

        let mut js = match this.js.try_borrow_mut() {
            Ok(js) => js,
            Err(_) => {
                root_cx.waker().wake_by_ref();
                return std::task::Poll::Pending;
            }
        };

        let poll = js.poll_value(&this.value, cx);

        // notify event when poll is ready so another poller will wait instead.
        if poll.is_ready() {
            cx.waker().wake_by_ref();
        }

        let result = futures::ready!(poll);

        match result {
            Ok(ok) => {
                cx.waker().wake_by_ref();

                std::task::Poll::Ready(Ok(ok))
            }
            Err(err) => {
                let it = err.downcast::<deno_core::error::JsError>();

                match it {
                    Ok(err) => std::task::Poll::Ready(Err(err.into())),
                    // probs related:
                    // https://github.com/denoland/deno/issues/15176
                    // https://github.com/denoland/deno/issues/13458
                    Err(err) => {
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

fn build_function<'s>(
    scope: &mut v8::HandleScope<'s>,
    callback: impl v8::MapFnTo<v8::FunctionCallback>,
) -> Local<'s, deno_core::v8::Function> {
    let function = v8::FunctionBuilder::new(callback);
    v8::FunctionBuilder::<v8::Function>::build(function, scope).unwrap()
}

fn fn_catch_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let errors = args.get(0);
    rv.set(errors);
}

pub enum TracingSpan {
    Span(tracing::Span),
    EnteredGuard(tracing::span::EnteredSpan),
}

std::thread_local! {
    static PROMISE_SPAN: RefCell<HashMap<NonZeroI32, TracingSpan>> = Default::default();
}

fn get_slot_span(
    hash: NonZeroI32,
    fun: impl FnOnce(&mut HashMap<NonZeroI32, TracingSpan>, tracing::Span),
) {
    PROMISE_SPAN.with(|slot| {
        let slot = &mut slot.borrow_mut();
        let span = match slot.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => match entry.remove() {
                TracingSpan::Span(span) => span,
                TracingSpan::EnteredGuard(guard) => {
                    let span = guard.exit();
                    slot.insert(hash, TracingSpan::EnteredGuard(span.clone().entered()));
                    span
                }
            },
            std::collections::hash_map::Entry::Vacant(_) => return,
        };

        fun(slot, span);
    })
}

extern "C" fn promise_hook(
    types: v8::PromiseHookType,
    current: v8::Local<v8::Promise>,
    parent: v8::Local<v8::Value>,
) {
    match types {
        v8::PromiseHookType::Init => {
            if !parent.is_null_or_undefined() {
                get_slot_span(parent.get_hash(), |slot, span| {
                    slot.insert(current.get_hash(), TracingSpan::Span(span));
                })
            }
        }
        v8::PromiseHookType::Before => get_slot_span(current.get_hash(), |slot, span| {
            slot.insert(
                current.get_hash(),
                TracingSpan::EnteredGuard(span.entered()),
            );
        }),
        v8::PromiseHookType::Resolve => get_slot_span(current.get_hash(), |slot, span| {
            slot.insert(current.get_hash(), TracingSpan::Span(span));
        }),
        v8::PromiseHookType::After => PROMISE_SPAN.with(|slot| {
            slot.borrow_mut().remove(&current.get_hash());
        }),
    }
}

pub enum RuntimeActorMsg {
    //
}

pub struct RuntimeActorSend {
    message: RuntimeActorMsg,
    span: tracing::Span,
}

pub struct RuntimeActor {
    sender: futures::channel::mpsc::Sender<RuntimeActorSend>,
}
