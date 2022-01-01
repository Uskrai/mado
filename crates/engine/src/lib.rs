mod chapter_downloader;
mod data;
mod image_downloader;
mod task_downloader;
pub mod timer;
pub use data::*;
mod engine;
pub use engine::*;

pub use chapter_downloader::ChapterDownloader;
pub use image_downloader::{ImageDownloader, ImageDownloaderConfig};
pub use task_downloader::TaskDownloader;

mod state;
pub use state::{MadoEngineState, MadoEngineStateObserver};

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
    async fn get_paths(&self) -> Vec<path::Utf8PathBuf>;
    async fn load(
        &self,
        path: path::Utf8PathBuf,
    ) -> Result<Vec<ArcMadoModule>, crate::ModuleLoadError>;
}

use crate::core::ArcMadoModule;
pub use mado_core as core;

pub mod path {
    pub use camino::Utf8Path;
    pub use camino::Utf8PathBuf;
}
