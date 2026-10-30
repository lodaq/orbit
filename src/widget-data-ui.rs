use crate::{
    AppTile, AzanSettings, CompletedReference, Launcher, LocalWidgets, PageItems, WidgetDataEntry,
    WidgetView, widget_data as data, widget_views as widgets,
};
use slint::{ComponentHandle, Model};
use std::{path::Path, sync::mpsc};

struct Request {
    category: i32,
    search: String,
    page: i32,
    action: Action,
}
enum Action {
    Load,
    Save(String, String, String, String),
    Delete(String),
    RestoreWord(String),
}
enum Work {
    Initialize,
    Data(Request),
    Widgets(Vec<widgets::Spec>, Vec<String>),
    Refresh(String),
    Finish(String, String),
    Completed(Vec<(String, String, String, bool)>),
    Audio,
    Play(String),
    Minute,
    Adhkar(String, String, String),
    AzanSave(
        String,
        crate::azan::Settings,
        Option<crate::azan::Schedule>,
        i32,
    ),
}
struct Reply {
    collection: data::Collection,
    page: i32,
    summary: String,
    counts: Vec<i32>,
    storage: Vec<u64>,
}

pub fn connect(ui: &Launcher) -> slint::Timer {
    let (sender, receiver) = mpsc::channel::<Work>();
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let mut db = None;
        let mut specs = Vec::<widgets::Spec>::new();
        for work in receiver {
            if matches!(work, Work::Audio) {
                choose_audio(&weak);
                continue;
            }
            let (force, opened) = match &work {
                Work::Widgets(new, opened) => {
                    specs = new.clone();
                    (String::new(), opened.clone())
                }
                Work::Refresh(id) => (id.clone(), Vec::new()),
                _ => (String::new(), Vec::new()),
            };
            if specs.is_empty() && db.is_none() && matches!(work, Work::Widgets(..) | Work::Minute)
            {
                continue;
            }
            let result = (|| {
                let path = crate::storage::config_path()
                    .map_err(|e| e.to_string())?
                    .with_file_name("widgets.db");
                if db.is_none() {
                    let database = data::open(&path)?;
                    crate::adhkar::record_completed(
                        &database,
                        &chrono::Local::now().format("%Y-%m-%d").to_string(),
                    )?;
                    db = Some(database);
                }
                let database = db.as_mut().expect("opened database");
                let reply = match &work {
                    Work::Adhkar(id, expected, action) => {
                        let spec = specs
                            .iter()
                            .find(|s| s.id == *id && matches!(s.category, 9 | 10))
                            .ok_or("Widget no longer exists")?;
                        let sound =
                            crate::adhkar::action(database, id, spec.category, expected, action)?;
                        if sound != 0 {
                            std::thread::spawn(move || {
                                crate::platform::play_adhkar_sound(sound == 2)
                            });
                        }
                        None
                    }
                    Work::AzanSave(id, settings, schedule, _) => {
                        if !specs.iter().any(|s| s.id == *id && s.category == 6) {
                            return Err("Widget no longer exists".into());
                        }
                        crate::azan::save(database, id, settings, schedule.as_ref())?;
                        None
                    }
                    Work::Data(request) => Some(handle(database, &path, request)?),
                    Work::Finish(id, entry) => {
                        let spec = specs
                            .iter()
                            .find(|s| s.id == *id)
                            .ok_or("Widget no longer exists")?;
                        widgets::finish(database, spec, entry)?;
                        None
                    }
                    Work::Completed(items) => {
                        for (id, date, title, done) in items {
                            data::save_task_title(database, id, title)?;
                            if *done {
                                data::complete(database, date, id, true)?;
                            } else {
                                widgets::remove_completion(database, id)?;
                            }
                        }
                        None
                    }
                    Work::Play(id) => {
                        play(database, &specs, id, &path)?;
                        None
                    }
                    _ => None,
                };
                Ok::<_, String>((reply, widgets::views(database, &specs, &force, &opened)?))
            })();
            let changed = matches!(
                &work,
                Work::Data(Request {
                    action: Action::Save(..) | Action::Delete(_) | Action::RestoreWord(_),
                    ..
                })
            );
            let data_request = matches!(&work, Work::Data(_));
            let azan_save = match &work {
                Work::AzanSave(id, _, _, session) => Some((id.clone(), *session)),
                _ => None,
            };
            let failed_specs = specs.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if let Some((id, session)) = azan_save {
                    let settings = ui.global::<AzanSettings>();
                    if settings.get_widget_id() == id && settings.get_session() == session {
                        settings.set_busy(false);
                        match &result {
                            Ok(_) => {
                                settings.invoke_close();
                                crate::toast::success(&ui.as_weak(), "Azan settings saved");
                            }
                            Err(error) => settings.set_error(error.clone().into()),
                        }
                    }
                    if result.is_err() {
                        return;
                    }
                }
                match result {
                    Ok((reply, views)) => {
                        if let Some(reply) = reply {
                            update(&ui, Ok(reply), changed);
                        }
                        ui.global::<LocalWidgets>()
                            .set_views(model(views.into_iter().map(view_model).collect()));
                    }
                    Err(error) => {
                        if data_request {
                            update(&ui, Err(error.clone()), false);
                        } else {
                            crate::toast::error(&ui.as_weak(), error.clone());
                        }
                        let views = failed_specs
                            .into_iter()
                            .map(|s| WidgetView {
                                id: s.id.into(),
                                category: s.category,
                                content: error.clone().into(),
                                ..Default::default()
                            })
                            .collect();
                        ui.global::<LocalWidgets>().set_views(model(views));
                    }
                }
            });
        }
    });
    connect_azan(ui, &sender);
    connect_data(ui, &sender);
    connect_widgets(ui, &sender);
    let _ = sender.send(Work::Initialize);
    let tick = sender.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(60),
        move || {
            let _ = tick.send(Work::Minute);
        },
    );
    timer
}

