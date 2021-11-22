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
  let mut modules = mado_rune::WebsiteModuleMap::default();

  let load_module = |path: &std::path::Path| -> Result<
    Vec<mado_rune::WebsiteModule>,
    Box<dyn std::error::Error>,
  > {
    mado_rune::Build::default()
      .with_path(path)?
      .build_for_module()?
      .error_missing_load_module(false)
      .build()
      .map_err(Into::into)
  };
  for it in std::fs::read_dir("../rune/script").unwrap() {
    let it = it.unwrap();
    if it.path().is_file() {
      let vec = load_module(&it.path());

      match vec {
        Ok(vec) => {
          for it in vec {
            modules.insert(it);
          }
        }

        Err(err) => {
          println!("{}", err);
        }
      }
    }
  }

  let model = mado_relm::AppModel {
    modules: Arc::new(modules),
  };

  let app = RelmApp::new(model);
  app.run()
}
