use std::sync::Arc;

use futures::{channel::mpsc, FutureExt, StreamExt};

use crate::{
    DownloadStatus, MadoEngineState, MadoEngineStateMsg, MadoModuleLoader,
    {TaskRunner, TaskScheduler},
};

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
        let mut rx = self.connect_state();

        let option = self.state().option().scheduler();
        let (scheduler, runner) = TaskScheduler::connect(self.state.clone(), option);

        let runner = Arc::new(runner);

        let scheduler = scheduler.run();

        let rx = async move {
            while let Some(msg) = rx.next().await {
                match msg {
                    MadoEngineMsg::Download(info) => {
                        tokio::spawn(self.download(runner.clone(), info));
                    }
                }
            }
        };

        let _ = futures::future::join(rx, scheduler).await;
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
        scheduler: Arc<TaskRunner>,
        info: Arc<crate::DownloadInfo>,
    ) -> impl std::future::Future<Output = impl Send> + Send + 'static {
        let state = self.state();
        async move {
            loop {
                let option = state.option();
                let it = std::panic::AssertUnwindSafe(scheduler.run(info.clone(), |info| {
                    crate::TaskDownloader::new(info.clone(), option.clone())
                }))
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
