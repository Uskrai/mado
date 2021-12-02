use super::{AppComponents, AppWidgets};
use crate::{
    download::DownloadMsg,
    manga_info::{MangaInfoParentModel, MangaInfoParentMsg},
};
use mado_core::{
    ArcMadoModule, ArcMadoModuleMap, MadoModuleMap, MutMadoModuleMap, MutexMadoModuleMap,
};
use mado_engine::{DownloadInfo, MadoEngineState, MadoMsg, MadoSender};
use relm4::{AppUpdate, Model};
use std::sync::Arc;

pub enum AppMsg {
    PushModule(ArcMadoModule),
    Download(DownloadInfo),
}

impl MangaInfoParentMsg for AppMsg {
    fn download(
        module: ArcMadoModule,
        manga: Arc<mado_core::MangaInfo>,
        chapters: Vec<Arc<mado_core::ChapterInfo>>,
        path: std::path::PathBuf,
    ) -> Self {
        Self::Download(DownloadInfo {
            module,
            manga,
            chapters,
            path,
        })
    }
}

pub struct AppModel<Map: MadoModuleMap> {
    modules: Arc<MutexMadoModuleMap<Map>>,
    /// state.send will be called on [`AppComponents::init_components`]
    pub(super) state: Arc<MadoEngineState>,
}

impl<Map: MadoModuleMap> AppModel<Map> {
    pub fn new(map: Map, state: Arc<MadoEngineState>) -> Self {
        Self {
            modules: std::sync::Arc::new(MutexMadoModuleMap::new(map)),
            state,
        }
    }
}

impl<Map: MadoModuleMap> MangaInfoParentModel for AppModel<Map> {
    fn get_website_module_map(&self) -> ArcMadoModuleMap {
        self.modules.clone()
    }
}

impl<Map: MadoModuleMap> Model for AppModel<Map> {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents<Map>;
}

impl<Map: MadoModuleMap> AppUpdate for AppModel<Map> {
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
                    module.get_domain(),
                    module.get_uuid()
                );
                self.modules.push_mut(module).unwrap();
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct RelmMadoSender {
    sender: relm4::Sender<AppMsg>,
    download_sender: relm4::Sender<DownloadMsg>,
}

impl RelmMadoSender {
    pub fn new(sender: relm4::Sender<AppMsg>, download_sender: relm4::Sender<DownloadMsg>) -> Self {
        Self {
            sender,
            download_sender,
        }
    }
}

impl MadoSender for RelmMadoSender {
    fn push_module(&self, module: ArcMadoModule) {
        self.sender.send(AppMsg::PushModule(module)).unwrap();
    }

    fn create_download_view(
        &self,
        download: Arc<DownloadInfo>,
        controller: mado_engine::DownloadController,
    ) {
        self.download_sender
            .send(DownloadMsg::CreateDownloadView(download, controller))
            .unwrap();
    }
}
