use crate::{Launcher, ShortcutEntry};
use serde::{Deserialize, Serialize};
use slint::winit_030::winit::keyboard::{Key, ModifiersState, NamedKey};
use slint::{ComponentHandle, Model};

#[derive(Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub action: i32,
    pub keys: String,
}

pub fn defaults() -> Vec<Shortcut> {
    [
        "Ctrl+K",
        "Escape",
        "Left",
        "Right",
        "Up",
        "Down",
        "Enter",
        "Tab",
        "Shift+Tab",
    ]
    .iter()
    .enumerate()
    .map(|(action, keys)| Shortcut {
        action: action as i32,
        keys: (*keys).into(),
    })
    .chain(std::iter::once(Shortcut {
        action: 12,
        keys: "Ctrl+W".into(),
    }))
    .chain(std::iter::once(Shortcut {
        action: 13,
        keys: "Ctrl+E".into(),
    }))
    .collect()
}

pub fn valid(bindings: &[Shortcut]) -> bool {
    bindings.iter().enumerate().all(|(index, binding)| {
        (0..14).contains(&binding.action)
            && !binding.keys.is_empty()
            && binding.keys.len() < 80
            && !bindings[..index]
                .iter()
                .any(|other| other.keys == binding.keys)
    })
}

pub fn model(bindings: Vec<Shortcut>) -> slint::ModelRc<ShortcutEntry> {
    std::rc::Rc::new(slint::VecModel::from(
        bindings
            .into_iter()
            .map(|binding| ShortcutEntry {
                action: binding.action,
                keys: binding.keys.into(),
            })
            .collect::<Vec<_>>(),
    ))
    .into()
}

fn chord(key: &Key, modifiers: ModifiersState) -> Option<String> {
    let key = match key {
        Key::Character(text) if text == " " => "Space".to_owned(),
        Key::Character(text) => text.to_uppercase(),
        Key::Named(NamedKey::Control | NamedKey::Alt | NamedKey::Shift | NamedKey::Super) => {
            return None;
        }
        Key::Named(named) => match named {
            NamedKey::ArrowLeft => "Left".into(),
            NamedKey::ArrowRight => "Right".into(),
            NamedKey::ArrowUp => "Up".into(),
            NamedKey::ArrowDown => "Down".into(),
            _ => format!("{named:?}"),
        },
        _ => return None,
    };
    let mut result = String::new();
    for (enabled, prefix) in [
        (modifiers.control_key(), "Ctrl+"),
        (modifiers.alt_key(), "Alt+"),
        (modifiers.shift_key(), "Shift+"),
        (modifiers.super_key(), "Super+"),
    ] {
        if enabled {
            result.push_str(prefix);
        }
    }
    result.push_str(&key);
    Some(result)
}

pub fn handle(launcher: &Launcher, key: &Key, modifiers: ModifiersState) -> bool {
    let Some(keys) = chord(key, modifiers) else {
        return false;
    };
    if launcher.get_shortcut_recording() {
        launcher.set_shortcut_draft(keys.into());
        launcher.set_shortcut_recording(false);
        return true;
    }
    if launcher.get_grid_navigation() && *key == Key::Named(NamedKey::F10) && modifiers.shift_key()
    {
        launcher.invoke_context_focused();
        return true;
    }
    if launcher.get_grid_navigation()
        && matches!(key, Key::Named(NamedKey::PageUp | NamedKey::PageDown))
    {
        launcher.invoke_page_key(if *key == Key::Named(NamedKey::PageUp) {
            -1
        } else {
            1
        });
        return true;
    }
    let action = launcher
        .get_shortcuts()
        .iter()
        .find(|binding| binding.keys == keys)
        .map(|binding| binding.action);
    if action.is_none()
        && launcher.get_search_editing()
        && modifiers.control_key()
        && matches!(key, Key::Character(text) if text.eq_ignore_ascii_case("z") || text.eq_ignore_ascii_case("y"))
    {
        let redo = modifiers.shift_key()
            || matches!(key, Key::Character(text) if text.eq_ignore_ascii_case("y"));
        crate::search_edit::history(launcher, redo);
        return true;
    }
    let Some(action) = action else {
        // Removed defaults must not fall through to native Slint shortcuts.
        return matches!(
            key,
            Key::Named(NamedKey::Enter | NamedKey::Tab | NamedKey::Escape)
        );
    };
    if launcher.get_busy() {
        return true;
    }
    match action {
        0 => launcher.invoke_focus_search(),
        1 => launcher.invoke_close_top_layer(),
        2..=5 => {
            if launcher.get_search_navigation() {
                launcher.invoke_move_search_selection(action - 2);
            } else if launcher.get_grid_navigation() {
                launcher.invoke_move_grid(action - 2);
            } else if action < 4 && !launcher.get_search_focused() {
                launcher.invoke_page_key(if action == 2 { -1 } else { 1 });
            } else {
                return false;
            }
        }
        6 if launcher.get_search_navigation() => launcher.invoke_run_activated_prefix(),
        6 if launcher.get_grid_navigation() => launcher.invoke_activate_grid(),
        7 if launcher.get_search_editing()
            && !launcher.get_english_completion().is_empty()
            && (!launcher.get_activated_prefix_id().is_empty()
                || launcher
                    .invoke_match_prefix(launcher.get_search_text())
                    .action_id
                    .is_empty()) =>
        {
            launcher.invoke_accept_next_word();
        }
        6..=8 => {
            let text = match action {
                6 => "\n",
                7 => "\t",
                _ => "\u{0019}",
            };
            launcher
                .window()
                .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.into() });
            launcher
                .window()
                .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: text.into() });
        }
        9 => launcher.set_settings(!launcher.get_settings()),
        10 if !launcher.get_layout_locked() => launcher.invoke_add_page(),
        11 if !launcher.get_layout_locked() => launcher.invoke_confirm_delete_page(),
        12 if launcher.get_search_editing() => delete_previous_word(launcher.window()),
        12 => return false,
        13 if launcher.get_search_editing() => launcher.invoke_accept_next_word(),
        13 => return false,
        _ => (),
    }
    true
}

