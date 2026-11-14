use rusqlite::{Connection, params};

// A task that TickTick could not add waits here until it is added by hand.
pub struct Task {
    pub id: String,
    pub query: String,
}

pub fn add(db: &Connection, query: &str) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Enter a task title".into());
    }
    db.execute(
        "INSERT INTO inbox_tasks(id,query) VALUES(?1,?2)",
        params![crate::apps::new_id()?, query],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list(db: &Connection) -> Result<Vec<Task>, String> {
    let mut statement = db
        .prepare("SELECT id,query FROM inbox_tasks ORDER BY rowid")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                query: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn task(db: &Connection, id: &str) -> Result<Task, String> {
    db.query_row(
        "SELECT id,query FROM inbox_tasks WHERE id=?1",
        [id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                query: row.get(1)?,
            })
        },
    )
    .map_err(|_| "This task is no longer in the Inbox.".into())
}

pub fn remove(db: &Connection, id: &str) -> Result<(), String> {
    db.execute("DELETE FROM inbox_tasks WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// The TickTick request runs outside the widget worker, so it opens the database itself.
pub fn save(query: &str) -> Result<(), String> {
    let db = open()?;
    add(&db, query)
}

fn open() -> Result<Connection, String> {
    let path = crate::storage::config_path()
        .map_err(|e| e.to_string())?
        .with_file_name("widgets.db");
    crate::widget_data::open(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(name: &str) -> (std::path::PathBuf, Connection) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let folder = std::env::temp_dir().join(format!("orbit-{name}-{stamp}"));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("widgets.db");
        let db = crate::widget_data::open(&path).unwrap();
        (folder, db)
    }

    #[test]
    fn a_failed_task_waits_until_it_is_added() {
        let (folder, db) = database("inbox");
        add(&db, "buy milk !high").unwrap();
        add(&db, "  ").unwrap_err();
        let tasks = list(&db).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].query, "buy milk !high");
        assert_eq!(task(&db, &tasks[0].id).unwrap().query, "buy milk !high");
        assert!(task(&db, "missing").is_err());
        remove(&db, &tasks[0].id).unwrap();
        assert!(list(&db).unwrap().is_empty());
        drop(db);
        std::fs::remove_dir_all(folder).unwrap();
    }
}
