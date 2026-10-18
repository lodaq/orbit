mod add_item;
mod apps;
mod config;
mod daemon;
mod icons;
mod platform;
mod recent;

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
