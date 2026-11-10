#[path = "add-item.rs"]
mod add_item;
mod adhkar;
mod ai;
mod apps;
mod azan;
mod canvas;
#[path = "canvas-edit.rs"]
mod canvas_edit;
mod config;
mod daemon;
#[path = "english-assistance.rs"]
mod english_assistance;
mod icons;
#[path = "ink.rs"]
mod ink;
#[path = "internet-usage.rs"]
mod internet_usage;
#[path = "item-drop.rs"]
mod item_drop;
#[cfg(not(windows))]
mod platform;
#[cfg(windows)]
#[path = "platform-windows.rs"]
mod platform;
mod recent;
#[path = "robot-data.rs"]
pub mod robot_data;
#[path = "robot-native.rs"]
mod robot_native;
#[path = "robot-provider.rs"]
mod robot_provider;
#[path = "robot-runtime.rs"]
pub mod robot_runtime;
#[path = "robot-tools.rs"]
pub mod robot_tools;
#[path = "robot-workbench.rs"]
mod robot_workbench;
#[path = "search-cursor.rs"]
mod search_cursor;
#[path = "search-edit.rs"]
mod search_edit;
mod shortcuts;
mod spatial;
mod storage;
#[path = "t3-providers.rs"]
mod t3_providers;
#[path = "text-direction.rs"]
mod text_direction;
mod ticktick;
#[path = "ticktick-ui.rs"]
mod ticktick_ui;
mod toast;
#[path = "website-icons.rs"]
mod website_icons;
#[path = "widget-data.rs"]
mod widget_data;
#[path = "widget-data-ui.rs"]
mod widget_data_ui;
#[path = "widget-views.rs"]
mod widget_views;

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
