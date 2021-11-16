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

use std::sync::Arc;

use gtk::prelude::*;
use mado_rune::WebsiteModuleMap;
use relm4::{AppUpdate, Components, Model, RelmComponent, Widgets};

use crate::manga_info::{self, MangaInfoParentModel};

pub struct App {
  pub modules: WebsiteModuleMap,
}

pub enum AppMsg {
  Increment,
  Exit,
}

pub struct AppModel {
  pub modules: Arc<WebsiteModuleMap>,
}

impl MangaInfoParentModel for AppModel {
  fn get_website_module_map(&self) -> Arc<WebsiteModuleMap> {
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
      AppMsg::Increment => {
        println!("Incremented");
      }
    }
    true
  }
}

pub struct AppComponents {
  manga_info: RelmComponent<manga_info::MangaInfoModel, AppModel>,
}

impl Components<AppModel> for AppComponents {
  fn init_components(
    parent_model: &AppModel,
    parent_widget: &AppWidgets,
    parent_sender: relm4::Sender<AppMsg>,
  ) -> Self {
    Self {
      manga_info: RelmComponent::new(
        parent_model,
        parent_widget,
        parent_sender,
      ),
    }
  }
}

#[relm4_macros::widget(pub)]
impl Widgets<AppModel, ()> for AppWidgets {
  view! {
    gtk::ApplicationWindow {
      set_title: Some("Mado"),
      set_child = Some(&gtk::Box) {
        set_orientation: gtk::Orientation::Vertical,

        append = &gtk::StackSwitcher {
          set_stack: Some(&stack)
        },

        // Download tab
        append: stack = &gtk::Stack {
          add_titled(Some("Download"), "Download") = &gtk::Frame {
            set_child = Some(&gtk::Box) {
              set_orientation: gtk::Orientation::Vertical,
              append = &gtk::Button {
                set_label: "Yer download"
              }
            }
          },
          // Manga Info tab
          add_titled(Some("Manga Info"), "Manga Info") = &gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            append: component!(components.manga_info.root_widget())
          },
          set_visible_child_name: component!("Manga Info"),
        },

      }
    }
  }
}
