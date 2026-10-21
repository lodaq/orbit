use crate::config::{Icon, InstalledApp, SystemAction};
pub use crate::storage::*;
use slint::ComponentHandle;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

const ADDRESS: &str = "127.0.0.1:47839";
pub fn listen() -> io::Result<Option<(File, TcpListener)>> {
    let listener = match TcpListener::bind(ADDRESS) {
        Ok(listener) => listener,
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => return Ok(None),
        Err(error) => return Err(error),
    };
    let path = config_path()?.with_extension("lock");
    std::fs::create_dir_all(path.parent().expect("config directory"))?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    Ok(Some((file, listener)))
}
pub fn toggle() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(mut stream) = TcpStream::connect(ADDRESS) {
        stream.write_all(b"toggle\n")?;
        return Ok(());
    }
    Command::new(std::env::current_exe()?)
        .arg("daemon")
        .creation_flags(0x08000000)
        .spawn()?;
    for _ in 0..200 {
        if let Ok(mut stream) = TcpStream::connect(ADDRESS) {
            stream.write_all(b"toggle\n")?;
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err("Orbit daemon did not start".into())
}
pub fn receive(listener: TcpListener, toggle: impl Fn(String) -> bool) {
    for mut stream in listener.incoming().flatten() {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut data = [0; 7];
        if stream.read_exact(&mut data).is_ok() && &data == b"toggle\n" && !toggle(String::new()) {
            break;
        }
    }
}
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    Ok(())
}
pub fn show_or_hide(launcher: &crate::Launcher, _: &str) -> Result<(), Box<dyn std::error::Error>> {
    if launcher.get_launcher_visible() {
        return hide(launcher);
    }
    launcher.invoke_reset_launcher();
    launcher.show()?;
    launcher.window().with_winit_window(|window| {
        window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
            window.current_monitor(),
        )));
        window.focus_window();
    });
    launcher.set_launcher_visible(true);
    crate::toast::on_window_shown(launcher);
    Ok(())
}
pub fn hide(launcher: &crate::Launcher) -> Result<(), Box<dyn std::error::Error>> {
    launcher.invoke_cancel_pointer();
    launcher.hide()?;
    launcher.set_launcher_visible(false);
    crate::toast::on_window_hidden(launcher);
    Ok(())
}
pub fn restore_after_chooser(launcher: &crate::Launcher) -> Result<(), Box<dyn std::error::Error>> {
    if !launcher.get_launcher_visible() {
        show_or_hide(launcher, "")?;
    }
    Ok(())
}
pub fn float_data_chooser(_: &str) {}
pub fn hide_on_input(launcher: &crate::Launcher) {
    let weak = launcher.as_weak();
    let mut modifiers = winit::keyboard::ModifiersState::empty();
    launcher.window().on_winit_window_event(move |_, event| {
        let Some(launcher) = weak.upgrade() else {
            return EventResult::Propagate;
        };
        match event {
            winit::event::WindowEvent::ModifiersChanged(state) => modifiers = state.state(),
            winit::event::WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Pressed =>
            {
                if crate::shortcuts::handle(&launcher, &event.logical_key, modifiers) {
                    return EventResult::PreventDefault;
                }
            }
            winit::event::WindowEvent::Focused(false) => {
                launcher.invoke_cancel_pointer();
                if !launcher.get_data_busy()
                    && !launcher.get_icon_busy()
                    && launcher.get_sheet_operation() != "choosing"
                {
                    let _ = hide(&launcher);
                }
            }
            _ => {}
        }
        EventResult::Propagate
    });
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        window: *mut std::ffi::c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
    fn CommandLineToArgvW(command: *const u16, count: *mut i32) -> *mut *mut u16;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn keybd_event(key: u8, scan: u8, flags: u32, extra: usize);
    fn LockWorkStation() -> i32;
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
pub fn open(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let address = wide(address);
    // ShellExecute uses the registered file/URL association without a command shell.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            address.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result <= 32 {
        return Err(format!("Windows could not open item ({result})").into());
    }
    Ok(())
}
pub fn launch(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = text.strip_prefix("@shell:") {
        return open(path);
    }
    let input = wide(text);
    let mut count = 0;
    // Windows parses its own quoting rules; the returned array must be freed once.
    let args = unsafe {
        let array = CommandLineToArgvW(input.as_ptr(), &mut count);
        if array.is_null() {
            return Err(io::Error::last_os_error().into());
        }
        let result: Vec<String> = (0..count as usize)
            .map(|index| {
                let arg = *array.add(index);
                let mut len = 0;
                while *arg.add(len) != 0 {
                    len += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(arg, len))
            })
            .collect();
        LocalFree(array.cast());
        result
    };
    let (program, args) = args.split_first().ok_or("Command is empty")?;
    Command::new(program)
        .args(args)
        .creation_flags(0x08000200)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn launch_shell_action(script: &str, argument: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "orbit-action-{}-{}.ps1",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(b"\xef\xbb\xbfparam([string]$OrbitQuery)\r\n")?;
    file.write_all(script.as_bytes())?;
    drop(file);
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&path)
        .arg(argument)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = std::fs::remove_file(path);
    });
    Ok(())
}

