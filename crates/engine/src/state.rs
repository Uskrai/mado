use std::sync::Arc;

use mado_core::{
    ArcMadoModule, ArcMadoModuleMap, DefaultMadoModuleMap, MutMadoModuleMap, MutexMadoModuleMap,
};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};

use crate::{DownloadInfo, DownloadRequest};

#[derive(Default)]
pub struct MadoEngineState {
    modules: Arc<MutexMadoModuleMap<DefaultMadoModuleMap>>,
    tasks: RwLock<Vec<Arc<DownloadInfo>>>,
    observers: Mutex<Vec<BoxedMadoEngineStateObserver>>,
}

type DynMadoEngineStateObserver = dyn MadoEngineStateObserver;
type BoxedMadoEngineStateObserver = Box<DynMadoEngineStateObserver>;

impl MadoEngineState {
    pub fn new(
        modules: Arc<MutexMadoModuleMap<DefaultMadoModuleMap>>,
        tasks: Vec<Arc<DownloadInfo>>,
    ) -> Self {
        let tasks = RwLock::new(tasks);

        Self {
            modules,
            tasks,
            observers: Default::default(),
        }
    }
    pub fn modules(&self) -> ArcMadoModuleMap {
        self.modules.clone()
    }
    pub fn push_module(&self, module: ArcMadoModule) -> Result<(), mado_core::MadoModuleMapError> {
        self.modules.push_mut(module.clone())?;
        self.emit(move |it| it.on_push_module(module.clone()));
        Ok(())
    }

    pub fn download_request(&self, request: DownloadRequest) {
        let info = Arc::new(DownloadInfo::from_request(request));
        self.tasks.write().push(info.clone());
        self.emit(move |it| it.on_download(info.clone()));
    }

    /// Connect observer to state.
    ///
    /// This will also call on_* of previously pushed item.
    pub fn connect(&self, observer: impl MadoEngineStateObserver) {
        for it in self.tasks.write().iter() {
            observer.on_download(it.clone());
        }

        for it in self.modules.lock().unwrap().vec() {
            observer.on_push_module(it.clone());
        }

        self.connect_only(observer);
    }

    /// Connect without calling on_* method.
    pub fn connect_only(&self, observer: impl MadoEngineStateObserver) {
        self.observers.lock().push(Box::new(observer));
    }

    fn emit(&self, fun: impl Fn(&BoxedMadoEngineStateObserver)) {
        for it in self.observers.lock().iter() {
            fun(it);
        }
    }

    pub fn tasks(&self) -> RwLockReadGuard<'_, Vec<Arc<DownloadInfo>>> {
        self.tasks.read()
    }
}

pub trait MadoEngineStateObserver: Send + Sync + 'static {
    fn on_push_module(&self, module: ArcMadoModule);

    fn on_download(&self, info: Arc<DownloadInfo>);
}
