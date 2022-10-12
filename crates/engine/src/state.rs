use std::sync::Arc;

use mado_core::{
    ArcMadoModule, ArcMadoModuleMap, DefaultMadoModuleMap, MutMadoModuleMap, MutexMadoModuleMap,
};
use parking_lot::{RwLock, RwLockReadGuard};

use crate::{DownloadInfo, DownloadRequest, Observers};

#[derive(Default)]
pub struct MadoEngineState {
    modules: Arc<MutexMadoModuleMap<DefaultMadoModuleMap>>,
    tasks: RwLock<Vec<Arc<DownloadInfo>>>,
    observers: Observers<BoxObserver>,
}

macro_rules! ImplObserver {
    () => {
        impl FnMut(MadoEngineStateMsg<'_>) + Send + 'static
    }
}

pub type BoxObserver = Box<dyn FnMut(MadoEngineStateMsg<'_>) + Send>;

pub enum MadoEngineStateMsg<'a> {
    Download(&'a Arc<DownloadInfo>),
    PushModule(&'a ArcMadoModule),
}

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
        self.observers
            .emit(move |it| it(MadoEngineStateMsg::PushModule(&module)));
        Ok(())
    }

    pub fn download_request(&self, request: DownloadRequest) {
        let info = Arc::new(DownloadInfo::from_request(request));
        self.tasks.write().push(info.clone());
        self.observers
            .emit(move |it| it(MadoEngineStateMsg::Download(&info)));
    }

    /// Connect observer to state.
    ///
    /// This will also call on_* of previously pushed item.
    pub fn connect(&self, mut observer: ImplObserver!()) {
        for it in self.tasks.write().iter() {
            observer(MadoEngineStateMsg::Download(it));
        }

        for it in self.modules.lock().unwrap().vec() {
            observer(MadoEngineStateMsg::PushModule(it));
        }

        self.connect_only(observer);
    }

    /// Connect without calling on_* method.
    pub fn connect_only(&self, observer: ImplObserver!()) -> crate::ObserverHandle<BoxObserver> {
        self.observers.connect(Box::new(observer))
    }

    pub fn tasks(&self) -> RwLockReadGuard<'_, Vec<Arc<DownloadInfo>>> {
        self.tasks.read()
    }
}
