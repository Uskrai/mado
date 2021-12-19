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

type DynMadoEngineStateObserver = dyn MadoEngineStateObserver + Send + Sync + 'static;
type BoxedMadoEngineStateObserver = Box<DynMadoEngineStateObserver>;

impl MadoEngineState {
    pub fn modules(&self) -> ArcMadoModuleMap {
        self.modules.clone()
    }
    pub fn push_module(&self, module: ArcMadoModule) -> Result<(), mado_core::MadoModuleMapError> {
        self.modules.push_mut(module.clone())?;
        self.emit(move |it| it.on_push_module(module.clone()));
        Ok(())
    }

    pub fn download_request(&self, request: DownloadRequest) {
        let info = Arc::new(DownloadInfo::new(request));
        self.tasks.write().push(info.clone());
        self.emit(move |it| it.on_download(info.clone()));
    }

    pub fn connect(&self, observer: impl MadoEngineStateObserver + Send + Sync + 'static) {
        let observer = Box::new(observer);

        for it in self.tasks.write().iter() {
            observer.on_download(it.clone());
        }

        for it in self.modules.lock().unwrap().vec() {
            observer.on_push_module(it.clone());
        }

        self.observers.lock().push(observer);
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
