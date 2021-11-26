use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;

pub use gtk::prelude::*;
pub use gtk::subclass::prelude::*;
use mado_core::ChapterInfo;
use relm4::{ComponentUpdate, Model, Widgets};

#[derive(Debug)]
pub struct ChapterListModel {
  chapters: VecChapters,
}

pub enum ChapterListMsg {}

#[derive(Debug, Clone)]
struct ListStore {
  inner: gio::ListStore,
}

impl Default for ListStore {
  fn default() -> Self {
    Self {
      inner: gio::ListStore::new(gio::glib::types::Type::OBJECT),
    }
  }
}

impl std::ops::Deref for ListStore {
  type Target = gio::ListStore;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub trait HasVecChapters {
  fn get_vec_chapter_info(&self) -> VecChapters;
}

impl Model for ChapterListModel {
  type Msg = ChapterListMsg;
  type Widgets = ChapterListWidgets;
  type Components = ();
}

#[derive(Default, Debug)]
pub struct CheckChapterInfo {
  info: Arc<ChapterInfo>,
  active: Cell<bool>,
}

impl From<Arc<ChapterInfo>> for CheckChapterInfo {
  fn from(info: Arc<ChapterInfo>) -> Self {
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

/// Widget that show Chapter with checkbox
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
          Some(&gtk::MultiSelection::new(Some(&model.chapters.views.inner))),
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

#[derive(Default, Clone, Debug)]
pub struct VecChapters {
  inner: RefCell<Vec<Rc<CheckChapterInfo>>>,
  views: ListStore,
}

impl VecChapters {
  fn borrow_mut(&self) -> RefMut<Vec<Rc<CheckChapterInfo>>> {
    self.inner.borrow_mut()
  }

  fn borrow(&self) -> Ref<Vec<Rc<CheckChapterInfo>>> {
    self.inner.borrow()
  }

  pub fn push(&self, chapter: Arc<ChapterInfo>) {
    let chapter = Rc::new(CheckChapterInfo::from(chapter));
    self.borrow_mut().push(chapter.clone());
    self.views.append(&GChapterInfo::to_gobject(chapter));
  }

  pub fn for_each_selected(&self, mut f: impl FnMut(usize, &Arc<ChapterInfo>)) {
    self
      .borrow()
      .iter()
      .enumerate()
      .filter(|(_, v)| v.active.get())
      .for_each(|(i, v)| f(i, &v.info));
  }

  pub fn clear(&self) {
    self.borrow_mut().clear();
    self.views.remove_all();
  }
}

impl<ParentModel> ComponentUpdate<ParentModel> for ChapterListModel
where
  ParentModel: Model + HasVecChapters,
{
  fn init_model(parent: &ParentModel) -> Self {
    Self {
      chapters: parent.get_vec_chapter_info(),
    }
  }

  fn update(
    &mut self,
    msg: Self::Msg,
    _: &Self::Components,
    _: relm4::Sender<Self::Msg>,
    _: relm4::Sender<ParentModel::Msg>,
  ) {
    match msg {}
  }
}
