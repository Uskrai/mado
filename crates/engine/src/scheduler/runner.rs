use std::sync::Arc;

use futures::FutureExt;

use crate::{watcher::DownloadInfoWatcher, DownloadStatus, TaskDownloader};

pub struct TaskRunner {}

impl TaskRunner {
    pub fn new() -> Self {
        Self {}
    }

    #[tracing::instrument(
        skip_all,
        fields(
            module_uuid = %download.module_uuid(),
            url = ?download.url().map(|it| it.as_str()),
        )
    )]
    pub async fn run<F>(&self, download: Arc<crate::DownloadInfo>, mut create_download: F)
    where
        F: FnMut(&Arc<crate::DownloadInfo>) -> TaskDownloader,
    {
        let status = DownloadInfoWatcher::connect(download.clone());

        loop {
            tracing::trace!("waiting for resumed");
            status.wait_status(DownloadStatus::is_resumed).await;
            download.set_status(DownloadStatus::resumed(
                crate::DownloadResumedStatus::Waiting,
            ));

            tracing::trace!("waiting for module");
            let _ = download.wait_module().await;

            download.set_status(DownloadStatus::resumed(crate::DownloadResumedStatus::Queue));

            tracing::trace!("waiting for downloading");
            status.wait_status(DownloadStatus::is_downloading).await;

            tracing::trace!("download resumed");
            let paused = status
                .wait_status(DownloadStatus::is_paused)
                .inspect(|_| tracing::trace!("download paused"))
                .map(|_| Ok::<(), mado_core::Error>(()));

            let dl = create_download(&download).run();

            futures::pin_mut!(dl, paused);

            let (result, _) = futures::future::select(dl, paused).await.factor_first();

            if let Err(err) = result {
                tracing::error!("{}", err);
                download.set_status(DownloadStatus::error(err));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mado_core::{
        DefaultMadoModuleMap, MockMadoModule, MutMadoModuleMap, MutexMadoModuleMap, Url, Uuid,
    };

    use crate::{DownloadChapterInfo, DownloadInfo, DownloadOption, LateBindingModule};

    use super::*;

    #[test]
    pub fn test_status() {
        futures::executor::block_on(async {
            let runner = TaskRunner::new();

            let uuid = Uuid::from_u128(1);
            let map = Arc::new(MutexMadoModuleMap::new(DefaultMadoModuleMap::new()));
            let late = LateBindingModule::WaitModule(map.clone(), uuid);

            let info = Arc::new(
                DownloadInfo::builder()
                    .order(0)
                    .module(late.clone())
                    .status(DownloadStatus::paused())
                    .build(),
            );

            let option = DownloadOption::default();
            let runner = runner.run(info.clone(), move |info| {
                TaskDownloader::new(info.clone(), option.clone())
            });

            let test = async {
                let sleep = || crate::timer::sleep(std::time::Duration::from_millis(10));
                sleep().await;
                assert!(info.status().is_paused());
                info.resume(true);
                sleep().await;
                assert!(info.status().as_resumed().unwrap().is_waiting());

                let mut module = MockMadoModule::new();
                module.expect_uuid().return_const(uuid);
                module
                    .expect_domain()
                    .return_const(Url::try_from("http://localhost").unwrap());
                map.push_mut(Arc::new(module)).unwrap();

                sleep().await;
                crate::timer::sleep(crate::LATE_BINDING_MODULE_SLEEP_TIME).await;
                assert!(info.status().as_resumed().unwrap().is_queue());

                info.set_status(DownloadStatus::resumed(
                    crate::DownloadResumedStatus::Downloading,
                ));
                sleep().await;

                assert!(info.status().is_finished());
            };

            futures::pin_mut!(runner, test);
            let _ = futures::future::select(runner, test).await.factor_first();
        });
    }

    #[test]
    pub fn test_error() {
        let runner = TaskRunner::new();

        let uuid = Uuid::from_u128(1);
        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(uuid);
        module
            .expect_domain()
            .return_const(Url::try_from("http://localhost").unwrap());

        module
            .expect_get_chapter_images()
            .returning(|_, _| Err(mado_core::Error::ExternalError(anyhow::anyhow!("error"))));

        let module = Arc::new(module);

        let map = Arc::new(MutexMadoModuleMap::new(DefaultMadoModuleMap::new()));
        let late = LateBindingModule::WaitModule(map.clone(), uuid);

        map.push_mut(module.clone()).unwrap();

        let info = Arc::new(
            DownloadInfo::builder()
                .order(0)
                .module(module.clone())
                .chapters(vec![Arc::new(DownloadChapterInfo::new(
                    late,
                    "1".to_string(),
                    "".to_string(),
                    "".into(),
                    DownloadStatus::resumed(Default::default()),
                ))])
                .status(DownloadStatus::resumed(
                    crate::DownloadResumedStatus::Downloading,
                ))
                .build(),
        );

        let option = DownloadOption::default();
        let runner = runner.run(info.clone(), move |info| {
            TaskDownloader::new(info.clone(), option.clone())
        });

        futures::executor::block_on(async {
            let test = async {
                let sleep = || crate::timer::sleep(std::time::Duration::from_millis(10));
                sleep().await;
                info.set_status(DownloadStatus::resumed(
                    crate::DownloadResumedStatus::Downloading,
                ));
                sleep().await;
                assert!(info.status().is_error());
            };

            futures::pin_mut!(test, runner);
            let _ = futures::future::select(test, runner).await;
        });
    }
}
