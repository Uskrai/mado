use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{DownloadInfo, MadoEngineState, MadoModuleLoader, MadoMsg, MadoSender};

pub struct MadoEngine {
    run: Mutex<()>,
    loader: Mutex<Box<dyn MadoModuleLoader + Send>>,

    state: Arc<MadoEngineState>,
    recv: Mutex<Receiver>,
}

pub type Sender = tokio::sync::mpsc::UnboundedSender<MadoMsg>;
pub type Receiver = tokio::sync::mpsc::UnboundedReceiver<MadoMsg>;

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
        let (sender, recv) = tokio::sync::mpsc::unbounded_channel();
        let state = Arc::new(MadoEngineState::new(sender));
        let recv = Mutex::new(recv);

        Self {
            loader: Mutex::new(Box::new(loader)),
            run: Default::default(),
            state,
            recv,
        }
    }

    pub fn state(&self) -> Arc<MadoEngineState> {
        self.state.clone()
    }

    fn create_download(
        &self,
        download: DownloadInfo,
        sender: Arc<dyn MadoSender>,
    ) -> impl std::future::Future<Output = ()> {
        async move {
            let (download_sender, mut recv) = crate::download_channel();

            let download = Arc::new(download);
            sender.create_download_view(download.clone(), download_sender);

            let _ = recv.await_start().await;

            for _ in &download.chapters {
                //
            }

            while let Some(msg) = recv.recv().await {
                match msg {
                    crate::MadoDownloadMsg::Start(_) => {}
                }
            }
        }
    }

    /// Run Event lopo.
    pub async fn run(self) {
        let sender = self.await_sender().await.unwrap();
        let _guard = self.run.lock().await;

        self.load_module(sender.clone()).await;
        self.event_loop(sender).await;
    }

    async fn recv(&self) -> Option<MadoMsg> {
        self.recv.lock().await.recv().await
    }

    async fn event_loop(&self, sender: Arc<dyn MadoSender>) {
        let mut recv = self.recv.lock().await;
        while let Some(msg) = recv.recv().await {
            match msg {
                MadoMsg::Start(_) => {
                    tracing::warn!("Multiple MadoMsg::Start will be ignored");
                }

                MadoMsg::Download(download) => {
                    let sender = sender.clone();
                    let downloader = self.create_download(download, sender.clone());
                    tokio::task::spawn(downloader);
                }
            }
        }
    }

    async fn await_sender(&self) -> Option<Arc<dyn MadoSender + 'static>> {
        while let Some(msg) = self.recv().await {
            if let MadoMsg::Start(sender) = msg {
                return Some(sender);
            }
        }
        None
    }

    async fn load_module(&self, sender: Arc<dyn MadoSender>) {
        let loader = self.loader.lock().await;

        let paths = loader.get_paths().await;

        for it in paths {
            match loader.load(it.clone()).await {
                Ok(modules) => {
                    for it in modules {
                        sender.push_module(it);
                    }
                }
                Err(err) => {
                    tracing::error!("error loading {}: {}", it.display(), err);
                }
            }
        }
    }
}
