use anyhow::Context;
use mado_core::ArcMadoModule;
use mado_engine::{
    path::Utf8PathBuf, MadoEngine, MadoEngineState, MadoModuleLoader, ModuleLoadError,
};
use relm4::RelmApp;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

use std::sync::Arc;

pub struct Loader;
#[async_trait::async_trait]
impl MadoModuleLoader for Loader {
    async fn get_paths(&self) -> Vec<Utf8PathBuf> {
        let mut dir = tokio::fs::read_dir("../rune/script").await.unwrap();

        let mut paths = Vec::new();
        loop {
            let it = dir.next_entry().await;
            match it {
                Ok(Some(it)) => {
                    if it.path().is_file() {
                        let it = Utf8PathBuf::from_path_buf(it.path());
                        match it {
                            Ok(it) => paths.push(it),
                            Err(it) => tracing::error!("{:?} is not a valid utf8 path", it),
                        }
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
        path: Utf8PathBuf,
    ) -> Result<Vec<mado_core::ArcMadoModule>, ModuleLoadError> {
        let result = tokio::task::spawn_blocking(move || load_module(&path))
            .await
            .unwrap();

        result.map_err(Into::into)
    }
}

pub fn load_module(path: &Utf8PathBuf) -> Result<Vec<ArcMadoModule>, ModuleLoadError> {
    let build = mado_rune::Build::default().with_path(path.as_std_path())?;

    let vec = build
        .build_for_module()
        .with_context(|| format!("Error builiding {}", path))?
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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();

    let state = MadoEngineState::new(Default::default(), Vec::new());
    let mado = MadoEngine::new(state);
    let model = mado_relm::AppModel::new(mado.state());

    tokio::spawn(mado.load_module(Loader));
    tokio::spawn(mado.run());

    let app = RelmApp::new(model);
    app.run();
}
