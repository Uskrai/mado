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

use relm4::RelmApp;

use std::sync::Arc;

#[tokio::main]
pub async fn main() {
  let module = mado_rune::WebsiteModuleBuilder::default();
  let mut modules = mado_rune::WebsiteModuleMap::default();
  for it in std::fs::read_dir("../rune/script").unwrap() {
    let it = it.unwrap();
    if it.path().is_file() {
      for it in module.load_path(&it.path()).unwrap() {
        modules.insert(it);
      }
    }
  }

  let model = mado_relm::AppModel {
    modules: Arc::new(modules),
  };

  let app = RelmApp::new(model);
  app.run()
}
