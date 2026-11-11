use crate::{Launcher, ShortcutEntry};
use serde::{Deserialize, Serialize};
use slint::winit_030::winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
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
        "Ctrl+S",
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
    .chain(canvas_defaults())
    .collect()
}

pub fn canvas_defaults() -> Vec<Shortcut> {
    [
        (14, "Ctrl+Z"),
        (15, "Ctrl+Shift+Z"),
        (16, "Ctrl+X"),
        (17, "Ctrl+C"),
        (18, "Ctrl+V"),
        (19, "Ctrl+H"),
        (20, "Ctrl+J"),
        (20, "Ctrl+L"),
        (21, "1"),
        (22, "2"),
        (23, "3"),
        (24, "4"),
    ]
    .into_iter()
    .map(|(action, keys)| Shortcut {
        action,
        keys: keys.into(),
    })
    .collect()
}

pub fn valid(bindings: &[Shortcut]) -> bool {
    bindings.iter().enumerate().all(|(index, binding)| {
        matches!(binding.action, 0..=9 | 12..=24)
            && !binding.keys.is_empty()
            && binding.keys.len() < 80
            && !bindings[..index]
                .iter()
                .any(|other| other.keys == binding.keys)
    })
}

// Frame shortcuts carry a modifier so they never take a plain keystroke from search typing.
pub fn validate_frame_keys(
    keys: &str,
    id: &str,
    bindings: &[Shortcut],
    frames: &[crate::config::CanvasFrame],
) -> Result<(), String> {
    if keys.is_empty() {
        return Ok(());
    }
    if keys.len() >= 80 {
        return Err("Shortcut is too long".into());
    }
    let modified = keys
        .split('+')
        .any(|part| matches!(part, "Ctrl" | "Alt" | "Super"));
    if !modified {
        return Err("Use Ctrl, Alt or Super in a Frame shortcut".into());
    }
    if bindings.iter().any(|binding| binding.keys == keys) {
        return Err("Shortcut is already used by an action".into());
    }
    if frames
        .iter()
        .any(|frame| frame.id != id && frame.keys == keys)
    {
        return Err("Shortcut is already used by another Frame".into());
    }
    Ok(())
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

fn shortcut_key(key: &Key, physical: PhysicalKey, modifiers: ModifiersState) -> Key {
    if !(modifiers.control_key() || modifiers.alt_key() || modifiers.super_key())
        || !matches!(key, Key::Character(text) if !text.is_ascii())
    {
        return key.clone();
    }
    use KeyCode::*;
    let letters = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    letters
        .iter()
        .position(|code| physical == PhysicalKey::Code(*code))
        .map(|index| Key::Character(char::from(b'a' + index as u8).to_string().into()))
        .unwrap_or_else(|| key.clone())
}

pub fn handle_event(
    launcher: &Launcher,
    event: &slint::winit_030::winit::event::KeyEvent,
    modifiers: ModifiersState,
) -> bool {
    let key = shortcut_key(&event.logical_key, event.physical_key, modifiers);
    if handle(launcher, &key, modifiers) {
        return true;
    }
    // Native editing must receive the same letter as Orbit's shortcut matcher.
    if key != event.logical_key
        && launcher.get_search_editing()
        && let Key::Character(text) = key
    {
        launcher
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed {
                text: text.as_str().into(),
            });
        launcher
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased {
                text: text.as_str().into(),
            });
        return true;
    }
    false
}

