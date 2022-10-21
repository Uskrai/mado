use std::{collections::HashMap, sync::Arc};

use mado_engine::{path::Utf8PathBuf, DownloadChapterImageInfo};
use rusqlite::{params, Connection, Error};

use crate::{
    download_chapters::DownloadChapterPK, query::DownloadChapterImageInfoJoin,
    status::DownloadStatus,
};

#[derive(Debug)]
pub struct DownloadChapterImage {
    pub pk: DownloadChapterImagePK,
    pub download_chapter_id: DownloadChapterPK,
    pub name: Option<String>,
    pub image_url: String,
    pub extension: String,
    pub path: Utf8PathBuf,
    pub status: DownloadStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct DownloadChapterImagePK {
    pub id: i64,
    pub dl_pk: DownloadChapterPK,
}

impl DownloadChapterImagePK {
    pub fn new(dl_pk: DownloadChapterPK, id: i64) -> Self {
        Self { id, dl_pk }
    }
}

#[derive(Debug)]
pub struct InsertDownloadChapterImage<'a> {
    pub download_chapter_id: i64,
    pub image_url: &'a str,
    pub name: &'a Option<String>,
    pub extension: &'a str,
    pub path: &'a str,
    pub status: DownloadStatus,
}

pub fn insert(conn: &Connection, model: InsertDownloadChapterImage<'_>) -> Result<usize, Error> {
    conn.execute(
        "INSERT INTO download_chapter_images (download_chapter_id, image_url, name, extension, path, status)
        VALUES (:download_chapter_id, :image_url, :name, :extension, :path, :status)",
        rusqlite::named_params! {
            ":download_chapter_id": model.download_chapter_id,
            ":image_url": model.image_url,
            ":extension": model.extension,
            ":name": model.name,
            ":path": model.path,
            ":status": model.status
        },
    )
}

pub fn insert_info(
    conn: &Connection,
    dl_pk: DownloadChapterPK,
    it: &DownloadChapterImageInfo,
) -> Result<DownloadChapterImagePK, Error> {
    let model = InsertDownloadChapterImage {
        download_chapter_id: dl_pk.id,
        image_url: &it.image().id,
        extension: &it.image().extension,
        name: &it.image().name,
        path: it.path().as_str(),
        status: From::from(&*it.status()),
    };

    insert(conn, model)?;
    let id = conn.last_insert_rowid();

    Ok(DownloadChapterImagePK { id, dl_pk })
}

pub fn load(
    conn: &Connection,
) -> Result<HashMap<DownloadChapterPK, Vec<DownloadChapterImage>>, Error> {
    let mut map: HashMap<DownloadChapterPK, Vec<DownloadChapterImage>> = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT id, download_chapter_id, image_url, name, path, status, extension FROM download_chapter_images",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let dl_pk = DownloadChapterPK::new(row.get("download_chapter_id")?);

        let pk = DownloadChapterImagePK::new(dl_pk, row.get("id")?);

        let chapter = DownloadChapterImage {
            pk,
            download_chapter_id: dl_pk,
            name: row.get("name")?,
            image_url: row.get("image_url")?,
            path: row.get::<_, String>("path")?.into(),
            extension: row.get("extension")?,
            status: row.get("status")?,
        };

        let chapters = map.entry(dl_pk).or_default();
        chapters.push(chapter);
    }

    Ok(map)
}

pub fn update_status(
    conn: &Connection,
    pk: DownloadChapterImagePK,
    status: DownloadStatus,
) -> Result<usize, Error> {
    conn.execute(
        "UPDATE download_chapters SET status = ? WHERE id = ? AND download_id = ?",
        params![status, pk.id, pk.dl_pk.id],
    )
}

pub fn update_images(
    conn: &mut Connection,
    pk: DownloadChapterPK,
    images: Vec<Arc<DownloadChapterImageInfo>>,
) -> Result<Vec<DownloadChapterImageInfoJoin>, Error> {
    let conn = conn.transaction()?;
    let mut vec = vec![];
    delete_images(&conn, pk)?;

    for it in images {
        insert_info(&conn, pk, &it)?;
        let id = conn.last_insert_rowid();

        vec.push(DownloadChapterImageInfoJoin {
            pk: DownloadChapterImagePK::new(pk, id),
            image: it,
        });
    }

    conn.commit()?;

    Ok(vec)
}

pub fn delete_images(conn: &Connection, pk: DownloadChapterPK) -> Result<usize, Error> {
    conn.execute(
        "DELETE FROM download_chapter_images WHERE download_chapter_id = ?",
        params![pk.id],
    )
}

#[cfg(test)]
mod tests {
    use mado_core::ChapterImageInfo;

    use super::*;
    use crate::{download_chapters::InsertDownloadChapter, downloads::InsertDownload, tests::*};

    #[test]
    fn insert_test() {
        let mut db = connection();

        crate::downloads::insert(
            &db,
            InsertDownload {
                title: "title",
                module_id: &Default::default(),
                path: "path",
                url: None,
                status: "Finished".into(),
            },
        )
        .unwrap();

        crate::download_chapters::insert(
            &db,
            InsertDownloadChapter {
                download_id: 1,
                title: "title",
                chapter_id: "chapter-id",
                path: "path",
                status: "Finished".into(),
            },
        )
        .unwrap();

        insert(
            &db,
            InsertDownloadChapterImage {
                download_chapter_id: 1,
                name: &None,
                image_url: "image-url",
                extension: "extension",
                path: "path",
                status: "Finished".into(),
            },
        )
        .unwrap();

        let images = load(&db).unwrap();

        let pk = DownloadChapterPK::new(1);
        assert_eq!(images.len(), 1);
        let vec = &images[&pk];
        assert_eq!(vec.len(), 1);

        let it = &vec[0];

        assert_eq!(it.pk, DownloadChapterImagePK::new(pk, 1));
        assert_eq!(it.download_chapter_id, DownloadChapterPK::new(1));
        assert_eq!(it.image_url, "image-url");
        assert_eq!(it.path, "path");
        assert_eq!(it.status, "Finished".into());

        update_images(
            &mut db,
            pk,
            vec![Arc::new(DownloadChapterImageInfo::new(
                ChapterImageInfo {
                    id: "image-id".to_string(),
                    name: Some("iho".to_string()),
                    extension: "png".to_string(),
                },
                "path-changed".into(),
                mado_engine::DownloadStatus::paused(),
            ))],
        )
        .unwrap();

        let updated = load(&db).unwrap();

        assert_eq!(updated.len(), 1);
        let vec = &updated[&pk];
        assert_eq!(vec.len(), 1);

        let it = &vec[0];

        assert_eq!(it.download_chapter_id, DownloadChapterPK::new(1));
        assert_eq!(it.image_url, "image-id");
        assert_eq!(it.path, "path-changed");
        assert_eq!(it.status, "Paused".into());

        delete_images(&db, pk).unwrap();
        assert_eq!(load(&db).unwrap().len(), 0);
    }
}
