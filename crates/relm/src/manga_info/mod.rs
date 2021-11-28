mod chapter_list;
mod components;
mod model;
mod widgets;

pub use components::MangaInfoComponents;
use mado_core::ArcWebsiteModuleMap;
pub use model::MangaInfoModel;
pub use widgets::MangaInfoWidgets;

type Msg = MangaInfoMsg;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct ChapterInfoWidget {
    root: gtk::Label,
}

pub trait MangaInfoParentModel {
    fn get_website_module_map(&self) -> ArcWebsiteModuleMap;
}

#[derive(Debug)]
pub enum MangaInfoMsg {
    Download,
    ShowError(mado_core::Error),
    /// Get info from string
    /// string should be convertible to URL
    GetInfo(String),
    Update(mado_core::MangaInfo),
    Clear,
}
