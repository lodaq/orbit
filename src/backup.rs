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
}

pub fn transfer_data(import: bool) -> Result<Option<String>, String> {
    let _chooser_guard =
        crate::platform::float_data_chooser(if import { "Import data" } else { "Export data" });
    let chooser = rfd::FileDialog::new().add_filter("Orbit backup", &["json"]);
    let path = if import {
        chooser.set_title("Import data").pick_file()
    } else {
        chooser
            .set_title("Export data")
            .set_file_name("orbit-backup.json")
            .save_file()
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
    let config = serde_json::from_slice(&super::read_config()?.ok_or("Missing config.json")?)?;
    let recent = serde_json::from_slice(&super::read_recent()?.unwrap_or_else(|| b"[]".to_vec()))?;
    let installed_apps =
        serde_json::from_slice(&super::read_installed_apps()?.unwrap_or_else(|| b"[]".to_vec()))?;
    let mut backup = Backup {
        version: 1,
        config,
        recent,
        installed_apps,
        icons: BTreeMap::new(),
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
    let backup: Backup = serde_json::from_slice(&data)?;
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
    if let Err(error) = backup.write(&staging) {
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
        if self.version != 1 || !self.config.valid() {
            return Err("Unsupported backup or invalid pages".into());
        }
        for name in self.icons.keys() {
            if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
                return Err("Invalid icon filename".into());
            }
        }
        for item in self.config.pages.iter().flat_map(|page| &page.items) {
            match item {
                Item::Widget { .. } => {}
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
