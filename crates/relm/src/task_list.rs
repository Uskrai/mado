use gtk::{gio, prelude::*};

use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::task::{DownloadView, DownloadViewController, GDownloadItem};

#[derive(Debug)]
pub enum TaskListMsg {
    Setup(gtk::ListItem),
    Bind(gtk::ListItem),
}

#[derive(Clone)]
pub struct TaskListModel {
    tasks: gio::ListStore,
}

fn create_selection_model(model: &TaskListModel) -> gtk::MultiSelection {
    gtk::MultiSelection::new(Some(&model.tasks))
}

#[relm4::component(pub)]
impl SimpleComponent for TaskListModel {
    type Widgets = TaskListWidgets;

    type Init = gio::ListStore;
    type Input = TaskListMsg;
    type Output = ();

    fn init(
        tasks: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { tasks };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            TaskListMsg::Setup(item) => {
                let download = item.item().unwrap().downcast::<GDownloadItem>().unwrap();
                let info = download.borrow();
                let info = &info.info;
                let view = DownloadView::from(info.as_ref());
                let _ = DownloadViewController::connect(view.clone(), info.clone());
                item.set_child(Some(&view.widget));
            }
            TaskListMsg::Bind(_) => {
                //
            }
        }
    }

    view! {
        gtk::ListView {
            set_model: Some(&create_selection_model(&model)),

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
