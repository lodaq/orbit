#[path = "backup.rs"]
mod backup;
pub use backup::transfer_data;

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
pub(crate) fn config_path() -> io::Result<PathBuf> {
    #[cfg(windows)]
    let directory = PathBuf::from(
        std::env::var_os("APPDATA").ok_or_else(|| io::Error::other("APPDATA is not set"))?,
    );
    #[cfg(not(windows))]
    let directory = match std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is not set"))?,
        )
        .join(".config"),
    };
    Ok(directory.join("orbit").join("config.json"))
}

pub fn read_config() -> io::Result<Option<Vec<u8>>> {
    match fs::read(config_path()?) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn backup_config() -> io::Result<()> {
    let path = config_path()?;
    fs::rename(&path, path.with_extension("json.bad"))
}

pub fn write_config(data: &[u8]) -> io::Result<()> {
    let path = config_path()?;
    fs::create_dir_all(path.parent().expect("config has a directory"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn recent_path() -> io::Result<PathBuf> {
    Ok(config_path()?.with_file_name("recent.json"))
}

pub fn read_recent() -> io::Result<Option<Vec<u8>>> {
    match fs::read(recent_path()?) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn backup_recent() -> io::Result<()> {
    let path = recent_path()?;
    fs::rename(&path, path.with_extension("json.bad"))
}

pub fn write_recent(data: &[u8]) -> io::Result<()> {
    let path = recent_path()?;
    fs::create_dir_all(path.parent().expect("recent has a directory"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn installed_apps_path() -> io::Result<PathBuf> {
    Ok(config_path()?.with_file_name("installed-apps.json"))
}

pub fn read_installed_apps() -> io::Result<Option<Vec<u8>>> {
    match fs::read(installed_apps_path()?) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn backup_installed_apps() -> io::Result<()> {
    let path = installed_apps_path()?;
    fs::rename(&path, path.with_extension("json.bad"))
}

pub fn write_installed_apps(data: &[u8]) -> io::Result<()> {
    let path = installed_apps_path()?;
    fs::create_dir_all(path.parent().expect("installed apps have a directory"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn icons_directory() -> io::Result<PathBuf> {
    let directory = config_path()?
        .parent()
        .expect("config has a directory")
        .join("icons");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

pub fn icon_path(name: &str) -> io::Result<PathBuf> {
    Ok(icons_directory()?.join(name))
}

pub fn read_icon(name: &str) -> io::Result<Vec<u8>> {
    fs::read(icon_path(name)?)
}

pub fn write_icon(name: &str, data: &[u8]) -> io::Result<()> {
    let path = icon_path(name)?;
    let temporary = path.with_extension("tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn remove_icon(name: &str) {
    if let Ok(path) = icon_path(name) {
        let _ = fs::remove_file(path);
    }
}

pub fn fetch_icon(url: &str) -> Option<(String, Vec<u8>)> {
    let (scheme, address) = url.split_once("://")?;
    let host = address.split(['/', '?', '#']).next()?;
    if host.is_empty()
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-:".contains(&byte))
    {
        return None;
    }
    if let Some((_, page)) = fetch_url(url, "html")
        && let Some(icon) = icon_link(&page, scheme, host)
        && let Some(image) = fetch_url(&icon, "")
    {
        return Some(image);
    }
    for extension in ["svg", "png", "jpg", "ico"] {
        let address = format!("{scheme}://{host}/favicon.{extension}");
        if let Some(image) = fetch_url(&address, extension) {
            return Some(image);
        }
    }
    fetch_url(
        &format!("https://www.google.com/s2/favicons?domain_url={host}&sz=128"),
        "png",
    )
}

fn fetch_url(address: &str, extension: &str) -> Option<(String, Vec<u8>)> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=http,https",
            "--max-time",
            "4",
            "--max-filesize",
            "2097152",
            address,
        ])
        .output()
        .ok()?;
    (output.status.success() && is_icon(&output.stdout, extension))
        .then_some((extension.into(), output.stdout))
}

fn icon_link(html: &[u8], scheme: &str, host: &str) -> Option<String> {
    let html = std::str::from_utf8(html).ok()?;
    let mut remaining = html;
    while let Some(start) = remaining.to_ascii_lowercase().find("<link") {
        remaining = &remaining[start..];
        let end = remaining.find('>')?;
        let tag = &remaining[..end];
        remaining = &remaining[end + 1..];
        let Some(rel) = attribute(tag, "rel") else {
            continue;
        };
        if !rel
            .split_ascii_whitespace()
            .any(|word| word.eq_ignore_ascii_case("icon"))
        {
            continue;
        }
        let Some(href) = attribute(tag, "href") else {
            continue;
        };
        let address = if href.starts_with("https://") || href.starts_with("http://") {
            href.to_string()
        } else if let Some(path) = href.strip_prefix("//") {
            format!("{scheme}://{path}")
        } else if href.starts_with('/') {
            format!("{scheme}://{host}{href}")
        } else {
            format!("{scheme}://{host}/{href}")
        };
        return Some(address);
    }
    None
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut offset = 5;
    while offset < bytes.len() {
        while offset < bytes.len() && (bytes[offset].is_ascii_whitespace() || bytes[offset] == b'/')
        {
            offset += 1;
        }
        let start = offset;
        while offset < bytes.len() && !bytes[offset].is_ascii_whitespace() && bytes[offset] != b'='
        {
            offset += 1;
        }
        if start == offset {
            break;
        }
        let key = &tag[start..offset];
        while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }
        if offset == bytes.len() || bytes[offset] != b'=' {
            continue;
        }
        offset += 1;
        while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }
        if offset == bytes.len() {
            break;
        }
        let quote = matches!(bytes[offset], b'"' | b'\'').then_some(bytes[offset]);
        if quote.is_some() {
            offset += 1;
        }
        let start = offset;
        while offset < bytes.len()
            && quote.map_or(!bytes[offset].is_ascii_whitespace(), |quote| {
                bytes[offset] != quote
            })
        {
            offset += 1;
        }
        let value = &tag[start..offset];
        if quote.is_some() && offset < bytes.len() {
            offset += 1;
        }
        if key.eq_ignore_ascii_case(name) {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod icon_tests {
    use super::icon_link;

    #[test]
    fn reads_site_icon_with_attributes_in_either_order() {
        assert_eq!(
            icon_link(
                br#"<link rel="stylesheet" href="x.css"><link href="/brand.png" sizes="64x64" rel="icon">"#,
                "https",
                "example.com",
            ),
            Some("https://example.com/brand.png".into()),
        );
    }

    #[test]
    fn reads_shortcut_icon_rel_tokens() {
        assert_eq!(
            icon_link(
                br#"<link rel="shortcut icon" href="/favicon.png">"#,
                "https",
                "example.com",
            ),
            Some("https://example.com/favicon.png".into()),
        );
    }

    #[test]
    fn reads_absolute_site_icon_url() {
        assert_eq!(
            icon_link(
                br#"<link rel="icon" href="https://icons.example.com/brand.png">"#,
                "https",
                "example.com",
            ),
            Some("https://icons.example.com/brand.png".into()),
        );
    }
}

pub fn choose_path(directory: bool) -> Option<PathBuf> {
    let dialog = rfd::FileDialog::new();
    if directory {
        dialog.pick_folder()
    } else {
        dialog.pick_file()
    }
}

fn is_icon(data: &[u8], extension: &str) -> bool {
    match extension {
        "svg" => {
            let data = data.trim_ascii_start();
            data.starts_with(b"<svg") || data.starts_with(b"<?xml")
        }
        "png" => data.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" => data.starts_with(b"\xff\xd8\xff"),
        "ico" => data.starts_with(&[0, 0, 1, 0]),
        "html" => data.starts_with(b"<!DOCTYPE html") || data.starts_with(b"<html"),
        "" => {
            data.starts_with(b"<svg")
                || data.starts_with(b"<?xml")
                || data.starts_with(b"\x89PNG\r\n\x1a\n")
                || data.starts_with(b"\xff\xd8\xff")
                || data.starts_with(&[0, 0, 1, 0])
        }
        _ => false,
    }
}

pub fn choose_icon() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose icon")
        .add_filter("Icon", &["png", "jpg", "jpeg", "svg", "ico"])
        .pick_file()
}

pub fn read_chosen_icon(value: &str) -> Result<(String, Vec<u8>), String> {
    use std::hash::{Hash, Hasher};
    let path = PathBuf::from(value);
    let data = if path.is_absolute() {
        fs::read(&path)
    } else {
        read_icon(value)
    }
    .map_err(|_| "Couldn't read icon")?;
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "svg" | "ico") {
        return Err("Choose a PNG, JPEG, SVG or ICO icon.".into());
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hash);
    Ok((format!("chosen-{:x}.{extension}", hash.finish()), data))
}
