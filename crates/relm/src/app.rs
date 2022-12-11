use crate::{
    download::{DownloadModel, DownloadMsg, DownloadOutputMsg},
    manga_info::{MangaInfoModel, MangaInfoMsg, MangaInfoOutput},
};
use gtk::prelude::*;
use mado::engine::{DownloadRequest, MadoEngineState, MadoEngineStateMsg};
use mado::{core::ArcMadoModule, engine::path::Utf8PathBuf};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
};
use std::sync::Arc;

#[derive(Debug)]
pub enum AppMsg {
    PushModule(ArcMadoModule),
    DownloadRequest(DownloadRequest),
    OpenManga {
        url: mado_core::Url,
        path: Utf8PathBuf,
    },
    ChangeVisibleChild(String),
    Error(mado::core::Error),
}

pub struct AppModel {
    /// state.send will be called on [`AppComponents::init_components`]
    pub(super) state: Arc<MadoEngineState>,

    downloads: Controller<DownloadModel>,
    manga_info: Controller<MangaInfoModel>,

    visible_child: String,
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

pub fn convert_downloads(msg: DownloadOutputMsg) -> AppMsg {
    match msg {
        DownloadOutputMsg::OpenManga { url, path } => AppMsg::OpenManga { url, path },
    }
}

// pub fn convert_downloads(msg: )

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
        let downloads = DownloadModel::builder()
            .launch(())
            .forward(sender.input_sender(), convert_downloads);

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
            visible_child: "Download".to_string(),
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
            AppMsg::OpenManga { url, path } => {
                self.manga_info.emit(MangaInfoMsg::GetInfo {
                    url: url.to_string(),
                    path: Some(path.to_string()),
                });
                self.visible_child = "Manga Info".to_string();
            }
            AppMsg::ChangeVisibleChild(string) => {
                self.visible_child = string;
            }
            AppMsg::Error(error) => {
                let error = format!("{:#}", error);
                gtk::MessageDialog::builder()
                    .text(&error)
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

                #[name = "stack"]
                append = &gtk::Stack {
                    // Download tab
                    #[name = "download"]
                    add_titled[Some("Download"), "Download"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        append: model.downloads.widget()
                    },
                    // Manga Info tab
                    #[name = "manga_info"]
                    add_titled[Some("Manga Info"), "Manga Info"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        append: model.manga_info.widget()
                    },

                    #[track(Some(model.visible_child.as_str()) != stack.visible_child_name().as_ref().map(|it| it.as_str()))]
                    set_visible_child_name: &model.visible_child,

                    connect_visible_child_name_notify[sender] => move |stack| {
                        if let Some(it) = stack.visible_child_name() {
                            sender.input(AppMsg::ChangeVisibleChild(it.to_string()))
                        }
                    }
                },

            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mado::engine::MadoEngine;
    use mado_core::{DefaultMadoModuleMap, MangaInfo, MutexMadoModuleMap, Url, Uuid};

    use super::*;
    use crate::tests::*;

    fn state() -> MadoEngine{
        let map = DefaultMadoModuleMap::new();
        let map = MutexMadoModuleMap::new(map);
        let map = Arc::new(map);
        let state = MadoEngineState::new(map, Default::default());

        MadoEngine::new(state)
    }

    #[gtk::test]
    fn test_app() {
        let mado = state();
        let app = AppModel::builder().launch(mado.state()).detach();

        let mut module = mado_core::MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(1));
        module
            .expect_domain()
            .return_const(Url::parse("https://localhost").unwrap());

        let module = Arc::new(module);

        mado.state().push_module(module.clone()).unwrap();

        app.emit(AppMsg::DownloadRequest(DownloadRequest::new(
            module,
            Arc::new(MangaInfo::default()),
            vec![],
            "path".into(),
            None,
            mado::engine::DownloadRequestStatus::Pause,
        )));
        run_loop();

        assert_eq!(app.model().downloads.model().task_len(), 1);
    }

    #[gtk::test]
    fn test_open_manga() {
        let mado = state();
        let app = AppModel::builder().launch(mado.state()).detach();

        let url = Url::parse("https://localhost").unwrap();
        let path = Utf8PathBuf::from("path");
        app.emit(AppMsg::OpenManga { url, path });
        run_loop();

        assert_eq!(app.model().visible_child, "Manga Info");
    }
}
