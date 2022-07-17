use gio::{prelude::Cast, traits::ListModelExt};
use gtk::prelude::{CheckButtonExt, GridExt, SelectionModelExt};
use relm4::{ComponentUpdate, Model};

use super::{ChapterListWidgets, GChapterInfo, GChapterInfoItem, VecChapters};

pub trait ChapterListParentModel: Model {
    fn get_vec_chapter_info(&self) -> VecChapters;
}

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

impl Model for ChapterListModel {
    type Msg = ChapterListMsg;
    type Widgets = ChapterListWidgets;
    type Components = ();
}

const CHECK_BUTTON_ROW: i32 = 0;
const CHECK_BUTTON_COLUMN: i32 = 0;

impl ChapterListModel {
    /// Create gtk::Grid from ChapterInfo
    pub fn create_chapter_info(chapter: GChapterInfo) -> gtk::Grid {
        let check = gtk::CheckButton::default();
        let label = gtk::Label::builder()
            .label(&format!("{}", chapter.borrow().info))
            .build();

        let grid = gtk::Grid::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        grid.attach(&check, CHECK_BUTTON_COLUMN, CHECK_BUTTON_ROW, 1, 1);
        grid.attach(&label, 2, 0, 1, 1);
        grid.set_column_spacing(5);

        check.connect_toggled(move |it| {
            chapter.borrow().active.set(it.is_active());
            // chapter.borrow().active.update(|_| it.is_active());
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

impl<ParentModel> ComponentUpdate<ParentModel> for ChapterListModel
where
    ParentModel: ChapterListParentModel,
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

                child.set_active(item.data().borrow().active.get());
            }
            ChapterListMsg::Activate(list) => {
                let model = list.model().unwrap();
                let selection = model.selection();
                Self::for_each(&model, |i, it| {
                    if selection.contains(i) {
                        let it = it.downcast::<GChapterInfo>().unwrap();
                        let it = it.borrow();
                        it.active.set(!it.active.get());

                        // Notify model that value has changed
                        model.selection_changed(i, 1);
                    }
                });
            }
        }
    }
}
