use chrono::{Datelike, Local, NaiveDate};
use serde::Deserialize;
use std::{
    process::Command,
    time::{Duration, Instant},
};

const REFRESH: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct Usage {
    pub total_value: String,
    pub total_unit: String,
    pub down_value: String,
    pub down_unit: String,
    pub up_value: String,
    pub up_unit: String,
    pub updated: String,
    pub down_ratio: f32,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            total_value: "0".into(),
            total_unit: "B".into(),
            down_value: "0".into(),
            down_unit: "B".into(),
            up_value: "0".into(),
            up_unit: "B".into(),
            updated: "--:--".into(),
            down_ratio: 0.5,
        }
    }
}

#[derive(Default)]
pub struct Cache {
    pub usage: Usage,
    last_attempt: Option<Instant>,
}

impl Cache {
    pub fn refresh_if_due(&mut self) {
        if self
            .last_attempt
            .is_some_and(|last| last.elapsed() < REFRESH)
        {
            return;
        }
        self.last_attempt = Some(Instant::now());
        match fetch() {
            Ok(usage) => self.usage = usage,
            Err(error) => eprintln!("orbit: Internet Usage refresh failed: {error}"),
        }
    }
}

#[derive(Deserialize)]
struct VnStat {
    #[serde(default)]
    interfaces: Vec<Interface>,
}

#[derive(Deserialize)]
struct Interface {
    #[serde(default)]
    traffic: Traffic,
}

#[derive(Default, Deserialize)]
struct Traffic {
    #[serde(default)]
    day: Vec<Day>,
}

#[derive(Deserialize)]
struct Day {
    date: Date,
    rx: u64,
    tx: u64,
}

#[derive(Deserialize)]
struct Date {
    year: i32,
    month: u32,
    day: u32,
}

fn fetch() -> Result<Usage, String> {
    let output = Command::new("vnstat")
        .args(["--json", "d", "2"])
        .output()
        .map_err(|error| format!("cannot run vnstat: {error}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if error.is_empty() {
            format!("vnstat exited with {}", output.status)
        } else {
            error
        });
    }
    let data: VnStat = serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let now = Local::now();
    let today = now.date_naive();
    let (download, upload) = today_totals(&data, today);
    Ok(usage(download, upload, now.format("%H:%M").to_string()))
}

fn today_totals(data: &VnStat, today: NaiveDate) -> (u64, u64) {
    data.interfaces
        .iter()
        .flat_map(|interface| &interface.traffic.day)
        .filter(|entry| {
            entry.date.year == today.year()
                && entry.date.month == today.month()
                && entry.date.day == today.day()
        })
        .fold((0u64, 0u64), |(rx, tx), entry| {
            (rx.saturating_add(entry.rx), tx.saturating_add(entry.tx))
        })
}

fn usage(download: u64, upload: u64, updated: String) -> Usage {
    let (down_value, down_unit) = human(download);
    let (up_value, up_unit) = human(upload);
    let (total_value, total_unit) = human(download.saturating_add(upload));
    let total = download as f64 + upload as f64;
    Usage {
        total_value,
        total_unit,
        down_value,
        down_unit,
        up_value,
        up_unit,
        updated,
        down_ratio: if total == 0.0 {
            0.5
        } else {
            (download as f64 / total) as f32
        },
    }
}

fn human(bytes: u64) -> (String, String) {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let value = if unit == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    };
    (value, UNITS[unit].into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_binary_units_and_download_share() {
        let usage = usage(954_329_989, 449_430_159, "01:19".into());
        assert_eq!(
            (usage.down_value.as_str(), usage.down_unit.as_str()),
            ("910.12", "MiB")
        );
        assert_eq!(
            (usage.up_value.as_str(), usage.up_unit.as_str()),
            ("428.61", "MiB")
        );
        assert_eq!(
            (usage.total_value.as_str(), usage.total_unit.as_str()),
            ("1.31", "GiB")
        );
        assert!((usage.down_ratio - 0.6800).abs() < 0.001);
    }

    #[test]
    fn no_usage_has_even_capsule_share() {
        assert_eq!(usage(0, 0, "00:00".into()).down_ratio, 0.5);
    }

    #[test]
    fn sums_only_today_across_interfaces() {
        let json = r#"{"interfaces":[
            {"traffic":{"day":[
                {"date":{"year":2026,"month":11,"day":1},"rx":10,"tx":3},
                {"date":{"year":2026,"month":10,"day":31},"rx":100,"tx":30}
            ]}},
            {"traffic":{"day":[
                {"date":{"year":2026,"month":11,"day":1},"rx":20,"tx":7}
            ]}}
        ]}"#;
        let data: VnStat = serde_json::from_str(json).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        assert_eq!(today_totals(&data, today), (30, 10));
    }
}
