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

use std::cell::Cell;

pub use gtk::prelude::*;
pub use gtk::subclass::prelude::*;
use mado_core::ChapterInfo;
use relm4::{ComponentUpdate, Model, Widgets};

#[derive(Debug)]
pub struct ChapterListModel {
  chapters: gio::ListStore,
}

pub enum ChapterListMsg {
  Push(ChapterInfo),
  Extend(Vec<ChapterInfo>),
  Clear,
}

impl Model for ChapterListModel {
  type Msg = ChapterListMsg;
  type Widgets = ChapterListWidgets;
  type Components = ();
}

#[derive(Default, Debug)]
pub struct CheckChapterInfo {
  info: ChapterInfo,
  active: Cell<bool>,
}

impl From<ChapterInfo> for CheckChapterInfo {
  fn from(info: ChapterInfo) -> Self {
    Self {
      info,
      active: Cell::default(),
    }
  }
}

crate::gobject::struct_wrapper!(
  GChapterInfo,
  crate::manga_info::chapter_list::CheckChapterInfo,
  "MadoRelmChapterInfo",
  info_wrapper
);
use info_wrapper::GChapterInfo;

fn create_list_store() -> gio::ListStore {
  gio::ListStore::new(gio::glib::types::Type::OBJECT)
}

#[relm4_macros::widget(pub)]
impl<ParentModel: Model> Widgets<ChapterListModel, ParentModel>
  for ChapterListWidgets
{
  view! {
    gtk::ScrolledWindow {
      set_vexpand : true,
      set_child = Some(&gtk::ListView) {
        set_factory = Some(&gtk::SignalListItemFactory) {
          connect_bind:
            ChapterListModel::connect_bind,
        },
        set_model:
          Some(&gtk::MultiSelection::new(Some(&model.chapters))),
        set_single_click_activate: false,

        connect_activate: ChapterListModel::connect_activate,
      },

    }
  }
}

impl ChapterListModel {
  pub fn connect_bind(_: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
    let child = item.item().unwrap().downcast::<GChapterInfo>().unwrap();
    let child = child.borrow();
    if let Some(chapter) = child.as_ref() {
      if let None = item.child() {
        item.set_child(Some(&Self::create_chapter_info(&chapter.info)));
      }

      let child = item.child().unwrap().downcast::<gtk::Grid>().unwrap();
      let check = child
        .child_at(0, 0)
        .unwrap()
        .downcast::<gtk::CheckButton>()
        .unwrap();

      check.set_active(chapter.active.get());
    }
  }

  fn connect_activate(list: &gtk::ListView, _: u32) {
    let model = list.model().unwrap();
    let selection = model.selection();
    Self::for_each(&model, |i, it| {
      if selection.contains(i) {
        let it = it.downcast::<GChapterInfo>().unwrap();
        let it = it.borrow();
        if let Some(it) = it.as_ref() {
          it.active.set(!it.active.get())
        }

        model.selection_changed(i, 1);
      }
    });
  }

  pub fn create_chapter_info(chapter: &ChapterInfo) -> gtk::Widget {
    use std::fmt::Write;
    let mut label = String::new();

    macro_rules! write_if {
      ($name:ident, $fmt:literal) => {
        match &chapter.$name {
          Some(val) => {
            write!(label, $fmt, val).unwrap();
          }
          None => {}
        }
      };
    }

    write_if!(volume, "Vol. {} ");
    write_if!(chapter, "Chapter {}");
    write_if!(title, ": {}");
    write_if!(scanlator, "[{}]");

    let check = gtk::CheckButton::default();
    let label = gtk::Label::builder().label(&label).build();

    let grid = gtk::Grid::builder()
      .orientation(gtk::Orientation::Horizontal)
      .build();

    grid.attach(&check, 0, 0, 1, 1);
    grid.attach(&label, 2, 0, 1, 1);
    grid.set_column_spacing(5);

    grid.upcast()
  }

  fn for_each(
    model: &gtk::SelectionModel,
    call: impl Fn(u32, gtk::glib::Object),
  ) {
    let mut i = 0;
    while let Some(it) = model.item(i) {
      call(i, it);
      i += 1;
    }
  }
}

impl ChapterListModel {
  pub fn push(&self, chapter: ChapterInfo) {
    let gchapter = GChapterInfo::to_gobject(chapter.into());
    self.chapters.append(&gchapter);
  }
}

impl<ParentModel> ComponentUpdate<ParentModel> for ChapterListModel
where
  ParentModel: Model,
{
  fn init_model(_: &ParentModel) -> Self {
    Self {
      chapters: create_list_store(),
    }
  }

  fn update(
    &mut self,
    msg: Self::Msg,
    _: &Self::Components,
    _: relm4::Sender<Self::Msg>,
    _: relm4::Sender<ParentModel::Msg>,
  ) {
    match msg {
      ChapterListMsg::Push(chapter) => {
        self.push(chapter);
      }
      ChapterListMsg::Extend(chapters) => {
        for it in chapters {
          self.push(it);
        }
      }

      ChapterListMsg::Clear => {
        self.chapters.remove_all();
      }
    }
  }
}
