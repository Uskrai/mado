use std::{collections::HashMap, sync::Arc};

use mado_engine::{
    core::{ArcMadoModuleMap, ChapterImageInfo},
    DownloadChapterImageInfo, DownloadChapterInfo, DownloadInfo, LateBindingModule,
};
use rusqlite::{Connection, Error};

use crate::{
    download_chapter_images::{DownloadChapterImage, DownloadChapterImagePK},
    download_chapters::{DownloadChapter, DownloadChapterPK},
    downloads::{Download, DownloadPK},
};

#[derive(Debug)]
pub struct DownloadJoin {
    pub download: Download,
    pub chapters: Vec<DownloadChapterJoin>,
}

#[derive(Debug)]
pub struct DownloadChapterJoin {
    pub chapter: DownloadChapter,
    pub images: Vec<DownloadChapterImageJoin>,
}

#[derive(Debug)]
pub struct DownloadChapterImageJoin {
    image: DownloadChapterImage,
}

pub fn load_download_join(conn: &Connection) -> Result<Vec<DownloadJoin>, Error> {
    let downloads = crate::downloads::load(conn)?;
    let mut download_index_map = HashMap::new();

    let mut download_joins = Vec::new();
    for (i, download) in downloads.into_iter().enumerate() {
        download_index_map.insert(download.pk, i);
        download_joins.push(DownloadJoin {
            download,
            chapters: Vec::new(),
        });
    }

    let mut chapter_index_map = HashMap::new();
    let chapters = crate::download_chapters::load(conn)?;

    for (download_id, it) in chapters {
        for (i, ch) in it.iter().enumerate() {
            chapter_index_map.insert(ch.pk, (download_id, i));
        }
        let index = download_index_map[&download_id];

        debug_assert_eq!(download_joins[index].download.pk, download_id);
        download_joins[index].chapters = it
            .into_iter()
            .map(|it| DownloadChapterJoin {
                chapter: it,
                images: vec![],
            })
            .collect();
    }

    let images = crate::download_chapter_images::load(conn)?;
    for (chapter_id, it) in images {
        let (download_id, chapter_index) = chapter_index_map[&chapter_id];
        let download_index = download_index_map[&download_id];

        debug_assert_eq!(download_joins[download_index].download.pk, download_id);
        debug_assert_eq!(
            download_joins[download_index].chapters[chapter_index]
                .chapter
                .pk,
            chapter_id
        );
        download_joins[download_index].chapters[chapter_index].images = it
            .into_iter()
            .map(|it| DownloadChapterImageJoin { image: it })
            .collect();
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
    pub images: Vec<DownloadChapterImageInfoJoin>,
}

pub struct DownloadChapterImageInfoJoin {
    pub pk: DownloadChapterImagePK,
    pub image: Arc<DownloadChapterImageInfo>,
}

pub fn load_download_info_join(
    conn: &Connection,
    module_map: ArcMadoModuleMap,
) -> Result<Vec<DownloadInfoJoin>, Error> {
    let module = crate::module::load_map(conn)?;

    let joins = load_download_join(conn)?;
    let mut downloads = Vec::new();

    for join in joins {
        let download = join.download;
        let dl_pk = download.pk;

        let chapters = join.chapters;

        let module =
            LateBindingModule::WaitModule(module_map.clone(), module[&download.module_pk].uuid);

        let chapters_join: Vec<_> = chapters
            .into_iter()
            .map(|chapter| {
                let pk = chapter.chapter.pk;
                let images = chapter.images;
                let chapter = chapter.chapter;
                let chapter = Arc::new(DownloadChapterInfo::new(
                    module.clone(),
                    chapter.chapter_id,
                    chapter.title,
                    chapter.path,
                    chapter.status.into(),
                ));

                let images: Vec<_> = images
                    .into_iter()
                    .map(|it| {
                        let pk = it.image.pk;

                        let image = it.image;
                        let image = Arc::new(DownloadChapterImageInfo::new(
                            ChapterImageInfo {
                                id: image.image_url,
                                extension: image.extension,
                                name: image.name,
                            },
                            image.path,
                            image.status.into(),
                        ));

                        DownloadChapterImageInfoJoin { pk, image }
                    })
                    .collect();

                chapter.set_images(images.iter().map(|it| it.image.clone()).collect());

                DownloadChapterInfoJoin {
                    pk,
                    chapter,
                    images,
                }
            })
            .collect();

        let chapters: Vec<_> = chapters_join.iter().map(|it| it.chapter.clone()).collect();

        let info = Arc::new(DownloadInfo::new(
            0,
            module.clone(),
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

pub fn delete_finished_image(conn: &Connection) -> Result<usize, Error> {
    conn.execute(
        r#"
        DELETE FROM download_chapter_images
        WHERE id IN (
            SELECT image.id FROM download_chapter_images image
                INNER JOIN download_chapters chapter ON image.download_chapter_id = chapter.id
            WHERE chapter.status = "Finished" AND image.status = "Finished"
        );
    "#,
        [],
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{downloads::DownloadPK, module::InsertModule, tests::*};

    #[test]
    fn insert_test() {
        let mut db = connection();

        let module = crate::module::insert_pk(
            &mut db,
            InsertModule {
                uuid: &Default::default(),
                name: "Default",
            },
        )
        .unwrap();

        const CHAPTER_LENGTH: u8 = u8::MAX;
        let info = setup_info(CHAPTER_LENGTH);

        let insert = crate::downloads::insert_info(&mut db, module, &info).unwrap();
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
