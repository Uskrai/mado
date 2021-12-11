mod data;
pub use data::*;
mod engine;
pub use engine::*;

use mado_core::ArcMadoModule;

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
    async fn get_paths(&self) -> Vec<std::path::PathBuf>;
    async fn load(
        &self,
        path: std::path::PathBuf,
    ) -> Result<Vec<ArcMadoModule>, crate::ModuleLoadError>;
}
