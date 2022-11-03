use std::collections::HashMap;

use mado_engine::{
    path::{Utf8Path, Utf8PathBuf},
    DownloadChapterImageInfo, DownloadChapterInfo,
};
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
    pub path_relative: bool,
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
    pub path_relative: bool,
    pub status: DownloadStatus,
}

pub fn insert(conn: &Connection, model: InsertDownloadChapterImage<'_>) -> Result<usize, Error> {
    conn.execute(
        "INSERT INTO download_chapter_images (download_chapter_id, image_url, name, extension, path, status, path_relative)
        VALUES (:download_chapter_id, :image_url, :name, :extension, :path, :status, :path_relative)",
        rusqlite::named_params! {
            ":download_chapter_id": model.download_chapter_id,
            ":image_url": model.image_url,
            ":extension": model.extension,
            ":name": model.name,
            ":path": model.path,
            ":path_relative": model.path_relative,
            ":status": model.status
        },
    )
}

pub fn into_relative(parent: &Utf8Path, child: &Utf8Path) -> (bool, String) {
    match child.strip_prefix(parent) {
        Ok(path) => (true, path.to_string()),
        Err(_) => (false, child.to_string()),
    }
}

pub fn insert_info(
    conn: &Connection,
    dl_pk: DownloadChapterPK,
    parent: &DownloadChapterInfo,
    it: &DownloadChapterImageInfo,
) -> Result<DownloadChapterImagePK, Error> {
    let (relative, path) = into_relative(parent.path(), it.path());

    let model = InsertDownloadChapterImage {
        download_chapter_id: dl_pk.id,
        image_url: &it.image().id,
        extension: &it.image().extension,
        name: &it.image().name,
        path: &path,
        path_relative: relative,
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
        "SELECT id, download_chapter_id, image_url, name, path, status, extension, path_relative FROM download_chapter_images",
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
            path_relative: row.get("path_relative")?,
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
   info: &DownloadChapterInfo,
) -> Result<Vec<DownloadChapterImageInfoJoin>, Error> {
    let conn = conn.transaction()?;
    let mut vec = vec![];
    delete_images(&conn, pk)?;

    for it in info.images().iter() {
        insert_info(&conn, pk, info, it)?;
        let id = conn.last_insert_rowid();

        vec.push(DownloadChapterImageInfoJoin {
            pk: DownloadChapterImagePK::new(pk, id),
            image: it.clone(),
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
    use std::sync::Arc;

    use mado_core::{ChapterImageInfo, DefaultMadoModuleMap};

    use super::*;
    use crate::{download_chapters::InsertDownloadChapter, downloads::InsertDownload, tests::*};

    #[test]
    fn insert_test() {
        let mut db = connection();

        let module_id = crate::module::insert(
            &db,
            crate::module::InsertModule {
                uuid: &Default::default(),
                name: "Test Module",
            },
        )
        .unwrap();

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
                path_relative: false,
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

        let module = mado_engine::LateBindingModule::WaitModule(
            Arc::new(DefaultMadoModuleMap::default()),
            Default::default(),
        );

        let info_path: Utf8PathBuf = "path".into();
        let info = Arc::new(DownloadChapterInfo::new(
            module,
            "id".to_string(),
            "title".to_string(),
            info_path.clone(),
            mado_engine::DownloadStatus::paused(),
        ));

        let image_info = ChapterImageInfo {
            id: "image-id".to_string(),
            name: Some("iho".to_string()),
            extension: "png".to_string(),
        };
        info.set_images(vec![
            Arc::new(DownloadChapterImageInfo::new(
                image_info.clone(),
                info_path.join("path-changed"),
                mado_engine::DownloadStatus::paused(),
            )),
            Arc::new(DownloadChapterImageInfo::new(
                image_info,
                "path-changed".into(),
                mado_engine::DownloadStatus::paused(),
            )),
        ]);

        update_images(&mut db, pk, &info).unwrap();

        let updated = load(&db).unwrap();

        assert_eq!(updated.len(), 1);
        let vec = &updated[&pk];
        assert_eq!(vec.len(), 2);

        let it = &vec[0];

        assert_eq!(it.download_chapter_id, DownloadChapterPK::new(1));
        assert_eq!(it.image_url, "image-id");
        assert_eq!(it.path, "path-changed");
        assert_eq!(it.status, "Paused".into());

        let it = &vec[1];

        assert_eq!(it.download_chapter_id, DownloadChapterPK::new(1));
        assert_eq!(it.image_url, "image-id");
        assert_eq!(it.path, "path-changed");
        assert_eq!(it.status, "Paused".into());

        delete_images(&db, pk).unwrap();
        assert_eq!(load(&db).unwrap().len(), 0);
    }
}
