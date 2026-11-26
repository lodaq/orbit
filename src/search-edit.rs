use crate::Launcher;
use slint::ComponentHandle;
use slint::private_unstable_api::re_exports::{ItemRc, TextInput, WindowAdapter, WindowInner};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;

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

fn with_focused_input(
    ui: &Launcher,
    action: impl FnOnce(Pin<&TextInput>, &Rc<dyn WindowAdapter>, &ItemRc),
) {
    let inner = WindowInner::from_pub(ui.window());
    let Some(item) = inner.focus_item.borrow().upgrade() else {
        return;
    };
    let Some(input) = item.downcast::<TextInput>() else {
        return;
    };
    action(input.as_pin_ref(), &inner.window_adapter(), &item);
}

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
    with_focused_input(ui, |input, adapter, item| {
        input.set_selection_offsets(adapter, item, start as i32, end as i32);
        if text.is_empty() {
            input.delete_selection(
                adapter,
                item,
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
    });
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
    with_focused_input(ui, |input, adapter, item| {
        let previous = ui.get_search_text().to_string();
        if redo {
            input.redo(adapter, item);
        } else {
            input.undo(adapter, item);
        }
        let stack = if redo { &REDO } else { &UNDO };
        let transaction = stack.with(|stack| stack.borrow().last().cloned());
        let Some(mut transaction) = transaction.filter(|t| ui.get_search_text() == t.middle) else {
            return;
        };
        if redo {
            input.redo(adapter, item);
        } else {
            input.undo(adapter, item);
        }
        let expected = if redo {
            &transaction.after
        } else {
            &transaction.before
        };
        if ui.get_search_text() != expected {
            if redo {
                input.undo(adapter, item);
            } else {
                input.redo(adapter, item);
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
    });
}

pub fn paste(ui: &Launcher) {
    if ui.get_search_editing() {
        with_focused_input(ui, |input, adapter, item| input.paste(adapter, item));
    }
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
        key(&ui, "/al تقرير");
        replace(&ui, "/al تقرير", 1, 3, "Alpha");
        assert_eq!(ui.get_search_text(), "/Alpha تقرير");
        history(&ui, false);
        assert_eq!(ui.get_search_text(), "/al تقرير");
        history(&ui, true);
        assert_eq!(ui.get_search_text(), "/Alpha تقرير");
        replace(&ui, "stale text", 0, 10, "incorrect");
        assert_eq!(ui.get_search_text(), "/Alpha تقرير");
        replace(&ui, "/Alpha تقرير", 8, 9, "wrong");
        assert_eq!(ui.get_search_text(), "/Alpha تقرير");
    }
}
