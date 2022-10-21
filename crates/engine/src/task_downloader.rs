use std::{
    io::Write,
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        Arc,
    },
};

use event_listener::Event;
use futures::{channel::mpsc, FutureExt, StreamExt};

use crate::{DownloadChapterImageInfo, DownloadChapterInfo, DownloadStatus};

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

            let paused = status
                .wait_status(DownloadStatus::is_paused)
                .map(|_| Ok::<(), mado_core::Error>(()));

            let dl = self.download();

            futures::pin_mut!(dl, paused);

            let (result, _) = futures::future::select(dl, paused).await.factor_first();

            if let Err(err) = result {
                tracing::error!("{}", err);
                self.info.set_status(DownloadStatus::error(err));
            }
        }
    }

    async fn download(&self) -> Result<(), mado_core::Error> {
        let _ = self.info.wait_module().await;
        self.info.set_status(DownloadStatus::downloading());
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

        let mut get_images = self.get_chapter_images(it.clone()).await;

        let mut images = vec![];
        while let Some(image) = get_images.next().await {
            let image = image?;
            self.download_image(image.clone()).await?;
            images.push(image);
            it.set_images(images.clone());
        }

        it.set_status(DownloadStatus::Finished);

        Ok(())
    }

    pub async fn get_chapter_images(
        &self,
        it: Arc<DownloadChapterInfo>,
    ) -> impl futures::Stream<Item = Result<Arc<DownloadChapterImageInfo>, mado_core::Error>> + 'static
    {
        let module = self.info.wait_module().await;

        let (image_tx, image_rx) = chapter_task_channel();

        let chapter_id = it.chapter_id().to_string();

        let mut get_images = async move {
            module
                .get_chapter_images(&chapter_id, Box::new(image_tx))
                .await
        }
        .fuse()
        .boxed();

        let mut stream = image_rx.enumerate().map(move |(i, image)| {
            let i = i + 1;
            let filename = format!("{:0>4}.{}", i, image.extension);
            let path = it.path().join(filename);

            let image = DownloadChapterImageInfo::new(image, path, it.status().clone());
            Ok(Arc::new(image))
        });

        futures::stream::poll_fn(move |cx| {
            if let std::task::Poll::Ready(Err(err)) = get_images.as_mut().poll(cx) {
                std::task::Poll::Ready(Some(Err(err)))
            } else {
                stream.poll_next_unpin(cx)
            }
        })
    }

    pub async fn download_image(
        &self,
        image: Arc<DownloadChapterImageInfo>,
    ) -> Result<(), mado_core::Error> {
        let module = self.info.wait_module().await;
        self.info
            .set_status(DownloadStatus::resumed(DownloadResumedStatus::Downloading));

        let path = image.path();
        let image = image.image();
        let exists = path.exists();

        const RETRY_LIMIT: usize = 10;
        const TIMEOUT: u64 = 10;
        let retry = Arc::new(RETRY_LIMIT.into());
        let timeout = Arc::new(TIMEOUT.into());

        if !exists {
            let task = ImageDownloader::new(module.clone(), image.clone(), Config(retry, timeout));

            tracing::trace!("Start downloading {} {:?}", path, image);

            let buffer = task.download().await?;

            tracing::trace!("Finished downloading {} {:?}", path, image);

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut file = std::fs::File::create(path).unwrap();
            file.write_all(&buffer)?;
            self.info.set_status(DownloadStatus::finished());
            tracing::trace!("Finished writing to {}", path);
        } else {
            tracing::trace!("File {} already exists, skipping...", path);
        }
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use camino::Utf8PathBuf;
    use futures::StreamExt;
    use httpmock::Method::GET;
    use mado_core::{ChapterImageInfo, DefaultMadoModuleMap, MockMadoModule, Uuid};
    use mockall::predicate::{always, eq};

    use crate::{
        tests::server_url, DownloadChapterInfo, DownloadInfo, DownloadStatus, LateBindingModule,
        TaskDownloader,
    };

    use super::DownloadInfoWatcher;

    // TODO: improve this test to not use fs
    #[test]
    fn download_image_test() {
        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(1));

        let firstinfo = ChapterImageInfo {
            id: "1".to_string(),
            extension: "png".to_string(),
            name: Some("1.png".to_string()),
        };

        let i1 = firstinfo.clone();
        module
            .expect_get_chapter_images()
            .with(eq("1"), always())
            .returning(move |_, mut a| {
                a.add(firstinfo.clone());
                Ok(())
            });

        let mock = httpmock::MockServer::start();
        let h = mock.mock(|when, then| {
            when.path("/test").method(GET);
            then.body("testtest");
        });

        let client = mado_core::http::Client::default();
        let url = server_url(h.server_address()).join("/test").unwrap();

        module
            .expect_download_image()
            .with(eq(i1))
            .returning(move |_| Ok(client.get(url.clone()).into()));

        let module = Arc::new(module);

        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let chapter = Arc::new(DownloadChapterInfo::new(
            module.clone().into(),
            "1".to_string(),
            "title".to_string(),
            path.join("1"),
            DownloadStatus::waiting(),
        ));
        let info = Arc::new(DownloadInfo::new(
            module.into(),
            "title".to_string(),
            vec![chapter],
            path.clone(),
            None,
            DownloadStatus::waiting(),
        ));

        futures::executor::block_on(async {
            let downloader = TaskDownloader::new(info.clone());

            let status = DownloadInfoWatcher::connect(info.clone());

            let fut = status.wait_status(|s| !s.is_resumed());

            let runner = downloader.run();

            futures::pin_mut!(fut);
            futures::pin_mut!(runner);
            let _ = futures::future::select(fut, runner).await;

            assert_eq!(*info.status(), DownloadStatus::finished());
        });

        assert_eq!(
            std::fs::read_to_string(path.join("1").join("0001.png")).unwrap(),
            "testtest"
        );

        assert_eq!(info.chapters().len(), 1);
        assert_eq!(info.chapters()[0].images().len(), 1);
        assert_eq!(
            info.chapters()[0].images()[0].path(),
            path.join("1").join("0001.png")
        );
        temp.close().unwrap();
    }

    #[test]
    fn get_chapter_test() {
        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(1));

        let firstinfo = ChapterImageInfo {
            id: "1".to_string(),
            extension: "png".to_string(),
            name: Some("1.png".to_string()),
        };

        let i1 = firstinfo.clone();
        module
            .expect_get_chapter_images()
            .with(eq("1"), always())
            .returning(move |_, mut a| {
                a.add(firstinfo.clone());

                Ok(())
            });

        let module = Arc::new(module);
        let chapter = Arc::new(DownloadChapterInfo::new(
            module.clone().into(),
            "1".to_string(),
            "title".to_string(),
            Default::default(),
            DownloadStatus::waiting(),
        ));
        let info = Arc::new(DownloadInfo::new(
            module.into(),
            "title".to_string(),
            vec![chapter.clone()],
            Default::default(),
            None,
            DownloadStatus::waiting(),
        ));

        let downloader = TaskDownloader::new(info);

        futures::executor::block_on(async {
            let mut it = downloader.get_chapter_images(chapter).await.enumerate();
            while let Some((i, image)) = it.next().await {
                match i {
                    0 => {
                        assert_eq!(*image.unwrap().image(), i1);
                    }
                    1 => {
                        image.unwrap_err();
                    }
                    _ => unreachable!(),
                }
            }
        });
    }

    #[test]
    fn watcher_test() {
        let map = DefaultMadoModuleMap::default();

        let module = LateBindingModule::WaitModule(Arc::new(map), Uuid::from_u128(1));
        let info = DownloadInfo::new(
            module,
            "title".to_string(),
            vec![],
            Default::default(),
            None,
            crate::DownloadStatus::error("Error"),
        );

        let info = Arc::new(info);
        let watcher = DownloadInfoWatcher::connect(info.clone());

        futures::executor::block_on(async {
            let future = watcher.wait_status(DownloadStatus::is_paused);
            crate::timer::timeout(std::time::Duration::from_millis(10), future)
                .await
                .unwrap_err();

            let future = watcher.wait_status(DownloadStatus::is_completed);
            info.resume(true);
            assert!(info.status().is_resumed());
            assert!(!info.status().is_completed());
            crate::timer::timeout(std::time::Duration::from_millis(10), future)
                .await
                .unwrap_err();

            let future = watcher.wait_status(DownloadStatus::is_resumed);
            crate::timer::timeout(std::time::Duration::from_millis(10), future)
                .await
                .unwrap();
        });
    }
}
