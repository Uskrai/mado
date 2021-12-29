use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub use async_io::Timer;

#[derive(Debug)]
pub struct Elapsed;
impl std::error::Error for Elapsed {}
impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "deadline has elapsed".fmt(f)
    }
}

pub fn timeout<F>(duration: Duration, f: F) -> Timeout<F>
where
    F: std::future::Future,
{
    let timer = Timer::after(duration);

    Timeout { timer, future: f }
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

        if let Poll::Ready(_) = me.timer.poll(cx) {
            return Poll::Ready(Err(Elapsed));
        }

        Poll::Pending
    }
}
