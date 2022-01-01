use std::{future::Future, time::Duration};

use futures::{AsyncWrite, AsyncWriteExt};
use mado_core::{ArcMadoModule, ChapterImageInfo};

pub trait ImageDownloaderConfig {
    type Buffer: AsyncWrite + Unpin;

    fn should_retry(&self, retry_count: usize) -> bool;
    fn timeout(&self) -> Duration;

    fn buffer(&self) -> Self::Buffer;
}

/// Run future returned by fun until the future return Ok or should_retry return false.
/// limit will be called with retry count and after fun is awaited
#[inline]
pub async fn do_while_err_or<F, R, O, E, L>(mut fun: F, mut should_retry: L) -> Result<O, E>
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
            Ok(ok) => break Ok(ok),
            Err(err) => err,
        };

        retry += 1;

        let stop = should_retry(retry);

        tracing::error!(
            "{}, {}",
            error,
            if stop { "Stopping..." } else { "Retrying..." }
        );

        if stop {
            break Err(error);
        }
    }
}

pub struct ImageDownloader<C> {
    module: ArcMadoModule,
    image: ChapterImageInfo,
    config: C,
}

impl<C> ImageDownloader<C>
where
    C: ImageDownloaderConfig,
{
    pub fn new(module: ArcMadoModule, image: ChapterImageInfo, config: C) -> Self {
        Self {
            module,
            image,
            config,
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
    pub async fn download(self) -> Result<C::Buffer, mado_core::Error> {
        do_while_err_or(
            || async {
                let mut buffer = self.config.buffer();

                self.download_without_retry(&mut buffer).await?;

                Ok(buffer)
            },
            |retry| self.config.should_retry(retry),
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
        let timeout = crate::timer::timeout(duration, future);

        let result = timeout
            .await
            .map_err(|elapsed| mado_core::Error::ExternalError(elapsed.into()))?;

        Ok(result)
    }

    pub async fn download_without_retry(
        &self,
        buffer: &mut C::Buffer,
    ) -> Result<(), mado_core::Error> {
        let request = self
            .module
            .download_image(self.image.clone())
            .await
            .unwrap();

        match request {
            mado_core::RequestBuilder::Http(request) => self.download_http(request, buffer).await,
        }
    }

    pub async fn download_http(
        &self,
        request: mado_core::http::RequestBuilder,
        buffer: &mut C::Buffer,
    ) -> Result<(), mado_core::Error> {
        const BUFFER_SIZE: usize = 1024;
        let mut total = 0;
        let timeout = self.config.timeout();

        let response = request.send().await?;
        let mut stream = response.stream();

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

            buffer.write_all(buf).await?;
        }
    }
}
