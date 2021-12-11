use parking_lot::Mutex;
use std::sync::Arc;

use atomic::Atomic;
use mado_core::{ArcMadoModule, ChapterInfo, MangaInfo};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Eq, Ord)]
pub enum DownloadStatus {
    Resumed,
    Paused,
}

#[derive(Debug)]
pub struct DownloadInfo {
    module: ArcMadoModule,
    manga: Arc<MangaInfo>,
    chapters: Vec<Arc<ChapterInfo>>,
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

        Self {
            module,
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

    /// Get a reference to the download info's module.
    pub fn module(&self) -> &ArcMadoModule {
        &self.module
    }

    /// Get a reference to the downloaded chapters.
    pub fn chapters(&self) -> &[Arc<ChapterInfo>] {
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
