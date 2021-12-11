use std::sync::Arc;

use gtk::prelude::WidgetExt;
use mado_engine::DownloadInfo;
use relm4::{ComponentUpdate, Components, Model, RelmComponent, Widgets};
use relm4_macros::widget;

use super::task_list::TaskListParentModel;

pub enum DownloadMsg {
    CreateDownloadView(Arc<DownloadInfo>),
}

pub struct DownloadModel {
    list: gio::ListStore,
}

impl Model for DownloadModel {
    type Msg = DownloadMsg;
    type Widgets = DownloadWidgets;
    type Components = DownloadComponents;
}

impl TaskListParentModel for DownloadModel {
    fn get_list(&self) -> gio::ListStore {
        self.list.clone()
    }
}

impl<ParentModel: Model> ComponentUpdate<ParentModel> for DownloadModel {
    fn init_model(_: &ParentModel) -> Self {
        Self {
            list: gio::ListStore::new(gtk::glib::Type::OBJECT),
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
            DownloadMsg::CreateDownloadView(info) => {
                let object = super::task_list::DownloadItem { info };
                let object = super::task_list::GDownloadItem::to_gobject(object);
                self.list.append(&object);
            }
        }
    }
    //
}

#[widget(pub)]
impl<ParentModel> Widgets<DownloadModel, ParentModel> for DownloadWidgets
where
    ParentModel: Model,
{
    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hexpand: true,
            set_child: Some(components.task_list.root_widget())
        }
    }
}

pub struct DownloadComponents {
    task_list: RelmComponent<super::task_list::TaskListModel, DownloadModel>,
}

impl Components<DownloadModel> for DownloadComponents {
    fn init_components(
        parent_model: &DownloadModel,
        parent_sender: relm4::Sender<DownloadMsg>,
    ) -> Self {
        Self {
            task_list: RelmComponent::new(parent_model, parent_sender),
        }
    }

    fn connect_parent(&mut self, parent: &<DownloadModel as Model>::Widgets) {
        self.task_list.connect_parent(parent);
    }
}
