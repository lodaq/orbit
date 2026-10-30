use rusqlite::{Connection, OptionalExtension, params};
use std::{fs, path::Path};

#[path = "widget-starters.rs"]
mod starters;
pub const TABLES: [&str; 11] = [
    "quotes",
    "facts",
    "motivation",
    "affirmations",
    "vocabulary",
    "quran_player",
    "azan_reminder",
    "habits",
    "achievements",
    "morning_adhkar",
    "evening_adhkar",
];
pub const PAGE_SIZE: i32 = 30;

#[derive(Clone)]
pub struct Entry {
    pub id: String,
    pub content: String,
    pub attribution: String,
    pub resource: String,
    pub learned: bool,
}

pub struct Collection {
    pub entries: Vec<Entry>,
    pub count: i32,
    pub bytes: u64,
}

pub fn table(category: i32) -> Result<&'static str, String> {
    TABLES
        .get(category as usize)
        .copied()
        .ok_or_else(|| "Choose a data category.".into())
}

pub fn open(path: &Path) -> Result<Connection, String> {
    fs::create_dir_all(path.parent().ok_or("Missing Orbit data folder")?)
        .map_err(|e| e.to_string())?;
    let mut db = Connection::open(path).map_err(|e| e.to_string())?;
    db.busy_timeout(std::time::Duration::from_secs(3))
        .map_err(|e| e.to_string())?;
    let version: i32 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version > 6 {
        return Err("This widget database needs a newer version of Orbit.".into());
    }
    if version < 2 {
        migrate(&mut db, version)?;
    }
    if version < 3 {
        crate::adhkar::migrate(&mut db)?;
    }
    if version < 4 {
        crate::adhkar::migrate_evening(&mut db)?;
    }
    if version < 5 {
        crate::azan::migrate(&mut db)?;
    }
    if version < 6 {
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch("CREATE TABLE achievement_task_titles(id TEXT PRIMARY KEY, title TEXT NOT NULL); PRAGMA user_version=6;").map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(db)
}

fn migrate(db: &mut Connection, version: i32) -> Result<(), String> {
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for name in &TABLES[..8] {
        tx.execute_batch(&format!("CREATE TABLE {name}(id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT NOT NULL CHECK(length(trim(content))>0), attribution TEXT NOT NULL DEFAULT '');")).map_err(|e| e.to_string())?;
    }
    tx.execute_batch("ALTER TABLE vocabulary ADD COLUMN learned INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE quran_player ADD COLUMN audio_path TEXT NOT NULL DEFAULT '';
        ALTER TABLE azan_reminder ADD COLUMN date TEXT NOT NULL DEFAULT '';
        CREATE TABLE achievements(id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL UNIQUE, finished_ids TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(finished_ids)));
        CREATE TABLE widget_rotation(id TEXT PRIMARY KEY, category INTEGER NOT NULL, date TEXT NOT NULL, entry_id INTEGER NOT NULL, seen TEXT NOT NULL DEFAULT '[]');").map_err(|e| e.to_string())?;
    if version == 1 {
        for (category, name) in TABLES[..5].iter().enumerate() {
            tx.execute(&format!("INSERT INTO {name}(id,content,attribution) SELECT id,content,attribution FROM content WHERE category=?1"), [category as i32]).map_err(|e| e.to_string())?;
        }
        tx.execute_batch("DROP TABLE content;")
            .map_err(|e| e.to_string())?;
    }
    for &(name, content, attribution, resource) in starters::TEXT {
        let extra = match name {
            "quran_player" => "audio_path",
            "azan_reminder" => "date",
            _ => "attribution",
        };
        if matches!(name, "quran_player" | "azan_reminder") {
            tx.execute(
                &format!("INSERT INTO {name}(content,attribution,{extra}) VALUES(?1,?2,?3)"),
                params![content, attribution, resource],
            )
            .map_err(|e| e.to_string())?;
        } else {
            tx.execute(
                &format!("INSERT INTO {name}(content,attribution) VALUES(?1,?2)"),
                params![content, attribution],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tx.execute(
        "INSERT INTO achievements(date,finished_ids) VALUES(?1,'[]')",
        [chrono::Local::now().format("%Y-%m-%d").to_string()],
    )
    .map_err(|e| e.to_string())?;
    tx.pragma_update(None, "user_version", 2)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn row_id(category: i32, id: &str) -> Result<i64, String> {
    id.strip_prefix(&format!("{}:", table(category)?))
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "This entry belongs to a different collection.".into())
}

pub fn validate(
    category: i32,
    content: &str,
    attribution: &str,
    resource: &str,
) -> Result<(), String> {
    table(category)?;
    if category == 8 {
        return Err("Achievements are recorded from completed items.".into());
    }
    if content.trim().is_empty() || content.chars().count() > 20_000 {
        return Err("Enter content using 1–20,000 characters.".into());
    }
    if attribution.chars().count()
        > if matches!(category, 9 | 10) {
            4000
        } else {
            500
        }
    {
        return Err("Attribution is too long.".into());
    }
    if matches!(category, 9 | 10)
        && !resource
            .parse::<u32>()
            .is_ok_and(|n| (1..=10000).contains(&n))
    {
        return Err("Enter a repetition count from 1 to 10,000.".into());
    }
    if category == 6 {
        chrono::NaiveDate::parse_from_str(resource, "%Y-%m-%d")
            .map_err(|_| "Use YYYY-MM-DD for the schedule date.")?;
        for line in content.lines() {
            prayer_time(line)?;
        }
    }
    Ok(())
}

pub fn save(
    db: &Connection,
    category: i32,
    id: &str,
    content: &str,
    attribution: &str,
    resource: &str,
) -> Result<(), String> {
    let name = table(category)?;
    validate(category, content, attribution, resource)?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    let db = &tx;
    let key = if id.is_empty() {
        db.execute(
            &format!("INSERT INTO {name}(content,attribution) VALUES(?1,?2)"),
            params![content, attribution],
        )
        .map_err(|e| e.to_string())?;
        db.last_insert_rowid()
    } else {
        let key = row_id(category, id)?;
        if db
            .execute(
                &format!("UPDATE {name} SET content=?1,attribution=?2 WHERE id=?3"),
                params![content, attribution, key],
            )
            .map_err(|e| e.to_string())?
            != 1
        {
            return Err("This entry no longer exists.".into());
        }
        key
    };
    if matches!(category, 5 | 6 | 9 | 10) {
        let field = match category {
            5 => "audio_path",
            9 | 10 => "repetitions",
            _ => "date",
        };
        db.execute(
            &format!("UPDATE {name} SET {field}=?1 WHERE id=?2"),
            params![resource, key],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn delete(db: &Connection, category: i32, id: &str) -> Result<(), String> {
    let name = table(category)?;
    if category == 8 {
        return Err(
            "Undo a completion on its source widget to remove it from achievements.".into(),
        );
    }
    if db
        .execute(
            &format!("DELETE FROM {name} WHERE id=?1"),
            [row_id(category, id)?],
        )
        .map_err(|e| e.to_string())?
        != 1
    {
        return Err("This entry no longer exists.".into());
    }
    Ok(())
}

fn columns(category: i32) -> &'static str {
    match category {
        5 => "audio_path",
        6 => "date",
        9 | 10 => "repetitions",
        _ => "''",
    }
}

pub fn entry(db: &Connection, category: i32, key: i64) -> Result<Entry, String> {
    let name = table(category)?;
    db.query_row(
        &format!(
            "SELECT content,attribution,{},{} FROM {name} WHERE id=?1",
            columns(category),
            if category == 4 { "learned" } else { "0" }
        ),
        [key],
        |r| {
            Ok(Entry {
                id: format!("{name}:{key}"),
                content: r.get(0)?,
                attribution: r.get(1)?,
                resource: r.get(2)?,
                learned: r.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn collection(
    db: &Connection,
    category: i32,
    search: &str,
    page: i32,
) -> Result<Collection, String> {
    if category == 8 {
        return achievement_history(db, search, page);
    }
    let name = table(category)?;
    let filter = "instr(lower(content),lower(?1))>0 OR instr(lower(attribution),lower(?1))>0";
    let count = db
        .query_row(
            &format!("SELECT count(*) FROM {name} WHERE {filter}"),
            [search],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let bytes:i64=db.query_row(&format!("SELECT coalesce(sum(length(CAST(content AS BLOB))+length(CAST(attribution AS BLOB))),0) FROM {name}"),[],|r|r.get(0)).map_err(|e|e.to_string())?;
    let mut statement = db
        .prepare(&format!(
            "SELECT id FROM {name} WHERE {filter} ORDER BY id DESC LIMIT ?2 OFFSET ?3"
        ))
        .map_err(|e| e.to_string())?;
    let keys = statement
        .query_map(
            params![
                search,
                PAGE_SIZE,
                i64::from(page.max(0)) * i64::from(PAGE_SIZE)
            ],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let entries = keys
        .into_iter()
        .map(|key| entry(db, category, key))
        .collect::<Result<_, _>>()?;
    Ok(Collection {
        entries,
        count,
        bytes: bytes.max(0) as u64,
    })
}

pub fn prayer_time(line: &str) -> Result<(&str, chrono::NaiveTime), String> {
    let (name, time) = line
        .trim()
        .rsplit_once(' ')
        .ok_or("Use one prayer name and HH:MM time per line.")?;
    let time = chrono::NaiveTime::parse_from_str(time, "%H:%M")
        .map_err(|_| "Use HH:MM for prayer times.")?;
    if name.trim().is_empty() {
        return Err("Enter a prayer name.".into());
    }
    Ok((name, time))
}

pub fn finished_ids(db: &Connection, date: &str) -> Result<Vec<String>, String> {
    use rusqlite::OptionalExtension;
    let json: Option<String> = db
        .query_row(
            "SELECT finished_ids FROM achievements WHERE date=?1",
            [date],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(json.as_deref().unwrap_or("[]")).map_err(|e| e.to_string())
}

pub fn complete(db: &Connection, date: &str, id: &str, done: bool) -> Result<(), String> {
    let mut ids = finished_ids(db, date)?;
    let before = ids.clone();
    if done && !ids.iter().any(|v| v == id) {
        ids.push(id.into());
    }
    if !done {
        ids.retain(|v| v != id);
    }
    if ids == before {
        return Ok(());
    }
    let json = serde_json::to_string(&ids).map_err(|e| e.to_string())?;
    db.execute("INSERT INTO achievements(date,finished_ids) VALUES(?1,?2) ON CONFLICT(date) DO UPDATE SET finished_ids=excluded.finished_ids",params![date,json]).map_err(|e|e.to_string())?;
    Ok(())
}

pub fn save_task_title(db: &Connection, id: &str, title: &str) -> Result<(), String> {
    let title = title.lines().collect::<Vec<_>>().join(" ");
    if title.trim().is_empty() {
        return Ok(());
    }
    db.execute("INSERT INTO achievement_task_titles(id,title) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET title=excluded.title WHERE title<>excluded.title", params![id, title]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn achievement_text(db: &Connection, date: &str) -> Result<String, String> {
    let ids = finished_ids(db, date)?;
    let mut lines = Vec::new();
    for id in ids {
        if id.starts_with("ticktick:") {
            let title: Option<String> = db
                .query_row(
                    "SELECT title FROM achievement_task_titles WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            lines.push(format!("✓ {}", title.as_deref().unwrap_or("TickTick task")));
            continue;
        }
        let label = TABLES[..8]
            .iter()
            .position(|t| id.starts_with(&format!("{t}:")))
            .and_then(|c| {
                row_id(c as i32, &id)
                    .ok()
                    .and_then(|key| entry(db, c as i32, key).ok())
            })
            .map(|e| e.content.lines().next().unwrap_or("").to_owned())
            .unwrap_or_else(|| {
                if id.starts_with("morning_adhkar:") {
                    "Morning Remembrances".into()
                } else if id.starts_with("evening_adhkar:") {
                    "Evening Remembrances".into()
                } else {
                    "Removed item".into()
                }
            });
        lines.push(format!("✓ {label}"));
    }
    Ok(if lines.is_empty() {
        "No completed items today.".into()
    } else {
        lines.join("\n")
    })
}

fn achievement_history(db: &Connection, search: &str, page: i32) -> Result<Collection, String> {
    let count = db
        .query_row(
            "SELECT count(*) FROM achievements WHERE instr(date,?1)>0",
            [search],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let mut statement=db.prepare("SELECT id,date FROM achievements WHERE instr(date,?1)>0 ORDER BY date DESC LIMIT ?2 OFFSET ?3").map_err(|e|e.to_string())?;
    let days = statement
        .query_map(
            params![
                search,
                PAGE_SIZE,
                i64::from(page.max(0)) * i64::from(PAGE_SIZE)
            ],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for (id, date) in days {
        entries.push(Entry {
            id: format!("achievements:{id}"),
            content: achievement_text(db, &date)?,
            attribution: date,
            resource: String::new(),
            learned: false,
        });
    }
    let bytes = entries.iter().map(|e| e.content.len() as u64).sum();
    Ok(Collection {
        entries,
        count,
        bytes,
    })
}
pub fn folder_bytes(path: &Path) -> std::io::Result<u64> {
    let mut bytes = 0;
    if !path.exists() {
        return Ok(0);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_file() {
            bytes += entry.metadata()?.len();
        } else if kind.is_dir() {
            bytes += folder_bytes(&entry.path())?;
        }
    }
    Ok(bytes)
}

pub fn size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("orbit-{name}-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        path.join("widgets.db")
    }

    #[test]
    fn achievements_show_saved_ticktick_titles_after_restart() {
        let path = path("achievement-titles");
        let db = open(&path).unwrap();
        db.execute_batch("DROP TABLE achievement_task_titles; PRAGMA user_version=5;")
            .unwrap();
        drop(db);
        let db = open(&path).unwrap();
        complete(&db, "2026-10-28", "ticktick:task-one", true).unwrap();
        complete(&db, "2026-10-28", "ticktick:unknown", true).unwrap();
        db.execute(
            "INSERT INTO achievement_task_titles(id,title) VALUES(?1,?2)",
            params!["ticktick:task-one", "Read a chapter — اقرأ فصلًا"],
        )
        .unwrap();
        drop(db);
        let db = open(&path).unwrap();
        assert_eq!(
            achievement_text(&db, "2026-10-28").unwrap(),
            "✓ Read a chapter — اقرأ فصلًا\n✓ TickTick task"
        );
        assert_eq!(
            collection(&db, 8, "2026-10-28", 0).unwrap().entries[0].content,
            "✓ Read a chapter — اقرأ فصلًا\n✓ TickTick task"
        );
        save_task_title(&db, "ticktick:task-one", "Updated title\nwith details").unwrap();
        save_task_title(&db, "ticktick:task-one", " ").unwrap();
        assert!(
            achievement_text(&db, "2026-10-28")
                .unwrap()
                .starts_with("✓ Updated title with details\n")
        );
        complete(&db, "2026-10-28", "ticktick:task-one", false).unwrap();
        assert_eq!(
            achievement_text(&db, "2026-10-28").unwrap(),
            "✓ TickTick task"
        );
        drop(db);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn migration_preserves_original_rows_and_seeds_only_once() {
        let path = path("migration");
        let old = Connection::open(&path).unwrap();
        old.execute_batch("CREATE TABLE content(id INTEGER PRIMARY KEY AUTOINCREMENT, category INTEGER NOT NULL, content TEXT NOT NULL, attribution TEXT NOT NULL DEFAULT ''); INSERT INTO content VALUES(41,0,'My saved quote مرحبا','Mine'); PRAGMA user_version=1;").unwrap();
        drop(old);
        let db = open(&path).unwrap();
        let entry = entry(&db, 0, 41).unwrap();
        assert_eq!(entry.content, "My saved quote مرحبا");
        assert_eq!(entry.id, "quotes:41");
        assert_eq!(collection(&db, 0, "", 0).unwrap().count, 4);
        db.execute("DELETE FROM quotes", []).unwrap();
        drop(db);
        let db = open(&path).unwrap();
        assert_eq!(collection(&db, 0, "", 0).unwrap().count, 0);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='content'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        drop(db);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn evening_migration_preserves_morning_edits_and_progress_and_seeds_once() {
        let path = path("evening-migration");
        let mut db = Connection::open(&path).unwrap();
        migrate(&mut db, 0).unwrap();
        crate::adhkar::migrate(&mut db).unwrap();
        save(&db, 9, "morning_adhkar:1", "My morning edit", "Mine", "8").unwrap();
        let saved = r#"{"current":1,"counts":{"morning_adhkar:1":8}}"#;
        db.execute(
            "INSERT INTO adhkar_progress VALUES('morning-widget','2026-10-27',?1)",
            [saved],
        )
        .unwrap();
        drop(db);

        let db = open(&path).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            6
        );
        let morning = entry(&db, 9, 1).unwrap();
        assert_eq!(
            (
                morning.content.as_str(),
                morning.attribution.as_str(),
                morning.resource.as_str()
            ),
            ("My morning edit", "Mine", "8")
        );
        assert_eq!(db.query_row("SELECT progress FROM adhkar_progress WHERE id='morning-widget' AND date='2026-10-27'", [], |r| r.get::<_, String>(0)).unwrap(), saved);
        assert_eq!(collection(&db, 10, "", 0).unwrap().count, 25);
        let evening = entry(&db, 10, 22).unwrap();
        assert_eq!(evening.id, "evening_adhkar:22");
        assert_eq!(evening.resource, "3");
        assert!(evening.attribution.contains("يُقال في المساء فقط."));
        assert!(delete(&db, 9, &evening.id).is_err());
        db.execute("DELETE FROM evening_adhkar", []).unwrap();
        drop(db);

        let db = open(&path).unwrap();
        assert_eq!(collection(&db, 10, "", 0).unwrap().count, 0);
        drop(db);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn evening_repetitions_and_typed_ids_stay_in_their_collection() {
        let path = path("evening-entries");
        let db = open(&path).unwrap();
        let morning = entry(&db, 9, 2).unwrap();
        let evening = entry(&db, 10, 2).unwrap();
        assert_ne!(morning.id, evening.id);
        assert!(save(&db, 10, &morning.id, "Wrong collection", "", "1").is_err());
        assert!(delete(&db, 9, &evening.id).is_err());
        assert!(validate(10, "ذكر", "", "0").is_err());
        assert!(validate(10, "ذكر", "", "10001").is_err());
        assert!(validate(10, "ذكر", &"ع".repeat(1000), "3").is_ok());
        save(&db, 10, &evening.id, "Edited evening", "Details", "7").unwrap();
        assert_eq!(entry(&db, 10, 2).unwrap().resource, "7");
        assert_eq!(entry(&db, 9, 2).unwrap().content, morning.content);
        save(&db, 10, "", "New evening", "", "4").unwrap();
        assert_eq!(
            collection(&db, 10, "New evening", 0).unwrap().entries[0].resource,
            "4"
        );
        drop(db);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn collections_preserve_duplicates_and_qualified_ids() {
        let path = path("collections");
        let db = open(&path).unwrap();
        save(&db, 0, "", "Hello مرحبا", "Author", "").unwrap();
        save(&db, 0, "", "Hello مرحبا", "Author", "").unwrap();
        let entries = collection(&db, 0, "Hello", 0).unwrap();
        assert_eq!(entries.count, 2);
        assert_ne!(entries.entries[0].id, entries.entries[1].id);
        let id = entries.entries[0].id.clone();
        save(&db, 0, &id, "Edited محتوى", "Mine", "").unwrap();
        assert!(delete(&db, 1, &id).is_err());
        drop(db);
        let db = open(&path).unwrap();
        assert_eq!(collection(&db, 0, "Edited", 0).unwrap().entries[0].id, id);
        delete(&db, 0, &id).unwrap();
        assert!(save(&db, 0, "", "   ", "", "").is_err());
        assert!(save(&db, 8, "", "fake completion", "", "").is_err());
        drop(db);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn completion_references_and_rotation_survive_restarts() {
        let path = path("rotation");
        let mut db = open(&path).unwrap();
        let spec = crate::widget_views::Spec {
            id: "quote-widget".into(),
            category: 0,
        };
        let mut shown = std::collections::HashSet::new();
        for _ in 0..3 {
            let view = crate::widget_views::views(&db, std::slice::from_ref(&spec), &spec.id, &[])
                .unwrap()
                .remove(0);
            assert!(shown.insert(view.entry_id));
        }
        let before = crate::widget_views::views(&db, std::slice::from_ref(&spec), "", &[])
            .unwrap()
            .remove(0)
            .entry_id;
        drop(db);
        db = open(&path).unwrap();
        assert_eq!(
            crate::widget_views::views(&db, std::slice::from_ref(&spec), "", &[]).unwrap()[0]
                .entry_id,
            before
        );
        let habit = collection(&db, 7, "", 0).unwrap().entries.remove(0);
        let spec = crate::widget_views::Spec {
            id: "habits".into(),
            category: 7,
        };
        crate::widget_views::finish(&db, &spec, &habit.id).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(finished_ids(&db, &today).unwrap(), vec![habit.id.clone()]);
        assert!(
            achievement_text(&db, &today)
                .unwrap()
                .contains(habit.content.lines().next().unwrap())
        );
        crate::widget_views::finish(&db, &spec, &habit.id).unwrap();
        assert!(finished_ids(&db, &today).unwrap().is_empty());
        assert_eq!(
            db.query_row(
                "SELECT finished_ids FROM achievements WHERE date=?1",
                [today],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "[]"
        );
        drop(db);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
