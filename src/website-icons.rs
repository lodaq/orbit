use crate::{Launcher, apps, config::Icon, icons, platform};
use slint::ComponentHandle;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

type ImageData = (String, Vec<u8>);
pub type Cache = Arc<Mutex<Option<(String, Option<ImageData>)>>>;

#[derive(Default)]
pub struct Draft {
    pub value: String,
    pub automatic: bool,
    pub attempted: bool,
    pub fetched: Option<ImageData>,
}

// Chosen images already use content-addressed names; other saved Website files
// came from the automatic fetcher. This also preserves mode through folders.
pub fn automatic(value: &str) -> bool {
    match icons::parse(value) {
        Ok(Icon::Letter) => true,
        Ok(Icon::File(name)) => !name.starts_with("chosen-"),
        _ => false,
    }
}

pub fn connect(launcher: &Launcher) -> Cache {
    let cache = Cache::default();
    let shared = cache.clone();
    let sequence = Arc::new(AtomicU64::new(0));
    let weak = launcher.as_weak();
    launcher.on_website_icon_automatic(|value| automatic(&value));
    launcher.on_preview_website_icon(move |url| {
        let request = sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let sequence = sequence.clone();
        let cache = shared.clone();
        let session = weak.upgrade().map(|ui| ui.get_website_icon_session());
        let weak = weak.clone();
        std::thread::spawn(move || {
            let fetched = platform::fetch_icon(&url);
            if sequence.load(Ordering::Relaxed) != request {
                return;
            }
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if Some(ui.get_website_icon_session()) != session
                    || !ui.get_sheet()
                    || !ui.get_website_auto_icon()
                    || ui.get_website_preview_url() != url
                    || ui.get_url_or_path() != url
                    || sequence.load(Ordering::Relaxed) != request
                {
                    return;
                }
                ui.set_website_icon_loading(false);
                let (data, preview) = match fetched {
                    Some((data, preview)) => (Some(data), Some(preview)),
                    None => (None, None),
                };
                ui.set_website_icon_status(
                    if preview.is_some() {
                        "Fetched icon"
                    } else {
                        "Couldn't fetch an icon. Choose an icon, retry, or save with a letter icon."
                    }
                    .into(),
                );
                ui.set_icon_preview_present(preview.is_some());
                ui.set_icon_preview(preview.map(slint::Image::from_rgba8).unwrap_or_default());
                *cache.lock().unwrap() = Some((url.into(), data));
            });
        });
    });
    cache
}

pub fn draft(cache: &Cache, url: &str, value: String, automatic: bool) -> Draft {
    let cached = cache.lock().unwrap();
    let matching = cached
        .as_ref()
        .filter(|(cached, _)| cached == url && value.is_empty());
    Draft {
        value,
        automatic,
        attempted: matching.is_some(),
        fetched: matching.and_then(|(_, data)| data.clone()),
    }
}

pub fn save(draft: Draft, current: Option<&Icon>) -> Result<Icon, String> {
    if !draft.automatic {
        return icons::parse(&draft.value);
    }
    if let Some((extension, data)) = draft.fetched {
        let name = format!("{}.{extension}", apps::new_id()?);
        platform::write_icon(&name, &data).map_err(|error| error.to_string())?;
        return Ok(Icon::File(name));
    }
    Ok(current.cloned().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_icon_mode_survives_shortcut_conversion() {
        for icon in [Icon::Letter, Icon::File("123.png".into())] {
            assert!(automatic(&serde_json::to_string(&icon).unwrap()));
        }
        for icon in [
            Icon::Text("X".into()),
            Icon::Svg("<svg></svg>".into()),
            Icon::File("chosen-123.png".into()),
        ] {
            assert!(!automatic(&serde_json::to_string(&icon).unwrap()));
        }
    }
    #[test]
    fn drafts_only_reuse_the_matching_url() {
        let cache = Cache::default();
        *cache.lock().unwrap() = Some(("https://a.test".into(), Some(("png".into(), vec![1]))));
        assert!(
            draft(&cache, "https://b.test", String::new(), true)
                .fetched
                .is_none()
        );
        assert!(
            draft(&cache, "https://a.test", String::new(), true)
                .fetched
                .is_some()
        );
        assert!(
            save(
                Draft {
                    automatic: true,
                    ..Draft::default()
                },
                None
            )
            .unwrap()
                == Icon::Letter
        );
    }
}
