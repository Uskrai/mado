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
mod manga_info;

use gtk::prelude::*;
use mado_core::Error;
use mado_core::{url::Url, ChapterInfo};
use mado_rune::{WebsiteModule, WebsiteModuleMap};
use relm4::{send, ComponentUpdate, Components, Model, RelmComponent, Widgets};
use tokio::task::JoinHandle;

use std::cell::{RefCell, RefMut};
use std::sync::Arc;

use chapter_list::ChapterListMsg;

type Msg = MangaInfoMsg;

#[derive(Debug, Clone)]
struct ChapterInfoWidget {
  root: gtk::Label,
}

pub struct MangaInfoModel {
  modules: Arc<WebsiteModuleMap>,
}

impl Model for MangaInfoModel {
  type Msg = MangaInfoMsg;
  type Widgets = MangaInfoWidgets;
  type Components = MangaInfoComponents;
}

pub trait HasWebsiteModuleMap {
  fn get_website_module_map(&self) -> Arc<WebsiteModuleMap>;
}

#[relm4_macros::widget(pub)]
impl<ParentModel> Widgets<MangaInfoModel, ParentModel> for MangaInfoWidgets
where
  ParentModel: Model + HasWebsiteModuleMap,
{
  view! {
    gtk::Box {
      set_orientation: gtk::Orientation::Vertical,
      append = &gtk::Box {
        set_orientation: gtk::Orientation::Horizontal,
        append : url_entry = &gtk::Entry {
          // make the entry fill width
          set_hexpand: true,
          set_placeholder_text: Some("Enter Manga URL here"),
          // when user press enter, emit activate to enter button
          // using emit_activate instead of emit_clicked because
          // it doesn't animate the "press"
          connect_activate(enter_button) => move |_| {
            enter_button.emit_activate();
          }
        },
        // enter button
        append : enter_button = &gtk::Button {
          set_label: "⏎",
          connect_clicked(sender, url_entry) => move |_| {
            send!(sender, Msg::GetInfo(url_entry.text().to_string()))
          }
        }
      },

      append: component!(components.chapters.root_widget()),
      append = &gtk::Box {
        set_orientation: gtk::Orientation::Horizontal,

        append: download_path = &gtk::Entry {
          set_hexpand: true,
          set_placeholder_text: Some("Enter Download Path"),
        },

        append: download_button = &gtk::Button {
          set_label: "Download",
          connect_clicked(sender) => move |_| {
            send!(sender, Msg::Download);
          }
        }
      }
    }
  }
}

