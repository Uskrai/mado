use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub use async_io::Timer;
use futures::{FutureExt, Stream, StreamExt};

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

pub fn sleep(duration: Duration) -> Timer {
    Timer::after(duration)
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

pub struct Delayed<T> {
    timer: Pin<Box<Timer>>,
    value: Option<T>,
}

impl<T> Unpin for Delayed<T> {}

impl<T> Delayed<T> {
    pub fn new(value: T, delay: Duration) -> Self {
        Delayed {
            value: Some(value),
            timer: Box::pin(sleep(delay)),
        }
    }
}

impl<T> std::future::Future for Delayed<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.timer.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => Poll::Ready(self.value.take().unwrap()),
        }
    }
}

pub struct Debouce<S>
where
    S: Stream,
{
    stream: S,
    duration: Duration,
    pending: Option<Delayed<S::Item>>,
}

impl<S> Stream for Debouce<S>
where
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        while let Poll::Ready(it) = self.stream.poll_next_unpin(cx) {
            match it {
                Some(it) => self.pending = Some(Delayed::new(it, self.duration)),
                None => {
                    if self.pending.is_none() {
                        return Poll::Ready(None);
                    }
                    break;
                }
            }
        }

        match self.pending.as_mut() {
            Some(pending) => match pending.poll_unpin(cx) {
                Poll::Ready(value) => {
                    let _ = self.pending.take();
                    Poll::Ready(Some(value))
                }
                Poll::Pending => Poll::Pending,
            },
            None => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

pub fn debounce<S>(stream: S, time: Duration) -> Debouce<S>
where
    S: Stream + Unpin,
{
    Debouce {
        stream,
        duration: time,
        pending: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Instant};

    use futures::SinkExt;
    use parking_lot::Mutex;

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

    #[test]
    fn test_delay() {
        let start = Instant::now();
        let delayed = futures::executor::block_on(Delayed::new(42, Duration::from_secs(1)));
        assert_eq!(start.elapsed().as_secs(), 1);
        assert_eq!(delayed, 42);
    }

    #[test]
    fn test_debounce() {
        futures::executor::block_on(async {
            let start = Instant::now();
            let (mut sender, receiver) = futures::channel::mpsc::channel(1024);
            let mut debounced = debounce(receiver, Duration::from_secs(1));
            let _ = sender.send(21).await;
            let _ = sender.send(42).await;
            assert_eq!(debounced.next().await, Some(42));
            assert_eq!(start.elapsed().as_secs(), 1);
            std::mem::drop(sender);
            assert_eq!(debounced.next().await, None);
        })
    }

    #[test]
    fn test_debounce_size_hint() {
        futures::executor::block_on(async {
            let (_sender, receiver) = futures::channel::mpsc::channel::<()>(1024);
            let hint = receiver.size_hint();
            let debounced = debounce(receiver, Duration::from_secs(1));
            assert_eq!(debounced.size_hint(), hint);
        })
    }

    #[test]
    fn test_debounce_order() {
        #[derive(Debug, PartialEq, Eq)]
        pub enum Message {
            Value(u64),
            SenderEnded,
            ReceiverEnded,
        }

        let (mut sender, receiver) = futures::channel::mpsc::channel(1024);
        let mut receiver = debounce(receiver, Duration::from_millis(100));
        let messages = Arc::new(Mutex::new(vec![]));

        futures::executor::block_on(async {
            futures::future::join(
                {
                    let messages = messages.clone();
                    async move {
                        for i in 0..10u64 {
                            let _ = sleep(Duration::from_millis(23 * i)).await;
                            let _ = sender.send(i).await;
                        }

                        messages.lock().push(Message::SenderEnded);
                    }
                },
                {
                    let messages = messages.clone();

                    async move {
                        while let Some(value) = receiver.next().await {
                            messages.lock().push(Message::Value(value));
                        }

                        messages.lock().push(Message::ReceiverEnded);
                    }
                },
            )
            .await;

            assert_eq!(
                messages.lock().as_slice(),
                &[
                    Message::Value(4),
                    Message::Value(5),
                    Message::Value(6),
                    Message::Value(7),
                    Message::Value(8),
                    Message::SenderEnded,
                    Message::Value(9),
                    Message::ReceiverEnded
                ]
            );
        });
    }
}
