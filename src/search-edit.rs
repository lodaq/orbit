use crate::Launcher;
use slint::ComponentHandle;
use slint::private_unstable_api::re_exports::{TextInput, WindowInner};

// Send replacements through native editing so Slint owns selection, callbacks and undo.
pub fn replace(ui: &Launcher, source: &str, start: usize, end: usize, text: &str) {
    if ui.get_search_text() != source
        || start > end
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return;
    }
    ui.invoke_focus_search();
    let inner = WindowInner::from_pub(ui.window());
    let Some(item) = inner.focus_item.borrow().upgrade() else {
        return;
    };
    let Some(input) = item.downcast::<TextInput>() else {
        return;
    };
    input.as_pin_ref().set_selection_offsets(
        &inner.window_adapter(),
        &item,
        start as i32,
        end as i32,
    );
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.into() });
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: text.into() });
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::platform::WindowEvent;

    fn key(ui: &Launcher, text: &str) {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    }

    #[test]
    fn replacement_preserves_native_undo_and_rejects_stale_text() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Launcher::new().unwrap();
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        ui.invoke_focus_search();
        key(&ui, "I want improve English.");
        replace(
            &ui,
            "I want improve English.",
            0,
            23,
            "I want to improve English.",
        );
        assert_eq!(ui.get_search_text(), "I want to improve English.");
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: "\u{0011}".into(),
        });
        key(&ui, "z");
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: "\u{0011}".into(),
        });
        assert_eq!(ui.get_search_text(), "I want improve English.");
        replace(&ui, "stale text", 0, 10, "incorrect");
        assert_eq!(ui.get_search_text(), "I want improve English.");
        ui.set_search_text("🙂 go home".into());
        replace(&ui, "🙂 go home", 1, 2, "wrong");
        assert_eq!(ui.get_search_text(), "🙂 go home");
    }
}
