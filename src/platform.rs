use crate::config::{InstalledApp, SystemAction};
use slint::ComponentHandle;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::{
    fs::DirBuilderExt,
    net::{UnixListener, UnixStream},
    process::CommandExt,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key, NamedKey};

fn directory() -> io::Result<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR is not set"))?;
    let path = PathBuf::from(runtime).join("orbit");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&path)?;
    Ok(path)
}

fn config_path() -> io::Result<PathBuf> {
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

pub fn scan_installed_apps() -> io::Result<Vec<InstalledApp>> {
    let mut directories = Vec::new();
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(home) = data_home {
        directories.push(home.join("applications"));
        directories.push(home.join("flatpak/exports/share/applications"));
    }
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    for directory in std::env::split_paths(&data_dirs) {
        directories.push(directory.join("applications"));
    }
    directories.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    let icons = installed_icon_files();
    let mut apps = std::collections::BTreeMap::new();
    for directory in directories {
        scan_application_directory(&directory, &mut apps, &icons)?;
    }
    let mut apps: Vec<_> = apps.into_values().collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    Ok(apps)
}

fn scan_application_directory(
    directory: &Path,
    apps: &mut std::collections::BTreeMap<String, InstalledApp>,
    icons: &std::collections::HashMap<String, PathBuf>,
) -> io::Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            scan_application_directory(&path, apps, icons)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "desktop")
            && let Some(app) = parse_desktop_entry(&path, icons)
        {
            apps.entry(app.id.clone()).or_insert(app);
        }
    }
    Ok(())
}

fn parse_desktop_entry(
    path: &Path,
    icons: &std::collections::HashMap<String, PathBuf>,
) -> Option<InstalledApp> {
    let text = fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut hidden = false;
    let mut no_display = false;
    let mut terminal = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        } else if in_entry {
            if let Some(value) = line.strip_prefix("Name=") {
                name = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("Exec=") {
                exec = Some(value.trim());
            } else if let Some(value) = line.strip_prefix("Icon=") {
                icon = Some(value.trim());
            } else if let Some(value) = line.strip_prefix("Hidden=") {
                hidden = value == "true";
            } else if let Some(value) = line.strip_prefix("NoDisplay=") {
                no_display = value == "true";
            } else if let Some(value) = line.strip_prefix("Terminal=") {
                terminal = value == "true";
            }
        }
    }
    if hidden || no_display || terminal {
        return None;
    }
    let name = name.filter(|name| !name.is_empty())?;
    let words = shlex::split(exec?).filter(|words| !words.is_empty())?;
    let words: Vec<_> = words
        .into_iter()
        .filter(|word| !word.contains('%'))
        .collect();
    let command = shlex::try_join(words.iter().map(String::as_str)).ok()?;
    if command.is_empty() {
        return None;
    }
    let id = path.to_string_lossy().into_owned();
    let icon = icon
        .and_then(|name| resolve_icon(name, icons))
        .and_then(|path| cache_installed_icon(&id, &path).ok())
        .unwrap_or_default();
    Some(InstalledApp {
        id,
        name,
        command,
        icon,
    })
}

fn installed_icon_files() -> std::collections::HashMap<String, PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
    ];
    let home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(home) = home {
        roots.insert(0, home.join("icons"));
    }
    let mut icons = std::collections::HashMap::new();
    for root in roots {
        collect_icon_files(&root, &mut icons);
    }
    icons
}

fn collect_icon_files(directory: &Path, icons: &mut std::collections::HashMap<String, PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect_icon_files(&path, icons);
        } else if supported_icon(&path)
            && let Some(name) = path.file_stem().and_then(std::ffi::OsStr::to_str)
        {
            icons.entry(name.to_lowercase()).or_insert(path);
        }
    }
}

fn supported_icon(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| matches!(extension, "svg" | "png" | "jpg" | "ico"))
}