impl MangaInfoModel {
  pub fn spawn_get_info(
    &self,
    components: &MangaInfoComponents,
    sender: relm4::Sender<Msg>,
    url: String,
  ) {
    let url = url.trim();

    // don't do anything when empty
    if url.is_empty() {
      return;
    }

    let result = self.get_module(&url);

    match result {
      Ok((url, module)) => {
        components.set_url(url.as_str());

        // store spawn closure first because we want to abort previous
        // JoinHandle first before spawn task
        let spawn = || {
          let sender = sender.clone();

          tokio::spawn(async move {
            use mado_core::WebsiteModule;
            let manga = module.get_info(url).await;

            match manga {
              Ok(manga) => {
                send!(sender, Msg::Update(manga));
              }
              Err(err) => {
                send!(sender, Msg::ShowError(err));
              }
            }
          })
        };

        // clear previous info
        send!(sender, Msg::Clear);

        let mut cell = components.get_cell_mut();

        cell.info_handle = cell
          .info_handle
          .as_ref()
          // abort previous get info first if exist
          .and_then(|v| {
            v.abort();
            None
          })
          // then get  handle
          .or_else(|| Some(spawn()));
      }
      Err(err) => {
        send!(sender, Msg::ShowError(err));
      }
    }
    //
  }
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

impl<T> ComponentUpdate<T> for MangaInfoModel
where
  T: Model + HasWebsiteModuleMap,
{
  fn init_model(parent_model: &T) -> Self {
    Self {
      modules: parent_model.get_website_module_map(),
    }
  }

  fn update(
    &mut self,
    msg: Self::Msg,
    components: &Self::Components,
    sender: relm4::Sender<Self::Msg>,
    _parent_sender: relm4::Sender<T::Msg>,
  ) {
    match msg {
      Msg::Download => {
        //
      }
      Msg::GetInfo(url) => {
        self.spawn_get_info(components, sender, url);
      }
      Msg::Update(manga) => {
        for it in manga.chapters {
          let sender = components.chapters.sender();
          send!(sender, ChapterListMsg::Push(it));
        }
      }
      Msg::Clear => {
        let sender = components.chapters.sender();
        send!(sender, ChapterListMsg::Clear);
      }

      Msg::ShowError(error) => {
        gtk::MessageDialog::builder()
          .message_type(gtk::MessageType::Error)
          .text(&error.to_string())
          .transient_for(&components.get_toplevel())
          .build()
          .show();
      }
    }
  }
}

impl MangaInfoWidgets {
  pub fn get_toplevel(&self) -> gtk::Widget {
    self.enter_button.clone().upcast::<gtk::Widget>()
  }
}

impl MangaInfoModel {
  fn get_module(&self, link: &str) -> Result<(Url, Arc<WebsiteModule>), Error> {
    let url = mado_core::url::fill_host(link)?;

    let module = self.modules.search_module(url.clone());

    match module {
      Some(module) => Ok((url, module)),
      None => Err(Error::UnsupportedUrl(link.to_string())),
    }
  }
}

#[derive(Default, Debug)]
pub struct MangaInfoCell {
  info_handle: Option<JoinHandle<()>>,
}

pub struct MangaInfoComponents {
  url_entry: gtk::Entry,
  chapters: RelmComponent<chapter_list::ChapterListModel, MangaInfoModel>,
  cell: RefCell<MangaInfoCell>,
}

impl MangaInfoComponents {
  pub fn push_chapter(&self, chapter: ChapterInfo) {
    let sender = self.chapters.sender();
    send!(sender, ChapterListMsg::Push(chapter));
  }

  pub fn extend_chapter(&self, chapters: Vec<ChapterInfo>) {
    let sender = self.chapters.sender();
    send!(sender, ChapterListMsg::Extend(chapters));
  }

  pub fn get_cell_mut(&self) -> RefMut<MangaInfoCell> {
    self.cell.borrow_mut()
  }
}

impl Components<MangaInfoModel> for MangaInfoComponents {
  fn init_components(
    parent: &MangaInfoModel,
    widget: &MangaInfoWidgets,
    sender: relm4::Sender<Msg>,
  ) -> Self {
    Self {
      url_entry: widget.url_entry.clone(),
      chapters: RelmComponent::new(parent, widget, sender),
      cell: RefCell::default(),
    }
  }
}

impl MangaInfoComponents {
  pub fn get_toplevel(&self) -> gtk::Window {
    crate::get_toplevel(self.url_entry.clone().upcast())
  }

  pub fn get_url(&self) -> String {
    self.url_entry.text().to_string()
  }

  pub fn set_url(&self, url: &str) {
    self.url_entry.set_text(url);
  }
}

#[allow(dead_code)]
fn debug_controller(model: gio::ListModel) {
  let mut i = 0;
  while let Some(item) = model.item(i) {
    i += 1;
    if let Some(controller) = item.downcast_ref::<gtk::ShortcutController>() {
      dbg!(controller.name().unwrap());
      let mut j = 0;
      while let Some(item) = controller.item(j) {
        j += 1;
        if let Some(item) = item.downcast_ref::<gtk::Shortcut>() {
          if let Some(trigger) = item.trigger() {
            dbg!(trigger.to_str());
          }
          if let Some(action) = item.action() {
            dbg!(action.to_str());
          }
        }
      }
    }
  }
}
