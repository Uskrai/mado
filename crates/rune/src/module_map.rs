/*
 *  Copyright (c) 2021 Uskrai
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use mado_core::url::Url;
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
    self
      .map
      .insert(module.get_domain().clone().into(), Arc::new(module));
  }

  pub fn get(&self, mut url: Url) -> Option<Arc<WebsiteModule>> {
    url.set_path("");
    url.set_query(None);

    self.map.get(&url).cloned()
  }
}
