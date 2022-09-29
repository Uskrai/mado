use std::collections::HashMap;

use mado_engine::core::{ArcMadoModule, Uuid};
use rusqlite::{Connection, Error};

#[derive(Debug)]
pub struct Module {
    pub pk: ModulePK,
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct ModulePK {
    pub id: i64,
}

#[derive(Clone)]
pub struct InsertModule<'a> {
    pub uuid: &'a Uuid,
    pub name: &'a str,
}

pub fn insert(conn: &Connection, model: InsertModule<'_>) -> Result<usize, Error> {
    conn.execute(
        "INSERT INTO modules (uuid, name)
        VALUES (:uuid, :name)",
        rusqlite::named_params! {
            ":uuid": model.uuid,
            ":name": model.name,
        },
    )
}

pub fn insert_info(conn: &mut Connection, module: ArcMadoModule) -> Result<Module, Error> {
    let pk = insert_pk(
        conn,
        InsertModule {
            uuid: &module.uuid(),
            name: module.name(),
        },
    )?;

    Ok(Module {
        pk,
        uuid: module.uuid(),
        name: module.name().to_string(),
    })
}

pub fn insert_pk(conn: &mut Connection, model: InsertModule<'_>) -> Result<ModulePK, Error> {
    let conn = conn.transaction()?;
    insert(&conn, model)?;

    let id = conn.last_insert_rowid();
    conn.commit()?;

    Ok(ModulePK { id })
}

pub fn load(conn: &Connection) -> Result<Vec<Module>, Error> {
    let mut stmt = conn.prepare("SELECT id, name, uuid FROM modules")?;
    let mut rows = stmt.query([])?;

    let mut downloads = Vec::new();

    while let Some(row) = rows.next()? {
        let download = Module {
            pk: ModulePK { id: row.get("id")? },
            name: row.get("name")?,
            uuid: row.get("uuid")?,
        };

        downloads.push(download);
    }

    Ok(downloads)
}

pub fn load_map(conn: &Connection) -> Result<HashMap<ModulePK, Module>, Error> {
    let vec = load(conn)?;
    let map = HashMap::from_iter(vec.into_iter().map(|it| (it.pk, it)));
    Ok(map)
}

#[cfg(test)]
mod tests {
    use mado_engine::core::{MadoModule, MockMadoModule};

    use super::*;
    use crate::tests::connection;
    use std::sync::Arc;

    #[test]
    pub fn insert_test() {
        let mut conn = connection();
        let mut module = MockMadoModule::new();
        module.expect_name().times(0..).return_const("Module".to_string());
        module.expect_uuid().times(0..).return_const(Uuid::new_v4());
        let module = Arc::new(module);

        insert(
            &conn,
            InsertModule {
                uuid: &Default::default(),
                name: "Module",
            },
        )
        .unwrap();

        let vec = load(&conn).unwrap();

        assert_eq!(vec.len(), 1);
        let it = &vec[0];
        assert_eq!(it.uuid, Default::default());
        assert_eq!(it.name, "Module");

        insert(
            &conn,
            InsertModule {
                uuid: &Default::default(),
                name: "Module",
            },
        )
        .unwrap_err();

        let it = insert_info(&mut conn, module.clone()).unwrap();
        assert_eq!(it.name, module.name().to_string());
        assert_eq!(it.uuid, module.uuid());

        insert_info(&mut conn, module).unwrap_err();
    }
}
