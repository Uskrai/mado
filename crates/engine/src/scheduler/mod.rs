use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use by_address::ByAddress;
use futures::StreamExt;
use parking_lot::Mutex;

use crate::{DownloadInfo, DownloadStatus, MadoEngineState};

use self::item::QueueItem;
pub use self::runner::TaskRunner;

mod item;
mod runner;

pub enum SchedulerMsg {
    NewQueue(Arc<DownloadInfo>),
    RemoveQueue(Arc<DownloadInfo>),
    OrderChanged(Arc<DownloadInfo>),
}

pub struct TaskScheduler {
    state: Arc<MadoEngineState>,
    option: Arc<TaskSchedulerOption>,
}

#[derive(Debug)]
pub struct TaskSchedulerOption {
    download_limit: AtomicUsize,
}

const _: () = {
    fn assert<T: Send + Sync>() {}

    fn assert_all() {
        assert::<TaskScheduler>();
    }
};

impl Default for TaskSchedulerOption {
    fn default() -> Self {
        Self {
            download_limit: AtomicUsize::new(usize::MAX),
        }
    }
}

impl TaskSchedulerOption {
    pub fn download_limit(&self) -> usize {
        self.download_limit.load(atomic::Ordering::Relaxed)
    }

    pub fn set_download_limit(&self, download_limit: usize) {
        self.download_limit
            .store(download_limit, atomic::Ordering::Relaxed);
    }
}

pub const TASK_SCHEDULER_DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

impl TaskScheduler {
    pub fn connect(
        state: Arc<MadoEngineState>,
        option: Arc<TaskSchedulerOption>,
    ) -> (Self, TaskRunner) {
        let runner = TaskRunner::new();

        (Self { state, option }, runner)
    }

    pub async fn run(self) {
        let vec = Mutex::new(Vec::<QueueItem>::new());

        let (mut connect_rx, handle) = self.connect_state();

        let (tx, schedule_rx) = futures::channel::mpsc::unbounded();
        let schedule_rx = async {
            let mut schedule_rx =
                crate::timer::debounce(schedule_rx, TASK_SCHEDULER_DEBOUNCE_DURATION);

            while let Some(_) = schedule_rx.next().await {
                tracing::trace!("reschedule download");

                let mut vec = vec.lock();

                vec.sort();
                vec.dedup();

                // count currently downloading task
                let mut downloading = vec
                    .iter()
                    .filter(|it| {
                        it.status()
                            .as_resumed()
                            .map(|it| it.is_downloading())
                            .unwrap_or(false)
                    })
                    .count();

                let limit = self.option.download_limit();

                // set queued item to downloading until downloading is not less than limit
                for it in vec.iter() {
                    if !(downloading < limit) {
                        break;
                    }

                    if it
                        .status()
                        .as_resumed()
                        .map(|it| it.is_queue())
                        .unwrap_or(false)
                    {
                        tracing::debug!("resuming {:?}", it);
                        it.set_status(DownloadStatus::resumed(
                            crate::DownloadResumedStatus::Downloading,
                        ));
                        downloading += 1;
                    }
                }
            }
        };

        let connect_rx = async {
            while let Some(msg) = connect_rx.next().await {
                match msg {
                    SchedulerMsg::NewQueue(info) => {
                        vec.lock().push(QueueItem::new(info));
                    }
                    SchedulerMsg::RemoveQueue(info) => {
                        let address = ByAddress(info);
                        vec.lock().retain(|it| it.0 != address);
                    }
                    SchedulerMsg::OrderChanged(_) => {}
                }
                let _ = tx.unbounded_send(());
            }
        };

        let _ = futures::join!(schedule_rx, connect_rx);

        handle.disconnect();
    }

