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
    PauseSelected,
    ResumeSelected,
}

pub struct DownloadModel {
    list: ListStore<DownloadItem>,
    task_list: Controller<TaskListModel>,
}

impl DownloadModel {
    pub fn resume(&mut self, resume: bool) {
        let selection = &self.task_list.model().selection;

        if let Some(model) = selection.model() {
            let selection = selection.selection();
            for (index, it) in model.into_iter().enumerate() {
                let it = it.unwrap();

                if selection.contains(index as u32) {
                    if let Some(it) = self.list.get_by_object(&it) {
                        it.info.resume(resume);
                    }
                }
            }
        }
    }
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

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DownloadMsg::CreateDownloadView(info) => {
                let object = DownloadItem { info };
                self.list.push(object);
            }
            DownloadMsg::PauseSelected => {
                self.resume(false);
            }
            DownloadMsg::ResumeSelected => {
                self.resume(true);
            }
        }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                #[name = "resume_button"]
                append = &gtk::Button {
                    set_label: "Resume",
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::ResumeSelected);
                    }
                },

                #[name = "pause_button"]
                append = &gtk::Button {
                    set_label: "Pause",
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::PauseSelected);
                    }
                },
            },

            append = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_child: Some(model.task_list.widget())
            }
        }
    }
}

impl DownloadModel {
    pub fn task_len(&self) -> usize {
        self.list.len()
    }
}

#[cfg(test)]
mod tests {
    use mado::engine::LateBindingModule;
    use mado_core::DefaultMadoModuleMap;

    use super::*;
    use crate::tests::*;

    #[gtk::test]
    pub fn resume_test() {
        let model = DownloadModel::builder().launch(()).detach();

        let modulemap = Arc::new(DefaultMadoModuleMap::new());
        let module = LateBindingModule::WaitModule(modulemap, Default::default());

        let create = || {
            Arc::new(
                DownloadInfo::builder()
                    .order(0)
                    .module(module.clone())
                    .chapters(vec![])
                    .status(mado::engine::DownloadStatus::paused())
                    .build(),
            )
        };

        let first = create();
        let second = create();

        model.emit(DownloadMsg::CreateDownloadView(first.clone()));
        model.emit(DownloadMsg::CreateDownloadView(second.clone()));
        run_loop();

        model
            .model()
            .task_list
            .model()
            .selection
            .select_item(0, true);

        model.widgets().resume_button.emit_clicked();
        run_loop();

        assert!(first.status().is_resumed());
        assert!(second.status().is_paused());

        model.widgets().pause_button.emit_clicked();
        // model.emit(DownloadMsg::PauseSelected);
        run_loop();

        assert!(first.status().is_paused());
        assert!(second.status().is_paused());
    }
}
