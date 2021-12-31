use std::future::Future;
use std::io::Write;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize},
    Arc,
};

use futures::{channel::mpsc, StreamExt};

use crate::ImageDownloader;

use crate::{DownloadChapterImageInfo, DownloadChapterInfo};

pub struct ChapterDownloader {
    rx: mpsc::Receiver<Arc<DownloadChapterImageInfo>>,
    info: Arc<DownloadChapterInfo>,
    retry_limit: Arc<AtomicUsize>,
    timeout: Arc<AtomicU64>,
}

impl ChapterDownloader {
    pub fn new(
        rx: mpsc::Receiver<Arc<DownloadChapterImageInfo>>,
        info: Arc<DownloadChapterInfo>,
        retry_limit: Arc<AtomicUsize>,
        timeout: Arc<AtomicU64>,
    ) -> Self {
        Self {
            rx,
            info,
            retry_limit,
            timeout,
        }
    }

    pub async fn run(mut self) -> Result<(), mado_core::Error> {
        tracing::trace!("Start downloading chapter {:?}", self.info);
        let chapter_path = self.info.path();

        std::fs::create_dir_all(&chapter_path).unwrap();

        while let Some(image) = self.rx.next().await {
            self.download_image(image).await?;
        }

        self.info.set_status(crate::DownloadStatus::Finished);
        tracing::trace!("Finished downloading chapter {:?}", self.info);

        Ok(())
    }

    fn download_image(
        &self,
        image: Arc<DownloadChapterImageInfo>,
    ) -> impl Future<Output = Result<(), mado_core::Error>> {
        let mut module = self.info.module();
        let limit = self.retry_limit.clone();
        let timeout = self.timeout.clone();

        async move {
            let path = image.path();
            let image = image.image();

            let module = module.wait().await;
            let exists = path.exists();

            if !exists {
                let task = ImageDownloader::new(module.clone(), image.clone(), limit, timeout);

                tracing::trace!("Start downloading {} {:?}", path, image);

                let buffer = task.download().await?;

                tracing::trace!("Finished downloading {} {:?}", path, image);
                let mut file = std::fs::File::create(&path).unwrap();
                file.write_all(&buffer)?;
                tracing::trace!("Finished writing to {}", path);
            } else {
                tracing::trace!("File {} already exists, skipping...", path);
            }
            Ok(())
        }
    }
}
