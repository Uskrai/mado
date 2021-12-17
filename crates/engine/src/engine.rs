use std::sync::Arc;

use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    watch,
};

use crate::{DownloadStatus, MadoEngineState, MadoEngineStateObserver, MadoModuleLoader};

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

            fn on_push_module_fail(&self, _: mado_core::MadoModuleMapError) {}

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
        DownloadTask::new(info).run()
    }
}

impl Default for MadoEngine {
    fn default() -> Self {
        Self::new()
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
        let mut status = {
            let (tx_status, rx_status) = watch::channel(self.info.status());

            let sender = DownloadTaskSender { status: tx_status };
            self.info.connect(Arc::new(sender));

            rx_status
        };

        loop {
            self.wait_status(&mut status, DownloadStatus::Resumed).await;
            let paused = self.wait_status(&mut status, DownloadStatus::Paused);
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
                    break;
                }
                Err(err) => {
                    tracing::error!("{}", err);
                }
            }
        }
    }

    async fn wait_status(&self, rx: &mut watch::Receiver<DownloadStatus>, status: DownloadStatus) {
        loop {
            if *rx.borrow_and_update() == status {
                return;
            }

            if rx.changed().await.is_err() {
                return;
            }
        }
    }

    async fn download(&self) -> Result<(), mado_core::Error> {
        let module = self.info.wait_module().await;
        for it in self.info.chapters() {
            let (task, receiver) = crate::chapter::create(it.clone());

            let receiver = receiver.run();
            let task = module.get_chapter_images(Box::new(task));

            tokio::try_join!(task, receiver)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct DownloadTaskSender {
    status: watch::Sender<DownloadStatus>,
}
impl crate::DownloadInfoObserver for DownloadTaskSender {
    fn on_status_changed(&self, status: crate::DownloadStatus) {
        self.status.send_replace(status);
    }
}
