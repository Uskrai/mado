use std::sync::Arc;

use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    watch,
};

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
        let mut rx = self.connect_state();

        while let Some(msg) = rx.recv().await {
            match msg {
                MadoEngineMsg::Download(info) => {
                    tokio::spawn(self.download(info));
                }
            }
        }
    }

    pub fn connect_state(&self) -> UnboundedReceiver<MadoEngineMsg> {
        pub struct MadoEngineSender(UnboundedSender<MadoEngineMsg>);

        impl MadoEngineStateObserver for MadoEngineSender {
            fn on_push_module(&self, _: mado_core::ArcMadoModule) {}

            fn on_download(&self, info: Arc<crate::DownloadInfo>) {
                self.0.send(MadoEngineMsg::Download(info)).unwrap();
            }
        }

        let (sender, recv) = tokio::sync::mpsc::unbounded_channel();
        self.state.connect(MadoEngineSender(sender));
        recv
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
        DownloadTask::new(info).run()
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

            let paused = status.wait_status(DownloadStatus::is_paused);
            let dl = self.download();
            let result = tokio::select! {
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

            let (task, receiver) = crate::chapter::create(it.clone());

            let receiver = receiver.run();
            let task = module.get_chapter_images(Box::new(task));

            tokio::try_join!(task, receiver)?;
            it.set_status(DownloadStatus::Finished);
        }
        self.info.set_status(DownloadStatus::Finished);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DownloadTaskSender {
    info: Arc<crate::DownloadInfo>,
    status: watch::Sender<()>,
}

impl DownloadTaskSender {
    pub fn connect(info: Arc<crate::DownloadInfo>) -> Arc<Self> {
        let (status, _) = watch::channel(());
        let this = Arc::new(Self {
            info: info.clone(),
            status,
        });

        info.connect(this.clone());
        this
    }

    pub async fn wait_status(&self, fun: impl Fn(&DownloadStatus) -> bool) {
        let mut rx = self.status.subscribe();
        loop {
            rx.borrow_and_update();
            if fun(&self.info.status()) {
                return;
            }

            if rx.changed().await.is_err() {
                return;
            }
        }
    }
}

impl crate::DownloadInfoObserver for DownloadTaskSender {
    fn on_status_changed(&self, _: &DownloadStatus) {
        self.status.send_replace(());
    }
}
