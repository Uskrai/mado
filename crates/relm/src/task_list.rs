use std::collections::HashMap;

use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::{
    list_model::{ListModel, ListModelBase},
    task::{DownloadItem, DownloadView, DownloadViewController},
};

#[derive(Debug)]
pub enum TaskListMsg {
    Setup(gtk::ListItem),
    Bind(gtk::ListItem),
    Teardown(gtk::ListItem),
}

#[derive(Clone)]
pub struct TaskListModel {
    tasks: ListModel<DownloadItem>,
    pub selection: gtk::MultiSelection,
    widget_map: HashMap<gtk::ListItem, DownloadViewController>,
}

fn create_selection_model(model: &ListModel<DownloadItem>) -> gtk::MultiSelection {
    gtk::MultiSelection::new(Some(&model.list_model()))
}

#[relm4::component(pub)]
impl SimpleComponent for TaskListModel {
    type Widgets = TaskListWidgets;

    type Init = ListModel<DownloadItem>;
    type Input = TaskListMsg;
    type Output = ();

    fn init(
        tasks: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selection = create_selection_model(&tasks);
        let model = Self {
            tasks,
            selection,
            widget_map: Default::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            TaskListMsg::Setup(item) => {
                if let Some(data) = self.tasks.get_by_object(&item.item().unwrap()) {
                    let info = data.info().clone();
                    let view = DownloadView::from(info.as_ref());
                    let controller = DownloadViewController::connect(view.clone(), info.clone());
                    item.set_child(Some(&view.widget));
                    self.widget_map.insert(item, controller);
                }
            }
            TaskListMsg::Bind(item) => {
                let controller = self.widget_map.get(&item);
                let info = item.item().and_then(|it| self.tasks.get_by_object(&it));

                let (controller, info) = match controller.zip(info) {
                    Some(it) => it,
                    None => return,
                };

                controller
                    .sender
                    .send(crate::task::DownloadMsg::InfoChanged(info.info().clone()))
                    .unwrap();
            }
            TaskListMsg::Teardown(item) => {
                self.widget_map.remove(&item);
            }
        }
    }

    view! {
        gtk::ListView {
            set_model: Some(&model.selection),

            #[wrap(Some)]
            set_factory = &gtk::SignalListItemFactory {
                connect_setup[sender] => move |_,item| {
                    sender.input(TaskListMsg::Setup(item.clone()));
                },
                connect_bind[sender] => move |_, item| {
                    sender.input(TaskListMsg::Bind(item.clone()));
                },
                connect_teardown[sender] => move |_, item| {
                    sender.input(TaskListMsg::Teardown(item.clone()));
                }
            }
        }
    }
}
