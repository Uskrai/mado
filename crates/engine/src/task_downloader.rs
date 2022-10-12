use std::{
    io::Write,
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        Arc,
    },
};

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
        let status = DownloadInfoWatcher::connect(self.info.clone());
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

            if let Err(err) = result {
                tracing::error!("{}", err);
                self.info.set_status(DownloadStatus::error(err));
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

        let (download_tx, mut download_rx) = mpsc::channel(1);

        let image_downloader = async {
            while let Some(image) = download_rx.next().await {
                self.download_image(image).await?;
            }

            Ok(())
        };

        let get_images = self.get_chapter_images(&it, download_tx);

        futures::try_join!(get_images, image_downloader)?;

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
                let filename = format!("{:0>4}.{}", i, image.extension);
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

    pub async fn download_image(
        &self,
        image: Arc<DownloadChapterImageInfo>,
    ) -> Result<(), mado_core::Error> {
        let module = self.info.wait_module().await;
        let path = image.path();
        let image = image.image();
        let exists = path.exists();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        const RETRY_LIMIT: usize = 10;
        const TIMEOUT: u64 = 10;
        let retry = Arc::new(RETRY_LIMIT.into());
        let timeout = Arc::new(TIMEOUT.into());

        struct Config(Arc<AtomicUsize>, Arc<AtomicU64>);
        impl crate::ImageDownloaderConfig for Config {
            type Buffer = Vec<u8>;

            fn should_retry(&self, retry_count: usize) -> bool {
                retry_count < self.0.load(atomic::Ordering::Relaxed)
            }

            fn timeout(&self) -> std::time::Duration {
                std::time::Duration::from_secs(self.1.load(atomic::Ordering::Relaxed))
            }

            fn buffer(&self) -> Self::Buffer {
                Vec::new()
            }
        }

        if !exists {
            let task = ImageDownloader::new(module.clone(), image.clone(), Config(retry, timeout));

            tracing::trace!("Start downloading {} {:?}", path, image);

            let buffer = task.download().await?;

            tracing::trace!("Finished downloading {} {:?}", path, image);
            let mut file = std::fs::File::create(path).unwrap();
            file.write_all(&buffer)?;
            tracing::trace!("Finished writing to {}", path);
        } else {
            tracing::trace!("File {} already exists, skipping...", path);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct DownloadInfoWatcher {
    info: Arc<crate::DownloadInfo>,
    event: Arc<Event>,
}

impl DownloadInfoWatcher {
    pub fn connect(info: Arc<crate::DownloadInfo>) -> Self {
        let event = Arc::new(Event::new());

        info.connect({
            let event = event.clone();

            move |_| {
                event.notify(usize::MAX);
            }
        });

        Self { info, event }
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
