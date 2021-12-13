use std::{io::Write, path::PathBuf, sync::Arc, time::Duration};

use futures::{Future, StreamExt};
use mado_core::{ArcMadoModule, ChapterImageInfo, ChapterInfo};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub fn create(
    chapter: Arc<ChapterInfo>,
    module: ArcMadoModule,
) -> (ChapterTask, ChapterTaskReceiver) {
    let (sender, recv) = tokio::sync::mpsc::unbounded_channel();

    let task = ChapterTask {
        sender,
        chapter: chapter.clone(),
    };

    let receiver = ChapterTaskReceiver {
        module,
        recv,
        chapter,
    };

    (task, receiver)
}

#[derive(Debug)]
pub struct ChapterTask {
    sender: UnboundedSender<mado_core::ChapterImageInfo>,
    chapter: Arc<ChapterInfo>,
}

struct ChapterImageTask {
    module: ArcMadoModule,
    image: ChapterImageInfo,
}

impl ChapterImageTask {
    fn new(module: ArcMadoModule, image: ChapterImageInfo) -> Self {
        Self { module, image }
    }

    #[tracing::instrument(
        level = "error",
        skip_all,
        fields(
            self.image = %self.image.id,
            self.module = %self.module.get_uuid()
        )
    )]
    pub async fn download(&self) -> Result<Vec<u8>, mado_core::Error> {
        let mut retry = 0;
        const RETRY_LIMIT: u32 = 10;
        let mut error;

        // using loop to simulate do_while
        loop {
            let mut buffer = Vec::new();
            let result = self.download_without_retry(&mut buffer).await;

            let err = match result {
                Ok(_) => return Ok(buffer),
                Err(err) => err,
            };

            error = err;
            retry += 1;

            let retry_limit_reached = retry >= RETRY_LIMIT;

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
                return Err(error);
            }
        }
    }

    async fn wait_timeout<F>(
        &self,
        future: F,
        duration: Duration,
    ) -> Result<F::Output, mado_core::Error>
    where
        F: Future,
    {
        let timeout = tokio::time::timeout(duration, future);

        let result = timeout
            .await
            .map_err(|elapsed| mado_core::Error::ExternalError(elapsed.into()))?;

        Ok(result)
    }

    pub async fn download_without_retry<W>(&self, buffer: &mut W) -> Result<(), mado_core::Error>
    where
        W: Write,
    {
        let mut stream = self
            .module
            .download_image(self.image.clone())
            .await
            .unwrap();

        // TODO: make timeout dynamic
        const TIMEOUT: u64 = 10;
        let timeout = Duration::from_secs(TIMEOUT);

        while let Some(bytes) = self.wait_timeout(stream.next(), timeout).await? {
            let bytes = bytes?;
            tracing::trace!("Writing {} bytes to buffer", bytes.len());
            buffer.write_all(&bytes)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ChapterTaskReceiver {
    module: ArcMadoModule,
    chapter: Arc<ChapterInfo>,
    recv: UnboundedReceiver<mado_core::ChapterImageInfo>,
}

impl ChapterTaskReceiver {
    pub async fn run(mut self) {
        tracing::trace!("Start downloading chapter {:?}", self.chapter);
        let title = self
            .chapter
            .title
            .clone()
            .unwrap_or_else(|| "0000".to_string());

        let chapter_path = PathBuf::from(title.clone());

        std::fs::create_dir_all(&chapter_path).unwrap();

        let mut i = 0;
        while let Some(image) = self.recv.recv().await {
            let filename = format!("{:0>5}.{}", i, image.extension.clone());
            let path = chapter_path.join(filename);

            self.download_image(path, image).await.unwrap();

            i += 1;
        }
        tracing::trace!("Finished downloading chapter {:?}", self.chapter);
    }

    fn download_image(
        &self,
        path: PathBuf,
        image: ChapterImageInfo,
    ) -> impl Future<Output = Result<(), mado_core::Error>> {
        let module = self.module.clone();
        async move {
            let exists = path.exists();

            if !exists {
                let task = ChapterImageTask::new(module.clone(), image.clone());

                tracing::trace!("Start downloading {} {:?}", path.display(), image);

                let buffer = task.download().await?;

                tracing::trace!("Finished downloading {} {:?}", path.display(), image);
                let mut file = std::fs::File::create(&path).unwrap();
                file.write_all(&buffer)?;
                tracing::trace!("Finished writing to {}", path.display());
            } else {
                tracing::trace!("File {} already exists, skipping...", path.display());
            }
            Ok(())
        }
    }
}

impl mado_core::ChapterTask for ChapterTask {
    fn add(&mut self, image: mado_core::ChapterImageInfo) {
        tracing::trace!("Sending image info {:?}", image);
        self.sender.send(image).unwrap();
    }

    fn get_chapter(&self) -> &ChapterInfo {
        &self.chapter
    }
}
