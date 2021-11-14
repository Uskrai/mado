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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub use gtk::prelude::*;
pub use gtk::subclass::prelude::*;
use mado_core::ChapterInfo;
use relm4::{ComponentUpdate, Model, Widgets};

#[derive(Debug)]
pub struct ChapterListModel {
  views: gio::ListStore,
  chapters: RefCell<Vec<Rc<CheckChapterInfo>>>,
  filter: RefCell<Option<FilterFunction>>,
}

pub struct FilterFunction {
  inner: Box<dyn Fn(&ChapterInfo) -> bool>,
}

impl FilterFunction {
  pub fn call(&self, chapter: &ChapterInfo) -> bool {
    (*self.inner)(chapter)
  }
}

impl<T> From<T> for FilterFunction
where
  T: Fn(&ChapterInfo) -> bool + 'static,
{
  fn from(v: T) -> Self {
    Self { inner: Box::new(v) }
  }
}

impl std::fmt::Debug for FilterFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let inner_ptr = &*self.inner as *const dyn Fn(&ChapterInfo) -> bool;
    f.debug_struct("FilterFunction")
      .field(&"inner", &inner_ptr)
      .finish()
  }
}

pub enum ChapterListMsg {
  Push(ChapterInfo),
  Extend(Vec<ChapterInfo>),
  // Filter to Check if ChapterInfo
  // should be displayed if true then
  // displayed
  #[allow(dead_code)]
  Filter(Option<FilterFunction>),
  Clear,
}

impl Model for ChapterListModel {
  type Msg = ChapterListMsg;
  type Widgets = ChapterListWidgets;
  type Components = ();
}

impl ChapterListModel {
  /// Push chapter. and if filter return true (or None)
  /// then push to view
  pub fn push(&self, chapter: ChapterInfo) {
    let chapter: Rc<CheckChapterInfo> = Rc::new(chapter.into());
    self.push_view(chapter.clone());
    self.chapters.borrow_mut().push(chapter);
  }

  /// Push to view if filter return true (or None)
  fn push_view(&self, chapter: Rc<CheckChapterInfo>) {
    if self.filter_view(&chapter.info) {
      let gchapter = GChapterInfo::to_gobject(chapter);
      self.views.append(&gchapter);
    }
  }

  fn filter_view(&self, chapter: &ChapterInfo) -> bool {
    self
      .filter
      .borrow()
      .as_ref()
      .map(|v| v.call(&chapter))
      .unwrap_or(true)
  }

  /// change filter and then re-push content to fit filter
  pub fn change_filter(&self, filter: Option<FilterFunction>) {
    self.filter.replace(filter);
    // call remove_all directly because we don't want to clear the content too
    self.views.remove_all();

    for it in self.chapters.borrow().iter() {
      self.push_view(it.clone());
    }
  }

  pub fn clear(&self) {
    self.views.remove_all();
    self.chapters.borrow_mut().clear();
  }
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
  std::rc::Rc<crate::manga_info::chapter_list::CheckChapterInfo>,
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
          Some(&gtk::MultiSelection::new(Some(&model.views))),
        set_single_click_activate: false,

        connect_activate: ChapterListModel::connect_activate,
      },

    }
  }
}

impl ChapterListModel {
  /// initialize Widget and keep view with state in sync
  pub fn connect_bind(_: &gtk::SignalListItemFactory, item: &gtk::ListItem) {
    let child = item.item().unwrap().downcast::<GChapterInfo>().unwrap();
    let child = child.borrow();
    if let Some(chapter) = child.as_ref() {
      let child = match item.child() {
        Some(child) => child.downcast::<gtk::Grid>().unwrap(),
        // create if child doesn't exists yet
        None => {
          let grid = Self::create_chapter_info(&chapter.info);
          item.set_child(Some(&grid));
          grid
        }
      };

      let check = child
        .child_at(0, 0)
        .unwrap()
        .downcast::<gtk::CheckButton>()
        .unwrap();

      // keep checkbox sync with its item
      check.set_active(chapter.active.get());
    }
  }

  /// toggle selected item when activate emitted from ListView
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

        // make sure connect_bind is called after
        // changing state to update view
        model.selection_changed(i, 1);
      }
    });
  }

  /// Create gtk::Grid from ChapterInfo
  pub fn create_chapter_info(chapter: &ChapterInfo) -> gtk::Grid {
    let check = gtk::CheckButton::default();
    let label = gtk::Label::builder().label(&format!("{}", chapter)).build();

    let grid = gtk::Grid::builder()
      .orientation(gtk::Orientation::Horizontal)
      .build();

    grid.attach(&check, 0, 0, 1, 1);
    grid.attach(&label, 2, 0, 1, 1);
    grid.set_column_spacing(5);

    grid
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

impl<ParentModel> ComponentUpdate<ParentModel> for ChapterListModel
where
  ParentModel: Model,
{
  fn init_model(_: &ParentModel) -> Self {
    Self {
      views: create_list_store(),
      chapters: Default::default(),
      filter: Default::default(),
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
        self.clear();
      }

      ChapterListMsg::Filter(filter) => {
        self.change_filter(filter);
      }
    }
  }
}
