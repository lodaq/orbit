use crate::{Launcher, apps, platform};
use slint::ComponentHandle;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some((_lock, listener)) = platform::listen()? else {
        return Ok(());
    };
    platform::init()?;
    let launcher = Launcher::new()?;
    ctrlc::set_handler(|| {
        let _ = slint::quit_event_loop();
    })?;
    crate::toast::init(&launcher);
    let _english = crate::english_assistance::connect(&launcher);
    crate::ai::connect(&launcher);
    let _widgets = crate::widget_data_ui::connect(&launcher);
    apps::connect(&launcher);
    crate::robot_workbench::connect(&launcher);
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
        platform::receive(listener, move |name, ticktick| {
            let weak = weak.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(launcher) = weak.upgrade() {
                    match platform::show_or_hide(&launcher, &name, ticktick) {
                        Ok(()) if ticktick => launcher.invoke_activate_ticktick_prefix(),
                        Ok(()) => {}
                        Err(error) => eprintln!("orbit: {error}"),
                    }
                }
            })
            .is_ok()
        });
    });
    slint::run_event_loop_until_quit()?;
    Ok(())
}
