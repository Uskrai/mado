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
pub use schema::{setup_schema, setup_schema_version, SCHEMA_VERSION};

#[cfg(test)]
mod tests {
    use super::*;
    use mado_engine::{
        core::ArcMadoModuleMap, DownloadChapterInfo, DownloadInfo, DownloadProgressStatus,
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
        for _ in 0..chapter_length.into() {
            vec.push(state.new_chapter());
        }

        Arc::new(DownloadInfo::new(
            state.module.clone(),
            "".to_string(),
            vec,
            Default::default(),
            None,
            mado_engine::DownloadStatus::InProgress(DownloadProgressStatus::Paused),
        ))
        //
    }

    pub struct State {
        pub module: LateBindingModule,
        pub map: ArcMadoModuleMap,
    }

    impl Default for State {
        fn default() -> Self {
            let uuid = Default::default();
            let state: MadoEngineState = Default::default();
            Self {
                module: LateBindingModule::WaitModule(state.modules(), uuid),
                map: state.modules(),
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
    }
    //
}
