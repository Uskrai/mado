use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::{io::Write, sync::Arc, time::Duration};

use futures::Future;
use futures::{channel::mpsc, StreamExt};
use mado_core::{ArcMadoModule, ChapterImageInfo};

use crate::{path::Utf8PathBuf, DownloadChapterInfo};

/// retry_limit is the limit of retry each download will do until it return error.
/// each download will be retried/canceled if timeout is reached.
pub fn create(
    info: Arc<DownloadChapterInfo>,
    retry_limit: Arc<AtomicUsize>,
    timeout: Arc<AtomicU64>,
) -> (ChapterTask, ChapterTaskReceiver) {
    let (sender, recv) = mpsc::unbounded();

    let task = ChapterTask {
        sender,
        info: info.clone(),
    };

    let receiver = ChapterTaskReceiver {
        recv,
        info,
        limit: retry_limit,
        timeout,
    };

    (task, receiver)
}

/// Run future returned by fun until the future return Ok or limit return true.
/// limit will be called with retry count and after fun is awaited
#[inline]
pub async fn do_while_err_or_n<F, R, O, E, L>(mut limit: L, mut fun: F) -> Result<O, E>
where
    F: FnMut() -> R,
    R: Future<Output = Result<O, E>>,
    E: std::fmt::Display,
    L: FnMut(usize) -> bool,
{
    let mut retry = 0;
    let mut error;

    // using loop to simulate do_while
    loop {
        let result = fun().await;

        error = match result {
            Ok(ok) => return Ok(ok),
            Err(err) => err,
        };

        retry += 1;

        let retry_limit_reached = limit(retry);

        tracing::error!(
            "{}, {}",
            error,
            if retry_limit_reached {
                "Stopping..."
            } else {
                "Retrying..."
            }
        );

        // return last error if retry limit reached.
        if retry_limit_reached {
            break Err(error);
        }
    }
}

#[derive(Debug)]
pub struct ChapterTask {
    sender: mpsc::UnboundedSender<mado_core::ChapterImageInfo>,
    info: Arc<DownloadChapterInfo>,
}

struct ChapterImageTask {
    module: ArcMadoModule,
    image: ChapterImageInfo,
    limit: Arc<AtomicUsize>,
    timeout: Arc<AtomicU64>,
}

impl ChapterImageTask {
    fn new(
        module: ArcMadoModule,
        image: ChapterImageInfo,
        limit: Arc<AtomicUsize>,
        timeout: Arc<AtomicU64>,
    ) -> Self {
        Self {
            module,
            image,
            limit,
            timeout,
        }
    }

    #[tracing::instrument(
        level = "error",
        skip_all,
        fields(
            self.image = %self.image.id,
            self.module = %self.module.uuid()
        )
    )]
    pub async fn download(&self) -> Result<Vec<u8>, mado_core::Error> {
        do_while_err_or_n(
            |retry| self.limit.load(atomic::Ordering::Acquire) < retry,
            || async move {
                let mut buffer = Vec::new();
                self.download_without_retry(&mut buffer).await?;
                Ok(buffer)
            },
        )
        .await
    }

    async fn wait_timeout<F>(
        &self,
        future: F,
        duration: Duration,
    ) -> Result<F::Output, mado_core::Error>
    where
        F: Future,
    {
        let timeout = crate::timeout::timeout(duration, future);

        let result = timeout
            .await
            .map_err(|elapsed| mado_core::Error::ExternalError(elapsed.into()))?;

        Ok(result)
    }

    pub async fn download_without_retry<W>(&self, buffer: &mut W) -> Result<(), mado_core::Error>
    where
        W: Write,
    {
        let stream = self
            .module
            .download_image(self.image.clone())
            .await
            .unwrap();

        match stream {
            mado_core::BodyStream::Http(stream) => self.download_http(stream, buffer).await,
        }
    }

    pub async fn download_http<W>(
        &self,
        mut stream: mado_core::http::ResponseStream,
        buffer: &mut W,
    ) -> Result<(), mado_core::Error>
    where
        W: Write,
    {
        const BUFFER_SIZE: usize = 1024;
        let mut total = 0;

        let timeout = self.timeout.load(atomic::Ordering::Acquire);
        let timeout = Duration::from_secs(timeout.into());

        loop {
            let mut buf = vec![0u8; BUFFER_SIZE];
            let size = self.wait_timeout(stream.read(&mut buf), timeout).await??;

            let (buf, _) = buf.split_at(size);

            if buf.is_empty() {
                return Ok(());
            }

            total += size;
            tracing::trace!(
                "Writing {} bytes to buffer, total: {} bytes",
                buf.len(),
                total
            );

            buffer.write_all(buf)?;
        }
    }
}

#[derive(Debug)]
pub struct ChapterTaskReceiver {
    info: Arc<DownloadChapterInfo>,
    recv: mpsc::UnboundedReceiver<mado_core::ChapterImageInfo>,
    limit: Arc<AtomicUsize>,
    timeout: Arc<AtomicU64>,
}

impl ChapterTaskReceiver {
    pub async fn run(mut self) -> Result<(), mado_core::Error> {
        tracing::trace!("Start downloading chapter {:?}", self.info);
        let chapter_path = self.info.path();

        std::fs::create_dir_all(&chapter_path).unwrap();

        let mut i = 1;
        while let Some(image) = self.recv.next().await {
            let filename = format!("{:0>5}.{}", i, image.extension.clone());
            let path = chapter_path.join(filename);

            self.download_image(path, image).await?;

            i += 1;
        }
        self.info.set_status(crate::DownloadStatus::Finished);
        tracing::trace!("Finished downloading chapter {:?}", self.info);

        Ok(())
    }

    fn download_image(
        &self,
        path: Utf8PathBuf,
        image: ChapterImageInfo,
    ) -> impl Future<Output = Result<(), mado_core::Error>> {
        let mut module = self.info.module();
        let limit = self.limit.clone();
        let timeout = self.timeout.clone();

        async move {
            let module = module.wait().await;
            let exists = path.exists();

            if !exists {
                let task = ChapterImageTask::new(module.clone(), image.clone(), limit, timeout);

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

impl mado_core::ChapterTask for ChapterTask {
    fn add(&mut self, image: mado_core::ChapterImageInfo) {
        tracing::trace!("Sending image info {:?}", image);
        self.sender.unbounded_send(image).ok();
    }

    fn get_chapter_id(&self) -> &str {
        self.info.chapter_id()
    }
}
