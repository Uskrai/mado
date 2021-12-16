use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::{MadoEngineState, MadoEngineStateObserver, MadoModuleLoader};

pub struct MadoEngine {
    state: Arc<MadoEngineState>,
}

const _: () = {
    fn assert<T: Send + Sync>() {}

    fn assert_all() {
        assert::<MadoEngine>();
    }
};

#[derive(Debug)]
pub enum MadoEngineMsg {
    Download(Arc<crate::DownloadInfo>),
}

impl MadoEngine {
    pub fn new() -> Self {
        let state = Arc::new(MadoEngineState::default());

        Self { state }
    }

    pub fn state(&self) -> Arc<MadoEngineState> {
        self.state.clone()
    }

    pub async fn run(self) {
        let (sender, mut recv) = tokio::sync::mpsc::unbounded_channel();
        self.state.connect(MadoEngineSender(sender));

        while let Some(msg) = recv.recv().await {
            match msg {
                MadoEngineMsg::Download(info) => {
                    tokio::spawn(self.download(info));
                }
            }
        }
    }

    pub fn load_module(
        &self,
        loader: impl MadoModuleLoader + 'static,
    ) -> impl std::future::Future<Output = impl Send> + Send + 'static {
        let state = self.state.clone();
        async move {
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

                let receiver = async move {
                    receiver.run().await;
                    Ok(())
                };
                let task = module.get_chapter_images(Box::new(task));

                tokio::try_join!(task, receiver).unwrap();
            }
        }
    }
}

impl Default for MadoEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MadoEngineSender(UnboundedSender<MadoEngineMsg>);

impl MadoEngineStateObserver for MadoEngineSender {
    fn on_push_module(&self, _: mado_core::ArcMadoModule) {}

    fn on_push_module_fail(&self, _: mado_core::MadoModuleMapError) {}

    fn on_download(&self, info: Arc<crate::DownloadInfo>) {
        self.0.send(MadoEngineMsg::Download(info)).unwrap();
    }
}
