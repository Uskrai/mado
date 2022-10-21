use futures::{channel::mpsc, StreamExt};
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

use mado_engine::{
    core::{ArcMadoModule, ArcMadoModuleMap, Uuid},
    DownloadChapterImageInfo, DownloadChapterInfo, DownloadChapterInfoMsg, DownloadInfo,
    MadoEngineState, MadoEngineStateMsg,
};

use crate::{
    download_chapter_images::DownloadChapterImagePK,
    download_chapters::DownloadChapterPK,
    downloads::DownloadPK,
    module::{InsertModule, Module},
    query::{DownloadChapterImageInfoJoin, DownloadChapterInfoJoin, DownloadInfoJoin},
    status::DownloadStatus,
    Database,
};

#[derive(Debug)]
pub enum DbMsg {
    NewDownload(Arc<DownloadInfo>),
    PushModule(ArcMadoModule),
    DownloadStatusChanged(DownloadPK, DownloadStatus),
    DownloadChapterStatusChanged(DownloadChapterPK, DownloadStatus),
    DownloadChapterImagesChanged(DownloadChapterPK, Vec<Arc<DownloadChapterImageInfo>>),
    DownloadChapterImageStatusChanged(DownloadChapterImagePK, DownloadStatus),
    Close,
}

pub struct Sender {}

pub struct Channel {
    rx: mpsc::UnboundedReceiver<DbMsg>,
    tx: mpsc::UnboundedSender<DbMsg>,
    db: Database,
    module: HashMap<Uuid, Module>,
    download_chapter_images: Mutex<HashMap<DownloadChapterPK, Vec<mado_engine::AnyObserverHandleSend>>>,
}

pub fn channel(db: Database) -> Channel {
    let (tx, rx) = mpsc::unbounded();
    Channel {
        db,
        rx,
        tx,
        module: HashMap::new(),
        download_chapter_images: Default::default(),
    }
}

impl Channel {
    /// Handle next message.
    pub fn try_next(&mut self) -> Result<(), rusqlite::Error> {
        if let Ok(Some(msg)) = self.rx.try_next() {
            self.handle_msg(msg)?;
        }
        Ok(())
    }

    /// Handle all message until empty or fail.
    pub fn try_all(&mut self) -> Result<(), rusqlite::Error> {
        while let Ok(Some(msg)) = self.rx.try_next() {
            self.handle_msg(msg)?;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), rusqlite::Error> {
        while let Some(msg) = self.rx.next().await {
            self.handle_msg(msg)?;
        }

        Ok(())
    }

    pub fn handle_msg(&mut self, msg: DbMsg) -> Result<(), rusqlite::Error> {
        match msg {
            DbMsg::NewDownload(info) => {
                let module = &self.module[info.module_uuid()];
                let dl = self.db.insert_download(module.pk, &info)?;
                self.connect_info(dl);
            }
            DbMsg::PushModule(module) => {
                self.push_module(InsertModule {
                    name: module.name(),
                    uuid: &module.uuid(),
                })?;
            }
            DbMsg::DownloadStatusChanged(id, status) => {
                self.db.update_download_status(id, status)?;
            }
            DbMsg::DownloadChapterStatusChanged(pk, status) => {
                self.db.update_download_chapter_status(pk, status)?;
            }
            DbMsg::DownloadChapterImagesChanged(ch_pk, images) => {
                let image = self.db.update_download_chapter_images(ch_pk, images)?;

                self.connect_download_chapter_images(ch_pk, image);
            }
            DbMsg::DownloadChapterImageStatusChanged(pk, status) => {
                self.db.update_download_chapter_image_status(pk, status)?;
            }
            DbMsg::Close => {
                self.sender().close_channel();
            }
        }

        Ok(())
    }

    pub fn push_module(&mut self, module: InsertModule<'_>) -> Result<(), rusqlite::Error> {
        let info = self.db.insert_module(module)?;
        self.module.insert(info.uuid, info);
        Ok(())
    }

    /// load DownloadInfo and connect to this.
    /// the returned result can be used to create MadoEngineState
    ///
    /// ```ignore
    /// # sea_orm::DbErr;
    /// # use mado_engine::MadoEngineState;
    /// # async fn main() -> Result<(), DbErr> {
    /// let channel = channel(db);
    /// let map = Arc::new(mado_core::MutexMadoModuleMap::new(
    ///     mado_core::DefaultMadoModuleMap::new(),
    /// ));
    ///
    /// let items = channel.load_connect(map.clone()).await?;
    ///
    /// let state = MadoEngineState::new(map, items);
    /// channel.connect_only(&state);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_connect(
        &self,
        module_map: ArcMadoModuleMap,
    ) -> Result<Vec<Arc<DownloadInfo>>, rusqlite::Error> {
        let infos = self.db.load_download_info(module_map)?;

        let mut vec = Vec::new();
        for it in infos {
            vec.push(it.info.clone());
            self.connect_info(it);
        }

        Ok(vec)
    }

