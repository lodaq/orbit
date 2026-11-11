use std::{
    io::Write,
    process::{Command, Stdio},
};

const TABLE: &str = "orbit_speed_limits";
const OWNER: &str = "Orbit Internet Usage";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Limits {
    pub download: u64,
    pub upload: u64,
}

impl Limits {
    pub fn parse(download: &str, upload: &str) -> Result<Self, String> {
        Ok(Self {
            download: rate(download)?,
            upload: rate(upload)?,
        })
    }

    pub fn values(&self) -> (String, String) {
        (kbps(self.download), kbps(self.upload))
    }

    fn rules(&self) -> String {
        let mut rules = format!(
            "add table inet {TABLE} {{ comment \"{OWNER}\"; }}\nflush table inet {TABLE}\n"
        );
        for (name, hook, interface, rate) in [
            ("download", "input", "iifname", self.download),
            ("upload", "output", "oifname", self.upload),
        ] {
            if rate == 0 {
                continue;
            }
            let burst = (rate / 10).max(65536);
            rules.push_str(&format!(
                "add chain inet {TABLE} {name} {{ type filter hook {hook} priority 0; policy accept; }}\n\
                 add rule inet {TABLE} {name} {interface} != \"lo\" limit rate over {rate} bytes/second burst {burst} bytes counter drop\n"
            ));
        }
        rules
    }
}

fn rate(text: &str) -> Result<u64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(0);
    }
    let error = "Enter a whole number from 100 to 10,000,000 kbps, or 0 for Unlimited.";
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(error.into());
    }
    let value = text.parse::<u64>().map_err(|_| error)?;
    if value != 0 && !(100..=10_000_000).contains(&value) {
        return Err(error.into());
    }
    Ok(value * 125)
}

fn kbps(rate: u64) -> String {
    (rate as f64 / 125.0).to_string()
}

fn nft(args: &[&str], input: Option<&str>) -> Result<serde_json::Value, String> {
    if !cfg!(target_os = "linux") {
        return Err("Internet speed limits require Linux and nftables.".into());
    }
    let mut child = Command::new("sudo")
        .args(["-n", "/usr/bin/nft"])
        .args(args)
        .env("LC_ALL", "C")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Cannot run nftables through sudo: {e}"))?;
    let written = if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("piped input")
            .write_all(input.as_bytes())
    } else {
        Ok(())
    };
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Cannot access Internet limits. Administrator access through sudo and nftables is required. {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    written.map_err(|e| e.to_string())?;
    if input.is_some() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}

pub fn read() -> Result<Limits, String> {
    let tables = nft(&["-j", "list", "tables"], None)?;
    if !tables["nftables"]
        .as_array()
        .ok_or("Invalid nftables response")?
        .iter()
        .any(|entry| entry["table"]["family"] == "inet" && entry["table"]["name"] == TABLE)
    {
        return Ok(Limits::default());
    }
    decode(&nft(&["-j", "list", "table", "inet", TABLE], None)?)
}

fn decode(data: &serde_json::Value) -> Result<Limits, String> {
    let entries = data["nftables"]
        .as_array()
        .ok_or("Invalid nftables response")?;
    if !entries
        .iter()
        .any(|entry| entry["table"]["comment"] == OWNER)
    {
        return Err("Internet limits table belongs to another tool; it was left unchanged.".into());
    }
    let mut limits = Limits::default();
    for entry in entries {
        let rule = &entry["rule"];
        let target = match rule["chain"].as_str() {
            Some("download") => &mut limits.download,
            Some("upload") => &mut limits.upload,
            _ => continue,
        };
        let limit = rule["expr"]
            .as_array()
            .and_then(|expr| expr.iter().find_map(|e| e.get("limit")))
            .ok_or("Internet limits have been changed outside Orbit")?;
        if limit["per"] != "second" || limit["rate_unit"] != "bytes" || limit["inv"] != true {
            return Err("Internet limits have been changed outside Orbit".into());
        }
        *target = limit["rate"]
            .as_u64()
            .ok_or("Invalid Internet limit rate")?;
    }
    Ok(limits)
}

pub fn apply(limits: &Limits) -> Result<Limits, String> {
    // Read first so an unrelated table with the same name is never flushed.
    read()?;
    nft(&["-f", "-"], Some(&limits.rules()))?;
    let active = read()?;
    if active != *limits {
        return Err(
            "Internet limits could not be verified. Reopen Limit to check their state.".into(),
        );
    }
    Ok(active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_speeds_and_converts_kilobits_to_bytes() {
        assert_eq!(
            Limits::parse("2500", "1000").unwrap(),
            Limits {
                download: 312500,
                upload: 125000
            }
        );
        assert_eq!(Limits::parse("", "0").unwrap(), Limits::default());
        for value in [
            "-1",
            "NaN",
            "inf",
            "1e3",
            "1; flush ruleset",
            "99",
            "10000001",
            "1.5",
            ".",
        ] {
            assert!(Limits::parse(value, "1000").is_err(), "{value}");
            assert!(Limits::parse("1000", value).is_err(), "{value}");
        }
        let limits = Limits::parse("100", "10000000").unwrap();
        assert_eq!(limits.values(), ("100".into(), "10000000".into()));
    }

    #[test]
    fn rules_are_atomic_and_only_limit_local_non_loopback_traffic() {
        let rules = Limits::parse("2000", "0").unwrap().rules();
        assert!(rules.contains("hook input"));
        assert!(rules.contains("iifname != \"lo\" limit rate over 250000 bytes/second"));
        assert!(!rules.contains("hook output"));
        assert!(!rules.contains("forward"));
        assert!(!rules.contains("flush ruleset"));
        assert!(!Limits::default().rules().contains("add chain"));
        assert!(
            decode(&serde_json::json!({"nftables": [{"table": {"comment": "someone else"}}]}))
                .is_err()
        );
    }
}