fn screenshot() -> Result<(), Box<dyn std::error::Error>> {
    let pictures = std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .ok_or("USERPROFILE is not set")?
        .join("Pictures/Orbit");
    std::fs::create_dir_all(&pictures)?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let path = pictures.join(format!("Screenshot-{time}.png"));
    let script = format!(
        "Add-Type -AssemblyName System.Drawing; Add-Type -AssemblyName System.Windows.Forms; $b=New-Object Drawing.Bitmap([Windows.Forms.Screen]::PrimaryScreen.Bounds.Width,[Windows.Forms.Screen]::PrimaryScreen.Bounds.Height); $g=[Drawing.Graphics]::FromImage($b); $g.CopyFromScreen(0,0,0,0,$b.Size); $b.Save('{}',[Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $b.Dispose()",
        path.display()
    );
    let encoded = base64(
        &script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
fn base64(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        for (index, value) in [
            a >> 2,
            ((a & 3) << 4) | (b >> 4),
            ((b & 15) << 2) | (c >> 6),
            c & 63,
        ]
        .into_iter()
        .enumerate()
        {
            out.push(if index >= chunk.len() + 1 {
                '='
            } else {
                CHARS[value as usize] as char
            });
        }
    }
    out
}

pub fn launch_action(action: &SystemAction) -> Result<(), Box<dyn std::error::Error>> {
    let key = match action {
        SystemAction::LockScreen => {
            // This OS function locks the interactive session.
            if unsafe { LockWorkStation() } == 0 {
                return Err(io::Error::last_os_error().into());
            }
            return Ok(());
        }
        SystemAction::LogOut => return launch("shutdown.exe /l"),
        SystemAction::Sleep => return launch("rundll32.exe powrprof.dll,SetSuspendState 0,1,0"),
        SystemAction::Screenshot => return screenshot(),
        SystemAction::VolumeUp => 0xaf,
        SystemAction::VolumeDown => 0xae,
        SystemAction::Mute => 0xad,
        SystemAction::PlayPause => 0xb3,
        SystemAction::NextTrack => 0xb0,
        SystemAction::PreviousTrack => 0xb1,
        SystemAction::MuteMicrophone => {
            return Err("Microphone mute is not available through Windows media keys".into());
        }
    };
    // Paired native media key events reach the user's active media session.
    unsafe {
        keybd_event(key, 0, 0, 0);
        keybd_event(key, 0, 2, 0);
    }
    Ok(())
}
pub fn scan_installed_apps() -> io::Result<Vec<InstalledApp>> {
    let script = r#"$shell=New-Object -ComObject Shell.Application; @($shell.Namespace('shell:AppsFolder').Items() | ForEach-Object { @{id=$_.Path;name=$_.Name} }) | ConvertTo-Json -Compress"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("Windows installed apps could not be read"));
    }
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let id = entry["id"].as_str()?;
            let name = entry["name"].as_str()?;
            Some(InstalledApp {
                id: id.into(),
                name: name.into(),
                command: format!("@shell:shell:AppsFolder\\{id}"),
                icon: Icon::Letter,
            })
        })
        .collect())
}

pub fn play_completion_sound() {
    let Ok(path) = config_path().map(|p| p.with_file_name("task-complete.wav")) else {
        return;
    };
    if !path.exists()
        && std::fs::write(&path, include_bytes!("../ui/assets/task-complete.wav")).is_err()
    {
        return;
    }
    let _ = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "& { param($p) (New-Object System.Media.SoundPlayer $p).PlaySync() }",
        ])
        .arg(path)
        .creation_flags(0x08000000)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
