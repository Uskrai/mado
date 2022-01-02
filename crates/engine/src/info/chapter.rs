use crate::{path::Utf8PathBuf, DownloadStatus, LateBindingModule};
use parking_lot::Mutex;

#[derive(Debug)]
pub struct DownloadChapterInfo {
    module: LateBindingModule,
    title: String,
    chapter_id: String,
    path: Utf8PathBuf,
    status: Mutex<DownloadStatus>,
    observers: Mutex<Vec<Box<dyn DownloadChapterInfoObserver>>>,
}

pub trait DownloadChapterInfoObserver: std::fmt::Debug + Send + Sync {
    fn on_status_changed(&self, status: &DownloadStatus);
}

impl DownloadChapterInfo {
    pub fn new(
        module: LateBindingModule,
        chapter_id: String,
        title: String,
        path: Utf8PathBuf,
        status: DownloadStatus,
    ) -> Self {
        Self {
            module,
            title,
            chapter_id,
            path,
            status: Mutex::new(status),
            observers: Default::default(),
        }
    }

    /// Get a reference to the download chapter info's module.
    pub fn module(&self) -> LateBindingModule {
        self.module.clone()
    }

    /// Get a reference to the download chapter info's path.
    pub fn path(&self) -> &Utf8PathBuf {
        &self.path
    }

    /// Get a reference to the download chapter info's status.
    pub fn status(&self) -> impl std::ops::Deref<Target = DownloadStatus> + '_ {
        self.status.lock()
    }

    pub fn set_status(&self, status: DownloadStatus) {
        let mut lock = self.status.lock();
        *lock = status;
        self.emit(|it| it.on_status_changed(&lock));
    }

    /// Get a reference to the download chapter info's title.
    ///
    /// this isn't necessarily ChapterInfo::title
    pub fn title(&self) -> &str {
        self.title.as_ref()
    }

    /// Get a reference to the chapter id.
    pub fn chapter_id(&self) -> &str {
        self.chapter_id.as_ref()
    }

    fn emit(&self, f: impl Fn(&Box<dyn DownloadChapterInfoObserver>)) {
        for it in self.observers.lock().iter() {
            f(it);
        }
    }
}
