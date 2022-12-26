use std::{ops::Index, sync::Arc};

use mado_core::{
    ArcMadoModule, ArcMadoModuleMap, DefaultMadoModuleMap, MutMadoModuleMap, MutexMadoModuleMap,
};
use parking_lot::{RwLock, RwLockReadGuard};

use crate::{DownloadInfo, DownloadRequest, Observers, DownloadOption};

#[derive(Default)]
pub struct DownloadTaskList {
    tasks: Vec<Arc<DownloadInfo>>,
    max_order: usize,
}

impl DownloadTaskList {
    pub fn push(&mut self, value: Arc<DownloadInfo>) {
        self.max_order = value.order().max(self.max_order);
        self.tasks.push(value);
    }

    pub fn iter(&self) -> std::slice::Iter<Arc<DownloadInfo>> {
        self.tasks.iter()
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Index<usize> for DownloadTaskList {
    type Output = Arc<DownloadInfo>;

    fn index(&self, index: usize) -> &Self::Output {
        self.tasks.index(index)
    }
}

#[derive(Default)]
pub struct MadoEngineState {
    modules: Arc<MutexMadoModuleMap<DefaultMadoModuleMap>>,
    tasks: RwLock<DownloadTaskList>,
    observers: Observers<BoxObserver>,
    option: DownloadOption,
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
        tasks: DownloadTaskList,
        option: DownloadOption,
    ) -> Self {
        let tasks = RwLock::new(tasks);

        Self {
            modules,
            tasks,
            observers: Default::default(),
            option,
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

    pub fn option(&self) -> DownloadOption {
        self.option.clone()
    }

    pub fn download_request(&self, request: DownloadRequest) {
        let mut tasks = self.tasks.write();
        let info = Arc::new(DownloadInfo::from_request(
            tasks.max_order + 1,
            request,
            self.option(),
        ));
        tasks.push(info.clone());

        self.observers
            .emit(move |it| it(MadoEngineStateMsg::Download(&info)));
    }

    /// Connect observer to state.
    ///
    /// This will also call on_* of previously pushed item.
    pub fn connect(&self, mut observer: ImplObserver!()) -> crate::ObserverHandle<BoxObserver> {
        for it in self.tasks().iter() {
            observer(MadoEngineStateMsg::Download(it));
        }

        for it in self.modules.lock().unwrap().vec() {
            observer(MadoEngineStateMsg::PushModule(it));
        }

        self.connect_only(observer)
    }

    /// Connect without calling on_* method.
    pub fn connect_only(&self, observer: ImplObserver!()) -> crate::ObserverHandle<BoxObserver> {
        self.observers.connect(Box::new(observer))
    }

    pub fn tasks(&self) -> RwLockReadGuard<'_, DownloadTaskList> {
        self.tasks.read()
    }
}

#[cfg(test)]
mod tests {
    use mado_core::Uuid;
    use mockall::automock;

    use crate::DownloadRequestStatus;

    use super::*;
    use mado_core::MockMadoModule;

    #[automock]
    pub trait Call {
        #[allow(clippy::needless_lifetimes)]
        fn handle_msg<'a>(&self, msg: MadoEngineStateMsg<'a>);
    }

    #[test]
    fn connect_test() {
        let state = MadoEngineState::default();
        assert!(state.tasks().is_empty());

        state
            .connect(|_| {
                unreachable!();
            })
            .disconnect();

        state.connect_only(|_| unreachable!()).disconnect();

        let mut module = MockMadoModule::new();
        let uuid = Uuid::from_u128(1);
        module.expect_uuid().times(0..).returning({
            let uuid = uuid;
            move || uuid
        });
        module
            .expect_domain()
            .times(0..)
            .return_const(mado_core::Url::parse("http://localhost").unwrap());

        let module = Arc::new(module);
        state.push_module(module.clone()).unwrap();

        let mut it = MockCall::new();
        it.expect_handle_msg().times(1).return_const(());
        state
            .connect(move |msg| {
                match msg {
                    MadoEngineStateMsg::Download(_) => unreachable!(),
                    MadoEngineStateMsg::PushModule(_) => it.handle_msg(msg),
                };
            })
            .disconnect();

        state.download_request(DownloadRequest::new(
            module,
            Default::default(),
            Default::default(),
            Default::default(),
            Some(mado_core::Url::parse("http://localhost").unwrap()),
            DownloadRequestStatus::Resume,
        ));

        let mut it = MockCall::new();
        it.expect_handle_msg()
            .times(1)
            .withf(|it| matches!(it, MadoEngineStateMsg::PushModule(_)))
            .return_const(());

        it.expect_handle_msg()
            .times(1)
            .withf(|it| match it {
                MadoEngineStateMsg::Download(download) => {
                    return download.url()
                        == Some(&mado_core::Url::parse("http://localhost").unwrap())
                }
                _ => false,
            })
            .return_const(());

        state
            .connect(move |msg| {
                it.handle_msg(msg);
            })
            .disconnect();
    }
}
