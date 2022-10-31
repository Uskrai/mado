use std::cell::{Cell, Ref, RefCell};
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
    gvec: RefCell<Vec<GChapterInfo>>,
    views: ListStore,
}

impl VecChapters {
    pub fn push(&self, chapter: Arc<ChapterInfo>) {
        let chapter = GChapterInfo::to_gobject(CheckChapterInfo::from(chapter));
        self.gvec.borrow_mut().push(chapter.clone());
        self.views.append(&chapter);
    }

    pub fn for_each(&self, mut f: impl FnMut(usize, &GChapterInfo)) {
        self.gvec
            .borrow()
            .iter()
            .enumerate()
            .for_each(|(i, v)| f(i, v));
    }

    pub fn for_each_selected(&self, mut f: impl FnMut(usize, &Arc<ChapterInfo>)) {
        self.gvec
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, v)| v.borrow().active.get())
            .for_each(|(i, v)| f(i, &v.borrow().info));
    }

    pub fn clear(&self) {
        self.gvec.borrow_mut().clear();
        self.views.remove_all();
    }

    pub fn create_selection_model(&self) -> gtk::MultiSelection {
        gtk::MultiSelection::new(Some(&self.views.inner))
    }

    pub fn borrow(&self) -> Ref<'_, Vec<GChapterInfo>> {
        self.gvec.borrow()
    }
}
