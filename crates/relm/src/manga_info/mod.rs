mod chapter_list;
mod components;
mod model;
mod widgets;

pub use components::MangaInfoComponents;
use mado_core::ArcWebsiteModuleMap;
pub use model::{MangaInfoModel, MangaInfoMsg};
pub use widgets::MangaInfoWidgets;

type Msg = MangaInfoMsg;

#[derive(Debug, Clone)]
struct ChapterInfoWidget {
    root: gtk::Label,
}

pub trait MangaInfoParentModel {
    fn get_website_module_map(&self) -> ArcWebsiteModuleMap;
}
