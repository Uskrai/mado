use std::sync::Arc;

use super::{AppComponents, AppWidgets};
use crate::manga_info::MangaInfoParentModel;
use mado_core::{ArcWebsiteModule, ArcWebsiteModuleMap, MutexWebsiteModuleMap, WebsiteModuleMap};
use relm4::{AppUpdate, Model};

pub enum AppMsg {
    Exit,
    PushModule(ArcWebsiteModule),
}

pub struct AppModel<Map: WebsiteModuleMap> {
    modules: Arc<MutexWebsiteModuleMap<Map>>,
}

impl<Map: WebsiteModuleMap> AppModel<Map> {
    pub fn new(map: Map) -> Self {
        Self {
            modules: Arc::new(MutexWebsiteModuleMap::new(map)),
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
