use crate::manga_info::MangaInfoModel;

use super::{AppModel, AppWidgets};
use relm4::{Components, RelmComponent};

pub struct AppComponents {
  pub(super) manga_info: RelmComponent<MangaInfoModel, AppModel>,
}

impl Components<AppModel> for AppComponents {
  fn init_components(
    parent_model: &AppModel,
    parent_widget: &AppWidgets,
    parent_sender: relm4::Sender<super::AppMsg>,
  ) -> Self {
    Self {
      manga_info: RelmComponent::new(
        parent_model,
        parent_widget,
        parent_sender,
      ),
    }
  }
}
