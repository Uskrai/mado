mod chapter_list;
mod components;
mod model;
mod widgets;

pub use components::MangaInfoComponents;
use mado_core::ArcMadoModuleMap;
pub use model::{MangaInfoModel, MangaInfoMsg, MangaInfoParentModel, MangaInfoParentMsg};
pub use widgets::MangaInfoWidgets;

type Msg = MangaInfoMsg;
