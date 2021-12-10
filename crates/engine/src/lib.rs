mod engine;
pub use engine::*;

use std::{fmt::Debug, sync::Arc};

use futures::FutureExt;
use tokio::sync::{
    mpsc::{error::SendError, UnboundedReceiver, UnboundedSender},
    Mutex,
};

use mado_core::{ArcMadoModule, ChapterInfo, MangaInfo};

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

pub trait MadoSender: Send + Sync + Debug {
    fn push_module(&self, module: ArcMadoModule);

    fn create_download_view(&self, download: Arc<DownloadInfo>, controller: DownloadSender);
}

#[derive(Debug)]
pub enum MadoDownloadMsg {
    Start(Box<dyn DownloadViewController>),
}

#[derive(Debug, Clone)]
pub struct DownloadSender {
    sender: UnboundedSender<MadoDownloadMsg>,
    start: UnboundedSender<Box<dyn DownloadViewController>>,
}

pub struct DownloadReceiver {
    recv: UnboundedReceiver<MadoDownloadMsg>,
    start: UnboundedReceiver<Box<dyn DownloadViewController>>,
}

fn download_channel() -> (DownloadSender, DownloadReceiver) {
    let (sender, recv) = tokio::sync::mpsc::unbounded_channel();
    let (start_sender, start_recv) = tokio::sync::mpsc::unbounded_channel();
    let sender = DownloadSender {
        sender,
        start: start_sender,
    };

    let recv = DownloadReceiver {
        recv,
        start: start_recv,
    };

    (sender, recv)
}

pub trait DownloadViewController: Send + Sync + Debug + 'static {
    //
}

impl DownloadSender {
    pub fn start(
        &self,
        view: impl DownloadViewController,
    ) -> Result<(), SendError<Box<dyn DownloadViewController>>> {
        self.start.send(Box::new(view))
    }

    pub fn resume(&self) {
        //
    }

    pub fn pause(&self) {
        //
    }
}

impl DownloadReceiver {
    pub async fn await_start(&mut self) -> Box<dyn DownloadViewController> {
        let controller = self.start.recv().await.unwrap();
        self.start.close();
        controller
    }

    pub async fn recv(&mut self) -> Option<MadoDownloadMsg> {
        self.recv.recv().await
    }
}

#[derive(Debug)]
pub struct DownloadInfo {
    pub module: ArcMadoModule,
    pub manga: Arc<MangaInfo>,
    pub chapters: Vec<Arc<ChapterInfo>>,
    pub path: std::path::PathBuf,
}

#[derive(Debug)]
pub enum MadoMsg {
    Start(Arc<dyn MadoSender>),
    Download(DownloadInfo),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
