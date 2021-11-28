use std::sync::Mutex;

use mado_core::{ArcWebsiteModule, WebsiteModuleMap};

mod model;
pub use model::*;

mod components;
pub use components::*;

mod widgets;
pub use widgets::*;

pub struct MutexWebsiteModuleMap<Map: WebsiteModuleMap> {
    map: Mutex<Map>,
}

impl<Map: WebsiteModuleMap> MutexWebsiteModuleMap<Map> {
    pub fn new(map: Map) -> Self {
        Self {
            map: Mutex::new(map),
        }
    }
}

impl<Map: WebsiteModuleMap> WebsiteModuleMap for MutexWebsiteModuleMap<Map> {
    fn get_by_uuid(&self, uuid: mado_core::Uuid) -> Option<ArcWebsiteModule> {
        self.map.lock().unwrap().get_by_uuid(uuid)
    }

    fn get_by_url(&self, url: mado_core::url::Url) -> Option<ArcWebsiteModule> {
        self.map.lock().unwrap().get_by_url(url)
    }

    fn push(&mut self, module: ArcWebsiteModule) {
        self.map.lock().unwrap().push(module)
    }
}
