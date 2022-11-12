// make sure to read https://www.sqlite.org/lang_altertable.html#otheralter
// before altering existing table and test it to prevent data loss

use rusqlite::{Connection, Error};

type SchemaFn = fn(&rusqlite::Connection) -> Result<(), rusqlite::Error>;
pub const SCHEMA_FUNCTION: [SchemaFn; 3] = [v1_schema, v2_schema, v3_schema];

fn schema_function_with_index() -> impl Iterator<Item = (i64, SchemaFn)> {
    SCHEMA_FUNCTION
        .into_iter()
        .enumerate()
        .map(|(index, it)| (index as i64 + 1, it))
}

/// create migration table if doesn't exists
/// then return the current version (0 if table doesn't exists)
fn create_migration(conn: &Connection) -> Result<i64, Error> {
    let stmt = r#"
        CREATE TABLE IF NOT EXISTS __migration (
            id INTEGER PRIMARY KEY NOT NULL,
            version INTEGER NOT NULL
        )
    "#;

    conn.execute(stmt, [])?;

    let rows: Option<i64> = conn.query_row(
        "SELECT MAX(version) as version from __migration",
        [],
        |row| row.get(0),
    )?;

    Ok(rows.unwrap_or(0))
}

fn v1_module() -> &'static str {
    r#"
        CREATE TABLE modules (
            id INTEGER PRIMARY KEY,
            uuid TEXT(36) NOT NULL UNIQUE,
            name TEXT NOT NULL
        );
    "#
}

fn v1_download() -> &'static str {
    r#"
        CREATE TABLE downloads (
            id INTEGER PRIMARY KEY,
            module_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            url TEXT,
            status TEXT NOT NULL,
            path TEXT NOT NULL,

            FOREIGN KEY (module_id)
                REFERENCES modules(id)
                ON DELETE RESTRICT
                ON UPDATE RESTRICT
        );
    "#
}

fn v1_download_chapter() -> &'static str {
    r#"
        CREATE TABLE download_chapters (
            id INTEGER PRIMARY KEY,
            download_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            chapter_id TEXT NOT NULL,
            status TEXT NOT NULL,
            path TEXT NOT NULL,

            FOREIGN KEY (download_id) 
                REFERENCES downloads(id)
                ON DELETE CASCADE
                ON UPDATE CASCADE
        );
    "#
}

fn v1_download_chapter_images() -> &'static str {
    r#"
    CREATE TABLE download_chapter_images (
        id INTEGER PRIMARY KEY,
        download_chapter_id INTEGER NOT NULL,
        image_url INTEGER NOT NULL,
        extension TEXT NOT NULL,
        name TEXT,
        path TEXT NOT NULL,
        status TEXT NOT NULL,

        FOREIGN KEY (download_chapter_id)
            REFERENCES download_chapters(id)
            ON DELETE CASCADE
            ON UPDATE CASCADE
    );
    "#
}

fn v2_download_status_index() -> &'static str {
    r"
        CREATE INDEX download_status_index ON downloads(status);
    "
}

fn v2_download_chapter_status_index() -> &'static str {
    r"
        CREATE INDEX download_chapter_status_index ON download_chapters(status);
    "
}

fn v3_add_order_to_downloads() -> &'static str {
    r"
        PRAGMA foreign_keys = OFF;
        BEGIN TRANSACTION;
        CREATE TABLE downloads_temp (
            id INTEGER PRIMARY KEY,
            module_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            url TEXT,
            status TEXT NOT NULL,
            path TEXT NOT NULL,
            `order` INTEGER NOT NULL,

            FOREIGN KEY (module_id)
                REFERENCES modules(id)
                ON DELETE RESTRICT
                ON UPDATE RESTRICT
        );

        INSERT INTO downloads_temp(id, module_id, title, url, status, path, `order`)
            SELECT id, module_id, title, url, status, path, ROWID as `order`
                FROM downloads;

        DROP TABLE downloads;

        ALTER TABLE downloads_temp RENAME TO downloads;
        PRAGMA foreign_key_check;

        COMMIT;
        PRAGMA foreign_keys = ON;
    "
}

fn v3_add_index_on_download_to_order() -> &'static str {
    r"
        CREATE INDEX download_order_index ON downloads(`order`);
    "
}