    fn connect_info(&self, join: DownloadInfoJoin) {
        for info in join.chapters {
            self.connect_download_chapter(info.pk, info.chapter.clone());

            self.connect_download_chapter_images(info.pk, info.images);
        }

        let tx = self.tx.clone();
        let dl_pk = join.pk;

        join.info.connect_only(move |msg| {
            match msg {
                mado_engine::DownloadInfoMsg::StatusChanged(status) => tx
                    .unbounded_send(DbMsg::DownloadStatusChanged(dl_pk, status.into()))
                    .ok(),
            };
        });
    }

    pub fn connect_only(&self, state: &MadoEngineState) {
        let sender = self.sender();
        state.connect_only({
            move |msg| match msg {
                MadoEngineStateMsg::Download(info) => {
                    sender.unbounded_send(DbMsg::NewDownload(info.clone())).ok();
                }
                MadoEngineStateMsg::PushModule(module) => {
                    sender
                        .unbounded_send(DbMsg::PushModule(module.clone()))
                        .ok();
                }
            }
        });
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<DbMsg> {
        self.tx.clone()
    }

    pub fn send(&self, msg: DbMsg) -> Result<(), mpsc::TrySendError<DbMsg>> {
        self.tx.unbounded_send(msg)
    }

    fn connect_download_chapter(&self, pk: DownloadChapterPK, info: Arc<DownloadChapterInfo>) {
        let tx = self.tx.clone();
        info.connect_only(move |msg| {
            match msg {
                DownloadChapterInfoMsg::StatusChanged(status) => {
                    tx.unbounded_send(DbMsg::DownloadChapterStatusChanged(pk, status.into()))
                }
                DownloadChapterInfoMsg::DownloadImagesChanged(images) => {
                    tx.unbounded_send(DbMsg::DownloadChapterImagesChanged(pk, images.clone()))
                }
            }
            .ok();
        });
    }

    fn connect_download_chapter_images(
        &self,
        ch_pk: DownloadChapterPK,
        info: Vec<DownloadChapterImageInfoJoin>,
    ) {
        let mut vec = Vec::new();
        for it in info {
            let handle = self.connect_download_chapter_image(ch_pk, it.pk, it.image);
            vec.push(handle);
        }

        // make sure observer that is attached before disconnected
        let before = self.download_chapter_images.lock().insert(ch_pk, vec);
        if let Some(handles) = before {
            for it in handles {
                it.disconnect();
            }
        }
    }

    fn connect_download_chapter_image(
        &self,
        _: DownloadChapterPK,
        pk: DownloadChapterImagePK,
        info: Arc<DownloadChapterImageInfo>,
    ) -> mado_engine::AnyObserverHandleSend {
        let tx = self.tx.clone();

        info.connect_only(move |msg| {
            match msg {
                mado_engine::DownloadChapterImageInfoMsg::StatusChanged(status) => {
                    tx.unbounded_send(DbMsg::DownloadChapterImageStatusChanged(pk, status.into()))
                }
            }
            .ok();
        })
        .send_handle_any()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mado_core::{MadoModule, MockMadoModule, Url};
    use mado_engine::{core::MangaInfo, path::Utf8PathBuf};

    use super::*;
    use crate::tests::*;

    fn mock_module(uuid: Uuid) -> MockMadoModule {
        let mut module = MockMadoModule::new();
        module.expect_name().times(0..).return_const("".to_string());
        module.expect_uuid().times(0..).return_const(uuid);
        module
            .expect_domain()
            .times(0..)
            .return_const(Url::from_str("http://localhost").unwrap());

        module
    }

    #[test]
    fn connect_test() {
        let db = connection();

        let state = State::default();
        // let info = setup_info_with_state(u8::MAX, &state);

        let mut rx = channel(Database::new(db).unwrap());
        rx.connect_only(&state.engine);

        let module = Arc::new(mock_module(Uuid::default()));

        state.engine.push_module(module.clone()).unwrap();
        rx.try_all().unwrap();
        assert!(rx.module.get(&module.uuid()).is_some());

        let req = mado_engine::DownloadRequest::new(
            module,
            Arc::new(MangaInfo::default()),
            vec![],
            Utf8PathBuf::from_str("./path").unwrap(),
            None,
            mado_engine::DownloadRequestStatus::Pause,
        );

        state.engine.download_request(req);
        rx.try_all().unwrap();
        let dl = rx.db.load_download().unwrap();
        assert_eq!(dl.len(), 1);
    }

    #[test]
    fn run_test() {
        let db = connection();

        let state = State::default();
        let info = setup_info_with_state(u8::MAX, &state);

        let mut rx = channel(Database::new(db).unwrap());
        rx.connect_only(&state.engine);

        let module = Arc::new(mock_module(Uuid::default()));

        state.engine.push_module(module).unwrap();

        rx.send(DbMsg::NewDownload(info.clone())).unwrap();
        rx.try_all().unwrap();

        {
            let it = rx.db.load_download().unwrap();
            assert_eq!(it.len(), 1);

            let status = it[0].download.status.clone();

            assert_eq!(status, DownloadStatus::Paused);
        }

        info.set_status(mado_engine::DownloadStatus::finished());
        rx.try_all().unwrap();

        {
            let it = rx.db.load_download().unwrap();
            assert_eq!(it.len(), 1);

            let status = it[0].download.status.clone();
            assert_eq!(status, DownloadStatus::Finished);
        }

        info.chapters()[0].set_status(mado_engine::DownloadStatus::Finished);
        rx.try_all().unwrap();

        {
            let it = rx.db.load_download().unwrap();
            let ch = &it[0].chapters[0];
            assert_eq!(ch.chapter.status, DownloadStatus::Finished);
        }

        {
            // test that it is connected
            let it = rx.load_connect(state.map.clone()).unwrap();
            assert_eq!(it.len(), 1);
            let status = it[0].status().clone();

            assert_eq!(DownloadStatus::Finished, status.into());

            it[0].set_status(mado_engine::DownloadStatus::paused());
            rx.try_all().unwrap();

            let it = rx.load_connect(state.map.clone()).unwrap();
            let status = it[0].status().clone();
            assert_eq!(DownloadStatus::Paused, status.into());
        }

        state.populate_chapter_image(info.chapters()[0].clone(), 2);
        rx.try_all().unwrap();

        {
            let it = rx.load_connect(state.map.clone()).unwrap();
            let ch = &it[0].chapters()[0];
            assert_eq!(ch.images().len(), 2);
            let image = &ch.images()[0];
            assert_eq!(DownloadStatus::Finished, image.status().clone().into());

            image.set_status(mado_engine::DownloadStatus::waiting());
            rx.try_all().unwrap();

            let it = rx.load_connect(state.map.clone()).unwrap();
            assert_eq!(
                DownloadStatus::Finished,
                it[0].chapters()[0].images()[0].status().clone().into()
            );
        }
    }

    #[test]
    #[ntest::timeout(1000)]
    pub fn close_test() {
        futures::executor::block_on(async {
            let db = connection();

            let state = State::default();

            let mut rx = channel(Database::new(db).unwrap());
            rx.connect_only(&state.engine);

            rx.send(DbMsg::Close).unwrap();
            rx.run().await.unwrap();
        });
    }
}
