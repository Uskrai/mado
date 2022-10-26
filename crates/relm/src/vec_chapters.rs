use std::cell::{Cell, Ref, RefCell, RefMut};
use std::sync::Arc;

pub use gtk::{gio, prelude::*, subclass::prelude::*};
use mado::core::ChapterInfo;

#[derive(Debug, Clone)]
pub struct ListStore {
    pub inner: gio::ListStore,
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

#[derive(Default, Debug)]
pub struct CheckChapterInfo {
    info: Arc<ChapterInfo>,
    active: Cell<bool>,
}

impl CheckChapterInfo {
    pub fn info(&self) -> &ChapterInfo {
        self.info.as_ref()
    }

    pub fn active(&self) -> bool {
        self.active.get()
    }

    pub fn set_active(&self, val: bool) {
        self.active.set(val)
    }
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
    crate::vec_chapters::CheckChapterInfo,
    "MadoRelmChapterInfo",
    info_wrapper
);
pub use info_wrapper::GChapterInfo;

#[derive(Debug)]
pub struct GChapterInfoItem {
    item: gtk::ListItem,
}

impl std::ops::Deref for GChapterInfoItem {
    type Target = gtk::ListItem;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl GChapterInfoItem {
    pub fn new(item: gtk::ListItem) -> Self {
        Self { item }
    }

    pub fn data(&self) -> GChapterInfo {
        self.item
            .item()
            .unwrap()
            .downcast::<GChapterInfo>()
            .expect("Expecting GChapterInfo")
    }
}

impl From<gtk::ListItem> for GChapterInfoItem {
    fn from(item: gtk::ListItem) -> Self {
        Self::new(item)
    }
}

#[derive(Default, Clone, Debug)]
pub struct VecChapters {
    inner: RefCell<Vec<GChapterInfo>>,
    views: ListStore,
}

impl VecChapters {
    fn borrow_mut(&self) -> RefMut<Vec<GChapterInfo>> {
        self.inner.borrow_mut()
    }

    fn borrow(&self) -> Ref<Vec<GChapterInfo>> {
        self.inner.borrow()
    }

    pub fn push(&self, chapter: Arc<ChapterInfo>) {
        let chapter = GChapterInfo::to_gobject(CheckChapterInfo::from(chapter));
        self.borrow_mut().push(chapter.clone());
        self.views.append(&chapter);
    }

    pub fn for_each_selected(&self, mut f: impl FnMut(usize, &Arc<ChapterInfo>)) {
        self.borrow()
            .iter()
            .enumerate()
            .filter(|(_, v)| v.borrow().active.get())
            .for_each(|(i, v)| f(i, &v.borrow().info));
    }

    pub fn clear(&self) {
        self.borrow_mut().clear();
        self.views.remove_all();
    }

    pub fn views(&self) -> &ListStore {
        &self.views
    }

    pub fn create_selection_model(&self) -> gtk::MultiSelection {
        gtk::MultiSelection::new(Some(&self.views.inner))
    }
}
