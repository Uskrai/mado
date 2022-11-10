use std::{cell::Cell, rc::Rc, sync::Arc};

use gtk::{
    gio::{prelude::Cast, traits::ListModelExt},
    prelude::*,
};
use mado_core::ChapterInfo;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::list_model::{ListModel, ListModelBase};

#[derive(Default, Debug, Clone)]
pub struct CheckChapterInfo {
    info: Arc<ChapterInfo>,
    active: Rc<Cell<bool>>,
}
impl From<Arc<ChapterInfo>> for CheckChapterInfo {
    fn from(info: Arc<ChapterInfo>) -> Self {
        Self::new(info, false)
    }
}

impl CheckChapterInfo {
    pub fn new(info: Arc<ChapterInfo>, active: bool) -> Self {
        Self {
            info,
            active: Rc::new(Cell::new(active)),
        }
    }

    pub fn info(&self) -> &Arc<ChapterInfo> {
        &self.info
    }

    pub fn active(&self) -> bool {
        self.active.get()
    }

    pub fn set_active(&self, val: bool) {
        self.active.set(val)
    }
}

// #[derive(Debug)]
pub struct ChapterListModel {
    #[allow(dead_code)]
    chapters: ListModel<CheckChapterInfo>,
    selection_model: gtk::MultiSelection,
}

#[derive(Debug)]
pub enum ChapterListMsg {
    Setup(gtk::ListItem),
    Change(gtk::ListItem),
    Activate,
}

const CHECK_BUTTON_ROW: i32 = 0;
const CHECK_BUTTON_COLUMN: i32 = 0;

impl ChapterListModel {
    /// Create gtk::Grid from ChapterInfo
    pub fn create_chapter_info(chapter: CheckChapterInfo) -> gtk::Grid {
        let check = gtk::CheckButton::default();
        let label = gtk::Label::builder()
            .label(&format!("{}", chapter.info()))
            .build();

        let grid = gtk::Grid::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        grid.attach(&check, CHECK_BUTTON_COLUMN, CHECK_BUTTON_ROW, 1, 1);
        grid.attach(&label, 2, 0, 1, 1);
        grid.set_column_spacing(5);

        check.connect_toggled(move |it| {
            chapter.set_active(it.is_active());
        });

        grid
    }

    fn get_check(grid: &gtk::Grid) -> Option<gtk::CheckButton> {
        grid.child_at(CHECK_BUTTON_COLUMN, CHECK_BUTTON_ROW)?
            .downcast::<gtk::CheckButton>()
            .ok()
    }

    fn for_each(&self, call: impl Fn(u32, gtk::glib::Object)) {
        let mut i = 0;
        while let Some(it) = self.selection_model.item(i) {
            call(i, it);
            i += 1;
        }
    }
}

/// Widget that show Chapter with checkbox
#[relm4::component(pub)]
impl SimpleComponent for ChapterListModel {
    type Widgets = ChapterListWidgets;
    type Init = ListModel<CheckChapterInfo>;

    type Input = ChapterListMsg;
    type Output = ();

    fn init(
        chapters: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selection_model = gtk::MultiSelection::new(Some(&chapters.list_model()));
        let model = ChapterListModel {
            chapters,
            selection_model,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            // Initialize Children
            ChapterListMsg::Setup(item) => {
                let data = item.item().and_then(|it| self.chapters.get_by_object(&it));

                if let Some(data) = data {
                    let grid = Self::create_chapter_info(data.clone());
                    item.set_child(Some(&grid));
                }
            }

            // Sync children with data
            ChapterListMsg::Change(item) => {
                let child = item.child().unwrap().downcast::<gtk::Grid>().unwrap();
                let child = Self::get_check(&child).unwrap();

                if let Some(data) = item.item().and_then(|it| self.chapters.get_by_object(&it)) {
                    child.set_active(data.active());
                }
            }
            ChapterListMsg::Activate => {
                let selection = self.selection_model.selection();
                self.for_each(|i, it| {
                    if selection.contains(i) {
                        if let Some(it) = self.chapters.get_by_object(&it) {
                            it.set_active(!it.active());

                            // Notify model that value has changed
                            self.selection_model.selection_changed(i, 1);
                        }
                    }
                });
            }
        }
    }

    view! {
        gtk::ScrolledWindow {
            set_vexpand : true,
            set_hexpand: true,
            #[wrap(Some)]
            set_child: list = &gtk::ListView {
                #[wrap(Some)]
                set_factory = &gtk::SignalListItemFactory {
                    connect_setup[sender] => move |_, item| {
                        sender.input(ChapterListMsg::Setup(item.clone()))
                    },

                    connect_bind[sender] => move |_, item| {
                        sender.input(ChapterListMsg::Change(item.clone()))
                    }
                },
                set_single_click_activate: false,
                connect_activate[sender] => move |_, _| {
                    sender.input(ChapterListMsg::Activate)
                },

                set_model: Some(&model.selection_model)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mado_core::ChapterInfo;
    use relm4::{Component, ComponentController};

    use super::*;
    use crate::{list_store::ListStore, tests::*};

    #[gtk::test]
    fn test_chapter_info() {
        let chapter = Arc::new(ChapterInfo::default());
        let chapter = CheckChapterInfo::from(chapter);

        let it = ChapterListModel::create_chapter_info(chapter.clone());

        let check = ChapterListModel::get_check(&it).expect("should exist");
        for i in [true, false] {
            check.set_active(i);
            run_loop();
            assert_eq!(chapter.active(), i);
        }
    }

    #[gtk::test]
    fn test_model() {
        let vec = ListStore::default();

        let window = gtk::ApplicationWindow::default();
        let model = ChapterListModel::builder().launch(vec.base()).detach();
        window.set_child(Some(model.widget()));

        let first = vec.push(CheckChapterInfo::new(
            Arc::new(ChapterInfo {
                id: "id".to_string(),
                ..Default::default()
            }),
            false,
        ));

        run_loop();

        assert_eq!(model.model().selection_model.n_items(), 1);

        for i in [true, false] {
            vec.get(&first).unwrap().set_active(i);
            model.emit(ChapterListMsg::Activate);
            run_loop();
            assert_eq!(vec.get(&first).unwrap().active(), i);
        }

        for i in [true, false] {
            model.model().selection_model.select_item(0, true);
            model.emit(ChapterListMsg::Activate);
            run_loop();
            assert_eq!(vec.get(&first).unwrap().active(), i);
        }
    }
}
