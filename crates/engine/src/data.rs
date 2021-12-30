use crate::path::Utf8PathBuf;
use futures::lock::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
use parking_lot::Mutex;
use std::sync::Arc;

use mado_core::{ArcMadoModule, ArcMadoModuleMap, ChapterInfo, MangaInfo, Url, Uuid};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadResumedStatus {
    Waiting,
    Downloading,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadProgressStatus {
    Resumed(DownloadResumedStatus),
    Paused,
    // we don't need StdError here because this is only used to shows to user
    Error(String),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadStatus {
    InProgress(DownloadProgressStatus),
    Finished,
}

impl DownloadStatus {
    pub fn is_resumed(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Resumed(..)))
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Paused))
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Finished)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Error(..)))
    }

    pub fn resumed(status: DownloadResumedStatus) -> Self {
        Self::InProgress(DownloadProgressStatus::Resumed(status))
    }

    pub fn paused() -> Self {
        Self::InProgress(DownloadProgressStatus::Paused)
    }

    pub fn error<S: std::fmt::Display>(error: S) -> Self {
        Self::InProgress(DownloadProgressStatus::Error(error.to_string()))
    }

    pub fn finished() -> Self {
        Self::Finished
    }
}

#[derive(Clone)]
pub enum LateBindingModule {
    Module(ArcMadoModule),
    WaitModule(ArcMadoModuleMap, Uuid),
}

impl std::fmt::Debug for LateBindingModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LateBindingModule::WaitModule(_, uuid) => f
                .debug_struct("LateBindingModule")
                .field("uuid", uuid)
                .finish(),
            LateBindingModule::Module(module) => f
                .debug_struct("LateBindingModule")
                .field("module", module)
                .finish(),
        }
    }
}

impl LateBindingModule {
    pub async fn wait(&mut self) -> ArcMadoModule {
        match self {
            LateBindingModule::Module(module) => module.clone(),
            LateBindingModule::WaitModule(map, uuid) => {
                let module = loop {
                    let module = map.get_by_uuid(*uuid);
                    if let Some(module) = module {
                        break module;
                    }

                    crate::timer::sleep_secs(1).await;
                };

                *self = Self::Module(module.clone());
                module
            }
        }
    }

    pub fn uuid(&self) -> Uuid {
        match self {
            LateBindingModule::Module(module) => module.uuid(),
            LateBindingModule::WaitModule(_, uuid) => *uuid,
        }
    }
}

#[derive(Debug)]
pub struct ModuleInfo {
    uuid: Uuid,
    module: AsyncMutex<LateBindingModule>,
}

impl ModuleInfo {
    pub fn new(module: LateBindingModule) -> Self {
        let uuid = module.uuid();
        Self {
            uuid,
            module: AsyncMutex::new(module),
        }
    }

    pub async fn lock(&self) -> AsyncMutexGuard<'_, LateBindingModule> {
        self.module.lock().await
    }
}

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
        &self.module.uuid
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

pub trait DownloadInfoObserver: std::fmt::Debug {
    fn on_status_changed(&self, status: &DownloadStatus);
}

type ArcDownloadInfoObserver = Arc<dyn DownloadInfoObserver + Send + Sync>;

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