pub fn handle(launcher: &Launcher, key: &Key, modifiers: ModifiersState) -> bool {
    let plain = !modifiers.control_key()
        && !modifiers.alt_key()
        && !modifiers.shift_key()
        && !modifiers.super_key();
    if launcher.get_search_editing()
        && plain
        && matches!(key, Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight))
    {
        return false;
    }
    if launcher.get_search_editing()
        && !launcher.get_script_active()
        && plain
        && let Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown) = key
    {
        launcher.invoke_navigate_search_history(if *key == Key::Named(NamedKey::ArrowUp) {
            -1
        } else {
            1
        });
        return true;
    }
    if launcher.get_grid_navigation()
        && let Key::Character(text) = key
        && matches!(text.as_str(), "+" | "-")
    {
        let factor = if text == "+" { 1.1 } else { 1.0 / 1.1 };
        launcher.invoke_zoom_canvas(factor);
        return true;
    }
    let Some(keys) = chord(key, modifiers) else {
        return false;
    };
    let list_focused = launcher
        .global::<crate::KeyboardNavigation>()
        .get_list_focused();
    if launcher.global::<crate::RobotWorkbench>().get_visible() {
        if launcher
            .get_shortcuts()
            .iter()
            .any(|binding| binding.action == 1 && binding.keys == keys)
        {
            launcher.invoke_close_top_layer();
            return true;
        }
        return false;
    }
    if launcher.global::<crate::InternetLimits>().get_visible()
        || !launcher
            .global::<crate::AzanSettings>()
            .get_widget_id()
            .is_empty()
    {
        if launcher
            .get_shortcuts()
            .iter()
            .any(|binding| binding.action == 1 && binding.keys == keys)
        {
            launcher.invoke_close_top_layer();
            return true;
        }
        return false;
    }
    if launcher.get_shortcut_recording() {
        launcher.set_shortcut_draft(keys.into());
        launcher.set_shortcut_recording(false);
        return true;
    }
    if launcher.get_frame_recording() {
        if keys == "Escape" {
            launcher.set_frame_keys_draft("".into());
        } else {
            launcher.set_frame_keys_draft(keys.into());
        }
        launcher.set_frame_recording(false);
        return true;
    }
    if launcher.get_english_active()
        && launcher.get_search_editing()
        && !launcher.get_busy()
        && launcher.get_english_source() == launcher.get_search_text()
    {
        let count = launcher.get_english_suggestions().row_count() as i32;
        if count > 0 {
            match keys.as_str() {
                "Ctrl+1" | "Ctrl+2" | "Ctrl+3" => {
                    let index = i32::from(keys.as_bytes()[5] - b'1');
                    if index < count {
                        launcher.invoke_english_apply(index);
                    }
                    return true;
                }
                "Ctrl+N" | "Ctrl+J" | "Ctrl+Down" | "Ctrl+P" | "Ctrl+K" | "Ctrl+Up" => {
                    let selected = launcher.get_english_suggestion_selection();
                    let down = matches!(keys.as_str(), "Ctrl+N" | "Ctrl+J" | "Ctrl+Down");
                    let next = if selected < 0 {
                        if down { 0 } else { count - 1 }
                    } else if down {
                        (selected + 1).min(count - 1)
                    } else {
                        (selected - 1).max(0)
                    };
                    launcher.set_english_suggestion_selection(next);
                    return true;
                }
                "Enter" if launcher.get_english_suggestion_selection() >= 0 => {
                    launcher.invoke_english_apply(launcher.get_english_suggestion_selection());
                    return true;
                }
                _ => (),
            }
        }
    }
    if list_focused
        && launcher.get_search_navigation()
        && !launcher.get_grid_navigation()
        && (launcher.get_script_active() && launcher.get_scripts().row_count() > 0
            || !launcher.get_script_active() && launcher.get_search_results().row_count() > 0)
    {
        match keys.as_str() {
            "Ctrl+N" | "Ctrl+J" | "Ctrl+Down" => {
                launcher.invoke_move_search_selection(3);
                return true;
            }
            "Ctrl+P" | "Ctrl+K" | "Ctrl+Up" => {
                launcher.invoke_move_search_selection(2);
                return true;
            }
            _ => (),
        }
    }
    if launcher.get_grid_navigation() && *key == Key::Named(NamedKey::F10) && modifiers.shift_key()
    {
        launcher.invoke_context_focused();
        return true;
    }
    if launcher.get_frame_shortcuts()
        && let Some(frame) = launcher
            .get_frames()
            .iter()
            .find(|frame| frame.keys == keys)
    {
        launcher.invoke_fit_frame(frame.id);
        return true;
    }
    let action = launcher
        .get_shortcuts()
        .iter()
        .find(|binding| binding.keys == keys)
        .map(|binding| binding.action);
    if action == Some(0) && keys == "Ctrl+K" && list_focused {
        return false;
    }
    if (action.is_none() || matches!(action, Some(14 | 15)))
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
        0 if keys == "Ctrl+K" && launcher.get_canvas_shortcuts() => launcher.invoke_cycle_frame(-1),
        0 => launcher.invoke_focus_search(),
        1 => launcher.invoke_close_top_layer(),
        14..=24 if !launcher.get_canvas_shortcuts() => return false,
        14..=18 => launcher.invoke_canvas_command(action - 14),
        19 => launcher.invoke_cycle_frame(-1),
        20 => launcher.invoke_cycle_frame(1),
        21..=24 => launcher.invoke_select_tool(action - 21),
        2..=5 => {
            if launcher.get_search_navigation() {
                launcher.invoke_move_search_selection(action - 2);
            } else if launcher.get_grid_navigation() {
                launcher.invoke_move_grid(action - 2);
            } else {
                return false;
            }
        }
        6 if launcher.get_ai_active() && launcher.get_search_focused() => {
            launcher.invoke_run_activated_prefix()
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
        12 if launcher.get_search_editing() => {
            if launcher.get_search_text().is_empty() {
                let text = slint::platform::Key::Backspace.into();
                launcher
                    .window()
                    .dispatch_event(slint::platform::WindowEvent::KeyPressed { text });
                launcher
                    .window()
                    .dispatch_event(slint::platform::WindowEvent::KeyReleased {
                        text: slint::platform::Key::Backspace.into(),
                    });
            } else {
                delete_previous_word(launcher.window());
            }
        }
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
    fn ctrl_s_toggles_settings() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Launcher::new().unwrap();
        ui.set_shortcuts(model(defaults()));
        ui.show().unwrap();
        assert!(!ui.get_settings());
        assert!(handle(
            &ui,
            &Key::Character("s".into()),
            ModifiersState::CONTROL
        ));
        assert!(ui.get_settings());
        assert!(handle(
            &ui,
            &Key::Character("s".into()),
            ModifiersState::CONTROL
        ));
        assert!(!ui.get_settings());
    }

    #[test]
    fn arabic_layout_keeps_modified_letter_shortcuts() {
        for (arabic, code, latin) in [
            ("ش", KeyCode::KeyA, "a"),
            ("ث", KeyCode::KeyE, "e"),
            ("ص", KeyCode::KeyW, "w"),
            ("ن", KeyCode::KeyK, "k"),
            ("ؤ", KeyCode::KeyC, "c"),
        ] {
            let key = Key::Character(arabic.into());
            assert_eq!(
                shortcut_key(&key, PhysicalKey::Code(code), ModifiersState::CONTROL),
                Key::Character(latin.into())
            );
            assert_eq!(
                shortcut_key(&key, PhysicalKey::Code(code), ModifiersState::empty()),
                key
            );
        }
        let key = Key::Character("q".into());
        assert_eq!(
            shortcut_key(
                &key,
                PhysicalKey::Code(KeyCode::KeyA),
                ModifiersState::CONTROL
            ),
            key
        );
    }

    #[test]
    fn deletes_words_in_search_and_keeps_native_undo_and_rebinding() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Launcher::new().unwrap();
        ui.on_text_is_rtl(|text| crate::text_direction::text_is_rtl(&text));
        ui.set_shortcuts(model(defaults()));
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        ui.invoke_focus_search();
        for (text, remaining) in [
            ("hello world", "hello "),
            ("hello عالم", "hello "),
            ("انا اسمي خالد مصباح", "انا اسمي خالد "),
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
        ui.set_search_text("".into());
        ui.invoke_activate_ticktick_prefix();
        assert!(!ui.get_activated_prefix_id().is_empty());
        assert!(handle(
            &ui,
            &Key::Character("w".into()),
            ModifiersState::CONTROL
        ));
        assert!(ui.get_activated_prefix_id().is_empty());
        assert!(ui.get_activated_prefix_name().is_empty());
        assert!(ui.get_search_focused());
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: "\u{0011}".into(),
        });
        let Key::Character(select_all) = shortcut_key(
            &Key::Character("ش".into()),
            PhysicalKey::Code(KeyCode::KeyA),
            ModifiersState::CONTROL,
        ) else {
            panic!("select all must be a letter")
        };
        input(&ui, select_all.as_str());
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: "\u{0011}".into(),
        });
        input(&ui, "عربي");
        assert_eq!(ui.get_search_text(), "عربي");
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
    fn suggestion_keys_select_apply_and_reset_without_submitting() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Launcher::new().unwrap();
        ui.set_shortcuts(model(defaults()));
        ui.set_english_suggestions_enabled(true);
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        ui.invoke_focus_search();
        input(&ui, "source");
        ui.set_english_source(ui.get_search_text());
        ui.set_english_suggestions(
            std::rc::Rc::new(slint::VecModel::from(vec![
                "one".into(),
                "two".into(),
                "three".into(),
            ]))
            .into(),
        );
        let applied = std::rc::Rc::new(std::cell::Cell::new(-1));
        let result = applied.clone();
        ui.on_english_apply(move |index| result.set(index));
        assert!(ui.get_english_active());
        assert_eq!(ui.get_english_suggestion_selection(), -1);
        for (key, expected) in [
            ("n", 0),
            ("j", 1),
            ("n", 2),
            ("j", 2),
            ("p", 1),
            ("k", 0),
            ("p", 0),
        ] {
            assert!(handle(
                &ui,
                &Key::Character(key.into()),
                ModifiersState::CONTROL
            ));
            assert_eq!(ui.get_english_suggestion_selection(), expected);
        }
        assert!(handle(
            &ui,
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty()
        ));
        assert_eq!(applied.get(), 0);
        for (key, expected) in [("1", 0), ("2", 1), ("3", 2)] {
            assert!(handle(
                &ui,
                &Key::Character(key.into()),
                ModifiersState::CONTROL
            ));
            assert_eq!(applied.get(), expected);
        }
        ui.set_english_suggestions(
            std::rc::Rc::new(slint::VecModel::from(vec!["new".into()])).into(),
        );
        ui.set_english_suggestion_selection(-1);
        assert!(handle(
            &ui,
            &Key::Character("p".into()),
            ModifiersState::CONTROL
        ));
        assert_eq!(ui.get_english_suggestion_selection(), 0);
        applied.set(-1);
        handle(&ui, &Key::Character("3".into()), ModifiersState::CONTROL);
        assert_eq!(applied.get(), -1);
        ui.set_english_source("stale".into());
        assert!(!handle(
            &ui,
            &Key::Character("n".into()),
            ModifiersState::CONTROL
        ));
        ui.set_settings(true);
        assert!(!handle(
            &ui,
            &Key::Character("j".into()),
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn retired_arrangement_actions_are_rejected_without_renumbering_search_actions() {
        for action in [10, 11] {
            assert!(!valid(&[Shortcut {
                action,
                keys: "Ctrl+P".into()
            }]));
        }
        assert!(valid(&[
            Shortcut {
                action: 12,
                keys: "Ctrl+W".into()
            },
            Shortcut {
                action: 13,
                keys: "Ctrl+E".into()
            }
        ]));
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
            action: 25,
            keys: "Ctrl+W".into()
        }]));
    }
}
