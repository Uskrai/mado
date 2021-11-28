use mado_core::{url::Url, WebsiteModule as _};
use std::{collections::HashMap, sync::Arc};

use crate::WebsiteModule;

#[derive(Default, Clone)]
pub struct WebsiteModuleMap {
    // map to domain and index inside `modules`
    // the modules order shouldn't be changed
    map: HashMap<Url, Arc<WebsiteModule>>,
}

impl WebsiteModuleMap {
    pub fn insert(&mut self, module: WebsiteModule) {
        let module = Arc::new(module);
        self.map.insert(module.get_domain(), module.clone());
    }

    pub fn get(&self, mut url: Url) -> Option<Arc<WebsiteModule>> {
        url.set_path("");
        url.set_query(None);

        self.map.get(&url).cloned()
    }
}
