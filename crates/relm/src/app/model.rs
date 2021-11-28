use super::{AppComponents, AppWidgets, MutexWebsiteModuleMap};
use crate::manga_info::MangaInfoParentModel;
use mado_core::{ArcWebsiteModule, ArcWebsiteModuleMap, WebsiteModuleMap};
use relm4::{AppUpdate, Model};

pub enum AppMsg {
    Exit,
    PushModule(ArcWebsiteModule),
}

pub struct AppModel {
    modules: ArcWebsiteModuleMap,
}

impl AppModel {
    pub fn new<Map: WebsiteModuleMap>(map: Map) -> Self {
        Self {
            modules: std::sync::Arc::new(MutexWebsiteModuleMap::new(map)),
        }
    }
}

impl MangaInfoParentModel for AppModel {
    fn get_website_module_map(&self) -> ArcWebsiteModuleMap {
        self.modules.clone()
    }
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    fn update(
        &mut self,
        msg: Self::Msg,
        _: &Self::Components,
        _: relm4::Sender<Self::Msg>,
    ) -> bool {
        match msg {
            AppMsg::Exit => {
                return false;
            }
            AppMsg::PushModule(_) => {
                //
            }
        }
        true
    }
}
