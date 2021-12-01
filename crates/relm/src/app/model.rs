use super::{AppComponents, AppWidgets};
use crate::manga_info::MangaInfoParentModel;
use mado_core::{
    ArcWebsiteModule, ArcWebsiteModuleMap, MutWebsiteModuleMap, MutexWebsiteModuleMap,
    WebsiteModuleMap,
};
use mado_engine::{MadoEngineState, MadoSender};
use relm4::{AppUpdate, Model};
use std::sync::Arc;

pub enum AppMsg {
    PushModule(ArcWebsiteModule),
}

pub struct AppModel<Map: WebsiteModuleMap> {
    modules: Arc<MutexWebsiteModuleMap<Map>>,
    /// state.send will be called on [`AppComponents::init_components`]
    pub(super) state: Arc<MadoEngineState>,
}

impl<Map: WebsiteModuleMap> AppModel<Map> {
    pub fn new(map: Map, state: Arc<MadoEngineState>) -> Self {
        Self {
            modules: std::sync::Arc::new(MutexWebsiteModuleMap::new(map)),
            state,
        }
    }
}

impl<Map: WebsiteModuleMap> MangaInfoParentModel for AppModel<Map> {
    fn get_website_module_map(&self) -> ArcWebsiteModuleMap {
        self.modules.clone()
    }
}

impl<Map: WebsiteModuleMap> Model for AppModel<Map> {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents<Map>;
}

impl<Map: WebsiteModuleMap> AppUpdate for AppModel<Map> {
    #[tracing::instrument(skip_all)]
    fn update(
        &mut self,
        msg: Self::Msg,
        _: &Self::Components,
        _: relm4::Sender<Self::Msg>,
    ) -> bool {
        match msg {
            AppMsg::PushModule(module) => {
                tracing::trace!(
                    "Pushing module domain:{}, uuid:{}",
                    module.get_domain(),
                    module.get_uuid()
                );
                self.modules.push_mut(module).unwrap();
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct RelmMadoSender {
    sender: relm4::Sender<AppMsg>,
}

impl RelmMadoSender {
    pub fn new(sender: relm4::Sender<AppMsg>) -> Self {
        Self { sender }
    }
}

impl MadoSender for RelmMadoSender {
    fn push_module(&self, module: ArcWebsiteModule) {
        self.sender.send(AppMsg::PushModule(module)).unwrap();
    }
}
