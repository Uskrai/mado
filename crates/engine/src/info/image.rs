use parking_lot::Mutex;

use crate::{core::ChapterImageInfo, path::Utf8PathBuf, DownloadStatus, ObserverHandle, Observers};
#[derive(Debug)]
pub struct DownloadChapterImageInfo {
    image: ChapterImageInfo,
    path: Utf8PathBuf,
    status: Mutex<DownloadStatus>,
    observers: Observers<BoxObserver>,
}

pub type BoxObserver = Box<dyn FnMut(DownloadChapterImageInfoMsg<'_>) + Send>;

macro_rules! ImplObserver {
    () => {
        impl FnMut(DownloadChapterImageInfoMsg<'_>) + Send + 'static
    }
}

// REMINDER: add new variant to connect
pub enum DownloadChapterImageInfoMsg<'a> {
    StatusChanged(&'a DownloadStatus),
}

impl DownloadChapterImageInfo {
    pub fn new(image: ChapterImageInfo, path: Utf8PathBuf, status: DownloadStatus) -> Self {
        Self {
            image,
            path,
            status: From::from(status),
            observers: Default::default(),
        }
    }

    pub fn image(&self) -> &ChapterImageInfo {
        &self.image
    }

    pub fn path(&self) -> &crate::path::Utf8Path {
        &self.path
    }

    /// Get a reference to the download chapter info's status.
    pub fn status(&self) -> impl std::ops::Deref<Target = DownloadStatus> + '_ {
        self.status.lock()
    }

    pub fn set_status(&self, status: DownloadStatus) {
        let mut lock = self.status.lock();
        *lock = status;
        self.observers
            .emit(|it| it(DownloadChapterImageInfoMsg::StatusChanged(&lock)));
    }

    pub fn connect(&self, mut observer: ImplObserver!()) -> ObserverHandle<BoxObserver> {
        observer(DownloadChapterImageInfoMsg::StatusChanged(&self.status()));

        self.connect_only(observer)
    }

    pub fn connect_only(&self, observer: ImplObserver!()) -> ObserverHandle<BoxObserver> {
        self.observers.connect(Box::new(observer))
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate;

    use crate::{DownloadChapterImageInfo, DownloadStatus};

    use super::DownloadChapterImageInfoMsg;

    mockall::mock! {
        pub Thing {
            fn on_status_changed(&self, status: &DownloadStatus);
            fn on_download(&self, info: &DownloadStatus);
        }
    }

    impl MockThing {
        fn handle(&self, msg: DownloadChapterImageInfoMsg<'_>) {
            match msg {
                DownloadChapterImageInfoMsg::StatusChanged(status) => {
                    self.on_status_changed(status)
                }
            }
        }

        fn handler(self) -> impl FnMut(DownloadChapterImageInfoMsg<'_>) + Send + 'static {
            move |msg: DownloadChapterImageInfoMsg<'_>| self.handle(msg)
        }
    }

    #[test]
    fn observe_test() {
        let info = DownloadChapterImageInfo::new(
            Default::default(),
            "path".into(),
            DownloadStatus::paused(),
        );

        {
            let mut mock = MockThing::default();
            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::paused()))
                .returning(|_| ());

            let _ = info.connect(mock.handler()).disconnect().unwrap();
        }

        {
            let mut mock = MockThing::new();
            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::paused()))
                .returning(|_| ());

            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::waiting()))
                .returning(|_| ());

            let handle = info.connect(mock.handler());

            info.set_status(DownloadStatus::waiting());
            let _ = handle.disconnect().unwrap();
            info.set_status(DownloadStatus::finished());
        }

        {
            let mut mock = MockThing::new();
            mock.expect_on_status_changed().never();
            let _ = info.connect_only(mock.handler()).disconnect().unwrap();
        }
    }
}
