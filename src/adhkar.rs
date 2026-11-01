use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct Period {
    pub category: i32,
    pub date: String,
    pub optimal: bool,
    pub window: String,
}

pub fn period(
    settings: &crate::azan::Settings,
    schedule: &crate::azan::Schedule,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Period, String> {
    let zone = crate::azan::validate(settings)?;
    let local = now.with_timezone(&zone);
    let times = schedule
        .content
        .lines()
        .map(crate::widget_data::prayer_time)
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let time = |name| {
        times
            .get(name)
            .copied()
            .ok_or_else(|| format!("Azan schedule is missing {name}."))
    };
    let fajr = time("Fajr")?;
    let dhuhr = time("Dhuhr")?;
    let asr = time("Asr")?;
    let maghrib = time("Maghrib")?;
    if !(fajr < dhuhr && dhuhr < asr && asr < maghrib) {
        return Err("Azan prayer times must be in order.".into());
    }
    let morning = local.time() >= fajr && local.time() < dhuhr;
    let optimal = if morning {
        times
            .get("Sunrise")
            .is_some_and(|sunrise| local.time() < *sunrise)
    } else {
        local.time() >= asr && local.time() < maghrib
    };
    let date = if local.time() < fajr {
        local
            .date_naive()
            .pred_opt()
            .ok_or("Invalid prayer date.")?
    } else {
        local.date_naive()
    };
    let mut window = if optimal {
        "Recommended window"
    } else {
        "Permissible window"
    }
    .to_string();
    if schedule.date != local.format("%Y-%m-%d").to_string() {
        window.push_str(" · Saved times are out of date; update Azan Reminder.");
    } else if morning && !times.contains_key("Sunrise") {
        window.push_str(" · Fetch Azan times to include sunrise.");
    }
    Ok(Period {
        category: if morning { 9 } else { 10 },
        date: date.to_string(),
        optimal,
        window,
    })
}

#[derive(Default, Deserialize, Serialize)]
struct Progress {
    current: usize,
    counts: BTreeMap<String, u32>,
}

impl Progress {
    // Return 1 for an entry completion, 2 for the complete collection.
    fn act(&mut self, entries: &[(String, u32)], action: &str) -> u8 {
        if action == "restart" {
            *self = Self::default();
            return 0;
        }
        if action == "previous" {
            self.current = self.current.saturating_sub(1);
            return 0;
        }
        let Some((id, required)) = entries.get(self.current) else {
            return 0;
        };
        let mut sound = 0;
        if action == "count" {
            let count = self.counts.entry(id.clone()).or_default();
            if *count < *required {
                *count += 1;
                if *count < *required {
                    return 0;
                }
                sound = 1;
            }
        } else if action != "skip" {
            return 0;
        }
        let next = (1..=entries.len())
            .map(|n| (self.current + n) % entries.len())
            .find(|i| self.counts.get(&entries[*i].0).copied().unwrap_or(0) < entries[*i].1);
        self.current = next.unwrap_or(entries.len());
        if next.is_none() && sound == 1 {
            2
        } else {
            sound
        }
    }
}

#[derive(Deserialize)]
struct Bundle {
    adhkar: Vec<Remembrance>,
}
#[derive(Deserialize)]
struct Remembrance {
    title: String,
    text: String,
    count: u32,
    count_note: Option<String>,
    virtue: Option<String>,
    source: Source,
    notes: Option<String>,
}
#[derive(Deserialize)]
struct Source {
    book: String,
    reference: String,
    grading: String,
}

pub fn migrate(db: &mut Connection) -> Result<(), String> {
    let tx = db.transaction().map_err(|e| e.to_string())?;
    seed(
        &tx,
        "morning_adhkar",
        include_str!("../data/morning-adhkar.json"),
    )?;
    tx.execute_batch("CREATE TABLE adhkar_progress(id TEXT PRIMARY KEY, date TEXT NOT NULL, progress TEXT NOT NULL CHECK(json_valid(progress)));").map_err(|e| e.to_string())?;
    tx.pragma_update(None, "user_version", 3)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn migrate_evening(db: &mut Connection) -> Result<(), String> {
    let tx = db.transaction().map_err(|e| e.to_string())?;
    seed(
        &tx,
        "evening_adhkar",
        include_str!("../data/evening-adhkar.json"),
    )?;
    tx.pragma_update(None, "user_version", 4)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

fn seed(db: &Connection, name: &str, json: &str) -> Result<(), String> {
    db.execute_batch(&format!("CREATE TABLE {name}(id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT NOT NULL, attribution TEXT NOT NULL DEFAULT '', repetitions TEXT NOT NULL DEFAULT '1');")).map_err(|e| e.to_string())?;
    let bundle: Bundle = serde_json::from_str(json).map_err(|e| e.to_string())?;
    for entry in bundle.adhkar {
        let details = format!(
            "{}المصدر: {} — {} ({}){}{}",
            entry
                .virtue
                .map(|v| format!("الفضل: {v}\n\n"))
                .unwrap_or_default(),
            entry.source.book,
            entry.source.reference,
            entry.source.grading,
            entry
                .count_note
                .map(|n| format!("\n\n{n}"))
                .unwrap_or_default(),
            entry.notes.map(|n| format!("\n\n{n}")).unwrap_or_default()
        );
        db.execute(
            &format!("INSERT INTO {name}(content,attribution,repetitions) VALUES(?1,?2,?3)"),
            params![
                format!("{}\n{}", entry.title, entry.text),
                details,
                entry.count.to_string()
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn entries(db: &Connection, category: i32) -> Result<Vec<crate::widget_data::Entry>, String> {
    if !matches!(category, 9 | 10) {
        return Err("Choose a remembrances collection.".into());
    }
    let name = crate::widget_data::table(category)?;
    let mut statement = db
        .prepare(&format!("SELECT id FROM {name} ORDER BY id"))
        .map_err(|e| e.to_string())?;
    let keys = statement
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    keys.map(|key| crate::widget_data::entry(db, category, key.map_err(|e| e.to_string())?))
        .collect()
}

fn load(db: &Connection, id: &str, date: &str) -> Result<Progress, String> {
    let saved: Option<String> = db
        .query_row(
            "SELECT progress FROM adhkar_progress WHERE id=?1 AND date=?2",
            params![id, date],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    saved
        .map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .unwrap_or_else(|| Ok(Progress::default()))
}

pub fn merge_progress(db: &Connection, keep: &str, removed: &str) -> Result<(), String> {
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    for category in [9, 10] {
        let table = crate::widget_data::table(category)?;
        let target = format!("{keep}/{table}");
        let donor = format!("{removed}/{table}");
        let mut statement = tx.prepare("SELECT date,progress FROM adhkar_progress WHERE id IN (?1,?2,?3,?4) ORDER BY date DESC").map_err(|e|e.to_string())?;
        let saved = statement
            .query_map(params![keep, removed, target, donor], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut date = String::new();
        let mut combined = Progress::default();
        for (day, json) in saved {
            let progress: Progress = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            if !progress
                .counts
                .keys()
                .any(|id| id.starts_with(&format!("{table}:")))
            {
                continue;
            }
            if date.is_empty() {
                date = day.clone();
                combined.current = progress.current;
            }
            if day != date {
                continue;
            }
            for (id, count) in progress.counts {
                if id.starts_with(&format!("{table}:")) {
                    let saved = combined.counts.entry(id).or_default();
                    *saved = (*saved).max(count);
                }
            }
        }
        if !date.is_empty() {
            let json = serde_json::to_string(&combined).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO adhkar_progress VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET date=excluded.date,progress=excluded.progress", params![target,date,json]).map_err(|e|e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

fn session(
    db: &Connection,
    id: &str,
    azan_id: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(String, Period), String> {
    let azan_id = azan_id.ok_or("Add an Azan Reminder and save its prayer times.")?;
    let (settings, schedule) = crate::azan::saved_schedule(db, azan_id)?
        .ok_or("Open Azan settings to Fetch and save prayer times.")?;
    let period = period(&settings, &schedule, now)?;
    let name = crate::widget_data::table(period.category)?;
    let session_id = format!("{id}/{name}");
    // Keep legacy rows intact while importing the matching collection once.
    db.execute("INSERT OR IGNORE INTO adhkar_progress SELECT ?1,date,progress FROM adhkar_progress WHERE id=?2 AND date=?3 AND EXISTS(SELECT 1 FROM json_each(json_extract(progress,'$.counts')) WHERE key LIKE ?4)", params![session_id,id,period.date,format!("{name}:%")]).map_err(|e|e.to_string())?;
    Ok((session_id, period))
}

pub fn automatic_view(
    db: &Connection,
    view: &mut crate::widget_views::View,
    azan_id: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), String> {
    let (session_id, period) = match session(db, &view.id, azan_id, now) {
        Ok(value) => value,
        Err(error) => {
            view.content = error;
            return Ok(());
        }
    };
    view.category = period.category;
    view.resource = period.window;
    view.recommended = period.optimal;
    let id = std::mem::replace(&mut view.id, session_id);
    let result = self::view(db, view, &period.date);
    view.id = id;
    if view.done {
        view.entry_id = format!("{}:complete", crate::widget_data::table(view.category)?);
    }
    result
}

pub fn automatic_action(
    db: &Connection,
    id: &str,
    azan_id: Option<&str>,
    expected: &str,
    action: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<u8, String> {
    let (session_id, period) = session(db, id, azan_id, now)?;
    if !expected.starts_with(&format!("{}:", crate::widget_data::table(period.category)?)) {
        return Ok(0);
    }
    action_on(
        db,
        &session_id,
        period.category,
        expected,
        action,
        &period.date,
    )
}

fn widget_id(id: &str) -> &str {
    id.strip_suffix("/morning_adhkar")
        .or_else(|| id.strip_suffix("/evening_adhkar"))
        .unwrap_or(id)
}

#[cfg(test)]
pub fn action(
    db: &Connection,
    id: &str,
    category: i32,
    expected: &str,
    action: &str,
) -> Result<u8, String> {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    action_on(db, id, category, expected, action, &date)
}

fn action_on(
    db: &Connection,
    id: &str,
    category: i32,
    expected: &str,
    action: &str,
    date: &str,
) -> Result<u8, String> {
    let entries = entries(db, category)?;
    let mut progress = load(db, id, date)?;
    if entries
        .iter()
        .any(|e| progress.counts.get(&e.id).copied().unwrap_or(0) < e.resource.parse().unwrap_or(1))
    {
        progress.current = progress.current.min(entries.len().saturating_sub(1));
    }
    // A queued click belongs to the entry that was on screen, never its successor.
    if action != "restart"
        && entries
            .get(progress.current)
            .map(|e| e.id.as_str())
            .unwrap_or("")
            != expected
    {
        return Ok(0);
    }
    let counts = entries
        .iter()
        .map(|e| (e.id.clone(), e.resource.parse().unwrap_or(1)))
        .collect::<Vec<_>>();
    let sound = progress.act(&counts, action);
    let saved = serde_json::to_string(&progress).map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO adhkar_progress VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET date=excluded.date,progress=excluded.progress", params![id,date,saved]).map_err(|e| e.to_string())?;
    if sound == 2 {
        crate::widget_data::complete(
            &tx,
            date,
            &format!("{}:{}", crate::widget_data::table(category)?, widget_id(id)),
            true,
        )?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(sound)
}

pub fn record_completed(db: &Connection, date: &str) -> Result<(), String> {
    let mut statement = db
        .prepare("SELECT id,progress FROM adhkar_progress WHERE date=?1")
        .map_err(|e| e.to_string())?;
    let saved = statement
        .query_map([date], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for category in [9, 10] {
        let entries = entries(db, category)?;
        if entries.is_empty() {
            continue;
        }
        for (id, json) in &saved {
            let progress: Progress = serde_json::from_str(json).map_err(|e| e.to_string())?;
            if entries.iter().all(|entry| {
                progress.counts.get(&entry.id).copied().unwrap_or(0)
                    >= entry.resource.parse().unwrap_or(1)
            }) {
                crate::widget_data::complete(
                    db,
                    date,
                    &format!("{}:{}", crate::widget_data::table(category)?, widget_id(id)),
                    true,
                )?;
            }
        }
    }
    Ok(())
}

pub fn view(
    db: &Connection,
    view: &mut crate::widget_views::View,
    date: &str,
) -> Result<(), String> {
    let entries = entries(db, view.category)?;
    let progress = load(db, &view.id, date)?;
    view.total = entries.len() as i32;
    if entries.is_empty() {
        view.content = "أضف الأذكار من Settings → Data".into();
        return Ok(());
    }
    view.done = entries.iter().all(|e| {
        progress.counts.get(&e.id).copied().unwrap_or(0) >= e.resource.parse().unwrap_or(1)
    });
    view.position = if view.done {
        view.total
    } else {
        progress.current.min(entries.len() - 1) as i32 + 1
    };
    if view.done {
        view.content = "تقبّل الله طاعتكم".into();
        return Ok(());
    }
    let entry = &entries[view.position as usize - 1];
    view.entry_id = entry.id.clone();
    view.content = entry.content.clone();
    view.details = entry.attribution.clone();
    view.required = entry.resource.parse().unwrap_or(1);
    view.remaining = view
        .required
        .saturating_sub(progress.counts.get(&entry.id).copied().unwrap_or(0) as i32);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_sessions_preserve_modes_midnight_and_legacy_progress() {
        use chrono::TimeZone;
        let path = std::env::temp_dir()
            .join(format!("orbit-automatic-{}", std::process::id()))
            .join("widgets.db");
        let db = crate::widget_data::open(&path).unwrap();
        let settings = crate::azan::Settings {
            timezone: "UTC".into(),
            ..Default::default()
        };
        let schedule = crate::azan::Schedule {
            date: "2026-11-01".into(),
            content: "Fajr 04:30\nSunrise 06:00\nDhuhr 12:00\nAsr 15:30\nMaghrib 18:00\nIsha 19:00"
                .into(),
        };
        crate::azan::save(&db, "azan", &settings, Some(&schedule)).unwrap();
        let morning = chrono::Utc.with_ymd_and_hms(2026, 11, 1, 5, 0, 0).unwrap();
        let evening = chrono::Utc.with_ymd_and_hms(2026, 11, 1, 16, 0, 0).unwrap();
        let midnight = chrono::Utc.with_ymd_and_hms(2026, 11, 2, 0, 0, 0).unwrap();
        assert_eq!(
            automatic_action(&db, "one", None, "morning_adhkar:1", "count", morning).unwrap_err(),
            "Add an Azan Reminder and save its prayer times."
        );
        assert_eq!(
            automatic_action(
                &db,
                "one",
                Some("azan"),
                "morning_adhkar:1",
                "count",
                morning
            )
            .unwrap(),
            1
        );
        assert_eq!(
            automatic_action(
                &db,
                "one",
                Some("azan"),
                "morning_adhkar:2",
                "count",
                evening
            )
            .unwrap(),
            0
        );
        assert_eq!(
            automatic_action(
                &db,
                "one",
                Some("azan"),
                "evening_adhkar:1",
                "count",
                evening
            )
            .unwrap(),
            1
        );
        automatic_action(
            &db,
            "one",
            Some("azan"),
            "evening_adhkar:2",
            "count",
            midnight,
        )
        .unwrap();
        assert_eq!(
            load(&db, "one/evening_adhkar", "2026-11-01")
                .unwrap()
                .counts["evening_adhkar:2"],
            1
        );
        assert_eq!(
            load(&db, "one/morning_adhkar", "2026-11-01")
                .unwrap()
                .current,
            1
        );
        assert!(
            load(&db, "other/evening_adhkar", "2026-11-01")
                .unwrap()
                .counts
                .is_empty()
        );
        automatic_action(
            &db,
            "one",
            Some("azan"),
            "evening_adhkar:2",
            "restart",
            midnight,
        )
        .unwrap();
        assert!(
            load(&db, "one/evening_adhkar", "2026-11-01")
                .unwrap()
                .counts
                .is_empty()
        );
        assert_eq!(
            load(&db, "one/morning_adhkar", "2026-11-01")
                .unwrap()
                .current,
            1
        );
        db.execute(
            "INSERT INTO adhkar_progress VALUES('old-morning','2026-11-01',?1)",
            [r#"{"current":1,"counts":{"morning_adhkar:1":1}}"#],
        )
        .unwrap();
        db.execute(
            "INSERT INTO adhkar_progress VALUES('old-evening','2026-11-01',?1)",
            [r#"{"current":2,"counts":{"evening_adhkar:1":1,"evening_adhkar:2":2}}"#],
        )
        .unwrap();
        merge_progress(&db, "old-morning", "old-evening").unwrap();
        merge_progress(&db, "old-morning", "old-evening").unwrap();
        assert_eq!(
            load(&db, "old-morning/morning_adhkar", "2026-11-01")
                .unwrap()
                .counts["morning_adhkar:1"],
            1
        );
        assert_eq!(
            load(&db, "old-morning/evening_adhkar", "2026-11-01")
                .unwrap()
                .counts["evening_adhkar:2"],
            2
        );
        drop(db);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn periods_use_azan_timezone_and_warn_about_stale_times() {
        use chrono::TimeZone;
        let settings = crate::azan::Settings::default();
        let mut schedule = crate::azan::Schedule {
            date: "2026-11-01".into(),
            content: "Fajr 04:30\nSunrise 06:00\nDhuhr 12:00\nAsr 15:30\nMaghrib 18:00".into(),
        };
        let now = chrono::Utc.with_ymd_and_hms(2026, 11, 1, 10, 0, 0).unwrap();
        assert_eq!(period(&settings, &schedule, now).unwrap().category, 10);
        schedule.date = "2026-10-31".into();
        assert!(
            period(&settings, &schedule, now)
                .unwrap()
                .window
                .contains("out of date")
        );
        schedule.content = "Fajr 04:30".into();
        assert!(period(&settings, &schedule, now).is_err());
        schedule.content = "Fajr 12:30\nDhuhr 12:00\nAsr 15:30\nMaghrib 18:00".into();
        assert!(period(&settings, &schedule, now).is_err());
    }

    #[test]
    fn prayer_boundaries_choose_mode_and_evening_date() {
        use chrono::TimeZone;
        let settings = crate::azan::Settings {
            timezone: "UTC".into(),
            ..Default::default()
        };
        let schedule = crate::azan::Schedule {
            date: "2026-11-01".into(),
            content: "Fajr 04:30\nSunrise 06:00\nDhuhr 12:00\nAsr 15:30\nMaghrib 18:00\nIsha 19:00"
                .into(),
        };
        for (hour, minute, category, date, optimal) in [
            (0, 0, 10, "2026-10-31", false),
            (4, 29, 10, "2026-10-31", false),
            (4, 30, 9, "2026-11-01", true),
            (6, 0, 9, "2026-11-01", false),
            (11, 59, 9, "2026-11-01", false),
            (12, 0, 10, "2026-11-01", false),
            (15, 30, 10, "2026-11-01", true),
            (18, 0, 10, "2026-11-01", false),
            (23, 59, 10, "2026-11-01", false),
        ] {
            let now = chrono::Utc
                .with_ymd_and_hms(2026, 11, 1, hour, minute, 0)
                .unwrap();
            let period = period(&settings, &schedule, now).unwrap();
            assert_eq!(
                (period.category, period.date.as_str(), period.optimal),
                (category, date, optimal)
            );
        }
    }

    #[test]
    fn completed_collections_record_daily_achievements_and_restart_retains_them() {
        let path = std::env::temp_dir()
            .join(format!("orbit-achievements-{}", std::process::id()))
            .join("widgets.db");
        let db = crate::widget_data::open(&path).unwrap();
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        for (category, table, label) in [
            (9, "morning_adhkar", "Morning Remembrances"),
            (10, "evening_adhkar", "Evening Remembrances"),
        ] {
            let collection = entries(&db, category).unwrap();
            let id = format!("widget-{category}");
            action(&db, &id, category, &collection[0].id, "skip").unwrap();
            assert!(
                !crate::widget_data::finished_ids(&db, &date)
                    .unwrap()
                    .contains(&format!("{table}:{id}"))
            );
            action(&db, &id, category, "", "restart").unwrap();
            for entry in &collection {
                for _ in 0..entry.resource.parse::<u32>().unwrap() {
                    action(&db, &id, category, &entry.id, "count").unwrap();
                }
            }
            assert!(
                crate::widget_data::achievement_text(&db, &date)
                    .unwrap()
                    .contains(label)
            );
            db.execute("DELETE FROM achievements", []).unwrap();
            record_completed(&db, &date).unwrap();
            assert!(
                crate::widget_data::achievement_text(&db, &date)
                    .unwrap()
                    .contains(label)
            );
            let changes = db.total_changes();
            record_completed(&db, &date).unwrap();
            assert_eq!(db.total_changes(), changes);
            action(&db, &id, category, "", "restart").unwrap();
            assert_eq!(
                crate::widget_data::finished_ids(&db, &date)
                    .unwrap()
                    .iter()
                    .filter(|value| **value == format!("{table}:{id}"))
                    .count(),
                1
            );
        }
        assert!(
            crate::widget_data::finished_ids(&db, "2000-01-01")
                .unwrap()
                .is_empty()
        );
        drop(db);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn repetitions_advance_only_on_completion_and_finish_once() {
        let mut progress = Progress::default();
        let entries = vec![("first".into(), 3), ("last".into(), 1)];
        assert_eq!(progress.act(&entries, "count"), 0);
        assert_eq!(progress.current, 0);
        assert_eq!(progress.act(&entries, "count"), 0);
        assert_eq!(progress.act(&entries, "count"), 1);
        assert_eq!(progress.current, 1);
        assert_eq!(progress.act(&entries, "count"), 2);
        assert_eq!(progress.act(&entries, "count"), 0);
        assert_eq!(progress.current, 2);
    }

    #[test]
    fn skip_does_not_complete_and_previous_preserves_counts() {
        let entries = vec![("first".into(), 3), ("last".into(), 1)];
        let mut progress = Progress::default();
        progress.act(&entries, "count");
        progress.act(&entries, "skip");
        assert_eq!(progress.act(&entries, "count"), 1);
        assert_eq!(progress.current, 0);
        assert_eq!(progress.counts["first"], 1);
        progress.act(&entries, "skip");
        assert_eq!(progress.current, 0);
        progress.act(&entries, "count");
        assert_eq!(progress.act(&entries, "count"), 2);
        progress.act(&entries, "restart");
        assert!(progress.counts.is_empty());
        assert_eq!(progress.current, 0);
    }
    #[test]
    fn saved_progress_is_per_instance_per_day_and_stale_clicks_do_not_advance() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("orbit-adhkar-{stamp}"))
            .join("widgets.db");
        let db = crate::widget_data::open(&path).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        for (category, name) in [(9, "morning_adhkar"), (10, "evening_adhkar")] {
            let id = format!("one-{category}");
            let first = format!("{name}:1");
            let second = format!("{name}:2");
            assert_eq!(entries(&db, category).unwrap().len(), 25);
            assert_eq!(action(&db, &id, category, &first, "count").unwrap(), 1);
            assert_eq!(action(&db, &id, category, &first, "count").unwrap(), 0);
            assert_eq!(load(&db, &id, &today).unwrap().current, 1);
            assert_eq!(load(&db, "two", &today).unwrap().current, 0);
            assert_eq!(load(&db, &id, "2000-01-01").unwrap().current, 0);
            action(&db, &id, category, &second, "count").unwrap();
            action(&db, &id, category, &second, "previous").unwrap();
        }
        drop(db);
        let db = crate::widget_data::open(&path).unwrap();
        for (category, name) in [(9, "morning_adhkar"), (10, "evening_adhkar")] {
            let progress = load(&db, &format!("one-{category}"), &today).unwrap();
            assert_eq!(progress.current, 0);
            assert_eq!(progress.counts[&format!("{name}:2")], 1);
            assert_eq!(progress.counts.len(), 2);
        }
        drop(db);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn actions_reject_other_collections_and_cross_collection_entry_ids() {
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&mut db).unwrap();
        migrate_evening(&mut db).unwrap();
        assert!(action(&db, "invalid", 0, "quotes:1", "count").is_err());
        assert_eq!(
            action(&db, "evening", 10, "morning_adhkar:1", "count").unwrap(),
            0
        );
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert!(load(&db, "evening", &today).unwrap().counts.is_empty());
        assert_eq!(
            action(&db, "evening", 10, "evening_adhkar:1", "count").unwrap(),
            1
        );
        assert!(load(&db, "morning", &today).unwrap().counts.is_empty());
    }
}
