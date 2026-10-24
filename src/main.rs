#[path = "add-item.rs"]
mod add_item;
mod ai;
mod apps;
mod config;
mod daemon;
#[path = "english-assistance.rs"]
mod english_assistance;
mod icons;
#[path = "item-drop.rs"]
mod item_drop;
#[cfg(not(windows))]
mod platform;
#[cfg(windows)]
#[path = "platform-windows.rs"]
mod platform;
mod recent;
#[path = "search-cursor.rs"]
mod search_cursor;
#[path = "search-edit.rs"]
mod search_edit;
mod shortcuts;
mod spatial;
mod storage;
mod ticktick;
#[path = "ticktick-ui.rs"]
mod ticktick_ui;
mod toast;
#[path = "website-icons.rs"]
mod website_icons;

slint::include_modules!();

fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [command] if command == "daemon" => daemon::run(),
        [command] if command == "toggle" => platform::toggle(false),
        [command] if command == "ticktick" => platform::toggle(true),
        _ => Err("usage: orbit daemon | orbit toggle | orbit ticktick".into()),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orbit: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
