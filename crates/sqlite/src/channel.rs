use futures::{channel::mpsc, StreamExt};
use std::{collections::HashMap, sync::Arc};

use mado_engine::{
    core::{ArcMadoModule, ArcMadoModuleMap, Uuid},
    DownloadChapterInfo, DownloadChapterInfoMsg, DownloadInfo,
    MadoEngineState, MadoEngineStateMsg,
};

use crate::{
    download_chapters::DownloadChapterPK,
    downloads::DownloadPK,
    module::{InsertModule, Module},
    query::DownloadInfoJoin,
    status::DownloadStatus,
    Database,
};

#[derive(Debug)]
pub enum DbMsg {
    NewDownload(Arc<DownloadInfo>),
    PushModule(ArcMadoModule),
    DownloadStatusChanged(DownloadPK, DownloadStatus),
    DownloadChapterStatusChanged(DownloadChapterPK, DownloadStatus),
    Close,
}

pub struct Sender {}

pub struct Channel {
    rx: mpsc::UnboundedReceiver<DbMsg>,
    tx: mpsc::UnboundedSender<DbMsg>,
    db: Database,
    module: HashMap<Uuid, Module>,
}

pub fn channel(db: Database) -> Channel {
    let (tx, rx) = mpsc::unbounded();
    Channel {
        db,
        rx,
        tx,
        module: HashMap::new(),
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
            DbMsg::Close => {}
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
            self.connect_download_chapter(info.pk, info.chapter);
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
            }
            .ok();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn run_test() {
        let db = connection();

        let state = State::default();
        let info = setup_info_with_state(u8::MAX, &state);

        let mut rx = channel(Database::new(db).unwrap());

        rx.push_module(InsertModule {
            name: "",
            uuid: &Default::default(),
        }).unwrap();

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

        {
            // test that it is connected
            let it = rx.load_connect(state.map.clone()).unwrap();
            assert_eq!(it.len(), 1);
            let status = it[0].status().clone();

            assert_eq!(DownloadStatus::Finished, status.into());

            it[0].set_status(mado_engine::DownloadStatus::paused());
            rx.try_all().unwrap();

            let it = rx.load_connect(state.map).unwrap();
            let status = it[0].status().clone();
            assert_eq!(DownloadStatus::Paused, status.into());
        }
    }
}
