use std::collections::HashMap;

use mado_engine::{path::Utf8PathBuf, DownloadChapterInfo};
use rusqlite::{params, Connection, Error};

use crate::{downloads::DownloadPK, status::DownloadStatus};

#[derive(Debug)]
pub struct DownloadChapter {
    pub pk: DownloadChapterPK,
    pub title: String,
    pub chapter_id: String,
    pub path: Utf8PathBuf,
    pub status: DownloadStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct DownloadChapterPK {
    pub id: i64,
    pub dl_pk: DownloadPK,
}

impl DownloadChapterPK {
    pub fn new(dl_pk: DownloadPK, id: i64) -> Self {
        Self { id, dl_pk }
    }
}

pub struct InsertDownloadChapter<'a> {
    pub id: i64,
    pub download_id: i64,
    pub title: &'a str,
    pub chapter_id: &'a str,
    pub path: &'a str,
    pub status: DownloadStatus,
}

pub fn insert(conn: &Connection, model: InsertDownloadChapter<'_>) -> Result<usize, Error> {
    conn.execute(
        "INSERT INTO download_chapters (id, download_id, title, chapter_id, path, status)
        VALUES (:id, :download_id, :title, :chapter_id, :path, :status)",
        rusqlite::named_params! {
            ":id": model.id,
            ":download_id": model.download_id,
            ":title": model.title,
            ":chapter_id": model.chapter_id,
            ":path": model.path,
            ":status": model.status
        },
    )
}

pub fn insert_info(
    conn: &Connection,
    pk: DownloadChapterPK,
    it: &DownloadChapterInfo,
) -> Result<usize, Error> {
    let model = InsertDownloadChapter {
        id: pk.id,
        download_id: pk.dl_pk.id,
        title: it.title(),
        chapter_id: it.chapter_id(),
        path: it.path().as_str(),
        status: From::from(&*it.status()),
    };

    insert(conn, model)
}

pub fn load(conn: &Connection) -> Result<HashMap<DownloadPK, Vec<DownloadChapter>>, Error> {
    let mut map: HashMap<DownloadPK, Vec<DownloadChapter>> = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT id, download_id, title, path, status, chapter_id FROM download_chapters",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let dl_pk = DownloadPK::new(row.get("download_id")?);

        let pk = DownloadChapterPK::new(dl_pk, row.get("id")?);

        let chapter = DownloadChapter {
            pk,
            title: row.get("title")?,
            chapter_id: row.get("chapter_id")?,
            path: row.get::<_, String>("path")?.into(),
            status: row.get("status")?,
        };

        let chapters = map.entry(dl_pk).or_default();
        chapters.push(chapter);
    }

    Ok(map)
}

pub fn update_status(
    conn: &Connection,
    pk: DownloadChapterPK,
    status: DownloadStatus,
) -> Result<usize, Error> {
    conn.execute(
        "UPDATE download_chapters SET status = ? WHERE id = ? AND download_id = ?",
        params![status, pk.id, pk.dl_pk.id],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{downloads::InsertDownload, tests::*};

    #[test]
    fn insert_test() {
        let db = connection();

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

        insert(
            &db,
            InsertDownloadChapter {
                id: 1,
                download_id: 1,
                title: "title",
                chapter_id: "chapter-id",
                path: "path",
                status: "Finished".into(),
            },
        )
        .unwrap();

        let chapters = load(&db).unwrap();

        let pk = DownloadPK::new(1);
        assert_eq!(chapters.len(), 1);
        let vec = &chapters[&pk];
        assert_eq!(vec.len(), 1);

        let it = &vec[0];
        assert_eq!(it.pk, DownloadChapterPK::new(pk, 1));
        assert_eq!(it.title, "title");
        assert_eq!(it.chapter_id, "chapter-id");
        assert_eq!(it.path, "path");
        assert_eq!(it.status, "Finished".into());
    }
}
