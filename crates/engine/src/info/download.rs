use crate::{
    core::{ChapterInfo, MangaInfo, Url, Uuid},
    path::Utf8PathBuf,
    ArcMadoModule, DownloadChapterInfo, DownloadProgressStatus, DownloadResumedStatus,
    DownloadStatus, LateBindingModule, ModuleInfo,
};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug)]
pub struct DownloadInfo {
    module: ModuleInfo,
    manga_title: String,
    chapters: Vec<Arc<DownloadChapterInfo>>,
    path: Utf8PathBuf,
    url: Option<Url>,
    status: Mutex<DownloadStatus>,
    observers: Mutex<Vec<ArcDownloadInfoObserver>>,
}

impl DownloadInfo {
    /// Create new Download info.
    pub fn new(
        module: LateBindingModule,
        title: String,
        chapters: Vec<Arc<DownloadChapterInfo>>,
        path: Utf8PathBuf,
        url: Option<Url>,
        status: DownloadStatus,
    ) -> Self {
        Self {
            module: ModuleInfo::new(module),
            manga_title: title,
            chapters,
            path,
            url,
            status: Mutex::new(status),
            observers: Mutex::new(Vec::new()),
        }
    }
    pub fn from_request(request: DownloadRequest) -> Self {
        let DownloadRequest {
            module,
            manga,
            chapters,
            path,
            status,
            url,
        } = request;

        let chapters = chapters
            .into_iter()
            .map(|it| {
                let title = it.to_string();
                let path = path.join(&title);
                DownloadChapterInfo::new(
                    LateBindingModule::Module(module.clone()),
                    it.id.clone(),
                    title,
                    path,
                    DownloadStatus::InProgress(status.into()),
                )
            })
            .map(Arc::new)
            .collect();

        Self::new(
            LateBindingModule::Module(module),
            manga.title.clone(),
            chapters,
            path,
            url,
            DownloadStatus::InProgress(status.into()),
        )
    }

    /// Get download info's status.
    pub fn status(&self) -> impl std::ops::Deref<Target = DownloadStatus> + '_ {
        self.status.lock()
    }

    /// Get a reference to the download info's path.
    pub fn path(&self) -> &Utf8PathBuf {
        &self.path
    }

    pub fn module_uuid(&self) -> &Uuid {
        &self.module.uuid()
    }

    pub fn manga_title(&self) -> &str {
        &self.manga_title
    }

    pub fn module_domain(&self) -> Option<&str> {
        self.url.as_ref().map(|url| url.domain()).flatten()
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    /// Wait for module to be available.
    ///
    /// if the module is already available, this will return immediately.
    pub async fn wait_module(&self) -> ArcMadoModule {
        self.module.lock().await.wait().await
    }

    /// Get a reference to the downloaded chapters.
    pub fn chapters(&self) -> &[Arc<DownloadChapterInfo>] {
        &self.chapters
    }

    /// Change download's status, then emit [`DownloadInfoObserver::on_status_changed`]
    pub fn set_status(&self, status: DownloadStatus) {
        let mut lock = self.status.lock();
        *lock = status;
        self.emit(|it| it.on_status_changed(&lock));
    }

    /// Resume Download
    pub fn resume(&self, resume: bool) {
        let status = if let DownloadStatus::InProgress(_) = *self.status() {
            if resume {
                DownloadProgressStatus::Resumed(DownloadResumedStatus::Waiting)
            } else {
                DownloadProgressStatus::Paused
            }
        } else {
            return;
        };

        self.set_status(DownloadStatus::InProgress(status));
    }

    /// Get a reference to the download info's manga's title.
    pub fn manga(&self) -> &str {
        &self.manga_title
    }

    /// Connect and send current state.
    pub fn connect(&self, observer: ArcDownloadInfoObserver) {
        let mut observers = self.observers.lock();

        observer.on_status_changed(&self.status());

        observers.push(observer);
    }

    /// Connect without sending current state
    pub fn connect_only(&self, observer: ArcDownloadInfoObserver) {
        self.observers.lock().push(observer);
    }

    fn emit(&self, fun: impl Fn(ArcDownloadInfoObserver)) {
        for it in self.observers.lock().iter() {
            fun(it.clone());
        }
    }
}

pub struct DownloadRequest {
    module: ArcMadoModule,
    manga: Arc<MangaInfo>,
    chapters: Vec<Arc<ChapterInfo>>,
    path: Utf8PathBuf,
    url: Option<Url>,
    status: DownloadRequestStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum DownloadRequestStatus {
    Resume,
    Pause,
}

impl From<DownloadRequestStatus> for DownloadProgressStatus {
    fn from(this: DownloadRequestStatus) -> Self {
        match this {
            DownloadRequestStatus::Resume => {
                DownloadProgressStatus::Resumed(DownloadResumedStatus::Waiting)
            }
            DownloadRequestStatus::Pause => DownloadProgressStatus::Paused,
        }
    }
}

impl DownloadRequest {
    pub fn new(
        module: ArcMadoModule,
        manga: Arc<MangaInfo>,
        chapters: Vec<Arc<ChapterInfo>>,
        path: Utf8PathBuf,
        url: Option<Url>,
        status: DownloadRequestStatus,
    ) -> Self {
        Self {
            module,
            manga,
            chapters,
            path,
            status,
            url,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
pub trait DownloadInfoObserver: std::fmt::Debug {
    fn on_status_changed(&self, status: &DownloadStatus);
}

type ArcDownloadInfoObserver = Arc<dyn DownloadInfoObserver + Send + Sync>;

#[cfg(test)]
mod tests {
    use mado_core::DefaultMadoModuleMap;
    use mockall::predicate;

    use super::*;

    #[test]
    fn download_observer() {
        let info = DownloadInfo::new(
            LateBindingModule::WaitModule(
                Arc::new(DefaultMadoModuleMap::new()),
                Default::default(),
            ),
            Default::default(),
            Vec::new(),
            Default::default(),
            None,
            DownloadStatus::paused(),
        );
        {
            let mut mock = MockDownloadInfoObserver::new();
            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::paused()))
                .returning(|_| ());

            mock.expect_on_status_changed().times(..).returning(|_| ());

            info.connect(Arc::new(mock));
        }

        {
            let mut mock = MockDownloadInfoObserver::new();
            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::paused()))
                .returning(|_| ());

            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::finished()))
                .returning(|_| ());

            info.connect(Arc::new(mock));

            info.set_status(DownloadStatus::finished());
        }

        {
            let mut mock = MockDownloadInfoObserver::new();
            mock.expect_on_status_changed().never();
            info.connect_only(Arc::new(mock));
        }
    }
}