    pub fn connect_state(
        &self,
    ) -> (
        futures::channel::mpsc::UnboundedReceiver<SchedulerMsg>,
        crate::AnyObserverHandleSend,
    ) {
        let (tx, connect_rx) = futures::channel::mpsc::unbounded::<SchedulerMsg>();

        let handle = self.state.connect(move |msg| {
            let tx = tx.clone();

            match msg {
                crate::MadoEngineStateMsg::Download(info) => {
                    let info = info.clone();
                    info.clone().connect(move |msg| match msg {
                        crate::DownloadInfoMsg::StatusChanged(status) => {
                            match status.as_resumed() {
                                Some(crate::DownloadResumedStatus::Queue) => {
                                    let _ = tx.unbounded_send(SchedulerMsg::NewQueue(info.clone()));
                                }
                                Some(crate::DownloadResumedStatus::Downloading) => {
                                    // this is handled below in schedule_rx
                                }
                                _ => {
                                    let msg = SchedulerMsg::RemoveQueue(info.clone());
                                    let _ = tx.unbounded_send(msg);
                                }
                            }
                        }
                        crate::DownloadInfoMsg::OrderChanged(_) => {
                            let _ = tx.unbounded_send(SchedulerMsg::OrderChanged(info.clone()));
                        }
                    });
                }
                crate::MadoEngineStateMsg::PushModule(_) => {}
            }
        });

        (connect_rx, handle.send_handle_any())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mado_core::{MockMadoModule, Uuid};

    use crate::{
        scheduler::TASK_SCHEDULER_DEBOUNCE_DURATION, DownloadRequest, DownloadStatus,
        MadoEngineState,
    };

    #[test]
    pub fn test_limit() {
        let state = MadoEngineState::default();

        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(1));

        let module = Arc::new(module);

        let create_request = || {
            DownloadRequest::new(
                module.clone(),
                Default::default(),
                vec![],
                "".into(),
                None,
                crate::DownloadRequestStatus::Pause,
            )
        };

        let (tx, rx) = std::sync::mpsc::channel();
        state.connect(move |msg| match msg {
            crate::MadoEngineStateMsg::Download(info) => tx.send(info.clone()).unwrap(),
            _ => {}
        });

        state.download_request(create_request());
        let first = rx.recv().unwrap();
        state.download_request(create_request());
        let second = rx.recv().unwrap();

        futures::executor::block_on(async {
            let option = Arc::new(super::TaskSchedulerOption::default());
            option.set_download_limit(0);

            let state = Arc::new(state);

            let scheduler = super::TaskScheduler::connect(state.clone(), option.clone()).0;

            let test = async {
                let sleep = || {
                    crate::timer::sleep(
                        TASK_SCHEDULER_DEBOUNCE_DURATION
                            .saturating_add(std::time::Duration::from_millis(10)),
                    )
                };
                sleep().await;

                assert!(first.status().is_paused());
                assert!(second.status().is_paused());

                let set_status = |status: DownloadStatus| {
                    first.set_status(status.clone());
                    second.set_status(status);
                };

                let set_queue =
                    || set_status(DownloadStatus::resumed(crate::DownloadResumedStatus::Queue));

                set_queue();
                sleep().await;

                assert!(first.status().is_queue());
                assert!(second.status().is_queue());

                option.set_download_limit(1);
                set_queue();

                sleep().await;
                assert!(first.status().is_downloading());
                assert!(second.status().is_queue());

                option.set_download_limit(0);
                set_queue();
                sleep().await;
                assert!(first.status().is_queue());
                assert!(second.status().is_queue());

                option.set_download_limit(2);
                set_queue();
                sleep().await;

                assert!(first.status().is_downloading());
                assert!(second.status().is_downloading());

                set_status(DownloadStatus::paused());
                sleep().await;
                assert!(first.status().is_paused());
                assert!(second.status().is_paused());

                second.set_status(DownloadStatus::queued());
                sleep().await;
                assert!(first.status().is_paused());
                assert!(second.status().is_downloading());

                first.set_status(DownloadStatus::queued());
                sleep().await;
                assert!(first.status().is_downloading());
                assert!(second.status().is_downloading());
            };

            let scheduler = scheduler.run();

            futures::pin_mut!(test, scheduler);
            futures::future::select(test, scheduler).await;
        });
    }
}
