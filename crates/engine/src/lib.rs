#![allow(dead_code, unused_variables, unused_imports)]

mod engine;

use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::Mutex;

use mado_core::ArcMadoModule;

mod state;
pub use state::MadoEngineState;

/// Error happen when Loading Module.
#[derive(Debug, thiserror::Error)]
pub enum ModuleLoadError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),
    #[error("{0}")]
    ExternalError(#[from] anyhow::Error),
}

/// Traits to Load [`crate::MadoModule`]
#[async_trait::async_trait]
pub trait MadoModuleLoader: Send + Sync {
    async fn get_paths(&self) -> Vec<std::path::PathBuf>;
    async fn load(
        &self,
        path: std::path::PathBuf,
    ) -> Result<Vec<ArcMadoModule>, crate::ModuleLoadError>;
}

pub trait MadoSender: Send + Sync + std::fmt::Debug {
    fn push_module(&self, module: ArcMadoModule);
}

#[derive(Debug)]
pub enum MadoMsg {
    Start(Arc<dyn MadoSender>),
}

pub struct MadoEngine {
    run: Mutex<()>,
    loader: Mutex<Box<dyn MadoModuleLoader + Send>>,

    state: Arc<MadoEngineState>,
    recv: Mutex<Receiver>,
}

pub type Sender = tokio::sync::mpsc::UnboundedSender<MadoMsg>;
pub type Receiver = tokio::sync::mpsc::UnboundedReceiver<MadoMsg>;

const _: () = {
    fn assert<T: Send + Sync>() {}

    fn assert_all() {
        assert::<MadoEngine>();
    }
};

pub struct DownloadInfo;

impl MadoEngine {
    pub fn new<Loader>(loader: Loader) -> Self
    where
        Loader: MadoModuleLoader + 'static,
    {
        let (sender, recv) = tokio::sync::mpsc::unbounded_channel();
        let state = Arc::new(MadoEngineState::new(sender));
        let recv = Mutex::new(recv);

        Self {
            loader: Mutex::new(Box::new(loader)),
            run: Default::default(),
            state,
            recv,
        }
    }

    pub fn state(&self) -> Arc<MadoEngineState> {
        self.state.clone()
    }

    pub fn download(download: DownloadInfo) {
        //
    }

    /// Run Event lopo.
    pub async fn run(self) {
        let sender = self.await_sender().await.unwrap();
        let guard = self.run.lock().await;
        let mut loader_fut = self.load_module(sender.clone()).boxed().fuse();

        futures::select! {
            loader = loader_fut => {
                println!("");
            }
        };
    }

    async fn await_sender(&self) -> Option<Arc<dyn MadoSender + 'static>> {
        while let Some(msg) = self.recv.lock().await.recv().await {
            if let MadoMsg::Start(sender) = msg {
                return Some(sender);
            }
        }
        None
    }

    async fn load_module(&self, sender: Arc<dyn MadoSender>) {
        let loader = self.loader.lock().await;

        let paths = loader.get_paths().await;

        for it in paths {
            match loader.load(it.clone()).await {
                Ok(modules) => {
                    for it in modules {
                        sender.push_module(it);
                    }
                }
                Err(err) => {
                    tracing::error!("error loading {}: {}", it.display(), err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
