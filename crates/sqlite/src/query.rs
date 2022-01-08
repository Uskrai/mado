use std::{collections::HashMap, sync::Arc};

use mado_engine::{core::ArcMadoModuleMap, DownloadChapterInfo, DownloadInfo, LateBindingModule};
use rusqlite::{Connection, Error};

use crate::{
    download_chapters::{DownloadChapter, DownloadChapterPK},
    downloads::{Download, DownloadPK},
};

#[derive(Debug)]
pub struct DownloadJoin {
    pub download: Download,
    pub chapters: Vec<DownloadChapter>,
}

pub fn load_download_join(conn: &Connection) -> Result<Vec<DownloadJoin>, Error> {
    let downloads = crate::downloads::load(&conn)?;
    let mut index_map = HashMap::new();

    let mut download_joins = Vec::new();
    for (i, download) in downloads.into_iter().enumerate() {
        index_map.insert(download.pk, i);
        download_joins.push(DownloadJoin {
            download,
            chapters: Vec::new(),
        });
    }

    let chapters = crate::download_chapters::load(&conn)?;

    for (download_id, it) in chapters {
        let index = index_map[&download_id];

        debug_assert_eq!(download_joins[index].download.pk, download_id);
        download_joins[index].chapters = it;
    }

    Ok(download_joins)
}

pub struct DownloadInfoJoin {
    pub pk: DownloadPK,
    pub info: Arc<DownloadInfo>,
    pub chapters: Vec<DownloadChapterInfoJoin>,
}

pub struct DownloadChapterInfoJoin {
    pub pk: DownloadChapterPK,
    pub chapter: Arc<DownloadChapterInfo>,
}

pub fn load_download_info_join(
    conn: &Connection,
    module_map: ArcMadoModuleMap,
) -> Result<Vec<DownloadInfoJoin>, Error> {
    let joins = load_download_join(conn)?;
    let mut downloads = Vec::new();

    for join in joins {
        let download = join.download;
        let dl_pk = download.pk;

        let chapters = join.chapters;

        let module = LateBindingModule::WaitModule(module_map.clone(), download.module_id);

        let chapters_join: Vec<_> = chapters
            .into_iter()
            .map(|chapter| {
                let pk = chapter.pk;
                let module = module.clone();
                let chapter = Arc::new(DownloadChapterInfo::new(
                    module,
                    chapter.chapter_id,
                    chapter.title,
                    chapter.path,
                    chapter.status.into(),
                ));

                DownloadChapterInfoJoin { pk, chapter }
            })
            .collect();

        let chapters: Vec<_> = chapters_join.iter().map(|it| it.chapter.clone()).collect();

        let info = Arc::new(DownloadInfo::new(
            LateBindingModule::WaitModule(module_map.clone(), download.module_id),
            download.title,
            chapters,
            download.path,
            download.url,
            download.status.into(),
        ));

        let join = DownloadInfoJoin {
            pk: dl_pk,
            info,
            chapters: chapters_join,
        };

        downloads.push(join);
    }

    Ok(downloads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{downloads::DownloadPK, tests::*};

    #[test]
    fn insert_test() {
        let mut db = connection();

        const CHAPTER_LENGTH: u8 = u8::MAX;
        let info = setup_info(CHAPTER_LENGTH);

        let insert = crate::downloads::insert_info(&mut db, &info).unwrap();
        assert_eq!(insert.pk, DownloadPK::new(1));

        let count: i64 = db
            .query_row(
                "SELECT COUNT(id) from download_chapters WHERE download_id=?",
                [insert.pk.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, CHAPTER_LENGTH as i64);

        let downloads = load_download_join(&db).unwrap();
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].chapters.len(), CHAPTER_LENGTH as usize);
    }

    #[test]
    fn delete_test() {
        let db = connection();

        // crate::downloads::Entity::insert(crate::downloads::default_model())
        //     .exec(&db.conn)
        //     .await
        //     .unwrap();

        // crate::download_chapters::Entity::insert(crate::download_chapters::default_model(1, 1))
        //     .exec(&db.conn)
        //     .await
        //     .unwrap();
        //
        // db.delete_download(1).await.unwrap();
    }

    #[test]
    fn load_test() {
        // let db = connection().await;
        // let state = State::default();
        //
        // const CHAPTER_LENGTH: usize = u8::MAX as usize;
        //
        // let mut vec = Vec::new();
        // for _ in 0..CHAPTER_LENGTH {
        //     vec.push(state.new_chapter());
        // }
        //
        // let info = DownloadInfo::new(
        //     state.module.clone(),
        //     "title".to_string(),
        //     vec,
        //     Default::default(),
        //     None,
        //     mado_engine::DownloadStatus::Finished,
        // );
        //
        // db.insert_download(&info).await.unwrap();
        // db.insert_download(&info).await.unwrap();
        // db.insert_download(&info).await.unwrap();
        //
        // let item = db.load_download().await.unwrap();
        //
        // assert_eq!(item.len(), 3);
        //
        // for (_, ch) in item {
        //     assert_eq!(ch.len(), CHAPTER_LENGTH);
        // }
    }
}
