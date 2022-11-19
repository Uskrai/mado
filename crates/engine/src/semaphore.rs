use std::{
    cmp::Reverse,
    sync::{atomic::AtomicUsize, Arc},
};

use atomic::Ordering::Relaxed;
use by_address::ByAddress;
use event_listener::Event;
use parking_lot::{Mutex, MutexGuard};
use priority_queue::PriorityQueue;

type PriorityItem = ByAddress<Arc<()>>;
type PriorityCollection = PriorityQueue<PriorityItem, Reverse<usize>>;
#[derive(Default, Debug)]
pub struct PrioritySemaphore {
    event: Event,
    queue: Mutex<PriorityCollection>,
    limit: AtomicUsize,
    acquired: AtomicUsize,
}

impl PrioritySemaphore {
    pub fn new(limit: usize) -> PrioritySemaphore {
        Self {
            limit: AtomicUsize::from(limit),
            ..Default::default()
        }
    }

    fn peek_guard(queue: &mut MutexGuard<PriorityCollection>) -> Option<PriorityItem> {
        while !queue.is_empty() {
            let peek = queue
                .peek()
                .filter(|(it, _)| Arc::strong_count(it) > 1)
                .map(|(it, _)| it)
                .cloned();

            if let Some(peek) = peek {
                return Some(peek);
            }

            queue.pop();
        }

        None
    }

    fn try_acquire_address_guard(
        &self,
        queue: &mut MutexGuard<PriorityCollection>,
        amount: usize,
        address: Arc<()>,
    ) -> Option<SemaphoreGuard> {
        if let Some(peeked) = Self::peek_guard(queue) {
            let acquired = self.acquired.load(Relaxed);
            let limit = self.limit();

            if peeked == ByAddress(address) && acquired + amount <= limit {
                self.acquired.fetch_add(amount, Relaxed);
                queue.pop();

                return Some(SemaphoreGuard {
                    semaphore: self,
                    amount,
                });
            }
        }

        None
    }
    fn try_acquire_address(&self, amount: usize, address: Arc<()>) -> Option<SemaphoreGuard> {
        self.try_acquire_address_guard(&mut self.queue.lock(), amount, address)
    }

    pub fn try_acquire(&self, priority: usize, amount: usize) -> Option<SemaphoreGuard> {
        let address = Arc::new(());
        self.queue
            .lock()
            .push(ByAddress(address.clone()), Reverse(priority));

        self.try_acquire_address(amount, address)
    }

    pub async fn acquire(&self, priority: usize, amount: usize) -> SemaphoreGuard {
        let address = Arc::new(());
        self.queue
            .lock()
            .push(ByAddress(address.clone()), Reverse(priority));

        loop {
            {
                let mut queue = self.queue.lock();

                let guard = self.try_acquire_address_guard(&mut queue, amount, address.clone());

                if self.acquired() < self.limit() {
                    self.notify_one();
                }

                if let Some(guard) = guard {
                    return guard;
                }
                drop(queue);
            }

            self.event.listen().await;
        }
    }

    pub fn set_limit(&self, limit: usize) {
        self.limit.store(limit, Relaxed);
        self.notify_one();
    }

    fn notify_one(&self) {
        self.event.notify(1);
    }

    pub fn acquired(&self) -> usize {
        self.acquired.load(Relaxed)
    }
    pub fn limit(&self) -> usize {
        self.limit.load(Relaxed)
    }

    fn release(&self, amount: usize) {
        self.acquired.fetch_sub(amount, Relaxed);
        self.event.notify(usize::MAX);
    }
}

#[derive(Debug)]
pub struct SemaphoreGuard<'a> {
    semaphore: &'a PrioritySemaphore,
    amount: usize,
}

impl SemaphoreGuard<'_> {
    pub fn forget(self) -> usize {
        let amount = self.amount;
        std::mem::forget(self);
        amount
    }
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.semaphore.release(self.amount);
    }
}

#[cfg(test)]
mod tests {

    use std::sync::mpsc;

    use futures::{Future, FutureExt};

    use super::*;

