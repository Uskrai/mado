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

mod chapter_list;
mod components;
mod model;
mod widgets;

pub use components::MangaInfoComponents;
use mado_rune::WebsiteModuleMap;
pub use model::MangaInfoModel;
pub use widgets::MangaInfoWidgets;

type Msg = MangaInfoMsg;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct ChapterInfoWidget {
  root: gtk::Label,
}

pub trait HasWebsiteModuleMap {
  fn get_website_module_map(&self) -> Arc<WebsiteModuleMap>;
}

#[derive(Debug)]
pub enum MangaInfoMsg {
  Download,
  ShowError(mado_core::Error),
  /// Get info from string
  /// string should be convertible to URL
  GetInfo(String),
  Update(mado_core::MangaInfo),
  Clear,
}
