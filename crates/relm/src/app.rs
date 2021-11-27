use std::sync::Mutex;

use gtk::prelude::*;
use mado_core::{ArcWebsiteModule, ArcWebsiteModuleMap, WebsiteModuleMap};
use relm4::{AppUpdate, Components, Model, RelmComponent, Widgets};

use crate::manga_info::{self, MangaInfoParentModel};

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

pub enum AppMsg {
  Increment,
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
      AppMsg::Increment => {
        println!("Incremented");
      }
      AppMsg::PushModule(_) => {
        //
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
