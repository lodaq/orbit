use crate::Launcher;
use slint::ComponentHandle;
use std::cell::RefCell;

#[derive(Clone)]
struct Replacement {
    before: String,
    middle: String,
    after: String,
}
thread_local! {
    static UNDO: RefCell<Vec<Replacement>> = const { RefCell::new(Vec::new()) };
    static REDO: RefCell<Vec<Replacement>> = const { RefCell::new(Vec::new()) };
}

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
    let mut after = source.to_owned();
    after.replace_range(start..end, text);
    let mut middle = source.to_owned();
    middle.replace_range(start..end, "");
    let transaction = Replacement {
        before: source.to_owned(),
        middle,
        after,
    };
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
    if text.is_empty() {
        input.as_pin_ref().delete_selection(
            &inner.window_adapter(),
            &item,
            slint::private_unstable_api::re_exports::TextChangeNotify::TriggerCallbacks,
        );
    } else {
        // A shortcut may still hold Ctrl. IME commit inserts text through Slint's
        // native editing history without treating it as another keyboard shortcut.
        let mut key_event = slint::private_unstable_api::re_exports::KeyEvent::default();
        key_event.text = text.into();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::internal(
                i_slint_core::input::InternalKeyEvent {
                    key_event,
                    event_type: i_slint_core::input::KeyEventType::CommitComposition,
                    ..Default::default()
                },
            ));
    }
    if start < end && !text.is_empty() {
        UNDO.with(|stack| {
            let mut stack = stack.borrow_mut();
            if stack.len() == 32 {
                stack.remove(0);
            }
            stack.push(transaction);
        });
        REDO.with(|stack| stack.borrow_mut().clear());
    }
}

// Slint records selection replacement as removal plus insertion. Group those
// native records without replacing its history or clipboard behavior.
pub fn history(ui: &Launcher, redo: bool) {
    let inner = WindowInner::from_pub(ui.window());
    let Some(item) = inner.focus_item.borrow().upgrade() else {
        return;
    };
    let Some(input) = item.downcast::<TextInput>() else {
        return;
    };
    let input = input.as_pin_ref();
    let adapter = inner.window_adapter();
    let previous = ui.get_search_text().to_string();
    if redo {
        input.redo(&adapter, &item);
    } else {
        input.undo(&adapter, &item);
    }
    let stack = if redo { &REDO } else { &UNDO };
    let transaction = stack.with(|stack| stack.borrow().last().cloned());
    let Some(mut transaction) = transaction.filter(|t| ui.get_search_text() == t.middle) else {
        return;
    };
    if redo {
        input.redo(&adapter, &item);
    } else {
        input.undo(&adapter, &item);
    }
    let expected = if redo {
        &transaction.after
    } else {
        &transaction.before
    };
    if ui.get_search_text() != expected {
        if redo {
            input.undo(&adapter, &item);
        } else {
            input.redo(&adapter, &item);
        }
        return;
    }
    stack.with(|stack| {
        stack.borrow_mut().pop();
    });
    if !redo {
        transaction.after = previous;
    }
    let destination = if redo { &UNDO } else { &REDO };
    destination.with(|stack| stack.borrow_mut().push(transaction));
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
        history(&ui, false);
        assert_eq!(ui.get_search_text(), "I want improve English.");
        replace(&ui, "stale text", 0, 10, "incorrect");
        assert_eq!(ui.get_search_text(), "I want improve English.");
        ui.set_search_text("🙂 He go home.".into());
        replace(&ui, "🙂 He go home.", 8, 10, "goes");
        assert_eq!(ui.get_search_text(), "🙂 He goes home.");
        history(&ui, false);
        assert_eq!(ui.get_search_text(), "🙂 He go home.");
        replace(&ui, "🙂 He go home.", 8, 11, "");
        assert_eq!(ui.get_search_text(), "🙂 He home.");
        history(&ui, false);
        assert_eq!(ui.get_search_text(), "🙂 He go home.");
        ui.set_search_text("🙂 go home".into());
        replace(&ui, "🙂 go home", 1, 2, "wrong");
        assert_eq!(ui.get_search_text(), "🙂 go home");
    }
}
