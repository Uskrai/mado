use std::sync::Arc;

use crate::{MadoEngineState, MadoModuleLoader};

pub struct MadoEngine {
    loader: Box<dyn MadoModuleLoader + Send>,
    state: Arc<MadoEngineState>,
}

const _: () = {
    fn assert<T: Send + Sync>() {}

    fn assert_all() {
        assert::<MadoEngine>();
    }
};

impl MadoEngine {
    pub fn new<Loader>(loader: Loader) -> Self
    where
        Loader: MadoModuleLoader + 'static,
    {
        let state = Arc::new(MadoEngineState::default());

        Self {
            loader: Box::new(loader),
            state,
        }
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
