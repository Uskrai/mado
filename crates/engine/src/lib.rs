#![allow(dead_code, unused_variables, unused_imports)]

mod engine;
pub use engine::*;

use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::{
    mpsc::{error::SendError, UnboundedSender},
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

pub trait MadoSender: Send + Sync + std::fmt::Debug {
    fn push_module(&self, module: ArcMadoModule);

    fn create_download_view(&self, download: Arc<DownloadInfo>, controller: DownloadController);
}

pub enum MadoDownloadMsg {
    Start(Box<dyn DownloadView>),
}

pub struct DownloadController {
    sender: UnboundedSender<MadoDownloadMsg>,
}

pub trait DownloadView: Send + Sync + 'static {
    //
}

impl DownloadController {
    pub fn start(&self, view: impl DownloadView) -> Result<(), SendError<MadoDownloadMsg>> {
        self.sender.send(MadoDownloadMsg::Start(Box::new(view)))
    }

    pub fn resume(&self) {
        //
    }

    pub fn pause(&self) {
        //
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