    fn noop_context() -> std::task::Context<'static> {
        std::task::Context::from_waker(futures::task::noop_waker_ref())
    }

    #[test]
    fn peek_none() {
        let s = PrioritySemaphore::new(0);

        assert!(PrioritySemaphore::peek_guard(&mut s.queue.lock()).is_none());
    }

    #[test]
    #[ntest::timeout(100)]
    fn order_test() {
        futures::executor::block_on(async {
            let s = PrioritySemaphore::new(0);

            let g2 = s.acquire(2, 1);
            let g1 = s.acquire(1, 1);

            futures::pin_mut!(g2, g1);

            let mut context = noop_context();

            assert!(g2.poll_unpin(&mut context).is_pending());
            assert!(g1.poll_unpin(&mut context).is_pending());

            crate::timer::sleep(std::time::Duration::from_millis(10)).await;
            s.set_limit(1);

            assert!(g2.poll_unpin(&mut context).is_pending());

            let guard = g1.poll(&mut context);
            assert!(guard.is_ready());
            assert!(g2.poll_unpin(&mut context).is_pending());
            drop(guard);
            assert!(g2.poll(&mut context).is_ready());
        })
    }

    #[test]
    #[ntest::timeout(100)]
    fn acquire_more_test() {
        futures::executor::block_on(async {
            let s = PrioritySemaphore::new(1);

            crate::timer::timeout(std::time::Duration::from_millis(10), s.acquire(1, 2))
                .await
                .map(|_| ())
                .expect_err("Acquire should not return");
        })
    }

    #[test]
    #[ntest::timeout(100)]
    fn forget_test() {
        futures::executor::block_on(async {
            let s = PrioritySemaphore::new(1);

            let g1 = s.acquire(1, 1).await;
            g1.forget();

            let g2 = s.acquire(1, 1);
            futures::pin_mut!(g2);

            let mut context = noop_context();
            assert!(g2.poll_unpin(&mut context).is_pending());

            s.release(1);
            assert!(g2.poll_unpin(&mut context).is_ready());
        })
    }

    #[test]
    #[ntest::timeout(100)]
    fn drop_acquire_test() {
        futures::executor::block_on(async {
            let s = PrioritySemaphore::new(0);

            let mut g1 = Box::pin(s.acquire(1, 1));
            let g2 = s.acquire(2, 1);

            futures::pin_mut!(g2);

            let mut context = noop_context();

            assert!(g1.poll_unpin(&mut context).is_pending());
            assert!(g2.poll_unpin(&mut context).is_pending());

            drop(g1);

            assert!(g2.poll_unpin(&mut context).is_pending());

            s.set_limit(1);

            assert!(g2.poll(&mut context).is_ready());
        })
    }

    // https://github.com/smol-rs/async-lock/blob/master/tests/semaphore.rs
    #[test]
    fn as_mutex() {
        use futures::executor::block_on;
        use std::thread;

        let s = Arc::new(PrioritySemaphore::new(1));
        let s2 = s.clone();
        let _t = thread::spawn(move || {
            block_on(async {
                let _g = s2.acquire(1, 1).await;
            });
        });
        block_on(async {
            let _g = s.acquire(1, 1).await;
        });
    }

    // https://github.com/smol-rs/async-lock/blob/master/tests/semaphore.rs
    #[test]
    #[ntest::timeout(100)]
    fn try_acquire() {
        let s = PrioritySemaphore::new(2);
        let g1 = s.try_acquire(1, 1).unwrap();
        let _g2 = s.try_acquire(1, 1).unwrap();

        assert!(s.try_acquire(1, 1).is_none());
        drop(g1);
        assert!(s.try_acquire(1, 1).is_some());
    }

    // https://github.com/smol-rs/async-lock/blob/master/tests/semaphore.rs
    #[test]
    fn stress() {
        use futures::executor::block_on;
        use std::thread;

        const THREAD: usize = 20;
        const COUNT: usize = if cfg!(miri) { 500 } else { 1_000 };

        let s = Arc::new(PrioritySemaphore::new(5));
        let (tx, rx) = mpsc::channel::<()>();

        for _ in 0..THREAD {
            let s = s.clone();
            let tx = tx.clone();

            thread::spawn(move || {
                block_on(async {
                    for _ in 0..COUNT {
                        crate::timer::timeout(std::time::Duration::from_secs(60), s.acquire(1, 1))
                            .await
                            .unwrap();
                    }
                    drop(tx);
                })
            });
        }

        drop(tx);
        let _ = rx.recv();

        let _g1 = s.try_acquire(1, 1).unwrap();
        let g2 = s.try_acquire(1, 1).unwrap();
        let _g3 = s.try_acquire(1, 1).unwrap();
        let _g4 = s.try_acquire(1, 1).unwrap();
        let _g5 = s.try_acquire(1, 1).unwrap();

        assert!(s.try_acquire(1, 1).is_none());
        drop(g2);
        assert!(s.try_acquire(1, 1).is_some());
    }

    // https://github.com/smol-rs/async-lock/blob/master/tests/semaphore.rs
    #[test]
    #[ntest::timeout(10000)]
    fn multi_resource() {
        use futures::executor::block_on;
        use std::thread;

        for _ in 0..100 {
            let s = Arc::new(PrioritySemaphore::new(2));
            let s2 = s.clone();
            let (tx1, rx1) = mpsc::channel();
            let (tx2, rx2) = mpsc::channel();
            let _t = thread::spawn(move || {
                block_on(async {
                    let _g = s2.acquire(2, 1).await;
                    let _ = rx2.recv();
                    tx1.send(()).unwrap();
                });
            });
            block_on(async {
                let _g = s.acquire(1, 1).await;
                tx2.send(()).unwrap();
                rx1.recv().unwrap();
            });
        }
    }
}