fn resolve_icon(name: &str, icons: &std::collections::HashMap<String, PathBuf>) -> Option<PathBuf> {
    let path = PathBuf::from(name);
    if path.is_absolute() && supported_icon(&path) && path.is_file() {
        return Some(path);
    }
    let stem = path.file_stem().and_then(std::ffi::OsStr::to_str)?;
    icons.get(&stem.to_lowercase()).cloned()
}

fn cache_installed_icon(id: &str, source: &Path) -> io::Result<crate::config::Icon> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    let extension = source
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("png");
    let name = format!("installed-{:x}.{extension}", hasher.finish());
    write_icon(&name, &fs::read(source)?)?;
    Ok(crate::config::Icon::File(name))
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

pub fn listen() -> io::Result<Option<(File, UnixListener)>> {
    let path = directory()?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.join("daemon.lock"))?;
    match lock.try_lock() {
        Ok(()) => (),
        Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
        Err(error) => return Err(io::Error::other(error)),
    }
    match fs::remove_file(path.join("daemon.sock")) {
        Ok(()) => (),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    Ok(Some((lock, UnixListener::bind(path.join("daemon.sock"))?)))
}

pub fn toggle() -> Result<(), Box<dyn std::error::Error>> {
    let path = directory()?.join("daemon.sock");
    let mut stream = match UnixStream::connect(&path) {
        Ok(stream) => stream,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            start(&path)?
        }
        Err(error) => return Err(error.into()),
    };
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(b"t")?;
    let mut reply = [0];
    stream.read_exact(&mut reply)?;
    if reply != *b"1" {
        return Err("daemon rejected toggle".into());
    }
    Ok(())
}