fn delete_previous_word(window: &slint::Window) {
    use slint::private_unstable_api::re_exports::{TextChangeNotify, TextInput, WindowInner};
    let window = WindowInner::from_pub(window);
    let Some(item) = window.focus_item.borrow().upgrade() else {
        return;
    };
    let Some(input) = item.downcast::<TextInput>() else {
        return;
    };
    let input = input.as_pin_ref();
    let adapter = window.window_adapter();
    let (anchor, cursor) = input.selection_anchor_and_cursor();
    if anchor == cursor {
        if cursor == 0 {
            return;
        }
        // Let Slint find the previous word, then retain only the part before
        // the original cursor. Native deletion preserves undo and clipboard.
        input.set_selection_offsets(&adapter, &item, (cursor - 1) as i32, (cursor - 1) as i32);
        input.select_word(&adapter, &item);
        let (start, _) = input.selection_anchor_and_cursor();
        input.set_selection_offsets(&adapter, &item, start as i32, cursor as i32);
    }
    input.delete_selection(&adapter, &item, TextChangeNotify::TriggerCallbacks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::platform::WindowEvent;

    fn input(ui: &Launcher, text: &str) {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    }

    #[test]
    fn deletes_words_in_search_and_keeps_native_undo_and_rebinding() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Launcher::new().unwrap();
        ui.set_shortcuts(model(defaults()));
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        ui.invoke_focus_search();
        for (text, remaining) in [
            ("hello world", "hello "),
            ("hello عالم", "hello "),
            ("🙂 hello", "🙂 "),
        ] {
            ui.set_search_text("".into());
            input(&ui, text);
            ui.window().dispatch_event(WindowEvent::KeyPressed {
                text: "\u{0011}".into(),
            });
            assert!(handle(
                &ui,
                &Key::Character("w".into()),
                ModifiersState::CONTROL
            ));
            assert_eq!(ui.get_search_text(), remaining, "query: {text}");
            input(&ui, "z");
            assert_eq!(ui.get_search_text(), text);
            // Deleting a word must not change which modifier keys are held.
            ui.window().dispatch_event(WindowEvent::KeyReleased {
                text: "\u{0011}".into(),
            });
        }
        ui.set_settings(true);
        assert!(!handle(
            &ui,
            &Key::Character("w".into()),
            ModifiersState::CONTROL
        ));
        ui.set_settings(false);
        let mut bindings = defaults();
        bindings
            .iter_mut()
            .find(|entry| entry.action == 12)
            .unwrap()
            .keys = "Alt+W".into();
        ui.set_shortcuts(model(bindings));
        ui.set_search_text("".into());
        input(&ui, "one two");
        assert!(!handle(
            &ui,
            &Key::Character("w".into()),
            ModifiersState::CONTROL
        ));
        assert!(handle(
            &ui,
            &Key::Character("w".into()),
            ModifiersState::ALT
        ));
        assert_eq!(ui.get_search_text(), "one ");
    }

    #[test]
    fn previous_word_default_is_valid_and_can_be_removed() {
        let mut bindings = defaults();
        assert!(valid(&bindings));
        assert!(
            bindings
                .iter()
                .any(|entry| entry.action == 12 && entry.keys == "Ctrl+W")
        );
        bindings.retain(|entry| entry.action != 12);
        assert!(valid(&bindings));
        assert!(!valid(&[Shortcut {
            action: 14,
            keys: "Ctrl+W".into()
        }]));
    }
}
