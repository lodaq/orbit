use crate::{Launcher, apps, platform};
use slint::ComponentHandle;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some((_lock, listener)) = platform::listen()? else {
        return Ok(());
    };
    platform::init()?;
    let launcher = Launcher::new()?;
    crate::toast::init(&launcher);
    apps::connect(&launcher);
    let weak = launcher.as_weak();
    launcher.on_hide_launcher(move || {
        if let Some(launcher) = weak.upgrade()
            && let Err(error) = platform::hide(&launcher)
        {
            eprintln!("orbit: {error}");
        }
    });
    platform::hide_on_input(&launcher);
    let weak = launcher.as_weak();
    std::thread::spawn(move || {
        platform::receive(listener, move |name| {
            let weak = weak.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(launcher) = weak.upgrade()
                    && let Err(error) = platform::show_or_hide(&launcher, &name)
                {
                    eprintln!("orbit: {error}");
                }
            })
            .is_ok()
        });
    });
    slint::run_event_loop_until_quit()?;
    Ok(())
}