fn start(path: &Path) -> io::Result<UnixStream> {
    let mut child = Command::new(std::env::current_exe()?)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(stream) = UnixStream::connect(path) {
            return Ok(stream);
        }
        if Instant::now() >= deadline || child.try_wait()?.is_some_and(|status| !status.success()) {
            return Err(io::Error::other("daemon did not start"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn receive(listener: UnixListener, toggle: impl Fn(String) -> bool) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { break };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut command = [0];
        if stream.read_exact(&mut command).is_err() || command != *b"t" {
            continue;
        }
        match cursor_monitor() {
            Ok(monitor) => {
                if !toggle(monitor) {
                    break;
                }
                let _ = stream.write_all(b"1");
            }
            Err(error) => {
                eprintln!("orbit: {error}");
                let _ = stream.write_all(b"0");
            }
        }
    }
}

fn hyprland(command: &str) -> io::Result<String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR is not set"))?;
    let instance = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")
        .ok_or_else(|| io::Error::other("Hyprland is not running"))?;
    let mut stream = UnixStream::connect(
        PathBuf::from(runtime)
            .join("hypr")
            .join(instance)
            .join(".socket.sock"),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    stream.set_write_timeout(Some(Duration::from_millis(200)))?;
    stream.write_all(command.as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    Ok(reply)
}

fn cursor_monitor() -> Result<String, Box<dyn std::error::Error>> {
    let cursor: serde_json::Value = serde_json::from_str(&hyprland("j/cursorpos")?)?;
    let monitors: Vec<serde_json::Value> = serde_json::from_str(&hyprland("j/monitors")?)?;
    let x = cursor["x"].as_f64().ok_or("missing cursor x")?;
    let y = cursor["y"].as_f64().ok_or("missing cursor y")?;
    for monitor in monitors {
        let mx = monitor["x"].as_f64().ok_or("missing monitor x")?;
        let my = monitor["y"].as_f64().ok_or("missing monitor y")?;
        let scale = monitor["scale"].as_f64().ok_or("missing monitor scale")?;
        let mut width = monitor["width"].as_f64().ok_or("missing monitor width")? / scale;
        let mut height = monitor["height"].as_f64().ok_or("missing monitor height")? / scale;
        if monitor["transform"].as_u64().unwrap_or(0) % 2 == 1 {
            std::mem::swap(&mut width, &mut height);
        }
        if x >= mx && x < mx + width && y >= my && y < my + height {
            return Ok(monitor["name"]
                .as_str()
                .ok_or("missing monitor name")?
                .into());
        }
    }
    Err("no monitor at cursor".into())
}

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    slint::set_xdg_app_id("orbit")?;
    Ok(())
}

pub fn show_or_hide(
    launcher: &crate::Launcher,
    monitor: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if launcher.get_launcher_visible() {
        return hide(launcher);
    }
    launcher.invoke_reset_launcher();
    if launcher.window().is_visible() {
        fullscreen(launcher.window(), monitor);
        restore(monitor)?;
        launcher.set_monitor_name("".into());
        focus_after_show(launcher);
    } else {
        launcher.set_monitor_name(monitor.into());
        launcher.window().show()?;
    }
    launcher.set_launcher_visible(true);
    launcher.window().request_redraw();
    Ok(())
}

fn restore(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let monitors: Vec<serde_json::Value> = serde_json::from_str(&hyprland("j/monitors")?)?;
    let monitor = monitors
        .iter()
        .find(|monitor| monitor["name"].as_str() == Some(name))
        .ok_or("monitor disappeared")?;
    let (workspace, legacy_workspace) =
        if monitor["specialWorkspace"]["id"].as_i64().unwrap_or(0) != 0 {
            let name = monitor["specialWorkspace"]["name"]
                .as_str()
                .ok_or("missing workspace")?;
            (serde_json::to_string(name)?, format!("special:{name}"))
        } else {
            let name = monitor["activeWorkspace"]["id"]
                .as_i64()
                .ok_or("missing workspace")?
                .to_string();
            (name.clone(), name)
        };
    dispatch(
        &format!(
            "hl.dsp.window.move({{workspace = {workspace}, follow = false, window = \"pid:{}\"}})",
            std::process::id()
        ),
        &format!(
            "movetoworkspacesilent {legacy_workspace},pid:{}",
            std::process::id()
        ),
    )?;
    Ok(())
}

pub fn hide(launcher: &crate::Launcher) -> Result<(), Box<dyn std::error::Error>> {
    if launcher.get_launcher_visible() {
        // Keeping the surface and buffers avoids Wayland window creation on every toggle.
        dispatch(
            &format!(
                "hl.dsp.window.move({{workspace = \"special:orbit\", follow = false, window = \"pid:{}\"}})",
                std::process::id()
            ),
            &format!(
                "movetoworkspacesilent special:orbit,pid:{}",
                std::process::id()
            ),
        )?;
        launcher.set_launcher_visible(false);
    }
    Ok(())
}

fn dispatch(command: &str, legacy: &str) -> io::Result<()> {
    let reply = hyprland(&format!("dispatch {command}"))?;
    if reply.trim() == "ok" {
        return Ok(());
    }
    let reply = hyprland(&format!("dispatch {legacy}"))?;
    if reply.trim() != "ok" {
        return Err(io::Error::other(reply));
    }
    Ok(())
}

fn focus_launcher() -> io::Result<()> {
    let focus = hyprland(&format!(
        "dispatch hl.dsp.focus({{window = \"pid:{}\"}})",
        std::process::id()
    ))?;
    let zorder = hyprland(&format!(
        "dispatch hl.dsp.window.alter_zorder({{mode = \"top\", window = \"pid:{}\"}})",
        std::process::id()
    ))?;
    if zorder.trim() != "ok" {
        return Err(io::Error::other(zorder));
    }
    if focus.trim() != "ok" {
        return Err(io::Error::other(focus));
    }
    Ok(())
}

fn fullscreen(window: &slint::Window, name: &str) {
    window.with_winit_window(|native| {
        let monitor = native
            .available_monitors()
            .find(|item| item.name().as_deref() == Some(name));
        native.set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor)));
    });
}

