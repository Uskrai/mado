#![allow(dead_code, unused_variables)]
use std::sync::Arc;

use gtk::prelude::*;
#[allow(unused_imports)]
use mado_engine::DownloadInfo;
use mado_engine::DownloadSender;

#[derive(Debug)]
pub struct DownloadItem {
    pub info: Arc<DownloadInfo>,
    pub controller: DownloadSender,
}

crate::gobject::struct_wrapper!(
    GDownloadItem,
    crate::download::task_list::DownloadItem,
    "MadoRelmDownloadInfo",
    info_wrapper
);
pub use info_wrapper::GDownloadItem;

use relm4::{send, ComponentUpdate, Model, Widgets};

pub enum TaskListMsg {
    Setup(gtk::ListItem),
    Bind(gtk::ListItem),
}

pub trait TaskListParentModel: Model {
    fn get_list(&self) -> gio::ListStore;
}

#[derive(Clone)]
pub struct TaskListModel {
    tasks: gio::ListStore,
    vec: Vec<GDownloadItem>,
}

impl Model for TaskListModel {
    type Msg = TaskListMsg;
    type Widgets = TaskListWidgets;
    type Components = ();
}

impl<ParentModel: TaskListParentModel> ComponentUpdate<ParentModel> for TaskListModel {
    fn init_model(parent: &ParentModel) -> Self {
        Self {
            tasks: parent.get_list(),
            vec: Vec::new(),
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
            TaskListMsg::Setup(item) => {
                let download = item.item().unwrap().downcast::<GDownloadItem>().unwrap();
                let view = DownloadView::from(download.borrow().info.as_ref());
                let _ = DownloadViewController::connect(view.clone(), &mut download.borrow_mut());
                item.set_child(Some(&view.widget));
            }
            TaskListMsg::Bind(_) => {
                //
            }
        }
    }
}

fn create_selection_model(model: &TaskListModel) -> gtk::MultiSelection {
    gtk::MultiSelection::new(Some(&model.tasks))
}

#[relm4_macros::widget(pub)]
impl<ParentModel> Widgets<TaskListModel, ParentModel> for TaskListWidgets
where
    ParentModel: Model,
{
    view! {
        gtk::ListView {
            set_model: Some(&create_selection_model(model)),

            set_factory = Some(&gtk::SignalListItemFactory) {
                connect_setup(sender) => move |_,item| {
                    send!(sender, TaskListMsg::Setup(item.clone()));
                },
                connect_bind(sender) => move |_, item| {
                    send!(sender, TaskListMsg::Bind(item.clone()));
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
struct DownloadView {
    widget: gtk::Box,
    label: gtk::Label,
}

impl From<&DownloadInfo> for DownloadView {
    fn from(info: &DownloadInfo) -> Self {
        let label = gtk::Label::new(Some(&info.manga.title));

        let widget = gtk::Box::new(gtk::Orientation::Vertical, 5);
        widget.append(&label);

        Self { widget, label }
    }
}

#[derive(Debug, Clone)]
struct DownloadViewController {
    sender: gtk::glib::Sender<()>,
}

impl DownloadViewController {
    pub fn connect(_: DownloadView, download: &mut DownloadItem) -> Self {
        use gtk::glib;

        let (sender, recv) = gtk::glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let this = Self { sender };

        recv.attach(None, |_| {
            //

            gtk::glib::Continue(true)
        });
        download.controller.start(this.clone()).unwrap();

        this
    }
}

impl mado_engine::DownloadViewController for DownloadViewController {
    //
}
