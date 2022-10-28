use std::sync::Arc;

use gtk::{gio, prelude::WidgetExt};
use mado::engine::DownloadInfo;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
};

use crate::task::{DownloadItem, GDownloadItem};
use crate::task_list::TaskListModel;

#[derive(Debug)]
pub enum DownloadMsg {
    CreateDownloadView(Arc<DownloadInfo>),
}

pub struct DownloadModel {
    list: gio::ListStore,
    task_list: Controller<TaskListModel>,
}

#[relm4::component(pub)]
impl SimpleComponent for DownloadModel {
    type Widgets = DownloadWidgets;
    // type Components = DownloadComponents;

    type Init = ();

    type Input = DownloadMsg;
    type Output = ();

    fn init(
        _: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list = gio::ListStore::new(gtk::glib::Type::OBJECT);

        let task_list = TaskListModel::builder().launch(list.clone()).detach();

        let model = Self { list, task_list };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            DownloadMsg::CreateDownloadView(info) => {
                let object = DownloadItem { info };
                let object = GDownloadItem::to_gobject(object);
                self.list.append(&object);
            }
        }
    }

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hexpand: true,
            set_child: Some(model.task_list.widget())
        }
    }
}