fn focus_after_show(launcher: &crate::Launcher) {
    let weak = launcher.as_weak();
    slint::Timer::single_shot(Duration::from_millis(60), move || {
        if let Some(launcher) = weak.upgrade()
            && launcher.get_launcher_visible()
            && let Err(error) = focus_launcher()
        {
            eprintln!("orbit: {error}");
        }
    });
}

pub fn hide_on_input(launcher: &crate::Launcher) {
    let weak = launcher.as_weak();
    let focus_generation = Rc::new(Cell::new(0_u64));
    let mut fullscreen_monitor = String::new();
    let mut control = false;
    launcher
        .window()
        .on_winit_window_event(move |window, event| {
            let Some(launcher) = weak.upgrade() else {
                return EventResult::Propagate;
            };
            if matches!(event, WindowEvent::RedrawRequested)
                && !launcher.get_monitor_name().is_empty()
            {
                let monitor = launcher.get_monitor_name().to_string();
                if fullscreen_monitor != monitor {
                    fullscreen(window, &monitor);
                    fullscreen_monitor = monitor.clone();
                }
                // The redraw path is the single place that restores the resident window.
                if let Err(error) = restore(&monitor) {
                    eprintln!("orbit: {error}");
                }
                launcher.set_monitor_name("".into());
                focus_after_show(&launcher);
            }
            match event {
                WindowEvent::ModifiersChanged(modifiers) => {
                    control = modifiers.state().control_key();
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed
                        && control
                        && event.logical_key == Key::Character("k".into()) =>
                {
                    launcher.invoke_focus_search();
                    return EventResult::PreventDefault;
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed
                        && matches!(
                            event.logical_key,
                            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight)
                        ) =>
                {
                    // Text fields consume arrows; page keys run only when no field has focus.
                    if !launcher.get_search_focused() {
                        launcher.invoke_page_key(
                            if event.logical_key == Key::Named(NamedKey::ArrowLeft) {
                                -1
                            } else {
                                1
                            },
                        );
                    }
                }
                WindowEvent::Focused(active) => {
                    let generation = focus_generation.get().wrapping_add(1);
                    focus_generation.set(generation);
                    if !active {
                        let focus_generation = focus_generation.clone();
                        let weak = weak.clone();
                        slint::Timer::single_shot(Duration::from_millis(120), move || {
                            if focus_generation.get() == generation
                                && let Some(launcher) = weak.upgrade()
                                && launcher.get_launcher_visible()
                                && let Err(error) = self::hide(&launcher)
                            {
                                eprintln!("orbit: {error}");
                            }
                        });
                    }
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed
                        && event.logical_key == Key::Named(NamedKey::Escape) =>
                {
                    launcher.invoke_close_top_layer();
                    return EventResult::PreventDefault;
                }
                _ => (),
            }
            EventResult::Propagate
        });
}

pub fn launch(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let args = shlex::split(text).ok_or("Command has an unclosed quote")?;
    let (program, args) = args.split_first().ok_or("Command is empty")?;
    detached(Command::new(program).args(args))?;
    Ok(())
}

pub fn launch_action(action: &SystemAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        SystemAction::LockScreen => detached(Command::new("loginctl").arg("lock-session"))?,
        SystemAction::Screenshot => {
            let directory = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or("HOME is not set")?
                .join("Pictures/Orbit");
            fs::create_dir_all(&directory)?;
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            detached(Command::new("grim").arg(directory.join(format!("Screenshot-{time}.png"))))?;
        }
        SystemAction::Sleep => detached(Command::new("systemctl").arg("suspend"))?,
        SystemAction::LogOut => detached(Command::new("hyprctl").args(["dispatch", "exit"]))?,
    }
    Ok(())
}

pub fn open(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    detached(Command::new("xdg-open").arg(address))?;
    Ok(())
}

fn detached(command: &mut Command) -> io::Result<()> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    // Reap the child without tying its lifetime to the daemon.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
