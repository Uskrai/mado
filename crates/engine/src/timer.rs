use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub use async_io::Timer;

#[derive(Debug, thiserror::Error)]
#[error("timeout reached")]
pub struct Elapsed;

pub fn timeout<F>(duration: Duration, f: F) -> Timeout<F>
where
    F: std::future::Future,
{
    let timer = Timer::after(duration);

    Timeout { timer, future: f }
}

pub async fn sleep(duration: Duration) {
    Timer::after(duration).await;
}

#[pin_project::pin_project]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Timeout<F>
where
    F: std::future::Future,
{
    #[pin]
    future: F,
    #[pin]
    timer: Timer,
}

impl<F> std::future::Future for Timeout<F>
where
    F: std::future::Future,
{
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();

        if let Poll::Ready(result) = me.future.poll(cx) {
            return Poll::Ready(Ok(result));
        }

        if me.timer.poll(cx).is_ready() {
            return Poll::Ready(Err(Elapsed));
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_test() {
        let secs = Duration::from_secs(1);
        let nanos = Duration::from_nanos(1);
        futures::executor::block_on(async move {
            timeout(secs, async { sleep(nanos).await }).await.unwrap();
            timeout(nanos, async { sleep(secs).await })
                .await
                .unwrap_err();

            timeout(
                Duration::from_millis(10),
                futures::future::poll_fn(|cx| {
                    cx.waker().wake_by_ref();
                    Poll::<()>::Pending
                }),
            )
            .await
            .unwrap_err();
        });
    }
}
