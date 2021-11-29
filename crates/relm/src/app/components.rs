use crate::manga_info::MangaInfoModel;

use super::{AppModel, AppWidgets};
use mado_core::WebsiteModuleMap;
use relm4::{Components, RelmComponent};

pub struct AppComponents<Map: WebsiteModuleMap> {
    pub(super) manga_info: RelmComponent<MangaInfoModel, AppModel<Map>>,
}

impl<Map: WebsiteModuleMap> Components<AppModel<Map>> for AppComponents<Map> {
    fn init_components(
        parent_model: &AppModel<Map>,
        parent_widget: &AppWidgets,
        parent_sender: relm4::Sender<super::AppMsg>,
    ) -> Self {
        Self {
            manga_info: RelmComponent::new(parent_model, parent_widget, parent_sender),
        }
    }
}
