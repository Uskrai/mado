use anyhow::Context;
use mado_core::ArcMadoModule;
use mado_engine::{MadoEngine, MadoModuleLoader, ModuleLoadError};
use relm4::RelmApp;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

use std::sync::Arc;

pub struct Loader;
#[async_trait::async_trait]
impl MadoModuleLoader for Loader {
    async fn get_paths(&self) -> Vec<std::path::PathBuf> {
        let mut dir = tokio::fs::read_dir("../rune/script").await.unwrap();

        let mut paths = Vec::new();
        loop {
            let it = dir.next_entry().await;
            match it {
                Ok(Some(it)) => {
                    if it.path().is_file() {
                        paths.push(it.path());
                    } else {
                        continue;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    tracing::error!("error loading: {}", err);
                    continue;
                }
            };
        }

        paths
    }

    async fn load(
        &self,
        path: std::path::PathBuf,
    ) -> Result<Vec<mado_core::ArcMadoModule>, ModuleLoadError> {
        let result = tokio::task::spawn_blocking(move || load_module(&path))
            .await
            .unwrap();

        result.map_err(Into::into)
    }
}

pub fn load_module(path: &std::path::Path) -> Result<Vec<ArcMadoModule>, ModuleLoadError> {
    let build = mado_rune::Build::default().with_path(path)?;

    let vec = build
        .build_for_module()
        .with_context(|| format!("Error builiding {}", path.display()))?
        .error_missing_load_module(false)
        .build()
        .map_err(anyhow::Error::from)?;

    let mut result = Vec::<ArcMadoModule>::with_capacity(vec.len());
    for it in vec {
        result.push(Arc::new(it));
    }

    Ok(result)
}

pub fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .init();

    let mado = MadoEngine::new(Loader);
    let model = mado_relm::AppModel::new(mado.state());

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let _guard = runtime.enter();
    tokio::spawn(async move {
        mado.run().await;
    });

    let app = RelmApp::new(model);
    app.run();
}
