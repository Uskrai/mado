use crate::{
    core::{ChapterInfo, MangaInfo, Url, Uuid},
    path::Utf8PathBuf,
    ArcMadoModule, DownloadChapterInfo, DownloadOption, DownloadProgressStatus,
    DownloadResumedStatus, DownloadStatus, LateBindingModule, ModuleInfo, ObserverHandle,
    Observers,
};
use parking_lot::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};
use typed_builder::TypedBuilder;

macro_rules! ImplObserver {
    () => {
        impl FnMut(DownloadInfoMsg) + Send + 'static

    }
}

pub type BoxObserver = Box<dyn FnMut(DownloadInfoMsg) + Send + 'static>;

#[derive(Debug, TypedBuilder)]
pub struct DownloadInfo {
    #[builder(setter(into))]
    order: AtomicUsize,
    #[builder(setter(into))]
    module: ModuleInfo,
    #[builder(setter(into))]
    status: Mutex<DownloadStatus>,

    #[builder(setter(into), default)]
    path: Utf8PathBuf,
    #[builder(setter(into), default)]
    manga_title: String,
    #[builder(default)]
    url: Option<Url>,
    #[builder(default)]
    chapters: Vec<Arc<DownloadChapterInfo>>,
    #[builder(default)]
    observers: Observers<BoxObserver>,
}

pub enum DownloadInfoMsg<'a> {
    StatusChanged(&'a DownloadStatus),
    OrderChanged(usize),
}

impl DownloadInfo {
    /// Create new Download info.
    pub fn new(
        order: usize,
        module: LateBindingModule,
        title: String,
        chapters: Vec<Arc<DownloadChapterInfo>>,
        path: Utf8PathBuf,
        url: Option<Url>,
        status: DownloadStatus,
    ) -> Self {
        Self {
            order: order.into(),
            module: module.into(),
            manga_title: title,
            chapters,
            path,
            url,
            status: Mutex::new(status),
            observers: Default::default(),
        }
    }

    pub fn from_request(order: usize, request: DownloadRequest, option: DownloadOption) -> Self {
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
                let path = path.join(&option.sanitize_filename(&title));
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
            order,
            LateBindingModule::Module(module),
            manga.title.clone(),
            chapters,
            path,
            url,
            DownloadStatus::InProgress(status.into()),
        )
    }

    pub fn order(&self) -> usize {
        self.order.load(atomic::Ordering::Relaxed)
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
        self.module.uuid()
    }

    pub fn manga_title(&self) -> &str {
        &self.manga_title
    }

    pub fn module_domain(&self) -> Option<&str> {
        self.url.as_ref().and_then(|url| url.domain())
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

    pub fn set_order(&self, order: usize) {
        self.order.store(order, atomic::Ordering::Relaxed);
        self.observers
            .emit(|it| it(DownloadInfoMsg::OrderChanged(order)));
    }

    /// Change download's status, then emit [`DownloadInfoObserver::on_status_changed`]
    #[tracing::instrument]
    pub fn set_status(&self, status: DownloadStatus) {
        tracing::trace!("setting status to {:?}", status);
        let mut lock = self.status.lock();
        *lock = status;
        self.observers
            .emit(|it| it(DownloadInfoMsg::StatusChanged(&lock)));
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
    pub fn connect(&self, mut observer: ImplObserver!()) -> ObserverHandle<BoxObserver> {
        observer(DownloadInfoMsg::StatusChanged(&self.status()));
        observer(DownloadInfoMsg::OrderChanged(self.order()));

        self.connect_only(observer)
    }

    /// Connect without sending current state
    pub fn connect_only(&self, observer: ImplObserver!()) -> ObserverHandle<BoxObserver> {
        self.observers.connect(Box::new(observer))
    }
}

#[derive(Debug)]
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

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    pub fn chapters(&self) -> &[Arc<ChapterInfo>] {
        self.chapters.as_ref()
    }

    pub fn module(&self) -> &ArcMadoModule {
        &self.module
    }
}

#[cfg(test)]
mod tests {
    use mado_core::MockMadoModule;

    use mado_core::DefaultMadoModuleMap;
    use mockall::predicate;

    use super::*;

    mockall::mock! {
        pub Thing {
            fn on_status_changed(&self, status: &DownloadStatus);
            fn on_download(&self, info: &DownloadStatus);
            fn on_order_changed(&self, index: usize);
        }
    }

    impl MockThing {
        fn handle(&self, msg: DownloadInfoMsg<'_>) {
            match msg {
                DownloadInfoMsg::StatusChanged(status) => self.on_status_changed(status),
                DownloadInfoMsg::OrderChanged(index) => self.on_order_changed(index),
            }
        }

        fn handler(self) -> impl FnMut(DownloadInfoMsg<'_>) + Send + 'static {
            move |msg: DownloadInfoMsg<'_>| self.handle(msg)
        }
    }

    #[test]
    fn download_observer() {
        let info = DownloadInfo::builder()
            .order(0)
            .module(LateBindingModule::WaitModule(
                Arc::new(DefaultMadoModuleMap::new()),
                Default::default(),
            ))
            .status(DownloadStatus::paused())
            .build();

        {
            let mut mock = MockThing::default();
            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::paused()))
                .returning(|_| ());
            mock.expect_on_order_changed()
                .once()
                .with(predicate::eq(0))
                .returning(|_| ());

            let _ = info.connect(mock.handler()).disconnect().unwrap();
        }

        {
            let mut mock = MockThing::new();
            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::paused()))
                .returning(|_| ());
            mock.expect_on_order_changed()
                .once()
                .with(predicate::eq(0))
                .returning(|_| ());

            mock.expect_on_status_changed()
                .once()
                .with(predicate::eq(DownloadStatus::waiting()))
                .returning(|_| ());

            mock.expect_on_order_changed()
                .once()
                .with(predicate::eq(1))
                .returning(|_| ());

            let handle = info.connect(mock.handler());

            info.set_status(DownloadStatus::waiting());
            info.set_order(1);
            let _ = handle.disconnect().unwrap();
            info.set_status(DownloadStatus::finished());
            info.set_order(2);
        }

        {
            let mut mock = MockThing::new();
            mock.expect_on_status_changed().never();
            mock.expect_on_order_changed().never();
            let _ = info.connect_only(mock.handler()).disconnect().unwrap();
        }
    }

    #[test]
    fn test_request() {
        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(1));
        let url = Url::parse("https://localhost").unwrap();
        module.expect_domain().return_const(url.clone());

        let download = DownloadInfo::from_request(
            0,
            DownloadRequest::new(
                Arc::new(module),
                Arc::new(MangaInfo::default()),
                vec![Default::default()],
                Default::default(),
                Some(url.clone()),
                DownloadRequestStatus::Resume,
            ),
            Default::default(),
        );

        assert_eq!(download.url(), Some(&url));
        assert_eq!(*download.module_uuid(), Uuid::from_u128(1));
    }

    #[test]
    fn test_resume() {
        let info = DownloadInfo::builder()
            .order(0)
            .module(LateBindingModule::WaitModule(
                Arc::new(DefaultMadoModuleMap::new()),
                Default::default(),
            ))
            .manga_title("Title")
            .path("path")
            .status(DownloadStatus::paused())
            .build();
        let info = Arc::new(info);

        info.resume(true);
        assert!(info.status().is_resumed());
        info.resume(false);
        assert!(info.status().is_paused());

        info.set_status(DownloadStatus::finished());
        assert!(info.status().is_finished());
        info.resume(true);
        assert!(info.status().is_finished());
        info.resume(false);
        assert!(info.status().is_finished());
    }
}
