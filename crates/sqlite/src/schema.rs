use rusqlite::{Connection, Error};

type SchemaFn = fn(&rusqlite::Connection) -> Result<(), rusqlite::Error>;
pub const SCHEMA_FUNCTION: [SchemaFn; 2] = [v1_schema, v2_schema];

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

pub fn setup_schema_version(conn: &Connection, version: i64) -> Result<(), Error> {
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
    conn.execute("PRAGMA foreign_keys = ON;", []).unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
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
}
