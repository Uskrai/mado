use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::{
    list_model::{ListModel, ListModelBase},
    task::{DownloadItem, DownloadView, DownloadViewController},
};

#[derive(Debug)]
pub enum TaskListMsg {
    Setup(gtk::ListItem),
    Bind(gtk::ListItem),
}

#[derive(Clone)]
pub struct TaskListModel {
    tasks: ListModel<DownloadItem>,
    pub selection: gtk::MultiSelection,
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
        let model = Self { tasks, selection };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            TaskListMsg::Setup(item) => {
                if let Some(data) = self.tasks.get_by_object(&item.item().unwrap()) {
                    let info = data.info().clone();
                    let view = DownloadView::from(info.as_ref());
                    let _ = DownloadViewController::connect(view.clone(), info.clone());
                    item.set_child(Some(&view.widget));
                }
            }
            TaskListMsg::Bind(_) => {
                //
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
            }
        }
    }
}
