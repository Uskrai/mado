use crate::{
    download::{DownloadModel, DownloadMsg},
    manga_info::{MangaInfoModel, MangaInfoOutput},
};
use gtk::prelude::*;
use mado::core::ArcMadoModule;
use mado::engine::{DownloadRequest, MadoEngineState, MadoEngineStateMsg};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
};
use std::sync::Arc;

#[derive(Debug)]
pub enum AppMsg {
    PushModule(ArcMadoModule),
    DownloadRequest(DownloadRequest),
    Error(mado::core::Error),
}

pub struct AppModel {
    /// state.send will be called on [`AppComponents::init_components`]
    pub(super) state: Arc<MadoEngineState>,

    downloads: Controller<DownloadModel>,
    manga_info: Controller<MangaInfoModel>,

    root: gtk::ApplicationWindow,
}

pub struct RelmMadoEngineStateObserver {
    sender: relm4::Sender<AppMsg>,
    download_sender: relm4::Sender<DownloadMsg>,
}

impl RelmMadoEngineStateObserver {
    pub fn new(sender: relm4::Sender<AppMsg>, download_sender: relm4::Sender<DownloadMsg>) -> Self {
        Self {
            sender,
            download_sender,
        }
    }

    pub fn connect(self, state: &Arc<MadoEngineState>) {
        state.connect(move |msg| {
            match msg {
                MadoEngineStateMsg::Download(info) => self
                    .download_sender
                    .send(DownloadMsg::CreateDownloadView(info.clone())),
                MadoEngineStateMsg::PushModule(module) => {
                    self.sender.send(AppMsg::PushModule(module.clone()))
                }
            };
        });
    }
}

pub fn convert_manga_list(msg: MangaInfoOutput) -> AppMsg {
    match msg {
        crate::manga_info::MangaInfoOutput::DownloadRequest(request) => {
            AppMsg::DownloadRequest(request)
        }
        crate::manga_info::MangaInfoOutput::Error(err) => AppMsg::Error(err),
    }
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Widgets = AppWidgets;

    type Init = Arc<MadoEngineState>;

    type Input = AppMsg;
    type Output = ();

    fn init(
        state: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let downloads = DownloadModel::builder().launch(()).detach();

        let manga_info = MangaInfoModel::builder()
            .launch(state.modules())
            .forward(sender.input_sender(), convert_manga_list);

        let observer = RelmMadoEngineStateObserver::new(
            sender.input_sender().clone(),
            downloads.sender().clone(),
        );
        observer.connect(&state);

        let model = Self {
            state,
            downloads,
            manga_info,

            root: root.clone(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    #[tracing::instrument(skip(self))]
    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            AppMsg::PushModule(module) => {
                tracing::trace!(
                    "Pushing module domain:{}, uuid:{}",
                    module.domain(),
                    module.uuid()
                );
            }
            AppMsg::DownloadRequest(info) => {
                self.state.download_request(info);
            }
            AppMsg::Error(error) => {
                gtk::MessageDialog::builder()
                    .text(error.to_string().as_str())
                    .transient_for(&self.root)
                    .build()
                    .show();
            }
        }
    }

    view! {
        gtk::ApplicationWindow {
            set_title: Some("Mado"),
            #[wrap(Some)]
            set_child = &gtk::Box {

                set_orientation: gtk::Orientation::Vertical,

                append = &gtk::StackSwitcher {
                    set_stack: Some(&stack)
                },

                append: stack = &gtk::Stack {
                    // Download tab
                    add_titled[Some("Download"), "Download"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        append: model.downloads.widget()
                    },
                    // Manga Info tab
                    add_titled[Some("Manga Info"), "Manga Info"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        append: model.manga_info.widget()
                    },
                    set_visible_child_name: "Download",
                },

            }
        }
    }
}
