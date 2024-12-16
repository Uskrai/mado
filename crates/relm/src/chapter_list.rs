use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

use gtk::prelude::*;
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
        println!("{:?} {} {}", self.info().index, val, self.info());
        self.active.set(val)
    }
}

// #[derive(Debug)]
pub struct ChapterListModel {
    #[allow(dead_code)]
    chapters: ListModel<CheckChapterInfo>,
    selection_model: gtk::MultiSelection,
    widget_map: HashMap<gtk::ListItem, ChapterInfoController>,
}

#[derive(Debug)]
pub enum ChapterListMsg {
    Setup(gtk::ListItem),
    Change(gtk::ListItem),
    Teardown(gtk::ListItem),
    Activate,
}

const CHECK_BUTTON_ROW: i32 = 0;
const CHECK_BUTTON_COLUMN: i32 = 0;

const LABEL_COLUMN: i32 = 2;
const LABEL_ROW: i32 = 0;

pub enum ChapterInfoMsg {
    SetActive(bool),
    ChangeChapter(CheckChapterInfo),
}

pub type ChapterInfoController = gtk::glib::Sender<ChapterInfoMsg>;

impl ChapterListModel {
    /// Create gtk::Grid from ChapterInfo
    pub fn create_chapter_info(chapter: CheckChapterInfo) -> (gtk::Grid, ChapterInfoController) {
        let (sender, recv) = gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);

        let check = gtk::CheckButton::default();
        let label = gtk::Label::builder()
            .label(&format!("{}", chapter.info()))
            .build();

        let grid = gtk::Grid::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        grid.attach(&check, CHECK_BUTTON_COLUMN, CHECK_BUTTON_ROW, 1, 1);
        grid.attach(&label, LABEL_COLUMN, LABEL_ROW, 1, 1);
        grid.set_column_spacing(5);


        let chapter = RefCell::new(chapter.clone());

        check.connect_toggled({
            let sender = sender.clone();
            move |it| {
                sender
                    .send(ChapterInfoMsg::SetActive(it.is_active()))
                    .unwrap();
            }
        });

        recv.attach(None, move |msg| {
            match msg {
                ChapterInfoMsg::SetActive(active) => chapter.borrow_mut().set_active(active),
                ChapterInfoMsg::ChangeChapter(change) => {
                    label.set_label(&format!("{}", change.info()));
                    check.set_active(change.active());
                    *chapter.borrow_mut() = change;
                }
            };

            gtk::glib::Continue(true)
        });


        (grid, sender)
    }

    fn get_label(grid: &gtk::Grid) -> Option<gtk::Label> {
        grid.child_at(LABEL_COLUMN, LABEL_ROW)?
            .downcast::<gtk::Label>()
            .ok()
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
            widget_map: Default::default(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            // Initialize Children
            ChapterListMsg::Setup(item) => {
                if let Some(it) = item.item().and_then(|it| self.chapters.get_by_object(&it)) {
                    let (grid, controller) = Self::create_chapter_info(it.clone());
                    item.set_child(Some(grid).as_ref());
                    self.widget_map.insert(item, controller);
                }
            }

            // Sync children with data
            ChapterListMsg::Change(item) => {
                let child = item.child().and_then(|it| it.downcast::<gtk::Grid>().ok());
                let child = match child {
                    Some(child) => child,
                    _ => return
                };

                let chapter = item.item().and_then(|it| self.chapters.get_by_object(&it));
                let controller = self.widget_map.get(&item);

                let (chapter, controller) = match chapter.zip(controller) {
                    Some(it) => it,
                    None => return,
                };

                controller
                    .send(ChapterInfoMsg::ChangeChapter(chapter.clone()))
                    .unwrap();
            }
            ChapterListMsg::Teardown(item) => {
                self.widget_map.remove(&item);
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
                    },

                    connect_teardown[sender] => move |_, item| {
                        sender.input(ChapterListMsg::Teardown(item.clone()))
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

        let (it, _) = ChapterListModel::create_chapter_info(chapter.clone());

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
