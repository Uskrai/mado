use std::sync::Arc;

use crate::{download::DownloadModel, manga_info::MangaInfoModel, RelmMadoSender};

use super::{AppModel, AppWidgets};
use mado_core::MadoModuleMap;
use mado_engine::MadoMsg;
use relm4::{Components, RelmComponent};

pub struct AppComponents<Map: MadoModuleMap> {
    pub(super) manga_info: RelmComponent<MangaInfoModel, AppModel<Map>>,
    pub(super) download: RelmComponent<DownloadModel, AppModel<Map>>,
}

impl<Map: MadoModuleMap> Components<AppModel<Map>> for AppComponents<Map> {
    fn init_components(
        parent_model: &AppModel<Map>,
        parent_widget: &AppWidgets,
        parent_sender: relm4::Sender<super::AppMsg>,
    ) -> Self {
        let this = Self {
            manga_info: RelmComponent::new(parent_model, parent_widget, parent_sender.clone()),
            download: RelmComponent::new(parent_model, parent_widget, parent_sender.clone()),
        };

        let sender = Arc::new(RelmMadoSender::new(parent_sender, this.download.sender()));
        let msg = MadoMsg::Start(sender);
        parent_model.state.send(msg).expect("can't send msesage");

        this
    }
}