fn v3_add_index_on_donwload_to_status() -> &'static str {
    v2_download_status_index()
}

fn insert_migration_version(conn: &Connection, version: i64) -> Result<usize, Error> {
    conn.execute("INSERT INTO __migration (version) VALUES (?)", [version])
}

fn v1_schema(conn: &Connection) -> Result<(), Error> {
    conn.execute(v1_module(), []).unwrap();
    conn.execute(v1_download(), []).unwrap();
    conn.execute(v1_download_chapter(), []).unwrap();
    conn.execute(v1_download_chapter_images(), []).unwrap();

    insert_migration_version(conn, 1)?;

    Ok(())
}

fn v2_schema(conn: &Connection) -> Result<(), Error> {
    conn.execute(v2_download_status_index(), []).unwrap();
    conn.execute(v2_download_chapter_status_index(), [])
        .unwrap();

    insert_migration_version(conn, 2)?;

    Ok(())
}

fn v3_schema(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(v3_add_order_to_downloads()).unwrap();
    conn.execute(v3_add_index_on_donwload_to_status(), [])
        .unwrap();
    conn.execute(v3_add_index_on_download_to_order(), [])
        .unwrap();

    insert_migration_version(conn, 3)?;

    Ok(())
}

pub fn setup_schema_version(conn: &Connection, version: i64) -> Result<(), Error> {
    conn.execute("PRAGMA foreign_keys = ON;", []).unwrap();
    create_migration(conn)?;

    for (index, it) in schema_function_with_index() {
        if version < index {
            it(conn)?;
        }
    }

    Ok(())
}

pub fn setup_schema(conn: &Connection) -> Result<(), Error> {
    let version = create_migration(conn)?;
    setup_schema_version(conn, version)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use mado_core::Uuid;

    use super::*;

    #[test]
    fn test_migration() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(create_migration(&conn).unwrap(), 0);
        insert_migration_version(&conn, 3).unwrap();

        assert_eq!(create_migration(&conn).unwrap(), 3);

        insert_migration_version(&conn, 2).unwrap();
        assert_eq!(create_migration(&conn).unwrap(), 3);
    }

    #[test]
    fn versioning_test() {
        let conn = Connection::open_in_memory().unwrap();
        create_migration(&conn).unwrap();
        for (index, it) in schema_function_with_index() {
            it(&conn).unwrap();
            assert_eq!(create_migration(&conn).unwrap(), index);
        }
    }

    #[test]
    fn setup_test() {
        let conn = Connection::open_in_memory().unwrap();
        setup_schema(&conn).unwrap();
    }

    #[test]
    fn test_v3_migrate() {
        let conn = Connection::open_in_memory().unwrap();

        create_migration(&conn).unwrap();
        v1_schema(&conn).unwrap();
        v2_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO modules (uuid, name)
                VALUES (:uuid, :name);",
            rusqlite::named_params! {
                ":uuid": Uuid::from_u128(1),
                ":name": "Name"
            },
        )
        .unwrap();

        conn.execute(
            "INSERT INTO downloads (title, module_id, path, url, status)
            VALUES (:title, :module, :path, :url, :status)",
            rusqlite::named_params! {
                ":title": "title",
                ":module": 1,
                ":path": "path",
                ":url": None::<String>,
                ":status": "Finished",

            },
        )
        .unwrap();

        conn.execute(
            "INSERT INTO download_chapters (download_id, title, chapter_id, path, status)
            VALUES (:download_id, :title, :chapter_id, :path, :status)",
            rusqlite::named_params! {
                ":download_id": 1,
                ":title": "title",
                ":chapter_id": "chapter",
                ":path": "path",
                ":status": "Finished"
            },
        )
        .unwrap();

        v3_schema(&conn).unwrap();

        let it = crate::downloads::load(&conn).unwrap();
        assert_eq!(it.len(), 1);
        assert_eq!(it[0].title, "title");
        assert_eq!(it[0].module_pk.id, 1);
        assert_eq!(it[0].path, "path");
        assert_eq!(it[0].status, "Finished".into());
        assert_eq!(it[0].order, 1);

        let it = crate::download_chapters::load(&conn).unwrap();
        assert_eq!(it.len(), 1);
    }
}
