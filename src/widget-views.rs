use crate::widget_data::{self as data, Entry};
use chrono::Local;
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Clone, PartialEq)]
pub struct Spec {
    pub id: String,
    pub category: i32,
}
pub struct View {
    pub id: String,
    pub category: i32,
    pub city: String,
    pub country: String,
    pub timezone: String,
    pub position: i32,
    pub total: i32,
    pub required: i32,
    pub remaining: i32,
    pub content: String,
    pub details: String,
    pub entry_id: String,
    pub done: bool,
    pub resource: String,
    pub rows: Vec<(Entry, bool)>,
}

pub fn category(variant: i32) -> Option<i32> {
    [0, 1, 2, 3, 4, 8, 5, 6, 7, 9, 10]
        .get(variant.checked_sub(9)? as usize)
        .copied()
}

pub fn views(
    db: &Connection,
    specs: &[Spec],
    force: &str,
    opened: &[String],
) -> Result<Vec<View>, String> {
    specs
        .iter()
        .map(|spec| {
            view(
                db,
                spec,
                spec.id == force || (opened.iter().any(|id| id == &spec.id) && spec.category == 4),
            )
        })
        .collect()
}

fn view(db: &Connection, spec: &Spec, force: bool) -> Result<View, String> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut view = View {
        id: spec.id.clone(),
        category: spec.category,
        city: String::new(),
        country: String::new(),
        timezone: String::new(),
        position: 0,
        total: 0,
        required: 0,
        remaining: 0,
        content: String::new(),
        details: String::new(),
        entry_id: String::new(),
        done: false,
        resource: String::new(),
        rows: Vec::new(),
    };
    if spec.category == 6 {
        crate::azan::view(db, &mut view, chrono::Utc::now())?;
        return Ok(view);
    }
    if matches!(spec.category, 9 | 10) {
        crate::adhkar::view(db, &mut view, &today)?;
        return Ok(view);
    }
    if spec.category == 8 {
        view.content = data::achievement_text(db, &today)?;
        view.details = today;
        return Ok(view);
    }
    if spec.category == 7 {
        let done = data::finished_ids(db, &today)?;
        let mut statement = db
            .prepare("SELECT id FROM habits ORDER BY id")
            .map_err(|e| e.to_string())?;
        let keys = statement
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for key in keys {
            let entry = data::entry(db, 7, key)?;
            let checked = done.contains(&entry.id);
            view.rows.push((entry, checked));
        }
        if view.rows.is_empty() {
            view.content = "Add habits in Settings → Data.".into();
        }
        view.details = today;
        return Ok(view);
    }
    let Some(entry) = select(db, spec, &today, force)? else {
        view.content = "No saved content. Add an entry in Settings → Data.".into();
        return Ok(view);
    };
    view.done = data::finished_ids(db, &today)?.contains(&entry.id);
    view.entry_id = entry.id;
    view.content = entry.content;
    view.details = entry.attribution;
    view.resource = entry.resource;
    if spec.category == 4 {
        view.done = db
            .query_row(
                "SELECT learned FROM vocabulary WHERE id=?1",
                [data::row_id(4, &view.entry_id)?],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
    }
    if matches!(spec.category, 3 | 4) && view.done {
        view.content = if spec.category == 3 {
            "Thank you. Done for today."
        } else {
            "Word learned. Well done."
        }
        .into();
    }
    Ok(view)
}

fn select(db: &Connection, spec: &Spec, today: &str, force: bool) -> Result<Option<Entry>, String> {
    let name = data::table(spec.category)?;
    let state: Option<(i32, String, i64, String)> = db
        .query_row(
            "SELECT category,date,entry_id,seen FROM widget_rotation WHERE id=?1",
            [&spec.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let mut seen = Vec::<i64>::new();
    if let Some((category, date, key, json)) = state
        && category == spec.category
    {
        if !force
            && date == today
            && let Ok(entry) = data::entry(db, category, key)
        {
            return Ok(Some(entry));
        }
        seen = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    }
    let learned = if spec.category == 4 { "learned=0" } else { "1" };
    let mut chosen = None;
    for _ in 0..2 {
        let json = serde_json::to_string(&seen).map_err(|e| e.to_string())?;
        chosen=db.query_row(&format!("SELECT id FROM {name} WHERE {learned} AND id NOT IN (SELECT value FROM json_each(?1)) ORDER BY random() LIMIT 1"),[json],|r|r.get::<_,i64>(0)).optional().map_err(|e|e.to_string())?;
        if chosen.is_some() {
            break;
        }
        seen.clear();
    }
    let Some(key) = chosen else {
        return Ok(None);
    };
    seen.push(key);
    db.execute("INSERT INTO widget_rotation(id,category,date,entry_id,seen) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET category=excluded.category,date=excluded.date,entry_id=excluded.entry_id,seen=excluded.seen",params![spec.id,spec.category,today,key,serde_json::to_string(&seen).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
    data::entry(db, spec.category, key).map(Some)
}

pub fn finish(db: &Connection, spec: &Spec, id: &str) -> Result<(), String> {
    if !matches!(spec.category, 3 | 4 | 7) {
        return Err("This widget does not record completions.".into());
    }
    let key = data::row_id(spec.category, id)?;
    data::entry(db, spec.category, key)?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    let done = if spec.category == 4 {
        tx.query_row("SELECT learned FROM vocabulary WHERE id=?1", [key], |r| {
            r.get::<_, bool>(0)
        })
        .map_err(|e| e.to_string())?
    } else {
        data::finished_ids(&tx, &date)?.iter().any(|v| v == id)
    };
    if spec.category == 4 {
        tx.execute(
            "UPDATE vocabulary SET learned=?1 WHERE id=?2",
            params![!done, key],
        )
        .map_err(|e| e.to_string())?;
    }
    if spec.category == 4 && done {
        remove_completion(&tx, id)?;
    } else {
        data::complete(&tx, &date, id, !done)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn remove_completion(db: &Connection, id: &str) -> Result<(), String> {
    let mut statement=db.prepare("SELECT date FROM achievements WHERE EXISTS (SELECT 1 FROM json_each(finished_ids) WHERE value=?1)").map_err(|e|e.to_string())?;
    let dates = statement
        .query_map([id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for date in dates {
        data::complete(db, &date, id, false)?;
    }
    Ok(())
}