fn model<T: Clone + 'static>(entries: Vec<T>) -> slint::ModelRc<T> {
    std::rc::Rc::new(slint::VecModel::from(entries)).into()
}
fn entry_model(entry: data::Entry, done: bool) -> WidgetDataEntry {
    WidgetDataEntry {
        preview: preview(&entry.content).into(),
        id: entry.id.into(),
        content: entry.content.into(),
        attribution: entry.attribution.into(),
        resource: entry.resource.into(),
        done: done || entry.learned,
    }
}
fn view_model(view: widgets::View) -> WidgetView {
    let (headline, content) =
        if matches!(view.category, 4 | 6 | 9 | 10) && (!view.done || view.category == 6) {
            view.content.split_once('\n').unwrap_or((&view.content, ""))
        } else {
            ("", view.content.as_str())
        };
    let rows: Vec<WidgetDataEntry> = if view.category == 8 {
        view.content
            .lines()
            .filter_map(|line| line.strip_prefix("✓ "))
            .map(|label| WidgetDataEntry {
                content: label.into(),
                done: true,
                ..Default::default()
            })
            .collect()
    } else {
        view.rows
            .into_iter()
            .map(|(entry, done)| entry_model(entry, done))
            .collect()
    };
    let completed = rows.iter().filter(|row| row.done).count() as i32;
    WidgetView {
        city: view.city.into(),
        country: view.country.into(),
        timezone: view.timezone.into(),
        position: view.position,
        total: view.total,
        required: view.required,
        remaining: view.remaining,
        headline: headline.into(),
        content: content.trim_start_matches('\n').into(),
        id: view.id.into(),
        category: view.category,
        details: view.details.into(),
        entry_id: view.entry_id.into(),
        done: view.done,
        completed,
        resource: view.resource.into(),
        rows: model(rows),
    }
}

