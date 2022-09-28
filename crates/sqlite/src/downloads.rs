use std::sync::Arc;

use mado_engine::{core::Url, path::Utf8PathBuf, DownloadInfo};
use rusqlite::{params, Connection, Error};

use crate::{
    download_chapters::DownloadChapterPK,
    module::ModulePK,
    query::{DownloadChapterInfoJoin, DownloadInfoJoin},
    status::DownloadStatus,
};

pub struct InsertDownload<'a> {
    pub title: &'a str,
    pub module_id: &'a i64,
    pub path: &'a str,
    pub url: Option<&'a Url>,
    pub status: DownloadStatus,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct DownloadPK {
    pub id: i64,
}

impl DownloadPK {
    pub fn new(id: i64) -> Self {
        Self { id }
    }
}

pub fn insert(conn: &Connection, model: InsertDownload<'_>) -> Result<usize, Error> {
    conn.execute(
        "INSERT INTO downloads (title, module_id, path, url, status)
        VALUES (:title, :module, :path, :url, :status)",
        rusqlite::named_params! {
            ":title": model.title,
            ":module": model.module_id,
            ":path": model.path,
            ":url": model.url,
            ":status": model.status
        },
    )
}

pub struct InsertDownloadPK {
    pub pk: DownloadPK,
    pub chapters_pk: Vec<DownloadChapterPK>,
}

pub fn insert_info(
    conn: &mut Connection,
    module: ModulePK,
    info: &Arc<DownloadInfo>,
) -> Result<DownloadInfoJoin, Error> {
    let transaction = conn.transaction()?;

    let model = InsertDownload {
        title: info.manga(),
        module_id: &module.id,
        path: info.path().as_str(),
        url: info.url(),
        status: From::from(&*info.status()),
    };

    insert(&transaction, model)?;
    let download_id = transaction.last_insert_rowid();
    let dl_pk = DownloadPK::new(download_id);

    let mut id = 1;

    let mut chapters = Vec::new();
    for it in info.chapters() {
        let pk = DownloadChapterPK::new(dl_pk, id);
        crate::download_chapters::insert_info(&transaction, pk, it).unwrap();

        chapters.push(DownloadChapterInfoJoin {
            pk,
            chapter: it.clone(),
        });
        id += 1;
    }

    transaction.commit()?;

    Ok(DownloadInfoJoin {
        pk: dl_pk,
        info: info.clone(),
        chapters,
    })
}

#[derive(Debug)]
pub struct Download {
    pub pk: DownloadPK,
    pub title: String,
    pub module_pk: ModulePK,
    pub path: Utf8PathBuf,
    pub url: Option<Url>,
    pub status: DownloadStatus,
}

pub fn load(conn: &Connection) -> Result<Vec<Download>, Error> {
    let mut stmt = conn.prepare("SELECT id, title, module_id, path, url, status FROM downloads")?;
    let mut rows = stmt.query([])?;

    let mut downloads = Vec::new();

    while let Some(row) = rows.next()? {
        let download = Download {
            pk: DownloadPK::new(row.get("id")?),
            title: row.get("title")?,
            module_pk: ModulePK {
                id: row.get("module_id")?,
            },
            path: row.get::<_, String>("path")?.into(),
            url: row
                .get::<_, Option<String>>("url")?
                .and_then(|it| it.parse().ok()),
            status: row.get("status")?,
        };

        downloads.push(download);
    }

    Ok(downloads)
}

pub fn update_status(
    conn: &Connection,
    pk: DownloadPK,
    status: DownloadStatus,
) -> Result<usize, Error> {
    conn.execute(
        "UPDATE downloads SET status = ? WHERE id = ?",
        params![status, pk.id],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn insert_test() {
        let db = connection();

        insert(
            &db,
            InsertDownload {
                title: "title",
                module_id: &Default::default(),
                path: "path",
                url: None,
                status: "Paused".into(),
            },
        )
        .unwrap();

        let vec = load(&db).unwrap();

        assert_eq!(vec.len(), 1);
        let it = &vec[0];
        assert_eq!(it.title, "title");
        assert_eq!(it.module_pk, Default::default());
        assert_eq!(it.path, "path");
        assert_eq!(it.url, None);
        assert_eq!(it.status, "Paused".into());

        insert(
            &db,
            InsertDownload {
                title: "title",
                module_id: &Default::default(),
                path: "path",
                url: Some(&"https://url.com".parse().unwrap()),
                status: "Finished".into(),
            },
        )
        .unwrap();

        let vec = load(&db).unwrap();
        assert_eq!(vec.len(), 2);
        let it = &vec[1];
        assert_eq!(it.title, "title");
        assert_eq!(it.module_pk, Default::default());
        assert_eq!(it.path, "path");
        assert_eq!(it.url, Some("https://url.com".parse().unwrap()));
        assert_eq!(it.status, "Finished".into());
    }

    #[test]
    fn update_status_test() {
        let mut db = connection();

        let module = crate::module::insert_pk(
            &mut db,
            crate::module::InsertModule {
                uuid: &Default::default(),
                name: "Default",
            },
        )
        .unwrap();

        let info = setup_info(1);

        let insert = crate::downloads::insert_info(&mut db, module, &info).unwrap();

        let get_status = |id: i64| {
            db.query_row::<DownloadStatus, _, _>(
                "SELECT status FROM downloads WHERE id = ?",
                [id],
                |row| row.get(0),
            )
            .unwrap()
        };

        assert_eq!(get_status(1), DownloadStatus::paused());
        update_status(&db, insert.pk, DownloadStatus::resumed()).unwrap();
        assert_eq!(get_status(1), DownloadStatus::resumed())
    }
}
