use std::sync::Arc;

use futures::{channel::mpsc, FutureExt, StreamExt};

use crate::{DownloadStatus, MadoEngineState, MadoEngineStateMsg, MadoModuleLoader};

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
    pub fn new(state: MadoEngineState) -> Self {
        let state = Arc::new(state);

        Self { state }
    }

    pub fn state(&self) -> Arc<MadoEngineState> {
        self.state.clone()
    }

    pub async fn run(self) {
        let rx = self.connect_state();

        rx.for_each(|msg| async {
            match msg {
                MadoEngineMsg::Download(info) => {
                    tokio::spawn(self.download(info));
                }
            }
        })
        .await;
    }

    pub fn connect_state(&self) -> mpsc::UnboundedReceiver<MadoEngineMsg> {
        let (tx, rx) = mpsc::unbounded();

        self.state.connect({
            move |msg| match msg {
                MadoEngineStateMsg::Download(info) => {
                    tx.unbounded_send(MadoEngineMsg::Download(info.clone()))
                        .ok();
                }
                MadoEngineStateMsg::PushModule(_) => {}
            }
        });
        rx
    }

    pub fn load_module(
        &self,
        loader: impl MadoModuleLoader + 'static,
    ) -> impl std::future::Future<Output = impl Send> + Send + 'static {
        let state = self.state.clone();
        async move {
            let paths = loader.get_paths().await;

            for path in paths {
                match loader.load(path.clone()).await {
                    Ok(modules) => {
                        for it in modules {
                            if let Err(err) = state.push_module(it) {
                                tracing::error!("error pushing {}: {}", path, err);
                            }
                        }
                    }
                    Err(err) => {
                        tracing::error!("error loading {}: {}", path, err);
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
            loop {
                let task = crate::TaskDownloader::new(info.clone());
                let it = std::panic::AssertUnwindSafe(task.run())
                    .catch_unwind()
                    .await;

                if let Err(e) = it {
                    if let Some(e) = e.downcast_ref::<&str>() {
                        info.set_status(DownloadStatus::error(e));
                    } else if let Some(e) = e.downcast_ref::<String>() {
                        info.set_status(DownloadStatus::error(e));
                    } else {
                        info.set_status(DownloadStatus::error("Cannot dechiper panic error!"));
                    }
                }
            }
        }
    }
}
