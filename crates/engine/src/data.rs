use crate::path::Utf8PathBuf;
use parking_lot::Mutex;
use std::sync::Arc;

use mado_core::{ArcMadoModule, ChapterInfo, MangaInfo};

use crate::MadoEngineState;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadResumedStatus {
    Waiting,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadProgressStatus {
    Resumed(DownloadResumedStatus),
    Paused,
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
}

#[derive(Clone)]
pub enum LateBindingModule {
    Module(ArcMadoModule),
    ModuleUUID(Arc<MadoEngineState>, mado_core::Uuid),
}

impl std::fmt::Debug for LateBindingModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LateBindingModule::ModuleUUID(_, uuid) => f
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
            LateBindingModule::ModuleUUID(state, uuid) => {
                let module = loop {
                    let module = state.modules().get_by_uuid(*uuid);
                    if let Some(module) = module {
                        break module;
                    }
                    tokio::task::yield_now().await;
                };

                *self = Self::Module(module.clone());
                module
            }
        }
    }
}

#[derive(Debug)]
pub struct DownloadInfo {
    module: tokio::sync::Mutex<LateBindingModule>,
    module_uuid: mado_core::Uuid,
    manga: Arc<MangaInfo>,
    chapters: Vec<Arc<DownloadChapterInfo>>,
    path: Utf8PathBuf,
    domain: mado_core::Url,
    status: Mutex<DownloadStatus>,
    observers: Mutex<Vec<ArcDownloadInfoObserver>>,
}

impl DownloadInfo {
    pub fn new(request: DownloadRequest) -> Self {
        let DownloadRequest {
            module,
            manga,
            chapters,
            path,
            status,
        } = request;

        let chapters = chapters
            .into_iter()
            .map(|it| {
                let path = path.join(it.to_string());
                DownloadChapterInfo::new(
                    LateBindingModule::Module(module.clone()),
                    it,
                    path,
                    DownloadStatus::InProgress(status.into()),
                )
            })
            .map(|it| Arc::new(it))
            .collect();

        let domain = module.get_domain();

        Self {
            module_uuid: module.get_uuid(),
            module: LateBindingModule::Module(module).into(),
            manga,
            chapters,
            path,
            domain,
            status: Mutex::new(DownloadStatus::InProgress(status.into())),
            observers: Mutex::default(),
        }
    }

    /// Get download info's status.
    pub fn status(&self) -> impl std::ops::Deref<Target = DownloadStatus> + '_ {
        self.status.lock()
    }

    /// Get a reference to the download info's path.
    pub fn path(&self) -> &Utf8PathBuf {
        &self.path
    }

    pub fn module_uuid(&self) -> &mado_core::Uuid {
        &self.module_uuid
    }

    pub fn domain(&self) -> &mado_core::Url {
        &self.domain
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

    /// Get a reference to the download info's manga.
    pub fn manga(&self) -> &Arc<MangaInfo> {
        &self.manga
    }

    pub fn connect(&self, observer: ArcDownloadInfoObserver) {
        let mut observers = self.observers.lock();

        observer.on_status_changed(&self.status());

        observers.push(observer);
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
    //
}

impl DownloadRequest {
    pub fn new(
        module: ArcMadoModule,
        manga: Arc<MangaInfo>,
        chapters: Vec<Arc<ChapterInfo>>,
        path: Utf8PathBuf,
        status: DownloadRequestStatus,
    ) -> Self {
        Self {
            module,
            manga,
            chapters,
            path,
            status,
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
    chapter: Arc<ChapterInfo>,
    path: Utf8PathBuf,
    status: Mutex<DownloadStatus>,
}

impl DownloadChapterInfo {
    pub fn new(
        module: LateBindingModule,
        chapter: Arc<ChapterInfo>,
        path: Utf8PathBuf,
        status: DownloadStatus,
    ) -> Self {
        Self {
            module,
            chapter,
            path,
            status: Mutex::new(status),
        }
    }

    /// Get a reference to the download chapter info's chapter.
    pub fn chapter(&self) -> &ChapterInfo {
        self.chapter.as_ref()
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
        *self.status.lock() = status;
    }
}
