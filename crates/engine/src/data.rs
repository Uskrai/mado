use parking_lot::Mutex;
use std::sync::Arc;

use atomic::Atomic;
use mado_core::{ArcMadoModule, ChapterInfo, MangaInfo};

use crate::MadoEngineState;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Eq, Ord)]
pub enum DownloadStatus {
    Resumed,
    Paused,
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
    manga: Arc<MangaInfo>,
    chapters: Vec<Arc<DownloadChapterInfo>>,
    path: std::path::PathBuf,
    status: Atomic<DownloadStatus>,
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
                    status,
                )
            })
            .map(|it| Arc::new(it))
            .collect();

        Self {
            module: LateBindingModule::Module(module).into(),
            manga,
            chapters,
            path,
            status: Atomic::new(status),
            observers: Mutex::default(),
        }
    }

    /// Get download info's status.
    pub fn status(&self) -> DownloadStatus {
        self.status.load(atomic::Ordering::SeqCst)
    }

    /// Get a reference to the download info's path.
    pub fn path(&self) -> &std::path::PathBuf {
        &self.path
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
        self.status.store(status, atomic::Ordering::SeqCst);
        self.emit(|it| it.on_status_changed(status));
    }

    /// Get a reference to the download info's manga.
    pub fn manga(&self) -> &Arc<MangaInfo> {
        &self.manga
    }

    pub fn connect(&self, observer: ArcDownloadInfoObserver) {
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
    path: std::path::PathBuf,
    status: DownloadStatus,
}

impl DownloadRequest {
    pub fn new(
        module: ArcMadoModule,
        manga: Arc<MangaInfo>,
        chapters: Vec<Arc<ChapterInfo>>,
        path: std::path::PathBuf,
        status: DownloadStatus,
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
    fn on_status_changed(&self, status: DownloadStatus);
}

type ArcDownloadInfoObserver = Arc<dyn DownloadInfoObserver + Send + Sync>;

#[derive(Debug)]
pub struct DownloadChapterInfo {
    module: LateBindingModule,
    chapter: Arc<ChapterInfo>,
    path: std::path::PathBuf,
    status: Atomic<DownloadStatus>,
}

impl DownloadChapterInfo {
    pub fn new(
        module: LateBindingModule,
        chapter: Arc<ChapterInfo>,
        path: std::path::PathBuf,
        status: DownloadStatus,
    ) -> Self {
        Self {
            module,
            chapter,
            path,
            status: Atomic::new(status),
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
    pub fn path(&self) -> &std::path::PathBuf {
        &self.path
    }

    /// Get a reference to the download chapter info's status.
    pub fn status(&self) -> DownloadStatus {
        self.status.load(atomic::Ordering::SeqCst)
    }
}
