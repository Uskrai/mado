use std::sync::Arc;

use mado_core::{
    ArcMadoModule, ArcMadoModuleMap, DefaultMadoModuleMap, MutMadoModuleMap, MutexMadoModuleMap,
};
use parking_lot::{Mutex, RwLock};

use crate::DownloadInfo;

#[derive(Debug, Default)]
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
    pub fn push_module(&self, module: ArcMadoModule) {
        match self.modules.push_mut(module.clone()) {
            Ok(_) => self.emit(move |it| it.on_push_module(module.clone())),
            Err(err) => self.emit(move |it| it.on_push_module_fail(err.clone())),
        }
    }

    pub fn download(&self, info: DownloadInfo) {
        let info = Arc::new(info);
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
}

pub trait MadoEngineStateObserver: std::fmt::Debug + Send + Sync + 'static {
    fn on_push_module(&self, module: ArcMadoModule);

    fn on_push_module_fail(&self, error: mado_core::MadoModuleMapError);

    fn on_download(&self, info: Arc<DownloadInfo>);
}