fn connect_data(ui: &Launcher, sender: &mpsc::Sender<Work>) {
    ui.on_widget_data_characters(|text| text.chars().count() as i32);
    let weak = ui.as_weak();
    let send = sender.clone();
    ui.on_load_widget_data(move || {
        if let Some(ui) = weak.upgrade() {
            send_data(&ui, &send, Action::Load);
        }
    });
    let weak = ui.as_weak();
    let send = sender.clone();
    ui.on_save_widget_data(move |id, content, attribution, resource| {
        if let Some(ui) = weak.upgrade() {
            send_data(
                &ui,
                &send,
                Action::Save(
                    id.into(),
                    content.into(),
                    attribution.into(),
                    resource.into(),
                ),
            );
        }
    });
    let weak = ui.as_weak();
    let send = sender.clone();
    ui.on_delete_widget_data(move |id| {
        if let Some(ui) = weak.upgrade() {
            send_data(&ui, &send, Action::Delete(id.into()));
        }
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    ui.on_restore_widget_word(move |id| {
        if let Some(ui) = weak.upgrade() {
            send_data(&ui, &send, Action::RestoreWord(id.into()));
        }
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    ui.on_choose_widget_audio(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_widget_data_busy(true);
            if send.send(Work::Audio).is_err() {
                ui.set_widget_data_busy(false);
            }
        }
    });
}

fn connect_widgets(ui: &Launcher, sender: &mpsc::Sender<Work>) {
    ui.global::<LocalWidgets>().on_lookup(|id, views| {
        views
            .iter()
            .find(|v| v.id == id)
            .unwrap_or_else(|| WidgetView {
                content: "Loading local data…".into(),
                ..Default::default()
            })
    });
    let send = sender.clone();
    ui.on_load_local_widgets(move |tiles, pages, opened| {
        let specs = pages
            .iter()
            .flat_map(|page: PageItems| page.items.iter().collect::<Vec<_>>())
            .filter_map(|tile: AppTile| {
                widgets::category(tile.widget_kind)
                    .filter(|_| tile.widget)
                    .map(|category| widgets::Spec {
                        id: tile.id.into(),
                        category,
                    })
            })
            .collect();
        let opened = if opened {
            tiles
                .iter()
                .filter(|tile: &AppTile| tile.widget)
                .map(|tile| tile.id.to_string())
                .collect()
        } else {
            Vec::new()
        };
        let _ = send.send(Work::Widgets(specs, opened));
    });
    let send = sender.clone();
    ui.global::<LocalWidgets>().on_refresh(move |id| {
        let _ = send.send(Work::Refresh(id.into()));
    });
    let send = sender.clone();
    ui.global::<LocalWidgets>().on_finish(move |id, entry| {
        let _ = send.send(Work::Finish(id.into(), entry.into()));
    });
    let send = sender.clone();
    ui.global::<LocalWidgets>().on_play(move |id| {
        let _ = send.send(Work::Play(id.into()));
    });
    let send = sender.clone();
    ui.global::<LocalWidgets>()
        .on_adhkar(move |id, expected, action| {
            let _ = send.send(Work::Adhkar(id.into(), expected.into(), action.into()));
        });
    let send = sender.clone();
    ui.on_record_widget_completions(move |items| {
        let items = items
            .iter()
            .map(|item: CompletedReference| {
                (
                    item.id.into(),
                    item.date.into(),
                    item.title.into(),
                    item.done,
                )
            })
            .collect();
        let _ = send.send(Work::Completed(items));
    });
}

fn send_data(ui: &Launcher, sender: &mpsc::Sender<Work>, action: Action) {
    if ui.get_widget_data_busy() {
        return;
    }
    ui.set_widget_data_busy(true);
    if matches!(action, Action::Save(..)) {
        ui.set_widget_data_search("".into());
        ui.set_widget_data_page(0);
    }
    let request = Request {
        category: ui.get_widget_data_category(),
        search: ui.get_widget_data_search().into(),
        page: ui.get_widget_data_page(),
        action,
    };
    if sender.send(Work::Data(request)).is_err() {
        ui.set_widget_data_busy(false);
        ui.set_widget_data_error(
            "The data worker is unavailable. Restart Orbit to try again.".into(),
        );
    }
}

fn handle(db: &rusqlite::Connection, path: &Path, request: &Request) -> Result<Reply, String> {
    match &request.action {
        Action::Load => {}
        Action::RestoreWord(id) => {
            let entry = data::entry(db, 4, data::row_id(4, id)?)?;
            if !entry.learned {
                return Err("This word is already in the rotation.".into());
            }
            widgets::finish(
                db,
                &widgets::Spec {
                    id: "data-vocabulary".into(),
                    category: 4,
                },
                id,
            )?;
        }
        Action::Save(id, content, attribution, resource) => {
            data::validate(request.category, content, attribution, resource)?;
            let old = if request.category == 5 && !id.is_empty() {
                data::entry(db, 5, data::row_id(5, id)?)?.resource
            } else {
                String::new()
            };
            let resource = if request.category == 5 {
                import_audio(resource, path)?
            } else {
                resource.clone()
            };
            if let Err(error) =
                data::save(db, request.category, id, content, attribution, &resource)
            {
                if resource != old && request.category == 5 {
                    cleanup_audio(db, path, &resource);
                }
                return Err(error);
            }
            if request.category == 5 && old != resource {
                cleanup_audio(db, path, &old);
            }
        }
        Action::Delete(id) => {
            let old = if request.category == 5 {
                data::entry(db, 5, data::row_id(5, id)?)?.resource
            } else {
                String::new()
            };
            data::delete(db, request.category, id)?;
            if request.category == 5 {
                cleanup_audio(db, path, &old);
            }
        }
    }
    let mut collection = data::collection(db, request.category, &request.search, request.page)?;
    let page = request
        .page
        .min((collection.count - 1).max(0) / data::PAGE_SIZE)
        .max(0);
    if page != request.page {
        collection = data::collection(db, request.category, &request.search, page)?;
    }
    let storage =
        storage_sizes(path.parent().expect("Orbit data folder")).unwrap_or_else(|_| vec![0; 4]);
    let summary = format!("{} used", data::size(storage.iter().sum()));
    let counts = data::TABLES
        .iter()
        .map(|table| {
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<i32>, _>>()?;
    Ok(Reply {
        collection,
        page,
        summary,
        counts,
        storage,
    })
}

fn update(ui: &Launcher, result: Result<Reply, String>, changed: bool) {
    ui.set_widget_data_busy(false);
    match result {
        Ok(Reply {
            collection,
            page,
            summary,
            counts,
            storage,
        }) => {
            ui.set_widget_data_counts(model(counts));
            let total = storage.iter().sum::<u64>().max(1) as f32;
            ui.set_widget_data_storage_fractions(model(
                storage.iter().map(|n| *n as f32 / total).collect(),
            ));
            ui.set_widget_data_storage_labels(model(
                ["Database", "Audio", "Icons", "Config and other"]
                    .iter()
                    .zip(&storage)
                    .map(|(name, bytes)| format!("{name} {}", data::size(*bytes)).into())
                    .collect(),
            ));
            ui.set_widget_data_count(collection.count);
            ui.set_widget_data_page(page);
            ui.set_widget_data_summary(summary.into());
            ui.set_widget_data_status(
                format!(
                    "{} matching entries · {} of text in this category",
                    collection.count,
                    data::size(collection.bytes)
                )
                .into(),
            );
            ui.set_widget_data_entries(model(
                collection
                    .entries
                    .into_iter()
                    .map(|e| entry_model(e, false))
                    .collect(),
            ));
            if changed {
                ui.set_widget_data_editing(false);
                ui.set_widget_data_delete_id("".into());
                crate::toast::success(&ui.as_weak(), "Saved data updated");
            }
            ui.set_widget_data_error("".into());
        }
        Err(e) => ui.set_widget_data_error(format!("Couldn't update widget data: {e}").into()),
    }
}

fn choose_audio(weak: &slint::Weak<Launcher>) {
    let _guard = crate::platform::float_data_chooser("Choose Quran audio");
    let path = rfd::FileDialog::new()
        .set_title("Choose Quran audio")
        .add_filter("Audio", &["mp3", "ogg", "wav", "flac", "m4a"])
        .pick_file();
    let _ = weak.upgrade_in_event_loop(move |ui| {
        ui.set_widget_data_busy(false);
        if let Some(path) = path {
            ui.set_widget_data_resource(path.to_string_lossy().as_ref().into());
        }
        let _ = crate::platform::restore_after_chooser(&ui);
    });
}

fn import_audio(resource: &str, database: &Path) -> Result<String, String> {
    if resource.is_empty() {
        return Ok(String::new());
    }
    let root = database.parent().expect("Orbit folder");
    let given = Path::new(resource);
    if !given.is_absolute()
        && resource.starts_with("audio/")
        && !given
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Ok(resource.into());
    }
    if !given.is_file() {
        return Err("Choose a local audio file.".into());
    }
    let extension = given.extension().and_then(|v| v.to_str()).unwrap_or("");
    if !["mp3", "ogg", "wav", "flac", "m4a"]
        .iter()
        .any(|v| extension.eq_ignore_ascii_case(v))
    {
        return Err("Choose a supported audio file.".into());
    }
    std::fs::create_dir_all(root.join("audio")).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let relative = format!("audio/{stamp}.{extension}");
    std::fs::copy(given, root.join(&relative)).map_err(|e| e.to_string())?;
    Ok(relative)
}

fn play(
    db: &rusqlite::Connection,
    specs: &[widgets::Spec],
    id: &str,
    database: &Path,
) -> Result<(), String> {
    let spec = specs
        .iter()
        .find(|s| s.id == id && s.category == 5)
        .ok_or("Quran widget no longer exists")?;
    let views = widgets::views(db, std::slice::from_ref(spec), "", &[])?;
    let resource = &views[0].resource;
    if resource.is_empty() {
        return Err("Add a local recitation in Settings → Data.".into());
    }
    let root = database
        .parent()
        .expect("Orbit folder")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let path = root
        .join(resource)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(root.join("audio")) {
        return Err("Audio must be stored in Orbit's audio folder.".into());
    }
    crate::platform::open(&path.to_string_lossy()).map_err(|e| e.to_string())
}

fn preview(content: &str) -> String {
    let mut chars = content.chars();
    let mut preview: String = chars.by_ref().take(180).collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

fn storage_sizes(folder: &Path) -> Result<Vec<u64>, String> {
    let total = data::folder_bytes(folder).map_err(|e| e.to_string())?;
    let audio = data::folder_bytes(&folder.join("audio")).map_err(|e| e.to_string())?;
    let icons = data::folder_bytes(&folder.join("icons")).map_err(|e| e.to_string())?;
    let database_size = folder
        .join("widgets.db")
        .metadata()
        .map_err(|e| e.to_string())?
        .len();
    Ok(vec![
        database_size,
        audio,
        icons,
        total.saturating_sub(database_size + audio + icons),
    ])
}

fn cleanup_audio(db: &rusqlite::Connection, database: &Path, relative: &str) {
    let file = Path::new(relative);
    if !relative.starts_with("audio/")
        || file
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return;
    }
    let references = db.query_row(
        "SELECT count(*) FROM quran_player WHERE audio_path=?1",
        [relative],
        |r| r.get::<_, i64>(0),
    );
    if matches!(references, Ok(0))
        && let Err(error) =
            std::fs::remove_file(database.parent().expect("Orbit folder").join(file))
        && error.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!("orbit: couldn't remove unused audio: {error}");
    }
}

fn azan_draft(ui: &Launcher) -> crate::azan::Settings {
    let draft = ui.global::<AzanSettings>();
    crate::azan::Settings {
        city: draft.get_city().trim().into(),
        country: draft.get_country().trim().into(),
        timezone: draft.get_timezone().trim().into(),
        schedule: None,
    }
}

fn connect_azan(ui: &Launcher, sender: &mpsc::Sender<Work>) {
    let weak = ui.as_weak();
    ui.global::<AzanSettings>().on_fetch(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let draft = ui.global::<AzanSettings>();
        if draft.get_busy() || draft.get_widget_id().is_empty() {
            return;
        }
        let settings = azan_draft(&ui);
        if let Err(error) = crate::azan::validate(&settings) {
            draft.set_error(error.into());
            return;
        }
        draft.invoke_invalidate();
        draft.set_busy(true);
        let id = draft.get_widget_id();
        let session = draft.get_session();
        let weak = ui.as_weak();
        std::thread::spawn(move || {
            let result = crate::azan::fetch(&settings);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                let draft = ui.global::<AzanSettings>();
                if draft.get_widget_id() != id || draft.get_session() != session {
                    return;
                }
                draft.set_busy(false);
                match result {
                    Ok(schedule) => {
                        draft.set_fetched_content(schedule.content.into());
                        draft.set_fetched_date(schedule.date.into());
                    }
                    Err(error) => draft.set_error(error.into()),
                }
            });
        });
    });
    let weak = ui.as_weak();
    let send = sender.clone();
    ui.global::<AzanSettings>().on_save(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let draft = ui.global::<AzanSettings>();
        if draft.get_busy() || draft.get_widget_id().is_empty() {
            return;
        }
        let settings = azan_draft(&ui);
        if let Err(error) = crate::azan::validate(&settings) {
            draft.set_error(error.into());
            return;
        }
        let schedule = (!draft.get_fetched_date().is_empty()).then(|| crate::azan::Schedule {
            date: draft.get_fetched_date().into(),
            content: draft.get_fetched_content().into(),
        });
        draft.set_busy(true);
        draft.set_error("".into());
        if send
            .send(Work::AzanSave(
                draft.get_widget_id().into(),
                settings,
                schedule,
                draft.get_session(),
            ))
            .is_err()
        {
            draft.set_busy(false);
            draft.set_error("The data worker is unavailable. Restart Orbit to try again.".into());
        }
    });
}
