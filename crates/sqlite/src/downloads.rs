use std::sync::Arc;

use mado_engine::{core::Url, path::Utf8PathBuf, DownloadInfo};
use rusqlite::{params, Connection, Error};

use crate::{
    download_chapters::DownloadChapterPK,
    module::ModulePK,
    query::{DownloadChapterImageInfoJoin, DownloadChapterInfoJoin, DownloadInfoJoin},
    status::DownloadStatus,
};

pub struct InsertDownload<'a> {
    pub order: usize,
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
        "INSERT INTO downloads (title, module_id, path, url, status, `order`)
        VALUES (:title, :module, :path, :url, :status, :order)",
        rusqlite::named_params! {
            ":title": model.title,
            ":module": model.module_id,
            ":path": model.path,
            ":url": model.url,
            ":status": model.status,
            ":order": model.order

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
        order: info.order(),
        title: info.manga(),
        module_id: &module.id,
        path: info.path().as_str(),
        url: info.url(),
        status: From::from(&*info.status()),
    };

    insert(&transaction, model)?;
    let download_id = transaction.last_insert_rowid();
    let dl_pk = DownloadPK::new(download_id);

    let mut chapters = Vec::new();
    for it in info.chapters() {
        let pk = crate::download_chapters::insert_info(&transaction, dl_pk, it).unwrap();

        let images = it
            .images()
            .iter()
            .map(|img| {
                let pk = crate::download_chapter_images::insert_info(&transaction, pk, img)?;

                Ok(DownloadChapterImageInfoJoin {
                    pk,
                    image: img.clone(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        chapters.push(DownloadChapterInfoJoin {
            pk,
            chapter: it.clone(),
            images,
        });
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
    pub order: usize,
    pub title: String,
    pub module_pk: ModulePK,
    pub path: Utf8PathBuf,
    pub url: Option<Url>,
    pub status: DownloadStatus,
}

pub fn load(conn: &Connection) -> Result<Vec<Download>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, `order`, title, module_id, path, url, status FROM downloads ORDER BY `order`",
    )?;
    let mut rows = stmt.query([])?;

    let mut downloads = Vec::new();

    while let Some(row) = rows.next()? {
        let download = Download {
            order: row.get("order")?,
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

pub fn update_order(conn: &Connection, pk: DownloadPK, order: usize) -> Result<usize, Error> {
    conn.execute(
        "UPDATE downloads SET `order` = ? WHERE id = ?",
        params![order, pk.id],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn insert_test() {
        let db = connection();

        let module_id = crate::module::insert(
            &db,
            crate::module::InsertModule {
                uuid: &Default::default(),
                name: "",
            },
        )
        .unwrap();

        insert(
            &db,
            InsertDownload {
                order: 10,
                title: "title",
                module_id: &module_id,
                path: "path",
                url: None,
                status: "Paused".into(),
            },
        )
        .unwrap();

        let vec = load(&db).unwrap();

        assert_eq!(vec.len(), 1);
        let it = &vec[0];
        assert_eq!(it.order, 10);
        assert_eq!(it.title, "title");
        assert_eq!(it.module_pk.id, module_id);
        assert_eq!(it.path, "path");
        assert_eq!(it.url, None);
        assert_eq!(it.status, "Paused".into());

        insert(
            &db,
            InsertDownload {
                order: 11,
                title: "title",
                module_id: &module_id,
                path: "path",
                url: Some(&"https://url.com".parse().unwrap()),
                status: "Finished".into(),
            },
        )
        .unwrap();

        let vec = load(&db).unwrap();
        assert_eq!(vec.len(), 2);
        let it = &vec[1];
        assert_eq!(it.order, 11);
        assert_eq!(it.title, "title");
        assert_eq!(it.module_pk.id, module_id);
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

    #[test]
    fn update_order_test() {
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
            db.query_row::<usize, _, _>("SELECT `order` FROM downloads WHERE id = ?", [id], |row| {
                row.get(0)
            })
            .unwrap()
        };

        assert_eq!(get_status(1), 0);
        update_order(&db, insert.pk, 10).unwrap();
        assert_eq!(get_status(1), 10)
    }

    #[test]
    fn sorted_test() {
        let db = connection();

        let module_id = crate::module::insert(
            &db,
            crate::module::InsertModule {
                uuid: &Default::default(),
                name: "",
            },
        )
        .unwrap();

        let first = InsertDownload {
            order: 2,
            title: "first",
            module_id: &module_id,
            path: "path",
            url: None,
            status: "Finished".into(),
        };

        let second = InsertDownload {
            order: 1,
            title: "second",
            module_id: &module_id,
            path: "path",
            url: None,
            status: "Finished".into(),
        };

        insert(&db, first).unwrap();
        insert(&db, second).unwrap();

        let vec = load(&db).unwrap();

        assert_eq!(vec.len(), 2);

        assert_eq!(vec[0].order, 1);
        assert_eq!(vec[0].title, "second");
        assert_eq!(vec[1].order, 2);
        assert_eq!(vec[1].title, "first");
    }
}
