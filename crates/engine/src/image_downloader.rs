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

        let stop = !should_retry(retry);

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

async fn wait_timeout<F>(future: F, duration: Duration) -> Result<F::Output, mado_core::Error>
where
    F: Future,
{
    let timeout = crate::timer::timeout(duration, future);

    let result = timeout
        .await
        .map_err(|elapsed| mado_core::Error::ExternalError(elapsed.into()))?;

    Ok(result)
}

pub async fn download_http<Buffer>(
    request: mado_core::http::RequestBuilder,
    buffer: &mut Buffer,
    mut timeout: impl FnMut() -> Duration,
) -> Result<(), mado_core::Error>
where
    Buffer: AsyncWrite + Unpin,
{
    const BUFFER_SIZE: usize = 1024;
    let mut total = 0;

    let response = request.send().await?;
    let mut stream = response.stream();

    loop {
        let mut buf = vec![0u8; BUFFER_SIZE];
        let size = wait_timeout(stream.read(&mut buf), timeout()).await??;

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
            mado_core::RequestBuilder::Http(request) => {
                download_http(request, buffer, || self.config.timeout()).await
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{
        net::SocketAddr,
        pin::Pin,
        sync::{atomic::AtomicUsize, Arc},
    };

    use parking_lot::Mutex;

    use super::*;

    #[test]
    fn retry_test() {
        futures::executor::block_on(async {
            let i = Arc::new(AtomicUsize::new(0));
            let set = |val| {
                i.store(val, atomic::Ordering::Relaxed);
            };
            let get = || i.load(atomic::Ordering::Relaxed);

            do_while_err_or(
                || async move {
                    if get() == 1 {
                        Ok(())
                    } else {
                        set(1);
                        Err("")
                    }
                },
                |retry| retry <= 1,
            )
            .await
            .unwrap();
            assert_eq!(get(), 1);

            set(0);
            const RETRY: usize = 10;

            do_while_err_or(
                || async {
                    set(get() + 1);
                    Result::<(), &str>::Err("")
                },
                |retry| retry < RETRY,
            )
            .await
            .unwrap_err();

            assert_eq!(get(), RETRY);

            do_while_err_or(|| async { Ok::<_, &str>(()) }, |_| unreachable!())
                .await
                .unwrap();
        });
    }

    #[derive(Default, Clone)]
    pub struct MutexVec(Arc<Mutex<Vec<u8>>>);
    impl futures::io::AsyncWrite for MutexVec {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            Pin::new(&mut *self.0.lock()).poll_write(cx, buf)
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            Pin::new(&mut *self.0.lock()).poll_flush(cx)
        }

        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            Pin::new(&mut *self.0.lock()).poll_close(cx)
        }
    }

    impl std::fmt::Display for MutexVec {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", String::from_utf8_lossy(&self.0.lock()))
        }
    }

    use httpmock::prelude::*;

    fn server_url(socket: &SocketAddr) -> mado_core::url::Url {
        mado_core::url::Url::try_from(format!("http://{}", socket).as_str()).unwrap()
    }

    #[test]
    fn download_test() {
        let mut buffer = MutexVec::default();

        let server = MockServer::start();

        let buff = buffer.clone();
        let _m = server.mock(move |when, then| {
            when.path("/test");

            then.body_stream(move || {
                let buff = buff.clone();
                futures::stream::unfold(0, move |state| {
                    let buff = buff.clone();
                    async move {
                        let val = match state {
                            0 => "test",
                            1 => {
                                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                                assert_eq!(buff.to_string(), "test");
                                "test"
                            }
                            _ => return None,
                        };

                        Some((Ok(val), state + 1))
                    }
                })
            });
        });

        let client = mado_core::http::Client::default();
        let server_url = server_url(_m.server_address());

        futures::executor::block_on(async {
            let request = client.get(server_url.join("/test").unwrap());
            download_http(request, &mut buffer, || {
                std::time::Duration::from_millis(20)
            })
            .await
            .unwrap();
            assert_eq!(buffer.to_string(), "testtest");
        });
    }

    #[test]
    fn timeout_test() {
        let mut buffer = MutexVec::default();

        let server = httpmock::MockServer::start();

        let _m = server.mock(move |when, then| {
            when.path("/timeout").method(GET);

            then.body_stream(move || {
                futures::stream::unfold(0, move |state| async move {
                    let val = match state {
                        0 => "t",
                        _ => {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                            return None;
                        }
                    };

                    Some((Ok(val), state + 1))
                })
            });
        });

        let server_url = server_url(_m.server_address());
        let client = mado_core::http::Client::default();

        futures::executor::block_on(async {
            let request = client.get(server_url.join("/timeout").unwrap());
            download_http(request, &mut buffer, || std::time::Duration::from_millis(2))
                .await
                .unwrap_err();
        });
    }
}
