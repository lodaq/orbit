use crate::{Launcher, add_item, apps, config::Icon, platform};
use slint::ComponentHandle;
use unicode_segmentation::UnicodeSegmentation;

pub fn first_letter(name: &str) -> String {
    name.graphemes(true).next().unwrap_or_default().into()
}

pub fn parse(value: &str) -> Result<Icon, String> {
    let icon = if value.is_empty() {
        Icon::Letter
    } else if value.trim_start().starts_with('<') {
        Icon::Svg(value.into())
    } else {
        serde_json::from_str(value).map_err(|_| "Couldn't read icon")?
    };
    match &icon {
        Icon::Text(text) if !valid_letter(text) => {
            return Err("Enter 1–2 characters or emojis.".into());
        }
        Icon::Svg(svg) if !add_item::valid_svg(svg) => {
            return Err("Enter valid, safe SVG markup.".into());
        }
        _ => {}
    }
    Ok(icon)
}

fn valid_letter(text: &str) -> bool {
    (1..=2).contains(&text.graphemes(true).count())
        && !text.chars().any(char::is_control)
        && !text.trim().is_empty()
}

pub fn connect(launcher: &Launcher) {
    let weak = launcher.as_weak();
    launcher.on_open_icon(move |value, name| {
        let Some(ui) = weak.upgrade() else { return };
        let icon = parse(&value).unwrap_or_default();
        ui.set_icon_session(ui.get_icon_session() + 1);
        ui.set_icon_error("".into());
        ui.set_icon_busy(false);
        ui.set_letter_draft(first_letter(&name).into());
        ui.set_svg_draft("".into());
        ui.set_icon_file_draft("".into());
        ui.set_icon_mode(
            match icon {
                Icon::Svg(svg) => {
                    ui.set_svg_draft(svg.into());
                    "SVG"
                }
                Icon::Text(text) => {
                    ui.set_letter_draft(text.into());
                    "Letter"
                }
                Icon::File(file) => {
                    ui.set_icon_file_draft(file.into());
                    "Choose icon"
                }
                Icon::Letter => "Letter",
            }
            .into(),
        );
        ui.set_picker(true);
    });
    choose_callback(launcher);
    save_callback(launcher);
}

fn choose_callback(launcher: &Launcher) {
    let weak = launcher.as_weak();
    launcher.on_choose_icon(move |session| {
        let weak = weak.clone();
        std::thread::spawn(move || {
            let path = platform::choose_icon();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if !ui.get_picker() || ui.get_icon_session() != session {
                    return;
                }
                ui.set_icon_busy(false);
                if let Some(path) = path {
                    ui.set_icon_file_draft(path.to_string_lossy().into_owned().into());
                    ui.set_icon_error("".into());
                }
            });
        });
    });
}

fn save_callback(launcher: &Launcher) {
    let weak = launcher.as_weak();
    launcher.on_save_icon(move |kind, value, session| {
        let weak = weak.clone();
        std::thread::spawn(move || {
            let result = prepare(&kind, &value);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if !ui.get_picker() || ui.get_icon_session() != session {
                    return;
                }
                ui.set_icon_busy(false);
                match result {
                    Ok((icon, preview)) => {
                        if ui.get_item_type() == "Website" {
                            ui.set_website_auto_icon(false);
                            ui.set_website_icon_loading(false);
                        }
                        ui.set_icon_value(serde_json::to_string(&icon).unwrap_or_default().into());
                        ui.set_icon_letter(match icon {
                            Icon::Text(text) => text.into(),
                            _ => "".into(),
                        });
                        ui.set_icon_preview_present(preview.is_some());
                        ui.set_icon_preview(
                            preview.map(slint::Image::from_rgba8).unwrap_or_default(),
                        );
                        ui.set_picker(false);
                    }
                    Err(error) => ui.set_icon_error(error.into()),
                }
            });
        });
    });
}

type Preview = Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>>;

fn prepare(kind: &str, value: &str) -> Result<(Icon, Preview), String> {
    match kind {
        "Letter" if valid_letter(value) => Ok((Icon::Text(value.into()), None)),
        "Letter" => Err("Enter 1–2 characters or emojis.".into()),
        "SVG" => {
            let icon = parse(&serde_json::to_string(&Icon::Svg(value.into())).unwrap_or_default())?;
            Ok((icon, Some(apps::decode_svg(value)?)))
        }
        "Choose icon" if !value.is_empty() => {
            let (name, data) = platform::read_chosen_icon(value)?;
            if name.ends_with(".svg")
                && !add_item::valid_svg(std::str::from_utf8(&data).unwrap_or_default())
            {
                return Err("Enter valid, safe SVG markup.".into());
            }
            let preview = apps::decode_icon(&data, &name)?;
            platform::write_icon(&name, &data).map_err(|error| error.to_string())?;
            Ok((Icon::File(name), Some(preview)))
        }
        _ => Err("Choose an icon first.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn letter_icons_count_visible_characters() {
        for text in ["A", "AB", "🎉", "👩🏽‍💻", "🇪🇬✨"] {
            assert!(valid_letter(text));
        }
        for text in ["", " ", "ABC", "🎉🎉🎉", "A\n"] {
            assert!(!valid_letter(text));
        }
    }
    #[test]
    fn icon_values_round_trip_without_losing_the_type() {
        for icon in [
            Icon::Letter,
            Icon::Text("🇪🇬✨".into()),
            Icon::File("chosen.png".into()),
        ] {
            let value = serde_json::to_string(&icon).unwrap();
            assert!(parse(&value).unwrap() == icon);
        }
    }
}
