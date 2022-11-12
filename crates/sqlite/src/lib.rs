mod channel;
mod database;
mod query;
mod schema;
mod status;

pub mod download_chapter_images;
pub mod download_chapters;
pub mod downloads;
pub mod module;

pub use channel::{channel, Channel, DbMsg, Sender};
pub use database::Database;
pub use query::load_download_join;
pub use schema::{setup_schema, setup_schema_version};

#[cfg(test)]
mod tests {
    use super::*;
    use mado_core::ChapterImageInfo;
    use mado_engine::{
        core::ArcMadoModuleMap, DownloadChapterImageInfo, DownloadChapterInfo, DownloadInfo,
        LateBindingModule, MadoEngineState,
    };
    use rusqlite::Connection;
    use std::sync::Arc;

    pub fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();

        schema::setup_schema(&connection).unwrap();
        connection
    }

    pub fn setup_info(chapter_length: impl Into<i64>) -> Arc<DownloadInfo> {
        let state = State::default();
        setup_info_with_state(chapter_length, &state)
    }

    pub fn setup_info_with_state(
        chapter_length: impl Into<i64>,
        state: &State,
    ) -> Arc<DownloadInfo> {
        let mut vec = Vec::new();
        for i in 0..chapter_length.into() {
            let ch = state.new_chapter();
            state.populate_chapter_image(ch.clone(), i % 4);
            vec.push(ch);
        }

        Arc::new(
            DownloadInfo::builder()
                .order(0)
                .module(state.module.clone())
                .chapters(vec)
                .status(mado_engine::DownloadStatus::paused())
                .build(),
        )
    }

    pub struct State {
        pub module: LateBindingModule,
        pub map: ArcMadoModuleMap,
        pub engine: Arc<MadoEngineState>,
    }

    impl Default for State {
        fn default() -> Self {
            let uuid = Default::default();
            let engine: MadoEngineState = Default::default();
            Self {
                module: LateBindingModule::WaitModule(engine.modules(), uuid),
                map: engine.modules(),
                engine: Arc::new(engine),
            }
        }
    }

    impl State {
        pub fn new_chapter(&self) -> Arc<DownloadChapterInfo> {
            Arc::new(DownloadChapterInfo::new(
                self.module.clone(),
                "id".to_string(),
                "title".to_string(),
                "path".into(),
                mado_engine::DownloadStatus::Finished,
            ))
        }

        pub fn new_image(&self) -> Arc<DownloadChapterImageInfo> {
            Arc::new(DownloadChapterImageInfo::new(
                ChapterImageInfo {
                    id: "id".to_string(),
                    name: Some("1.png".to_string()),
                    extension: "png".to_string(),
                },
                "path".into(),
                mado_engine::DownloadStatus::Finished,
            ))
        }

        pub fn populate_chapter_image(
            &self,
            chapter: Arc<DownloadChapterInfo>,
            length: impl Into<i64>,
        ) {
            let mut vec = Vec::new();

            for _ in 0..length.into() {
                vec.push(self.new_image());
            }

            chapter.set_images(vec);
        }
    }
    //
}
