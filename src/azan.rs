use crate::{widget_data as data, widget_views::View};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub city: String,
    pub country: String,
    pub timezone: String,
    pub schedule: Option<i64>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            city: "Sohag".into(),
            country: "Egypt".into(),
            timezone: "Africa/Cairo".into(),
            schedule: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Schedule {
    pub date: String,
    pub content: String,
}

pub fn migrate(db: &mut Connection) -> Result<(), String> {
    let tx = db.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch("CREATE TABLE azan_settings(id TEXT PRIMARY KEY, city TEXT NOT NULL, country TEXT NOT NULL, timezone TEXT NOT NULL, schedule_id INTEGER); PRAGMA user_version=5;").map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn validate(settings: &Settings) -> Result<Tz, String> {
    if settings.city.trim().is_empty()
        || settings.country.trim().is_empty()
        || settings.city.len() > 100
        || settings.country.len() > 100
    {
        return Err("Enter a city and country using 1–100 characters each.".into());
    }
    settings
        .timezone
        .parse()
        .map_err(|_| "Enter a named time zone, such as Africa/Cairo or Europe/London.".into())
}

pub fn settings(db: &Connection, id: &str) -> Result<Settings, String> {
    db.query_row(
        "SELECT city,country,timezone,schedule_id FROM azan_settings WHERE id=?1",
        [id],
        |r| {
            Ok(Settings {
                city: r.get(0)?,
                country: r.get(1)?,
                timezone: r.get(2)?,
                schedule: r.get(3)?,
            })
        },
    )
    .optional()
    .map(|v| v.unwrap_or_default())
    .map_err(|e| e.to_string())
}

pub fn save(
    db: &Connection,
    id: &str,
    settings: &Settings,
    schedule: Option<&Schedule>,
) -> Result<(), String> {
    validate(settings)?;
    let old = self::settings(db, id)?;
    let mut key = if old.city == settings.city
        && old.country == settings.country
        && old.timezone == settings.timezone
    {
        old.schedule
    } else {
        None
    };
    if schedule.is_none()
        && old.city == settings.city
        && old.country == settings.country
        && old.timezone == settings.timezone
        && old.schedule.is_some()
    {
        return Ok(());
    }
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    if let Some(schedule) = schedule {
        data::validate(6, &schedule.content, "", &schedule.date)?;
        tx.execute(
            "INSERT INTO azan_reminder(content,attribution,date) VALUES(?1,?2,?3)",
            params![
                schedule.content,
                format!(
                    "{}, {} · {} · Egyptian General Authority of Survey · Aladhan",
                    settings.city, settings.country, settings.timezone
                ),
                schedule.date
            ],
        )
        .map_err(|e| e.to_string())?;
        key = Some(tx.last_insert_rowid());
    }
    tx.execute("INSERT INTO azan_settings(id,city,country,timezone,schedule_id) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET city=excluded.city,country=excluded.country,timezone=excluded.timezone,schedule_id=excluded.schedule_id",params![id,settings.city,settings.country,settings.timezone,key]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn fetch(settings: &Settings) -> Result<Schedule, String> {
    let zone = validate(settings)?;
    let date = Utc::now().with_timezone(&zone).date_naive();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .max_redirects(0)
        .build()
        .into();
    let json = agent
        .get(format!(
            "https://api.aladhan.com/v1/timingsByCity/{}",
            date.format("%d-%m-%Y")
        ))
        .query("city", &settings.city)
        .query("country", &settings.country)
        .query("method", "5")
        .query("timezonestring", &settings.timezone)
        .call()
        .map_err(|_| {
            "Couldn't fetch prayer times. Check the connection and location, then try again."
                .to_string()
        })?
        .body_mut()
        .with_config()
        .limit(128 * 1024)
        .read_json::<serde_json::Value>()
        .map_err(|_| "Prayer times returned an unreadable response.".to_string())?;
    parse_schedule(&json, date, &settings.timezone)
}

pub fn parse_schedule(
    json: &serde_json::Value,
    date: NaiveDate,
    zone: &str,
) -> Result<Schedule, String> {
    let data = &json["data"];
    if json["code"].as_i64() != Some(200)
        || data["meta"]["timezone"].as_str() != Some(zone)
        || data["date"]["gregorian"]["date"].as_str() != Some(&date.format("%d-%m-%Y").to_string())
    {
        return Err("The response did not match the requested date and time zone.".into());
    }
    let mut lines = Vec::new();
    for prayer in ["Fajr", "Dhuhr", "Asr", "Maghrib", "Isha"] {
        let time = data["timings"][prayer]
            .as_str()
            .ok_or("The response is missing a prayer time.")?;
        let line = format!("{prayer} {time}");
        data::prayer_time(&line)?;
        lines.push(line);
    }
    Ok(Schedule {
        date: date.format("%Y-%m-%d").to_string(),
        content: lines.join("\n"),
    })
}

pub fn remaining(date: &str, line: &str, zone: Tz, now: DateTime<Utc>) -> Option<i64> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    if date != now.with_timezone(&zone).date_naive() {
        return None;
    }
    let (_, time) = data::prayer_time(line).ok()?;
    let prayer = zone.from_local_datetime(&date.and_time(time)).single()?;
    let seconds = (prayer.with_timezone(&Utc) - now).num_seconds();
    (seconds > 0).then_some((seconds + 59) / 60)
}

pub fn view(db: &Connection, view: &mut View, now: DateTime<Utc>) -> Result<(), String> {
    let settings = settings(db, &view.id)?;
    let zone = validate(&settings)?;
    view.city = settings.city;
    view.country = settings.country;
    view.timezone = settings.timezone;
    view.done = true;
    let entry = if let Some(id) = settings.schedule {
        data::entry(db, 6, id).ok()
    } else {
        None
    };
    let Some(entry) = entry else {
        view.content =
            "Set up prayer times\nOpen Azan settings to fetch and save today's schedule.".into();
        view.details = "No schedule saved for this location.".into();
        return Ok(());
    };
    view.entry_id = entry.id;
    let today = now.with_timezone(&zone).format("%Y-%m-%d").to_string();
    let next = entry
        .content
        .lines()
        .filter_map(|line| {
            remaining(&entry.resource, line, zone, now).map(|minutes| (line, minutes))
        })
        .min_by_key(|(_, minutes)| *minutes);
    view.done = entry.resource != today;
    if let Some((line, minutes)) = next {
        let (name, time) = data::prayer_time(line)?;
        view.content = format!("{name}\nIn {}h {}m", minutes / 60, minutes % 60);
        view.resource = time.format("%H:%M").to_string();
    } else if view.done {
        view.content="Update prayer times\nSaved times are out of date. Open settings to Fetch today's times.".into();
    } else {
        view.content = "Prayers complete\nToday's final prayer has passed.".into();
    }
    for line in entry.content.lines() {
        let (name, time) = data::prayer_time(line)?;
        view.rows.push((
            data::Entry {
                id: name.into(),
                content: name.into(),
                attribution: String::new(),
                resource: time.format("%H:%M").to_string(),
                learned: false,
            },
            next.is_some_and(|(next, _)| next == line),
        ));
    }
    view.details = format!("{} · {}", entry.resource, view.timezone);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn accepted_schedules_are_per_widget_and_location_changes_do_not_relabel_them() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE azan_reminder(id INTEGER PRIMARY KEY AUTOINCREMENT,content TEXT,attribution TEXT,date TEXT);").unwrap();
        migrate(&mut db).unwrap();
        let settings = Settings::default();
        let schedule = Schedule {
            date: "2026-10-28".into(),
            content: "Fajr 05:20\nDhuhr 12:00\nAsr 15:10\nMaghrib 17:30\nIsha 18:45".into(),
        };
        save(&db, "sohag", &settings, Some(&schedule)).unwrap();
        let saved = self::settings(&db, "sohag").unwrap();
        assert!(saved.schedule.is_some());
        assert_eq!(self::settings(&db, "another").unwrap().schedule, None);
        let changed = Settings {
            city: "Luxor".into(),
            ..settings
        };
        save(&db, "sohag", &changed, None).unwrap();
        assert_eq!(self::settings(&db, "sohag").unwrap().schedule, None);
        assert!(
            data::entry(&db, 6, saved.schedule.unwrap())
                .unwrap()
                .attribution
                .contains("Sohag")
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM azan_reminder", [], |r| r
                .get::<_, i32>(0))
                .unwrap(),
            1
        );
        let invalid = Settings {
            timezone: "Invalid/Zone".into(),
            ..changed.clone()
        };
        assert!(save(&db, "sohag", &invalid, Some(&schedule)).is_err());
        assert_eq!(self::settings(&db, "sohag").unwrap().city, changed.city);
    }

    #[test]
    fn stale_schedules_remain_visible_without_a_countdown() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE azan_reminder(id INTEGER PRIMARY KEY AUTOINCREMENT,content TEXT,attribution TEXT,date TEXT);").unwrap();
        migrate(&mut db).unwrap();
        let spec = crate::widget_views::Spec {
            id: "sohag".into(),
            category: 6,
        };
        let mut view = crate::widget_views::views(&db, &[spec], "", &[])
            .unwrap()
            .remove(0);
        let schedule = Schedule {
            date: "2026-10-28".into(),
            content: "Fajr 05:20\nDhuhr 12:00".into(),
        };
        save(&db, "sohag", &Settings::default(), Some(&schedule)).unwrap();
        self::view(
            &db,
            &mut view,
            Utc.with_ymd_and_hms(2026, 10, 29, 8, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(view.done);
        assert!(view.resource.is_empty());
        assert_eq!(view.rows.len(), 2);
        assert!(view.rows.iter().all(|(_, next)| !next));
        assert!(view.content.contains("out of date"));
    }

    #[test]
    fn accepts_named_time_zones_and_rejects_unknown_or_empty_locations() {
        assert_eq!(
            validate(&Settings::default()).unwrap(),
            chrono_tz::Africa::Cairo
        );
        let mut settings = Settings::default();
        settings.timezone = "Sohag".into();
        assert!(validate(&settings).is_err());
        settings.timezone = "UTC".into();
        settings.city.clear();
        assert!(validate(&settings).is_err());
    }
    #[test]
    fn countdown_uses_the_selected_zone_date_and_seasonal_offset() {
        let zone = chrono_tz::Africa::Cairo;
        let summer = Utc.with_ymd_and_hms(2026, 7, 1, 8, 0, 0).unwrap();
        let winter = Utc.with_ymd_and_hms(2026, 12, 1, 8, 0, 0).unwrap();
        assert_eq!(
            remaining("2026-07-01", "Dhuhr 12:00", zone, summer),
            Some(60)
        );
        assert_eq!(
            remaining("2026-12-01", "Dhuhr 12:00", zone, winter),
            Some(120)
        );
        assert_eq!(remaining("2026-12-01", "Fajr 05:00", zone, winter), None);
        assert_eq!(remaining("2026-11-30", "Dhuhr 12:00", zone, winter), None);
    }
    #[test]
    fn fetch_requires_all_five_prayers_matching_date_and_zone() {
        let mut json = serde_json::json!({"code":200,"data":{"date":{"gregorian":{"date":"28-10-2026"}},"meta":{"timezone":"Africa/Cairo"},"timings":{"Fajr":"05:20","Dhuhr":"12:00","Asr":"15:10","Maghrib":"17:30","Isha":"18:45"}}});
        let date = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let parsed = parse_schedule(&json, date, "Africa/Cairo").unwrap();
        assert!(parsed.content.contains("Maghrib 17:30"));
        assert_eq!(parsed.date, "2026-10-28");
        assert!(parse_schedule(&json, date, "UTC").is_err());
        json["data"]["timings"]["Fajr"] = "25:00".into();
        assert!(parse_schedule(&json, date, "Africa/Cairo").is_err());
    }
}
