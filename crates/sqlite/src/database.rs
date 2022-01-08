use std::sync::Arc;

use mado_engine::{core::ArcMadoModuleMap, DownloadInfo};
use rusqlite::{Connection, Error};

use crate::{
    download_chapters::DownloadChapterPK,
    downloads::DownloadPK,
    query::{DownloadInfoJoin, DownloadJoin},
    status::DownloadStatus,
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(conn: Connection) -> Result<Self, Error> {
        crate::setup_schema(&conn)?;

        Ok(Self { conn })
    }

    pub fn open(path: &str) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        Self::new(conn)
    }

    /// Insert download into database and return the id of inserted download
    pub fn insert_download(
        &mut self,
        download: &Arc<DownloadInfo>,
    ) -> Result<DownloadInfoJoin, Error> {
        crate::downloads::insert_info(&mut self.conn, download)
    }

    pub fn update_download_status(
        &self,
        pk: DownloadPK,
        status: DownloadStatus,
    ) -> Result<usize, Error> {
        crate::downloads::update_status(&self.conn, pk, status)
    }

    pub fn update_download_chapter_status(
        &self,
        pk: DownloadChapterPK,
        status: DownloadStatus,
    ) -> Result<usize, Error> {
        crate::download_chapters::update_status(&self.conn, pk, status)
    }

    pub fn load_download(&self) -> Result<Vec<DownloadJoin>, Error> {
        crate::query::load_download_join(&self.conn)
    }

    pub fn load_download_info(
        &self,
        module_map: ArcMadoModuleMap,
    ) -> Result<Vec<DownloadInfoJoin>, Error> {
        crate::query::load_download_info_join(&self.conn, module_map)
        // let joins = self.load_download()?;
        // let mut downloads = Vec::new();
        //
        // for join in joins {
        //     let download = join.download;
        //
        //     let chapters = join.chapters;
        //
        //     let mut chapters_id = Vec::new();
        //
        //     for it in &chapters {
        //         chapters_id.push(it.pk);
        //     }
        //
        //     let chapters = chapters
        //         .into_iter()
        //         .map(|chapter| {
        //             Arc::new(DownloadChapterInfo::new(
        //                 LateBindingModule::WaitModule(module_map.clone(), download.module_id),
        //                 chapter.chapter_id,
        //                 chapter.title,
        //                 chapter.path,
        //                 chapter.status.into(),
        //             ))
        //         })
        //         .collect();
        //
        //     let info = Arc::new(DownloadInfo::new(
        //         LateBindingModule::WaitModule(module_map.clone(), download.module_id),
        //         download.title,
        //         chapters,
        //         download.path,
        //         download.url,
        //         download.status.into(),
        //     ));
        //
        //     downloads.push((download.pk, chapters_id, info));
        // }
        //
        // Ok(downloads)
    }
}
