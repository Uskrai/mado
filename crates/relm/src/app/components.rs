use crate::{download::DownloadModel, manga_info::MangaInfoModel};

use super::AppModel;
use relm4::{Components, RelmComponent};

pub struct AppComponents {
    pub(super) manga_info: RelmComponent<MangaInfoModel, AppModel>,
    pub(super) download: RelmComponent<DownloadModel, AppModel>,
}

impl Components<AppModel> for AppComponents {
    fn init_components(
        parent_model: &AppModel,
        parent_sender: relm4::Sender<super::AppMsg>,
    ) -> Self {
        let this = Self {
            manga_info: RelmComponent::new(parent_model, parent_sender.clone()),
            download: RelmComponent::new(parent_model, parent_sender.clone()),
        };

        let observer = crate::RelmMadoEngineStateObserver::new(
            parent_model.state.clone(),
            parent_sender,
            this.download.sender(),
        );

        parent_model.state.connect(observer);

        this
    }

    fn connect_parent(&mut self, parent_widgets: &<AppModel as relm4::Model>::Widgets) {
        self.manga_info.connect_parent(parent_widgets);
        self.download.connect_parent(parent_widgets);
    }
}
