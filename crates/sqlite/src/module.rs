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

pub fn insert(conn: &Connection, model: InsertModule<'_>) -> Result<i64, Error> {
    let mut stmt = conn.prepare(
        "INSERT INTO modules (uuid, name)
            VALUES (:uuid, :name)
            ON CONFLICT(uuid)
                DO UPDATE SET name=:name
            RETURNING id, uuid, name;",
    )?;
    let mut ex = stmt.query(rusqlite::named_params! {
        ":uuid": model.uuid,
        ":name": model.name,
    })?;

    let it = ex.next()?.unwrap();
    it.get("id")
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
    let id = insert(&conn, model)?;

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
        module
            .expect_name()
            .times(0..)
            .return_const("Module".to_string());
        module
            .expect_uuid()
            .times(0..)
            .return_const(Uuid::from_u128(1));
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
        .unwrap();

        for _ in 0..2 {
            let it = insert_info(&mut conn, module.clone()).unwrap();
            assert_eq!(it.name, module.name().to_string());
            assert_eq!(it.uuid, module.uuid());
        }
    }
}
