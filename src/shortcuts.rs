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
    .collect()
}

pub fn valid(bindings: &[Shortcut]) -> bool {
    bindings.iter().enumerate().all(|(index, binding)| {
        (0..12).contains(&binding.action)
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
        6 if launcher.get_search_navigation() => launcher.invoke_activate_search_selection(),
        6 if launcher.get_grid_navigation() => launcher.invoke_activate_grid(),
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
        10 => launcher.invoke_add_page(),
        11 => launcher.invoke_confirm_delete_page(),
        _ => (),
    }
    true
}
