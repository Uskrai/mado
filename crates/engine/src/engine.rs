use std::sync::Arc;

use crate::{MadoEngineState, MadoEngineStateObserver, MadoModuleLoader};

pub struct MadoEngine<Loader>
where
    Loader: MadoModuleLoader + 'static + Send,
{
    loader: Option<Loader>,
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

        Self {
            loader: Some(loader),
            state,
        }
    }

    pub fn state(&self) -> Arc<MadoEngineState> {
        self.state.clone()
    }

    pub async fn run(mut self) {
        self.load_module().await;
        self.state.clone().connect(self);
    }

    fn load_module(&mut self) -> impl std::future::Future<Output = impl Send> + Send + 'static {
        let loader = self.loader.take();
        let state = self.state.clone();
        async move {
            let loader = loader?;

            let paths = loader.get_paths().await;

            for it in paths {
                match loader.load(it.clone()).await {
                    Ok(modules) => {
                        for it in modules {
                            state.push_module(it);
                        }
                    }
                    Err(err) => {
                        tracing::error!("error loading {}: {}", it.display(), err);
                    }
                }
            }

            Some(())
        }
    }

    fn download(
        &self,
        info: Arc<crate::DownloadInfo>,
    ) -> impl std::future::Future<Output = impl Send> + Send + 'static {
        async move {
            let module = info.wait_module().await;
            for it in info.chapters() {
                let (task, receiver) = crate::chapter::create(it.clone());

                let handler = tokio::spawn(receiver.run());
                module.get_chapter_images(Box::new(task)).await.unwrap();

                handler.await.unwrap();
            }
        }
    }
}

impl<Loader> MadoEngineStateObserver for MadoEngine<Loader>
where
    Loader: MadoModuleLoader + Send + 'static,
{
    fn on_push_module(&self, _: mado_core::ArcMadoModule) {}

    fn on_push_module_fail(&self, _: mado_core::MadoModuleMapError) {}

    fn on_download(&self, info: Arc<crate::DownloadInfo>) {
        tokio::spawn(self.download(info));
    }
}
