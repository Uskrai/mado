use std::sync::Arc;

use event_listener::Event;
use futures::{channel::mpsc, FutureExt, SinkExt, StreamExt};

use crate::{DownloadChapterImageInfo, DownloadChapterInfo, DownloadResumedStatus, DownloadStatus};

pub use super::*;

pub struct TaskDownloader {
    info: Arc<crate::DownloadInfo>,
}

impl TaskDownloader {
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
        let _ = self.info.wait_module().await;
        self.info
            .set_status(DownloadStatus::resumed(DownloadResumedStatus::Downloading));
        for it in self.info.chapters() {
            self.download_chapter(it.clone()).await?;
        }
        self.info.set_status(DownloadStatus::Finished);
        Ok(())
    }

    async fn download_chapter(&self, it: Arc<DownloadChapterInfo>) -> Result<(), mado_core::Error> {
        if it.status().is_completed() {
            return Ok(());
        }

        const RETRY_LIMIT: usize = 10;
        const TIMEOUT: u64 = 10;
        let retry = Arc::new(RETRY_LIMIT.into());
        let timeout = Arc::new(TIMEOUT.into());

        let (download_tx, download_rx) = mpsc::channel(1);
        let downloader = ChapterDownloader::new(download_rx, it.clone(), retry, timeout);

        let get_images = self.get_chapter_images(&it, download_tx);

        futures::try_join!(get_images, downloader.run())?;

        it.set_status(DownloadStatus::Finished);

        Ok(())
    }

    pub async fn get_chapter_images(
        &self,
        it: &DownloadChapterInfo,
        tx: mpsc::Sender<Arc<DownloadChapterImageInfo>>,
    ) -> Result<(), crate::core::Error> {
        let module = self.info.wait_module().await;
        let mut download_tx = tx;

        let (image_tx, mut image_rx) = chapter_task_channel();
        let get_images = module.get_chapter_images(it.chapter_id(), Box::new(image_tx));

        let mut i = 1;

        let receiver = async {
            while let Some(image) = image_rx.next().await {
                let filename = format!("{:0>5}.{}", i, image.extension);
                let path = it.path().join(filename);

                let image = DownloadChapterImageInfo::new(image, path);
                let image = Arc::new(image);

                download_tx
                    .send(image)
                    .await
                    .map_err(|e| crate::core::Error::ExternalError(e.into()))?;

                i += 1;
            }

            Ok(())
        };

        futures::try_join!(get_images, receiver)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct DownloadTaskSender {
    info: Arc<crate::DownloadInfo>,
    event: Arc<Event>,
}

impl DownloadTaskSender {
    pub fn connect(info: Arc<crate::DownloadInfo>) -> Self {
        let event = Arc::new(Event::new());

        info.connect(DownloadTaskNotifier(event.clone()));
        Self {
            info: info.clone(),
            event: event.clone(),
        }
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

#[derive(Debug)]
pub struct DownloadTaskNotifier(Arc<Event>);
impl crate::DownloadInfoObserver for DownloadTaskNotifier {
    fn on_status_changed(&self, _: &DownloadStatus) {
        self.0.notify(usize::MAX);
    }
}

fn chapter_task_channel() -> (ChapterTaskSender, ChapterTaskReceiver) {
    let (tx, rx) = mpsc::unbounded();

    let tx = ChapterTaskSender { tx };

    (tx, rx)
}

type ChapterTaskReceiver = mpsc::UnboundedReceiver<crate::core::ChapterImageInfo>;

struct ChapterTaskSender {
    tx: mpsc::UnboundedSender<crate::core::ChapterImageInfo>,
}

impl crate::core::ChapterTask for ChapterTaskSender {
    fn add(&mut self, image: mado_core::ChapterImageInfo) {
        self.tx.unbounded_send(image).ok();
    }
}
