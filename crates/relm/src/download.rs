use std::sync::Arc;

use gtk::prelude::*;
use mado::engine::DownloadInfo;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
};

use crate::list_store::ListStore;
use crate::task::DownloadItem;
use crate::task_list::TaskListModel;

#[derive(Debug)]
pub enum DownloadMsg {
    CreateDownloadView(Arc<DownloadInfo>),
}

pub struct DownloadModel {
    list: ListStore<DownloadItem>,
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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list = ListStore::default();
        let task_list = TaskListModel::builder().launch(list.base()).detach();

        let model = Self { list, task_list };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            DownloadMsg::CreateDownloadView(info) => {
                let object = DownloadItem { info };
                self.list.push(object);
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

impl DownloadModel {
    pub fn task_len(&self) -> usize {
        self.list.len()
    }
}
