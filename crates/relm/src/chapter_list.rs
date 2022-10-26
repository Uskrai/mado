use gtk::{
    gio::{prelude::Cast, traits::ListModelExt},
    prelude::*,
};
use relm4::{Component, ComponentParts, ComponentSender, SimpleComponent};

use crate::vec_chapters::{GChapterInfo, GChapterInfoItem, VecChapters};

#[derive(Debug)]
pub struct ChapterListModel {
    pub(super) chapters: VecChapters,
}

#[derive(Debug)]
pub enum ChapterListMsg {
    Setup(GChapterInfoItem),
    Change(GChapterInfoItem),
    Activate(gtk::ListView),
}

const CHECK_BUTTON_ROW: i32 = 0;
const CHECK_BUTTON_COLUMN: i32 = 0;

impl ChapterListModel {
    /// Create gtk::Grid from ChapterInfo
    pub fn create_chapter_info(chapter: GChapterInfo) -> gtk::Grid {
        let check = gtk::CheckButton::default();
        let label = gtk::Label::builder()
            .label(&format!("{}", chapter.borrow().info()))
            .build();

        let grid = gtk::Grid::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        grid.attach(&check, CHECK_BUTTON_COLUMN, CHECK_BUTTON_ROW, 1, 1);
        grid.attach(&label, 2, 0, 1, 1);
        grid.set_column_spacing(5);

        check.connect_toggled(move |it| {
            chapter.borrow().set_active(it.is_active());
        });

        grid
    }

    fn for_each(model: &gtk::SelectionModel, call: impl Fn(u32, gtk::glib::Object)) {
        let mut i = 0;
        while let Some(it) = model.item(i) {
            call(i, it);
            i += 1;
        }
    }
}

/// Widget that show Chapter with checkbox
#[relm4::component(pub)]
impl SimpleComponent for ChapterListModel {
    type Widgets = ChapterListWidgets;
    type Init = VecChapters;

    type Input = ChapterListMsg;
    type Output = ();

    fn init(
        chapters: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ChapterListModel { chapters };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            // Initialize Children
            ChapterListMsg::Setup(item) => {
                let grid = Self::create_chapter_info(item.data());
                item.set_child(Some(&grid));
            }

            // Sync children with data
            ChapterListMsg::Change(item) => {
                let child = item
                    .child()
                    .unwrap()
                    .downcast::<gtk::Grid>()
                    .unwrap()
                    .child_at(CHECK_BUTTON_COLUMN, CHECK_BUTTON_ROW)
                    .unwrap()
                    .downcast::<gtk::CheckButton>()
                    .unwrap();

                child.set_active(item.data().borrow().active());
            }
            ChapterListMsg::Activate(list) => {
                let model = list.model().unwrap();
                let selection = model.selection();
                Self::for_each(&model, |i, it| {
                    if selection.contains(i) {
                        let it = it.downcast::<GChapterInfo>().unwrap();
                        let it = it.borrow();
                        it.set_active(!it.active());

                        // Notify model that value has changed
                        model.selection_changed(i, 1);
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
            set_child = &gtk::ListView {
                #[wrap(Some)]
                set_factory = &gtk::SignalListItemFactory {
                    connect_setup[sender] => move |_, item| {
                        sender.input(ChapterListMsg::Setup(item.clone().into()))
                    },

                    connect_bind[sender] => move |_, item| {
                        sender.input(ChapterListMsg::Change(item.clone().into()))
                    }
                },
                set_single_click_activate: false,
                connect_activate[sender] => move |view, _| {
                    sender.input(ChapterListMsg::Activate(view.clone()))
                },

                set_model: Some(&model.chapters.create_selection_model())
            },
        }
    }
}
