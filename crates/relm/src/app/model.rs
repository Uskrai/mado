use super::{AppComponents, AppWidgets};
use crate::{
    download::DownloadMsg,
    manga_info::{MangaInfoParentModel, MangaInfoParentMsg},
};
use mado_core::{ArcMadoModule, ArcMadoModuleMap};
use mado_engine::{DownloadRequest, MadoEngineState, MadoEngineStateMsg};
use relm4::{AppUpdate, Model};
use std::sync::Arc;

pub enum AppMsg {
    PushModule(ArcMadoModule),
    DownloadRequest(DownloadRequest),
}

impl MangaInfoParentMsg for AppMsg {
    fn download_request(request: mado_engine::DownloadRequest) -> Self {
        AppMsg::DownloadRequest(request)
    }
}

pub struct AppModel {
    /// state.send will be called on [`AppComponents::init_components`]
    pub(super) state: Arc<MadoEngineState>,
}

impl AppModel {
    pub fn new(state: Arc<MadoEngineState>) -> Self {
        Self { state }
    }
}

impl MangaInfoParentModel for AppModel {
    fn get_website_module_map(&self) -> ArcMadoModuleMap {
        self.state.modules()
    }
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    #[tracing::instrument(skip_all)]
    fn update(
        &mut self,
        msg: Self::Msg,
        _: &Self::Components,
        _: relm4::Sender<Self::Msg>,
    ) -> bool {
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
        }
        true
    }
}

pub struct RelmMadoEngineStateObserver {
    #[allow(dead_code)]
    state: Arc<MadoEngineState>,
    sender: relm4::Sender<AppMsg>,
    download_sender: relm4::Sender<DownloadMsg>,
}

impl RelmMadoEngineStateObserver {
    pub fn new(
        state: Arc<MadoEngineState>,
        sender: relm4::Sender<AppMsg>,
        download_sender: relm4::Sender<DownloadMsg>,
    ) -> Self {
        Self {
            state,
            sender,
            download_sender,
        }
    }

    pub fn connect(self, state: &Arc<MadoEngineState>) {
        state.connect(move |msg| {
            match msg {
                MadoEngineStateMsg::Download(info) => self
                    .download_sender
                    .send(DownloadMsg::CreateDownloadView(info.clone()))
                    .ok(),
                MadoEngineStateMsg::PushModule(module) => {
                    self.sender.send(AppMsg::PushModule(module.clone())).ok()
                }
            };
        })
    }
}
