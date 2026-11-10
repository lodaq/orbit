use super::config_path;
use crate::config::{Config, Icon, InstalledApp, Item};
use crate::recent::Recent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Backup {
    version: u32,
    config: Config,
    recent: Recent,
    installed_apps: Vec<InstalledApp>,
    icons: BTreeMap<String, Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    robots: Option<crate::robot_data::Snapshot>,
}

pub fn transfer_data(import: bool) -> Result<Option<String>, String> {
    let title = if import { "Import data" } else { "Export data" };
    let _chooser_guard = crate::platform::float_file_chooser(title);
    let chooser = rfd::FileDialog::new()
        .set_title(title)
        .add_filter("Orbit backup", &["json"]);
    let path = if import {
        chooser.pick_file()
    } else {
        chooser.set_file_name("orbit-backup.json").save_file()
    };
    let Some(path) = path else { return Ok(None) };
    let result = if import {
        import_data(&path)
    } else {
        export_data(&path)
    };
    result.map(Some).map_err(|error| {
        format!(
            "Couldn't {} data: {error}",
            if import { "import" } else { "export" }
        )
    })
}

fn export_data(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let config = crate::config::decode(&super::read_config()?.ok_or("Missing config.json")?)?;
    let recent = serde_json::from_slice(&super::read_recent()?.unwrap_or_else(|| b"[]".to_vec()))?;
    let installed_apps =
        serde_json::from_slice(&super::read_installed_apps()?.unwrap_or_else(|| b"[]".to_vec()))?;
    let mut backup = Backup {
        version: 5,
        config,
        recent,
        installed_apps,
        icons: BTreeMap::new(),
        robots: Some(crate::robot_data::export(
            &config_path()?.with_file_name("robots.db"),
        )?),
    };
    let directory = config_path()?.with_file_name("icons");
    if directory.exists() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Invalid icon filename")?;
                backup.icons.insert(name, fs::read(entry.path())?);
            }
        }
    }
    backup.validate()?;
    let parent = path
        .parent()
        .ok_or("Missing backup directory")?
        .canonicalize()?;
    let data_directory = config_path()?
        .parent()
        .ok_or("Missing data directory")?
        .canonicalize()?;
    if parent.starts_with(data_directory) {
        return Err("Choose a backup location outside Orbit's data directory".into());
    }
    let data = serde_json::to_vec(&backup)?;
    if data.len() > 128 * 1024 * 1024 {
        return Err("Backup exceeds 128 MB".into());
    }
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temporary = parent.join(format!(".orbit-export-{suffix}.tmp"));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&data)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(format!("Data exported to {}", path.display()))
}

fn import_data(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut data)?;
    if data.len() > 128 * 1024 * 1024 {
        return Err("Backup exceeds 128 MB".into());
    }
    let mut backup: Backup = serde_json::from_slice(&data)?;
    let value: serde_json::Value = serde_json::from_slice(&data)?;
    backup.config = crate::config::decode(&serde_json::to_vec(&value["config"])?)?;
    backup.validate()?;
    let directory = config_path()?
        .parent()
        .ok_or("Missing data directory")?
        .to_path_buf();
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let staging = directory.with_file_name(format!("orbit-import-{suffix}"));
    let previous = directory.with_file_name(format!("orbit-before-import-{suffix}"));
    fs::create_dir(&staging)?;
    if let Err(error) = backup.write(&staging).and_then(|()| {
        if value["config"]["version"].as_u64().unwrap_or(1) < 5
            || value["config"].get("pages").is_some()
        {
            write_file(
                &staging.join(format!("backup-before-canvas-{suffix}.json")),
                &data,
            )?;
        }
        Ok(())
    }) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if let Err(error) = fs::rename(&directory, &previous) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&staging, &directory) {
        fs::rename(&previous, &directory)?;
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }
    Ok(format!(
        "Data imported. Previous data saved in {}",
        previous.display()
    ))
}

impl Backup {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(self.version, 1..=5) || !self.config.valid() {
            return Err("Unsupported backup or invalid Canvas".into());
        }
        if self.version < 4
            && self
                .robots
                .as_ref()
                .is_some_and(|robots| !robots.accounts.is_empty())
        {
            return Err("Provider accounts require backup version 4".into());
        }
        if self.version < 5
            && self
                .robots
                .as_ref()
                .is_some_and(|robots| !robots.catalog.is_empty())
        {
            return Err("Robot collections require backup version 5".into());
        }
        match (self.version, &self.robots) {
            (1, None) => {}
            (2..=5, Some(robots)) => robots.validate()?,
            _ => return Err("Missing or incompatible Robot backup data".into()),
        }
        for name in self.icons.keys() {
            if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
                return Err("Invalid icon filename".into());
            }
        }
        for item in self.config.items.iter() {
            match item {
                Item::Widget { .. } | Item::Robot { .. } => {}
                Item::App(app) => self.validate_icon(&app.icon)?,
                Item::Folder { apps, .. } => {
                    for app in apps {
                        self.validate_icon(&app.icon)?;
                    }
                }
                Item::Website { icon, .. }
                | Item::File { icon, .. }
                | Item::Directory { icon, .. } => self.validate_icon(icon)?,
            }
        }
        for app in &self.installed_apps {
            self.validate_icon(&app.icon)?;
        }
        Ok(())
    }

    fn validate_icon(&self, icon: &Icon) -> Result<(), Box<dyn std::error::Error>> {
        if let Icon::File(name) = icon
            && !self.icons.contains_key(name)
        {
            return Err(format!("Missing icon: {name}").into());
        }
        Ok(())
    }

    fn write(&self, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(robots) = &self.robots {
            crate::robot_data::Database::open(&directory.join("robots.db"))?.restore(robots)?;
        }
        fs::create_dir(directory.join("icons"))?;
        for (name, data) in &self.icons {
            write_file(&directory.join("icons").join(name), data)?;
        }
        write_file(
            &directory.join("config.json"),
            &serde_json::to_vec_pretty(&self.config)?,
        )?;
        write_file(
            &directory.join("recent.json"),
            &serde_json::to_vec(&self.recent)?,
        )?;
        write_file(
            &directory.join("installed-apps.json"),
            &serde_json::to_vec(&self.installed_apps)?,
        )?;
        Ok(())
    }
}

