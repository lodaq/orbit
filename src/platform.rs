use crate::config::{InstalledApp, SystemAction};
pub use crate::storage::*;
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

fn directory() -> io::Result<PathBuf> {
    let runtime = std::env::var_os("ORBIT_RUNTIME_DIRECTORY")
        .or_else(|| std::env::var_os("XDG_RUNTIME_DIR"))
        .ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR is not set"))?;
    let path = PathBuf::from(runtime).join("orbit");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&path)?;
    Ok(path)
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

pub fn validate_app_search_path(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_absolute()
        && (path.is_dir()
            || path
                .metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
}

pub fn scan_app_search_paths(paths: &[String]) -> io::Result<Vec<InstalledApp>> {
    let mut apps = std::collections::BTreeMap::new();
    for root in paths {
        let root = Path::new(root);
        let candidates = if root.is_dir() {
            fs::read_dir(root)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect::<Vec<_>>()
        } else {
            vec![root.to_path_buf()]
        };
        for path in candidates {
            if !path.is_file() || !validate_app_search_path(&path) {
                continue;
            }
            let id = path.to_string_lossy().into_owned();
            let name = if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("appimage"))
            {
                path.file_stem()
            } else {
                path.file_name()
            }
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
            let command = shlex::try_join([id.as_str()]).map_err(io::Error::other)?;
            apps.entry(id.clone()).or_insert(InstalledApp {
                id,
                name,
                command,
                icon: Default::default(),
            });
        }
    }
    Ok(apps.into_values().collect())
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
    let executable = Path::new(&words[0]);
    let available = if executable.is_absolute() {
        executable.is_file() && validate_app_search_path(executable)
    } else if executable.components().count() == 1 {
        std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|directory| {
                let path = directory.join(executable);
                path.is_file() && validate_app_search_path(&path)
            })
        })
    } else {
        false
    };
    if !available {
        return None;
    }
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
        PathBuf::from("/var/lib/flatpak/exports/share/icons"),
    ];
    let home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(home) = home {
        roots.insert(0, home.join("icons"));
        roots.push(home.join("flatpak/exports/share/icons"));
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
    if let Some(icon) = icons.get(&name.to_lowercase()) {
        return Some(icon.clone());
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

pub fn toggle(ticktick: bool) -> Result<(), Box<dyn std::error::Error>> {
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
    stream.write_all(if ticktick { b"k" } else { b"t" })?;
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

pub fn receive(listener: UnixListener, toggle: impl Fn(String, bool) -> bool) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { break };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut command = [0];
        if stream.read_exact(&mut command).is_err() || !matches!(command[0], b't' | b'k') {
            continue;
        }
        match cursor_monitor() {
            Ok(monitor) => {
                if !toggle(monitor, command == *b"k") {
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

pub struct FileChooserGuard {
    lua: bool,
}

impl Drop for FileChooserGuard {
    fn drop(&mut self) {
        let command = if self.lua {
            "eval hl.window_rule({name = \"orbit-file-chooser\", enabled = false})"
        } else {
            "keyword windowrule[orbit-file-chooser]:enable 0"
        };
        if let Err(error) = hyprland(command) {
            eprintln!("orbit file chooser: {error}");
        }
    }
}

pub fn float_file_chooser(title: &str) -> FileChooserGuard {
    // Apply placement before mapping so the chooser never tiles or clears Orbit's fullscreen.
    let title_match = serde_json::json!(format!("^{title}$")).to_string();
    let command = format!(
        "eval hl.window_rule({{name = \"orbit-file-chooser\", enabled = true, match = {{class = \"^xdg-desktop-portal-gtk$\", title = {title_match}}}, float = true, size = {{1100, 760}}, center = true, no_anim = true}})"
    );
    let lua = hyprland(&command).is_ok_and(|reply| reply.trim() == "ok");
    if !lua {
        for (field, value) in [
            ("match:class", "^xdg-desktop-portal-gtk$".to_owned()),
            ("match:title", format!("^{title}$")),
            ("float", "1".to_owned()),
            ("size", "1100 760".to_owned()),
            ("center", "1".to_owned()),
            ("no_anim", "1".to_owned()),
            ("enable", "1".to_owned()),
        ] {
            match hyprland(&format!(
                "keyword windowrule[orbit-file-chooser]:{field} {value}"
            )) {
                Ok(reply) if reply.trim() == "ok" => (),
                Ok(reply) => eprintln!("orbit file chooser: {reply}"),
                Err(error) => eprintln!("orbit file chooser: {error}"),
            }
        }
    }
    FileChooserGuard { lua }
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
    ticktick: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if launcher.get_launcher_visible() && !ticktick {
        return hide(launcher);
    }
    launcher.invoke_reset_launcher();
    if launcher.window().is_visible() {
        // The resident window stays fullscreen; requesting it again closes the special workspace.
        restore(monitor)?;
        fullscreen_launcher()?;
        launcher.set_monitor_name("".into());
        focus_after_show(launcher);
    } else {
        launcher.set_monitor_name(monitor.into());
        launcher.window().show()?;
    }
    launcher.set_launcher_visible(true);
    crate::toast::on_window_shown(launcher);
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
            (serde_json::to_string(name)?, name.to_owned())
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
        crate::toast::on_window_hidden(launcher);
    }
    Ok(())
}

pub fn restore_after_chooser(launcher: &crate::Launcher) -> Result<(), Box<dyn std::error::Error>> {
    let monitor = cursor_monitor()?;
    restore(&monitor)?;
    fullscreen_launcher()?;
    launcher.set_launcher_visible(true);
    crate::toast::on_window_shown(launcher);
    launcher.window().request_redraw();
    focus_after_show(launcher);
    Ok(())
}

fn fullscreen_launcher() -> io::Result<()> {
    let clients: Vec<serde_json::Value> = serde_json::from_str(&hyprland("j/clients")?)?;
    if clients.iter().any(|client| {
        client["pid"].as_u64() == Some(std::process::id().into())
            && client["fullscreen"].as_u64() == Some(2)
    }) {
        return Ok(());
    }
    let reply = hyprland(&format!(
        "dispatch hl.dsp.window.fullscreen({{mode = \"fullscreen\", action = \"toggle\", window = \"pid:{}\"}})",
        std::process::id()
    ))?;
    if reply.trim() == "ok" {
        Ok(())
    } else {
        Err(io::Error::other(reply))
    }
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
        {
            if let Err(error) = focus_launcher() {
                eprintln!("orbit: {error}");
            }
            if !launcher.get_settings() {
                launcher.invoke_focus_search();
            }
        }
    });
}

pub fn hide_on_input(launcher: &crate::Launcher) {
    let weak = launcher.as_weak();
    let focus_generation = Rc::new(Cell::new(0_u64));
    let mut fullscreen_monitor = String::new();
    let mut modifiers = winit::keyboard::ModifiersState::empty();
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
                // The redraw path is the single place that restores the resident window.
                if let Err(error) = restore(&monitor) {
                    eprintln!("orbit: {error}");
                }
                if fullscreen_monitor != monitor {
                    fullscreen(window, &monitor);
                    if let Err(error) = fullscreen_launcher() {
                        eprintln!("orbit: {error}");
                    }
                    fullscreen_monitor = monitor.clone();
                }
                launcher.set_monitor_name("".into());
                focus_after_show(&launcher);
            }
            match event {
                WindowEvent::MouseWheel {
                    phase: winit::event::TouchPhase::Started,
                    ..
                } => {
                    launcher.invoke_start_page_scroll();
                }
                WindowEvent::ModifiersChanged(state) => {
                    modifiers = state.state();
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed =>
                {
                    if crate::shortcuts::handle_event(&launcher, event, modifiers) {
                        return EventResult::PreventDefault;
                    }
                }
                WindowEvent::Focused(active) => {
                    let generation = focus_generation.get().wrapping_add(1);
                    focus_generation.set(generation);
                    if !active {
                        launcher.invoke_cancel_pointer();
                        let focus_generation = focus_generation.clone();
                        let weak = weak.clone();
                        slint::Timer::single_shot(Duration::from_millis(120), move || {
                            if focus_generation.get() == generation
                                && let Some(launcher) = weak.upgrade()
                                && launcher.get_launcher_visible()
                                && !launcher.get_data_busy()
                                && !launcher.get_icon_busy()
                                && !matches!(
                                    launcher.get_sheet_operation().as_str(),
                                    "choosing" | "choosing-background"
                                )
                                && let Err(error) = self::hide(&launcher)
                            {
                                eprintln!("orbit: {error}");
                            }
                        });
                    }
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

pub fn launch_shell_action(script: &str, argument: &str) -> Result<(), Box<dyn std::error::Error>> {
    detached(&mut shell_action_command(script, argument))?;
    Ok(())
}

fn shell_action_command(script: &str, argument: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(script)
        .arg("orbit-action")
        .arg(argument);
    command
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
        SystemAction::VolumeUp => {
            detached(Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"]))?
        }
        SystemAction::VolumeDown => {
            detached(Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"]))?
        }
        SystemAction::Mute => {
            detached(Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]))?
        }
        SystemAction::MuteMicrophone => {
            detached(Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"]))?
        }
        SystemAction::PlayPause => detached(Command::new("playerctl").arg("play-pause"))?,
        SystemAction::NextTrack => detached(Command::new("playerctl").arg("next"))?,
        SystemAction::PreviousTrack => detached(Command::new("playerctl").arg("previous"))?,
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

pub fn play_completion_sound() {
    play_sound(
        "task-complete.wav",
        include_bytes!("../ui/assets/task-complete.wav"),
    );
}

pub fn play_adhkar_sound(all_done: bool) {
    if all_done {
        play_sound(
            "adhkar-all-complete.wav",
            include_bytes!("../ui/assets/adhkar-all-complete.wav"),
        );
    } else {
        play_sound(
            "adhkar-complete.wav",
            include_bytes!("../ui/assets/adhkar-complete.wav"),
        );
    }
}

fn play_sound(name: &str, bytes: &[u8]) {
    let Ok(path) = config_path().map(|p| p.with_file_name(name)) else {
        return;
    };
    if !path.exists() && std::fs::write(&path, bytes).is_err() {
        return;
    }
    for player in ["pw-play", "paplay", "aplay"] {
        if Command::new(player)
            .arg(&path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
        {
            return;
        }
    }
}

#[cfg(test)]
mod shell_action_tests {
    use super::*;

    #[test]
    fn shell_action_query_is_one_literal_argument() {
        let marker = std::env::temp_dir().join(format!(
            "orbit-shell-argument-{}.marker",
            std::process::id()
        ));
        let _ = fs::remove_file(&marker);
        let argument = format!("$(touch '{}'); 'quoted' 雪", marker.display());
        let output = shell_action_command("printf '%s' \"$1\"", &argument)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), argument);
        assert!(!marker.exists(), "query text was interpreted as shell code");
    }
}

#[cfg(test)]
mod app_search_path_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn desktop_entries_skip_deleted_executables() {
        let root = std::env::temp_dir().join(format!("orbit-desktop-exec-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("Writing app.AppImage");
        let entry = root.join("writing.desktop");
        fs::write(&executable, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(
            &entry,
            format!(
                "[Desktop Entry]\nName=Writing\nExec=\"{}\" %F\n",
                executable.display()
            ),
        )
        .unwrap();
        let icons = Default::default();
        assert!(parse_desktop_entry(&entry, &icons).is_some());
        fs::remove_file(&executable).unwrap();
        assert!(parse_desktop_entry(&entry, &icons).is_none());
        fs::write(&entry, "[Desktop Entry]\nName=Shell\nExec=sh -c true\n").unwrap();
        assert!(parse_desktop_entry(&entry, &icons).is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn app_search_paths_include_executables_and_preserve_symlink_names() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("orbit-app-links-{}", std::process::id()));
        let bin = root.join("bin");
        fs::create_dir_all(bin.join("nested")).unwrap();
        let target = root.join("original");
        fs::write(&target, "#!/bin/sh\nprintf '%s' \"$0\"\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
        for name in ["Antigravity", "renamed.tool"] {
            symlink(&target, bin.join(name)).unwrap();
        }
        symlink(root.join("missing"), bin.join("broken")).unwrap();
        fs::write(bin.join("disabled"), "").unwrap();
        fs::copy(&target, bin.join("direct.sh")).unwrap();
        fs::copy(&target, bin.join("nested/hidden")).unwrap();
        let apps = scan_app_search_paths(&[bin.display().to_string()]).unwrap();
        assert_eq!(apps.len(), 3);
        for name in ["Antigravity", "renamed.tool", "direct.sh"] {
            let app = apps.iter().find(|app| app.name == name).unwrap();
            assert_eq!(app.id, bin.join(name).to_string_lossy());
            let words = shlex::split(&app.command).unwrap();
            let output = Command::new(&words[0]).output().unwrap();
            assert!(output.status.success());
            assert_eq!(String::from_utf8(output.stdout).unwrap(), app.id);
        }
        fs::remove_file(&target).unwrap();
        let apps = scan_app_search_paths(&[bin.display().to_string()]).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "direct.sh");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn app_search_paths_include_appimages_and_explicit_executables_once() {
        let root = std::env::temp_dir().join(format!("orbit-app-paths-{}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        for name in [
            "Editor's app.AppImage",
            "chrome-sandbox",
            "disabled.AppImage",
            "nested/hidden.AppImage",
        ] {
            let path = root.join(name);
            fs::write(&path, "").unwrap();
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(if name == "disabled.AppImage" {
                    0o644
                } else {
                    0o755
                }),
            )
            .unwrap();
        }
        let explicit = root.join("chrome-sandbox");
        let apps = scan_app_search_paths(&[
            root.display().to_string(),
            explicit.display().to_string(),
            explicit.display().to_string(),
        ])
        .unwrap();
        assert_eq!(apps.len(), 2);
        for app in apps {
            assert_eq!(shlex::split(&app.command).unwrap(), vec![app.id]);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
