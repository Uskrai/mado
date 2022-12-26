use std::sync::Arc;

use event_listener::Event;

use crate::{AnyObserverHandleSend, DownloadInfo, DownloadStatus};

#[derive(Debug)]
pub struct DownloadInfoWatcher {
    info: Arc<DownloadInfo>,
    event: Arc<Event>,
    observer: AnyObserverHandleSend,
}

impl Drop for DownloadInfoWatcher {
    fn drop(&mut self) {
        self.observer.clone().disconnect();
    }
}

impl DownloadInfoWatcher {
    pub fn connect(info: Arc<DownloadInfo>) -> Self {
        let event = Arc::new(Event::new());

        let observer = info
            .connect({
                let event = event.clone();

                move |_| {
                    event.notify(usize::MAX);
                }
            })
            .send_handle_any();

        Self {
            info,
            event,
            observer,
        }
    }

    pub async fn wait_status(&self, fun: impl Fn(&DownloadStatus) -> bool) {
        loop {
            if fun(&self.info.status()) {
                return;
            }

            self.event.listen().await;
        }
    }

    pub async fn wait_order(&self, fun: impl Fn(usize) -> bool) {
        loop {
            if fun(self.info.order()) {
                return;
            }

            self.event.listen().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use mado_core::{DefaultMadoModuleMap, Uuid};

    use crate::{DownloadInfo, LateBindingModule};

    use super::{Arc, DownloadInfoWatcher, DownloadStatus};

    #[test]
    fn watcher_test() {
        let map = DefaultMadoModuleMap::default();

        let module = LateBindingModule::WaitModule(Arc::new(map), Uuid::from_u128(1));
        let info = DownloadInfo::builder()
            .order(0)
            .module(module)
            .chapters(vec![])
            .status(crate::DownloadStatus::error("Error"))
            .build();

        let info = Arc::new(info);
        let watcher = DownloadInfoWatcher::connect(info.clone());

        futures::executor::block_on(async {
            let future = watcher.wait_status(DownloadStatus::is_paused);
            crate::timer::timeout(std::time::Duration::from_millis(10), future)
                .await
                .unwrap_err();

            let future = watcher.wait_status(DownloadStatus::is_finished);
            info.resume(true);
            assert!(info.status().is_resumed());
            assert!(!info.status().is_finished());
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