fn write_file(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robot_backup_is_versioned_and_restored_in_staging() {
        use crate::robot_data::{Database, Entry, McpServer, Model, Profile, Provider, Role};
        let mut db = Database::open(Path::new(":memory:")).unwrap();
        db.save_role(&Role {
            id: "writer".into(),
            name: "كاتب".into(),
            instructions: "Include linked sources.".into(),
        })
        .unwrap();
        db.save_profile(&Profile {
            id: "robot".into(),
            name: "Research".into(),
            project: "/tmp/project".into(),
            model: Model {
                account: None,
                provider: Provider::OpenAi,
                id: "existing-model".into(),
                ollama_address: None,
            },
            role_id: Some("writer".into()),
            skills: vec!["/tmp/skills/research/SKILL.md".into()],
            mcp_servers: vec![McpServer {
                id: "search".into(),
                command: "search-mcp".into(),
                arguments: vec![],
                secret_env: BTreeMap::from([("API_KEY".into(), "search-key".into())]),
                enabled_tools: vec!["search".into()],
            }],
        })
        .unwrap();
        db.start("conversation", "robot", "ابحث عن Rust", 10)
            .unwrap();
        db.append(
            "conversation",
            &Entry::Reply {
                text: "[Source](https://www.rust-lang.org)".into(),
            },
            20,
        )
        .unwrap();
        db.rate("conversation", 5).unwrap();
        db.save_catalog(&crate::robot_data::CatalogEntry {
            id: "review".into(),
            kind: "prompt".into(),
            name: "Review".into(),
            content: "Review the project".into(),
        })
        .unwrap();
        let backup = Backup {
            version: 5,
            config: Config::default(),
            recent: Recent { ids: vec![] },
            installed_apps: vec![],
            icons: BTreeMap::new(),
            robots: Some(db.snapshot().unwrap()),
        };
        let bytes = serde_json::to_vec(&backup).unwrap();
        let loaded: Backup = serde_json::from_slice(&bytes).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.robots, backup.robots);
        let directory = std::env::temp_dir().join(format!(
            "orbit-robot-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        loaded.write(&directory).unwrap();
        let mut expected = backup.robots.unwrap();
        expected.disable_tools();
        assert_eq!(
            crate::robot_data::export(&directory.join("robots.db")).unwrap(),
            expected
        );
        assert!(directory.join("config.json").exists());
        fs::remove_dir_all(directory).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        json["version"] = 4.into();
        assert!(
            serde_json::from_value::<Backup>(json.clone())
                .unwrap()
                .validate()
                .is_err()
        );
        json["robots"]["catalog"] = serde_json::json!([]);
        json["version"] = 2.into();
        serde_json::from_value::<Backup>(json.clone())
            .unwrap()
            .validate()
            .unwrap();
        json["version"] = 3.into();
        json.as_object_mut().unwrap().remove("robots");
        assert!(
            serde_json::from_value::<Backup>(json.clone())
                .unwrap()
                .validate()
                .is_err()
        );
        json["version"] = 1.into();
        serde_json::from_value::<Backup>(json)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn canvas_backup_roundtrip_keeps_signed_positions_view_recent_and_icons() {
        let mut config = Config::default();
        config.items.push(Item::Website {
            id: "site".into(),
            name: "Site".into(),
            url: "https://example.com".into(),
            icon: Icon::File("site.svg".into()),
        });
        config.positions.insert(
            "site".into(),
            crate::spatial::Rect {
                col: -100_000,
                row: 80_000,
                width: 144,
                height: 144,
            },
        );
        config.canvas.view = crate::canvas::View {
            x: 80.0,
            y: -90.0,
            zoom: 0.1,
        };
        let backup = Backup {
            version: 1,
            config,
            recent: crate::recent::Recent {
                ids: vec!["site".into()],
            },
            installed_apps: vec![],
            icons: BTreeMap::from([("site.svg".into(), b"<svg/>".to_vec())]),
            robots: None,
        };
        backup.validate().unwrap();
        let saved = serde_json::to_vec(&backup).unwrap();
        let loaded: Backup = serde_json::from_slice(&saved).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.config.positions, backup.config.positions);
        assert_eq!(loaded.config.canvas, backup.config.canvas);
        assert_eq!(loaded.recent.ids, backup.recent.ids);
        assert_eq!(loaded.icons, backup.icons);
    }
}
