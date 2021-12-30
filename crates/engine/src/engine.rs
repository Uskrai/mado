use std::sync::Arc;

use event_listener::Event;
use futures::{channel::mpsc, FutureExt, StreamExt};

use crate::{
    DownloadResumedStatus, DownloadStatus, MadoEngineState, MadoEngineStateObserver,
    MadoModuleLoader,
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
        pub struct MadoEngineSender(mpsc::UnboundedSender<MadoEngineMsg>);

        impl MadoEngineStateObserver for MadoEngineSender {
            fn on_push_module(&self, _: mado_core::ArcMadoModule) {}

            fn on_download(&self, info: Arc<crate::DownloadInfo>) {
                self.0.unbounded_send(MadoEngineMsg::Download(info)).ok();
            }
        }

        let (tx, rx) = mpsc::unbounded();
        self.state.connect(MadoEngineSender(tx));
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
            let task = DownloadTask::new(info.clone());
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

#[derive(Debug)]
pub enum DownloadTaskMsg {
    Status(crate::DownloadStatus),
}

pub struct DownloadTask {
    info: Arc<crate::DownloadInfo>,
}

impl DownloadTask {
    pub fn new(info: Arc<crate::DownloadInfo>) -> Self {
        Self { info }
    }

    pub async fn run(self) {
        let status = DownloadTaskSender::connect(self.info.clone());
        loop {
            status.wait_status(DownloadStatus::is_resumed).await;

            let paused = status.wait_status(DownloadStatus::is_paused).fuse();
            let dl = self.download().fuse();

            futures::pin_mut!(dl, paused);

            let result = futures::select! {
                _ = paused => {
                    continue;
                }
                r = dl => {
                    r
                }
            };

            match result {
                Ok(_) => {
                    self.info.set_status(DownloadStatus::Finished);
                    continue;
                }
                Err(err) => {
                    tracing::error!("{}", err);
                    self.info.set_status(DownloadStatus::error(err));
                }
            }
        }
    }

    async fn download(&self) -> Result<(), mado_core::Error> {
        let module = self.info.wait_module().await;
        self.info
            .set_status(DownloadStatus::resumed(DownloadResumedStatus::Downloading));
        for it in self.info.chapters() {
            if it.status().is_completed() {
                continue;
            }

            const RETRY_LIMIT: usize = 10;
            const TIMEOUT: u64 = 10;
            let retry = Arc::new(RETRY_LIMIT.into());
            let timeout = Arc::new(TIMEOUT.into());

            let (task, receiver) = crate::chapter::create(it.clone(), retry, timeout);

            let receiver = receiver.run();
            let task = module.get_chapter_images(it.chapter_id(), Box::new(task));

            futures::try_join!(task, receiver)?;
            it.set_status(DownloadStatus::Finished);
        }
        self.info.set_status(DownloadStatus::Finished);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DownloadTaskSender {
    info: Arc<crate::DownloadInfo>,
    event: Event,
}

impl DownloadTaskSender {
    pub fn connect(info: Arc<crate::DownloadInfo>) -> Arc<Self> {
        let event = Event::new();
        let this = Arc::new(Self {
            info: info.clone(),
            event,
        });

        info.connect(this.clone());
        this
    }

    pub async fn wait_status(&self, fun: impl Fn(&DownloadStatus) -> bool) {
        loop {
            if fun(&self.info.status()) {
                return;
            }

            self.event.listen().await;
        }
    }
}

impl crate::DownloadInfoObserver for DownloadTaskSender {
    fn on_status_changed(&self, _: &DownloadStatus) {
        self.event.notify(usize::MAX);
    }
}
