#[path = "add-item.rs"]
mod add_item;
mod apps;
mod config;
mod daemon;
mod icons;
#[path = "item-drop.rs"]
mod item_drop;
#[cfg(not(windows))]
mod platform;
#[cfg(windows)]
#[path = "platform-windows.rs"]
mod platform;
mod recent;
mod shortcuts;
mod spatial;
mod storage;
mod ticktick;
#[path = "ticktick-ui.rs"]
mod ticktick_ui;
mod toast;

slint::include_modules!();

fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [command] if command == "daemon" => daemon::run(),
        [command] if command == "toggle" => platform::toggle(),
        _ => Err("usage: orbit daemon | orbit toggle".into()),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orbit: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
