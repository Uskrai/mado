use std::sync::Arc;

use crate::{MadoEngineState, MadoModuleLoader};

pub struct MadoEngine<Loader>
where
    Loader: MadoModuleLoader + 'static + Send,
{
    loader: Loader,
    state: Arc<MadoEngineState>,
}

const _: () = {
    fn assert<T: Send + Sync>() {}

    fn assert_all<Loader>()
    where
        Loader: MadoModuleLoader + Send + 'static,
    {
        assert::<MadoEngine<Loader>>();
    }
};

impl<Loader> MadoEngine<Loader>
where
    Loader: MadoModuleLoader + Send + 'static,
{
    pub fn new(loader: Loader) -> Self {
        let state = Arc::new(MadoEngineState::default());

        Self { loader, state }
    }

    pub fn state(&self) -> Arc<MadoEngineState> {
        self.state.clone()
    }

    pub async fn run(self) {
        self.load_module().await;
    }

    async fn load_module(&self) {
        let loader = &self.loader;

        let paths = loader.get_paths().await;

        for it in paths {
            match loader.load(it.clone()).await {
                Ok(modules) => {
                    for it in modules {
                        let state = self.state.clone();
                        state.push_module(it);
                    }
                }
                Err(err) => {
                    tracing::error!("error loading {}: {}", it.display(), err);
                }
            }
        }
    }
}
