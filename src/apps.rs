use crate::add_item::{Inventory, RequestSequence};
use crate::config::{
    ActionKind, App, Config, Icon, Item, Launch, Prefix, SystemAction, UserAction,
};
use crate::{
    ActionChoice, AppTile, CanvasFrameView, FolderPreview, InstalledAppTile, Launcher, add_item,
    config, platform, recent,
};
use crate::{PrefixChoice, PrefixMatch, UserActionChoice};
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};

const ACTIONS: [(&str, &str, SystemAction); 11] = [
    ("lock-screen", "Lock screen", SystemAction::LockScreen),
    ("screenshot", "Take screenshot", SystemAction::Screenshot),
    ("sleep", "Sleep", SystemAction::Sleep),
    ("log-out", "Log out", SystemAction::LogOut),
    ("volume-up", "Volume up", SystemAction::VolumeUp),
    ("volume-down", "Volume down", SystemAction::VolumeDown),
    ("mute", "Mute", SystemAction::Mute),
    (
        "mute-microphone",
        "Mute microphone",
        SystemAction::MuteMicrophone,
    ),
    ("play-pause", "Play or pause", SystemAction::PlayPause),
    ("next-track", "Next track", SystemAction::NextTrack),
    (
        "previous-track",
        "Previous track",
        SystemAction::PreviousTrack,
    ),
];

const RECENT_DOCK_LIMIT: usize = 5;

type UrlOpener<'a> = dyn Fn(&str) -> Result<(), Box<dyn std::error::Error>> + 'a;

#[derive(Clone, Default)]
struct ResultRef {
    id: String,
    source: i32,
    folder_id: String,
}

impl From<AppTile> for ResultRef {
    fn from(tile: AppTile) -> Self {
        Self {
            id: tile.id.into(),
            source: tile.source,
            folder_id: tile.folder_id.into(),
        }
    }
}

enum Action {
    Canvas(crate::canvas::Mutation),
    Widget(usize, i32, i32, bool),
    AddPosition(i32, i32),
    ShowAll(f32, f32),
    ChangeWidget(String, usize, usize),
    DuplicateWidget(String),
    SelectTickTickList(String, String),
    TransferData(bool),
    SaveAppearance(i32, i32, i32),
    SaveBackgroundFolder(String),
    ChooseBackgroundImage,
    ChangeBackground,
    SaveDefaultSearchEngine(i32),
    SaveSmoothCursor(bool),
    SaveEnglish(bool, bool, bool, String),
    SaveAi(crate::ai::Settings),
    SaveEnglishModel(String, String, String, String),
    SaveLayoutLocked(bool),
    SaveShortcut(i32, String, String),
    DeleteShortcut(i32, String),
    SaveFrame(String, String, String, Vec<String>),
    DeleteFrame(String),
    FitFrame(String, f32, f32),
    SaveAnnotation(String, String, i32, crate::ink::Style, i32, i32),
    SaveFontSize(i32),
    StyleSelection(Vec<String>, crate::ink::Style),
    AddInk(Vec<crate::config::InkPoint>, crate::ink::Style),
    EraseInk(Vec<crate::config::InkPoint>),
    EditCanvas(i32, Vec<String>, i32, i32),
    SaveScript(String, String, String),
    DeleteScript(String),
    ChooseScript,
    RunScript(String, String),
    Search(String),
    SearchWeb(String, String),
    RunPrefix(String, String),
    SaveNamedCommand(String, String, String, String, String, String),
    DeleteNamedCommand(String),
    SavePrefix(String, String, String, String),
    DeletePrefix(String),
    FilterInstalled(String),
    OpenResult(ResultRef),
    SaveSearchResult(ResultRef, i32, i32, bool),
    OpenDock(ResultRef),
    Pin(String),
    Unpin(String),
    RemoveRecent(String),
    Save(String, String, String, String, u64),
    SaveWebsite(String, String, String, crate::website_icons::Draft, u64),
    ChoosePath(bool, u64),
    SavePath(String, String, bool, String, u64),
    SaveFolder(String, String, u64),
    AddRobot(String, String),
    RenameRobot(String, String),
    Delete(String),
    Open(String),
    OpenFolder(String),
    CloseFolder,
    LaunchFolder(String, String),
    Drop(String, i32, i32, String),
    AddPredefined(String, u64),
    AddInstalledApp(String, u64),
    RefreshInstalledApps(u64),
    SaveAppSearchPath(String, bool),
    SaveSearchPath(bool, i32, String),
    DeleteSearchPath(bool, usize),
    AddFileSearchExclusion(String),
    RemoveFileSearchExclusion(String),
}

struct Apps {
    edits: crate::canvas_edit::History,
    config: Config,
    add_position: (i32, i32),
    icons: HashMap<String, SharedPixelBuffer<Rgba8Pixel>>,
    open_folder: Option<String>,
    pending_reveal: Option<String>,
    inventory: Inventory,
    installed_filter: String,
    query: String,
    search_paths: Option<Vec<SearchPathResult>>,
    recent: recent::Recent,
}

struct SearchPathResult {
    name: String,
    path: String,
    directory: bool,
}

impl Action {
    fn changes_layout(&self) -> bool {
        match self {
            Self::EditCanvas(command, ..) => *command != 3,
            Self::Widget(..)
            | Self::ChangeWidget(..)
            | Self::DuplicateWidget(..)
            | Self::SaveAppearance(..)
            | Self::Pin(..)
            | Self::Unpin(..)
            | Self::Delete(..)
            | Self::SaveSearchResult(..)
            | Self::Drop(..)
            | Self::SaveFrame(..)
            | Self::DeleteFrame(..)
            | Self::StyleSelection(..)
            | Self::SaveAnnotation(..)
            | Self::AddInk(..)
            | Self::EraseInk(..)
            | Self::AddRobot(..)
            | Self::AddPredefined(..)
            | Self::AddInstalledApp(..)
            | Self::TransferData(true) => true,
            Self::Save(id, ..)
            | Self::SaveWebsite(id, ..)
            | Self::SavePath(id, ..)
            | Self::SaveFolder(id, ..) => id.is_empty(),
            _ => false,
        }
    }

    fn request_id(&self) -> Option<u64> {
        match self {
            Self::Save(.., request)
            | Self::SaveWebsite(.., request)
            | Self::ChoosePath(.., request)
            | Self::SavePath(.., request)
            | Self::SaveFolder(.., request)
            | Self::AddPredefined(_, request)
            | Self::AddInstalledApp(_, request)
            | Self::RefreshInstalledApps(request) => Some(*request),
            _ => None,
        }
    }
}

pub fn connect(launcher: &Launcher) {
    let (send, receive) = mpsc::channel();
    let requests = Arc::new(RequestSequence::default());
    crate::spatial::connect(launcher);
    crate::ink::connect(launcher);
    let canvas_sender = send.clone();
    crate::canvas::connect(launcher, move |change| {
        let _ = canvas_sender.send(Action::Canvas(change));
    });
    callbacks(launcher, send, requests.clone());
    crate::icons::connect(launcher);
    crate::ticktick_ui::connect(launcher);
    let weak = launcher.as_weak();
    std::thread::spawn(move || {
        let (config, banner) = config::load();
        let (recent, recent_banner) = recent::Recent::load();
        let (inventory, inventory_banner) = Inventory::load();
        let mut apps = Apps {
            edits: Default::default(),
            add_position: (0, 0),
            config,
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory,
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent,
        };
        let banner = apps.load_icons().err().unwrap_or(banner);
        let banner = if banner.is_empty() {
            recent_banner
        } else {
            banner
        };
        let banner = if banner.is_empty() {
            inventory_banner
        } else {
            banner
        };
        let canvas = apps.config.canvas.clone();
        let _ = weak.upgrade_in_event_loop(move |ui| crate::canvas::sync(&ui, &canvas, true));
        apps.update(&weak);
        finish(&weak, banner, false, false, None, true);
        for action in receive {
            if let Action::Canvas(change) = action {
                let result = crate::canvas::apply(&mut apps.config, change);
                let canvas = apps.config.canvas.clone();
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    crate::canvas::sync(&launcher, &canvas, result.is_err());
                    if let Err(error) = result {
                        crate::toast::error(&launcher.as_weak(), &error);
                    }
                });
                continue;
            }
            let request = action.request_id();
            if request.is_some_and(|request| !requests.is_current(request)) {
                continue;
            }
            let recent_id = apps.recent_id(&action);
            let remove_recent_id = match &action {
                Action::RemoveRecent(id) => Some(id.clone()),
                _ => None,
            };
            let launching = recent_id.is_some()
                || matches!(action, Action::OpenResult(ref item) if item.source < 0)
                || matches!(action, Action::SearchWeb(..))
                || matches!(action, Action::RunPrefix(..) | Action::RunScript(..));
            let saving = matches!(
                action,
                Action::Pin(_)
                    | Action::SaveSearchResult(..)
                    | Action::Unpin(_)
                    | Action::Save(..)
                    | Action::SaveWebsite(..)
                    | Action::SavePath(..)
                    | Action::SaveFolder(..)
                    | Action::AddRobot(..)
                    | Action::RenameRobot(..)
                    | Action::Delete(_)
                    | Action::Drop(..)
                    | Action::AddPredefined(..)
                    | Action::AddInstalledApp(..)
                    | Action::SaveSearchPath(..)
                    | Action::DeleteSearchPath(..)
                    | Action::AddFileSearchExclusion(..)
                    | Action::RemoveFileSearchExclusion(..)
                    | Action::SaveSmoothCursor(..)
                    | Action::SaveDefaultSearchEngine(..)
                    | Action::SaveNamedCommand(..)
                    | Action::DeleteNamedCommand(..)
                    | Action::SavePrefix(..)
                    | Action::DeletePrefix(..)
                    | Action::SaveFrame(..)
                    | Action::DeleteFrame(..)
            );
            let save_search_result = matches!(action, Action::SaveSearchResult(..));
            let drag_drop = matches!(action, Action::Drop(..) | Action::SaveSearchResult(..));
            let save_shortcut = matches!(action, Action::SaveShortcut(..));
            let save_frame = matches!(action, Action::SaveFrame(..));
            let ink_action = matches!(action, Action::AddInk(..) | Action::EraseInk(..));
            let script_edit = matches!(action, Action::SaveScript(..) | Action::DeleteScript(..));
            let script_run = matches!(action, Action::RunScript(..));
            let save_action = matches!(
                action,
                Action::SaveNamedCommand(..) | Action::DeleteNamedCommand(..)
            );
            let add_exclusion = matches!(action, Action::AddFileSearchExclusion(..));
            let save_search_path = matches!(action, Action::SaveSearchPath(..));
            let reset_after_save = saving
                && !matches!(
                    action,
                    Action::Drop(..)
                        | Action::RenameRobot(..)
                        | Action::SaveSearchPath(..)
                        | Action::DeleteSearchPath(..)
                        | Action::AddFileSearchExclusion(..)
                        | Action::RemoveFileSearchExclusion(..)
                        | Action::SaveSmoothCursor(..)
                        | Action::SaveDefaultSearchEngine(..)
                        | Action::SaveNamedCommand(..)
                        | Action::DeleteNamedCommand(..)
                        | Action::SavePrefix(..)
                        | Action::DeletePrefix(..)
                        | Action::SaveFrame(..)
                        | Action::DeleteFrame(..)
                );
            let choosing = matches!(action, Action::ChoosePath(..) | Action::ChooseScript);
            let update = !choosing
                && (!launching || matches!(action, Action::OpenResult(..) | Action::OpenDock(..)));
            let refreshing = matches!(action, Action::RefreshInstalledApps(_));
            let filtering = matches!(action, Action::FilterInstalled(_));
            let searching = matches!(action, Action::Search(..));
            let appearance = match &action {
                Action::SaveAppearance(size, gap, speed) => Some((*size, *gap, *speed)),
                _ => None,
            };
            let reveal_added = match &action {
                Action::Save(id, ..)
                | Action::SaveWebsite(id, ..)
                | Action::SavePath(id, ..)
                | Action::SaveFolder(id, ..) => id.is_empty(),
                Action::AddRobot(..) | Action::AddPredefined(..) | Action::AddInstalledApp(..) => {
                    true
                }
                _ => false,
            };
            let annotation_action = matches!(action, Action::SaveAnnotation(..));
            let action_feedback = success_message(&action);
            let before = ((action.changes_layout()
                || matches!(
                    action,
                    Action::Save(..)
                        | Action::SaveWebsite(..)
                        | Action::SavePath(..)
                        | Action::SaveFolder(..)
                        | Action::RenameRobot(..)
                        | Action::SelectTickTickList(..)
                ))
                && !matches!(
                    action,
                    Action::EditCanvas(0 | 1, ..) | Action::TransferData(..)
                ))
            .then(|| crate::canvas_edit::State::capture(&apps.config));
            let result = apps.apply(action, &requests, &weak);
            if result.is_ok()
                && let Some(before) = before
            {
                apps.edits.record(before, &apps.config);
            }
            if request.is_some_and(|request| !requests.is_current(request)) {
                continue;
            }
            let frame_error = if save_frame {
                result.as_ref().err().cloned()
            } else {
                None
            };
            if annotation_action && let Err(error) = &result {
                let error = error.clone();
                let _ = weak.upgrade_in_event_loop(move |ui| ui.set_annotation_error(error.into()));
            }
            let success = result.is_ok();
            if success
                && reveal_added
                && let Some(item) = apps.config.items.last()
            {
                let id = item.id().to_string();
                apps.reveal(&id, &weak);
            }
            if success && let Some((size, gap, speed)) = appearance {
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    launcher.invoke_appearance_saved(size, gap, speed);
                });
            }
            if refreshing && !success {
                apps.inventory
                    .fail(result.as_ref().err().cloned().unwrap_or_default());
                apps.update(&weak);
            }
            let recent_changed = success && recent_id.is_some_and(|id| apps.recent.record(&id));
            // RemoveRecent is applied in Apps::apply; save it here without removing twice.
            let recent_removed = success && remove_recent_id.is_some();
            if (recent_changed || recent_removed)
                && let Err(error) = apps.recent.save()
            {
                eprintln!("orbit: couldn't save Recent data: {error}");
                let weak = weak.clone();
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    crate::toast::error(&launcher.as_weak(), "Couldn't save Recent changes");
                });
            }
            if success && filtering {
                apps.update_installed_apps(&weak);
            } else if success && searching {
                apps.update_search_results(&weak);
            } else if success && (update || recent_changed || recent_removed) {
                apps.update(&weak);
            }
            if success {
                apps.publish_reveal(&weak);
            } else {
                apps.pending_reveal = None;
            }
            if save_search_result && success {
                let _ =
                    weak.upgrade_in_event_loop(|launcher| launcher.invoke_search_result_saved());
            }
            if drag_drop {
                let _ = weak.upgrade_in_event_loop(|launcher| launcher.invoke_finish_drag());
            }
            if save_shortcut && success {
                let _ = weak.upgrade_in_event_loop(|launcher| launcher.invoke_shortcut_saved());
            }
            if ink_action {
                let _ = weak.upgrade_in_event_loop(|ui| ui.invoke_ink_finished());
            }
            if save_frame {
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    if let Some(error) = frame_error {
                        launcher.invoke_frame_save_failed(error.into());
                    } else {
                        launcher.invoke_frame_saved();
                    }
                });
            }
            if save_action && success {
                let _ = weak.upgrade_in_event_loop(|launcher| launcher.invoke_action_saved());
            }
            if add_exclusion && success {
                let _ = weak.upgrade_in_event_loop(|launcher| {
                    launcher.invoke_file_search_exclusion_saved()
                });
            }
            if save_search_path && success {
                let _ = weak.upgrade_in_event_loop(|launcher| {
                    launcher.invoke_finish_search_path_save();
                });
            }
            if script_edit {
                let error = result.as_ref().err().cloned().unwrap_or_default();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_script_error(error.into());
                });
            }
            if script_run && let Err(error) = &result {
                let error = error.clone();
                let _ =
                    weak.upgrade_in_event_loop(move |ui| crate::toast::error(&ui.as_weak(), error));
            }
            finish(
                &weak,
                if script_edit || script_run {
                    String::new()
                } else {
                    result.err().unwrap_or_default()
                },
                reset_after_save && success,
                launching && success,
                if success {
                    if refreshing {
                        Some("Search locations refreshed")
                    } else {
                        action_feedback
                    }
                } else {
                    None
                },
                false,
            );
        }
    });
}

fn callbacks(launcher: &Launcher, send: Sender<Action>, requests: Arc<RequestSequence>) {
    let sender = send.clone();
    launcher.on_edit_canvas(move |command, selection, x, y| {
        use slint::Model;
        let selection = selection.iter().map(|id| id.to_string()).collect();
        let _ = sender.send(Action::EditCanvas(
            command,
            selection,
            x.round() as i32,
            y.round() as i32,
        ));
    });
    let prefix_ui = launcher.as_weak();
    launcher.on_match_prefix(move |text| {
        use slint::Model;
        let prefixes = prefix_ui
            .upgrade()
            .map(|ui| {
                ui.get_prefixes()
                    .iter()
                    .map(|prefix| Prefix {
                        id: prefix.id.to_string(),
                        name: prefix.name.to_string(),
                        token: prefix.token.to_string(),
                        action_id: prefix.action_id.to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        match_prefix(&prefixes, &text).unwrap_or(PrefixMatch {
            action_id: "".into(),
            name: "".into(),
            query: "".into(),
            query_length: 0,
            badge_width: 0,
        })
    });
    let action_ui = launcher.as_weak();
    launcher.on_action_index_for_id(move |id| {
        use slint::Model;
        action_ui
            .upgrade()
            .and_then(|ui| {
                ui.get_user_actions()
                    .iter()
                    .position(|action| action.id == id)
            })
            .unwrap_or(0) as i32
    });
    let sender = send.clone();
    launcher.on_save_appearance(move |size, gap, speed| {
        let _ = sender.send(Action::SaveAppearance(size, gap, speed));
    });
    let sender = send.clone();
    launcher.on_save_background_folder(move |path| {
        let _ = sender.send(Action::SaveBackgroundFolder(path.into()));
    });
    let sender = send.clone();
    launcher.on_change_background_image(move || {
        let _ = sender.send(Action::ChangeBackground);
    });
    let sender = send.clone();
    launcher.on_choose_background_image(move || {
        let _ = sender.send(Action::ChooseBackgroundImage);
    });
    let sender = send.clone();
    let cursor_ui = launcher.as_weak();
    launcher.on_sync_native_search_cursor(move |hidden| {
        if let Some(ui) = cursor_ui.upgrade() {
            crate::search_cursor::sync(ui.window(), hidden);
        }
    });
    let ai_sender = send.clone();
    let ai_ui = launcher.as_weak();
    launcher.on_save_ai(move || {
        if let Some(ui) = ai_ui.upgrade() {
            let _ = ai_sender.send(Action::SaveAi(crate::ai::Settings::from_ui(&ui)));
        }
    });
    let english_sender = send.clone();
    launcher.on_save_english(move |suggestions, autocomplete, grammar, address| {
        let _ = english_sender.send(Action::SaveEnglish(
            suggestions,
            autocomplete,
            grammar,
            address.into(),
        ));
    });
    let model_sender = send.clone();
    launcher.on_save_english_model(move |model, suggestions, completion, grammar| {
        let _ = model_sender.send(Action::SaveEnglishModel(
            model.into(),
            suggestions.into(),
            completion.into(),
            grammar.into(),
        ));
    });
    let lock_sender = send.clone();
    launcher.on_save_layout_locked(move |locked| {
        let _ = lock_sender.send(Action::SaveLayoutLocked(locked));
    });
    launcher.on_save_smooth_cursor(move |enabled| {
        let _ = sender.send(Action::SaveSmoothCursor(enabled));
    });
    let sender = send.clone();
    launcher.on_save_default_search_engine(move |index| {
        let _ = sender.send(Action::SaveDefaultSearchEngine(index));
    });
    let sender = send.clone();
    launcher.on_transfer_data(move |import| {
        let _ = sender.send(Action::TransferData(import));
    });
    let sender = send.clone();
    launcher
        .global::<crate::TextDirection>()
        .on_is_rtl(|text| crate::text_direction::text_is_rtl(&text));
    launcher.global::<crate::TextDirection>().on_lines(|text| {
        std::rc::Rc::new(slint::VecModel::from(
            text.split('\n')
                .map(slint::SharedString::from)
                .collect::<Vec<_>>(),
        ))
        .into()
    });
    launcher.on_text_is_rtl(|text| crate::text_direction::text_is_rtl(&text));
    launcher.on_first_letter(|name| crate::icons::first_letter(&name).into());
    launcher.on_valid_name(|name| add_item::valid_name(&name));
    launcher.on_valid_command(|command| add_item::valid_command(&command));
    launcher.on_valid_url(|url| add_item::valid_url(&url));
    launcher.on_script_filter(|text| {
        crate::scripts::query(&text)
            .map(|(filter, _)| filter.to_lowercase())
            .unwrap_or_default()
            .into()
    });
    launcher.on_script_needs_completion(|text, name| {
        crate::scripts::query(&text)
            .is_some_and(|(filter, argument)| argument.is_none() && name != filter)
    });
    launcher.on_script_index(|scripts, id| {
        use slint::Model;
        scripts
            .iter()
            .position(|script| script.id == id)
            .unwrap_or(0) as i32
    });
    let script_ui = launcher.as_weak();
    launcher.on_complete_script_name(move |name| {
        if let Some(ui) = script_ui.upgrade() {
            let text = ui.get_search_text();
            if let Some((filter, argument)) = crate::scripts::query(&text) {
                let end = 1 + filter.len() + usize::from(argument.is_some());
                crate::search_edit::replace(&ui, &text, 1, end, &format!("{name} "));
            }
        }
    });
    launcher.on_filter_scripts(move |scripts, filter| {
        use slint::Model;
        let matches = scripts
            .iter()
            .filter(|script| script.name.to_lowercase().contains(filter.as_str()))
            .collect::<Vec<_>>();
        std::rc::Rc::new(slint::VecModel::from(matches)).into()
    });
    launcher.on_save_script({
        let sender = send.clone();
        move |id, name, path| {
            let _ = sender.send(Action::SaveScript(id.into(), name.into(), path.into()));
        }
    });
    launcher.on_delete_script({
        let sender = send.clone();
        move |id| {
            let _ = sender.send(Action::DeleteScript(id.into()));
        }
    });
    launcher.on_choose_script({
        let sender = send.clone();
        move || {
            let _ = sender.send(Action::ChooseScript);
        }
    });
    launcher.on_run_script({
        let sender = send.clone();
        move |id, text| {
            let _ = sender.send(Action::RunScript(id.into(), text.into()));
        }
    });
    launcher.on_search(move |query| {
        let _ = sender.send(Action::Search(query.into()));
    });
    let sender = send.clone();
    let prefix_ui = launcher.as_weak();
    launcher.on_run_prefix(move |action, argument| {
        use slint::Model;
        if let Some(ui) = prefix_ui.upgrade()
            && ui
                .get_user_actions()
                .iter()
                .any(|item| item.id == action && item.kind == "AI")
        {
            ui.set_busy(false);
            ui.invoke_ai_submit();
            return;
        }
        if let Some(ui) = prefix_ui.upgrade()
            && ui
                .get_user_actions()
                .iter()
                .any(|item| item.id == action && item.kind == "TickTick")
        {
            crate::ticktick_ui::create_from_prefix(&ui, &argument);
            return;
        }
        let _ = sender.send(Action::RunPrefix(action.into(), argument.into()));
    });
    let sender = send.clone();
    launcher.on_search_web(move |query, engine| {
        let _ = sender.send(Action::SearchWeb(query.into(), engine.into()));
    });
    let sender = send.clone();
    launcher.on_save_user_action(move |id, name, kind, source, description, tokens| {
        let _ = sender.send(Action::SaveNamedCommand(
            id.into(),
            name.into(),
            kind.into(),
            source.into(),
            description.into(),
            tokens.into(),
        ));
    });
    let sender = send.clone();
    launcher.on_delete_user_action(move |id| {
        let _ = sender.send(Action::DeleteNamedCommand(id.into()));
    });
    let sender = send.clone();
    launcher.on_save_prefix(move |id, name, token, action| {
        let _ = sender.send(Action::SavePrefix(
            id.into(),
            name.into(),
            token.into(),
            action.into(),
        ));
    });
    let sender = send.clone();
    launcher.on_delete_prefix(move |id| {
        let _ = sender.send(Action::DeletePrefix(id.into()));
    });
    let sender = send.clone();
    launcher.on_open_result(move |item| {
        let _ = sender.send(Action::OpenResult(item.into()));
    });
    let sender = send.clone();
    launcher.on_open_dock_item(move |item| {
        let _ = sender.send(Action::OpenDock(item.into()));
    });
    let sender = send.clone();
    launcher.on_pin_item(move |id| {
        let _ = sender.send(Action::Pin(id.into()));
    });
    let sender = send.clone();
    launcher.on_unpin_dock_item(move |id| {
        let _ = sender.send(Action::Unpin(id.into()));
    });
    let sender = send.clone();
    launcher.on_remove_recent_item(move |id| {
        let _ = sender.send(Action::RemoveRecent(id.into()));
    });
    let sender = send.clone();
    let requests_for_save = requests.clone();
    launcher.on_save_app(move |index, name, command, svg| {
        let request = requests_for_save.issue();
        let _ = sender.send(Action::Save(
            index.into(),
            name.into(),
            command.into(),
            svg.into(),
            request,
        ));
    });
    let sender = send.clone();
    let requests_for_action = requests.clone();
    launcher.on_add_system_action(move |action| {
        let request = requests_for_action.issue();
        let _ = sender.send(Action::AddPredefined(action.into(), request));
    });
    let sender = send.clone();
    let requests_for_app = requests.clone();
    launcher.on_add_installed_app(move |id| {
        let request = requests_for_app.issue();
        let _ = sender.send(Action::AddInstalledApp(id.into(), request));
    });
    let sender = send.clone();
    let requests_for_refresh = requests.clone();
    launcher.on_refresh_installed_apps(move || {
        let request = requests_for_refresh.issue();
        let _ = sender.send(Action::RefreshInstalledApps(request));
    });
    let sender = send.clone();
    launcher.on_save_app_search_path(move |path, remove| {
        let _ = sender.send(Action::SaveAppSearchPath(path.into(), remove));
    });
    let sender = send.clone();
    launcher.on_save_search_path(move |directory, index, path| {
        let _ = sender.send(Action::SaveSearchPath(directory, index, path.into()));
    });
    let sender = send.clone();
    launcher.on_add_file_search_exclusion(move |name| {
        let _ = sender.send(Action::AddFileSearchExclusion(name.into()));
    });
    let sender = send.clone();
    launcher.on_remove_file_search_exclusion(move |name| {
        let _ = sender.send(Action::RemoveFileSearchExclusion(name.into()));
    });
    let sender = send.clone();
    launcher.on_delete_search_path(move |directory, index| {
        let _ = sender.send(Action::DeleteSearchPath(directory, index as usize));
    });
    let sender = send.clone();
    launcher.on_filter_installed_apps(move |filter| {
        let _ = sender.send(Action::FilterInstalled(filter.to_lowercase()));
    });
    let sender = send.clone();
    let requests_for_website = requests.clone();
    let website_icons = crate::website_icons::connect(launcher);
    launcher.on_save_website(move |index, name, url, value, automatic| {
        let request = requests_for_website.issue();
        let _ = sender.send(Action::SaveWebsite(
            index.into(),
            name.into(),
            url.to_string(),
            crate::website_icons::draft(&website_icons, &url, value.into(), automatic),
            request,
        ));
    });
    let sender = send.clone();
    let requests_for_path = requests.clone();
    launcher.on_choose_path(move |directory| {
        let request = requests_for_path.issue();
        let _ = sender.send(Action::ChoosePath(directory, request));
    });
    let sender = send.clone();
    let requests_for_save_path = requests.clone();
    launcher.on_save_path(move |index, path, directory, svg| {
        let request = requests_for_save_path.issue();
        let _ = sender.send(Action::SavePath(
            index.into(),
            path.into(),
            directory,
            svg.into(),
            request,
        ));
    });
    let sender = send.clone();
    let requests_for_folder = requests.clone();
    launcher.on_save_folder(move |index, name| {
        let request = requests_for_folder.issue();
        let _ = sender.send(Action::SaveFolder(index.into(), name.into(), request));
    });
    let requests_for_cancel = requests;
    launcher.on_cancel_add(move || requests_for_cancel.cancel());
    let sender = send.clone();
    launcher.on_delete_item(move |index| {
        let _ = sender.send(Action::Delete(index.into()));
    });
    let sender = send.clone();
    launcher.on_open_item(move |index| {
        let _ = sender.send(Action::Open(index.into()));
    });
    let sender = send.clone();
    launcher.on_add_robot(move |profile, name| {
        let _ = sender.send(Action::AddRobot(profile.into(), name.into()));
    });
    let sender = send.clone();
    launcher.on_rename_robot(move |profile, name| {
        let _ = sender.send(Action::RenameRobot(profile.into(), name.into()));
    });
    let sender = send.clone();
    launcher.on_save_shortcut(move |action, existing, keys| {
        let _ = sender.send(Action::SaveShortcut(action, existing.into(), keys.into()));
    });
    let sender = send.clone();
    launcher.on_delete_shortcut(move |action, keys| {
        let _ = sender.send(Action::DeleteShortcut(action, keys.into()));
    });
    let sender = send.clone();
    launcher.on_save_frame(move |id, name, keys, items| {
        use slint::Model;
        let items = items
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>();
        let _ = sender.send(Action::SaveFrame(
            id.into(),
            name.into(),
            keys.into(),
            items,
        ));
    });
    let sender = send.clone();
    launcher.on_delete_frame(move |id| {
        let _ = sender.send(Action::DeleteFrame(id.into()));
    });
    let sender = send.clone();
    launcher.on_fit_frame_action(move |id, width, height| {
        let _ = sender.send(Action::FitFrame(id.into(), width, height));
    });
    let sender = send.clone();
    launcher.on_add_ink(move |points, color, size, opacity, mode| {
        use slint::Model;
        let _ = sender.send(Action::AddInk(
            points
                .iter()
                .map(|point| crate::config::InkPoint {
                    x: point.x,
                    y: point.y,
                })
                .collect(),
            crate::ink::Style {
                color: color.into(),
                size,
                opacity,
                highlighter: mode == 8,
                arrow: mode == 7,
            },
        ));
    });
    let sender = send.clone();
    launcher.on_save_font_size(move |size| {
        let _ = sender.send(Action::SaveFontSize(size));
    });
    let sender = send.clone();
    launcher.on_style_selection({
        let sender = send.clone();
        move |ids, color, size, opacity| {
            use slint::Model;
            let _ = sender.send(Action::StyleSelection(
                ids.iter().map(|id| id.to_string()).collect(),
                crate::ink::Style {
                    color: color.into(),
                    size,
                    opacity,
                    ..Default::default()
                },
            ));
        }
    });
    launcher.on_save_annotation(move |id, text, kind, color, size, opacity, x, y| {
        let _ = sender.send(Action::SaveAnnotation(
            id.into(),
            text.into(),
            kind,
            crate::ink::Style {
                color: color.into(),
                size,
                opacity,
                ..Default::default()
            },
            x.round() as i32,
            y.round() as i32,
        ));
    });
    let sender = send.clone();
    launcher.on_erase_ink(move |points| {
        use slint::Model;
        let _ = sender.send(Action::EraseInk(
            points
                .iter()
                .map(|point| crate::config::InkPoint {
                    x: point.x,
                    y: point.y,
                })
                .collect(),
        ));
    });
    let sender = send.clone();
    launcher.on_set_add_position(move |x, y| {
        let _ = sender.send(Action::AddPosition(x.round() as i32, y.round() as i32));
    });
    let sender = send.clone();
    launcher.on_show_all_items(move |width, height| {
        let _ = sender.send(Action::ShowAll(width, height));
    });
    let sender = send.clone();
    launcher.on_add_widget(move |variant, col, row, auto| {
        let _ = sender.send(Action::Widget(variant as usize, col, row, auto));
    });
    let sender = send.clone();
    launcher.on_widget_action(move |id, width, height| {
        let _ = sender.send(Action::ChangeWidget(
            id.into(),
            width as usize,
            height as usize,
        ));
    });
    let sender = send.clone();
    launcher.on_duplicate_widget(move |id| {
        let _ = sender.send(Action::DuplicateWidget(id.into()));
    });
    let widget_ui = launcher.as_weak();
    launcher.on_ticktick_selected_project(move |id| {
        widget_ui
            .upgrade()
            .and_then(|ui| {
                use slint::Model;
                ui.get_items()
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| item.ticktick_project_id)
            })
            .unwrap_or_default()
    });
    let sender = send.clone();
    launcher.on_select_ticktick_list(move |widget, project| {
        let _ = sender.send(Action::SelectTickTickList(widget.into(), project.into()));
    });
    let sender = send.clone();
    launcher.on_save_search_result(move |item, col, row, pin| {
        let _ = sender.send(Action::SaveSearchResult(item.into(), col, row, pin));
    });
    let sender = send.clone();
    launcher.on_drop_item(move |id, col, row, into| {
        let _ = sender.send(Action::Drop(id.into(), col, row, into.into()));
    });
    let sender = send.clone();
    launcher.on_open_folder(move |index| {
        let _ = sender.send(Action::OpenFolder(index.into()));
    });
    let sender = send.clone();
    launcher.on_close_folder(move || {
        let _ = sender.send(Action::CloseFolder);
    });
    let sender = send.clone();
    launcher.on_launch_folder_app(move |folder, app| {
        let _ = sender.send(Action::LaunchFolder(folder.into(), app.into()));
    });
}

impl Apps {
    fn recent_id(&self, action: &Action) -> Option<String> {
        let id = match action {
            Action::Open(id) | Action::LaunchFolder(_, id) => id,
            Action::OpenResult(item) | Action::OpenDock(item) if item.source >= 0 => &item.id,
            _ => return None,
        };
        let tile = self.dock_tile(id)?;
        (!tile.folder && !tile.widget && tile.robot.is_empty()).then(|| id.clone())
    }

    fn load_icons(&mut self) -> Result<(), String> {
        let mut error = None;
        let mut icons = Vec::new();
        {
            for item in &self.config.items {
                match item {
                    Item::Annotation { .. } | Item::Widget { .. } | Item::Robot { .. } => {}
                    Item::App(app) => icons.push((app.id.clone(), app.icon.clone())),
                    Item::Website { id, icon, .. } => icons.push((id.clone(), icon.clone())),
                    Item::Folder { apps, .. } => {
                        for app in apps {
                            icons.push((app.id.clone(), app.icon.clone()));
                        }
                    }
                    Item::File { icon, id, .. } | Item::Directory { icon, id, .. } => {
                        icons.push((id.clone(), icon.clone()));
                    }
                }
            }
        }
        for (id, icon) in icons {
            if let Err(message) = self.load_icon(&id, &icon) {
                error = Some(message);
            }
        }
        for item in &self.config.items {
            if let Item::Annotation {
                id, kind: 3, image, ..
            } = item
            {
                if self.icons.contains_key(id) {
                    continue;
                }
                match platform::read_icon(image)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| canvas_image(&bytes, image))
                {
                    Ok(pixels) => {
                        self.icons.insert(id.clone(), pixels);
                    }
                    Err(message) => {
                        error = Some(message);
                    }
                }
            }
        }
        error.map_or(Ok(()), Err)
    }

    fn load_icon(&mut self, id: &str, icon: &Icon) -> Result<(), String> {
        let image = match icon {
            Icon::Letter | Icon::Text(_) => None,
            Icon::Svg(svg) => Some(decode_svg(svg)?),
            Icon::File(name) => {
                let data = match platform::read_icon(name) {
                    Ok(data) => data,
                    Err(_) => return Ok(()),
                };
                Some(decode_icon(&data, name)?)
            }
        };
        if let Some(mut pixels) = image {
            // Cache only the largest displayed size, rather than full-resolution system icons.
            if pixels.width() > 80 || pixels.height() > 80 {
                let source = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                    pixels.width(),
                    pixels.height(),
                    pixels.as_bytes(),
                )
                .ok_or("Couldn't read icon")?;
                let resized = image::imageops::thumbnail(&source, 80, 80);
                pixels = SharedPixelBuffer::clone_from_slice(
                    resized.as_raw(),
                    resized.width(),
                    resized.height(),
                );
            }
            self.icons.insert(id.into(), pixels);
        }
        Ok(())
    }

    fn apply(
        &mut self,
        action: Action,
        requests: &RequestSequence,
        weak: &slint::Weak<Launcher>,
    ) -> Result<(), String> {
        self.apply_with_url_opener(action, requests, weak, &platform::open)
    }

    fn apply_with_url_opener(
        &mut self,
        action: Action,
        requests: &RequestSequence,
        weak: &slint::Weak<Launcher>,
        open_url: &UrlOpener<'_>,
    ) -> Result<(), String> {
        if self.config.layout_locked && action.changes_layout() {
            return Err("Unlock layout to make changes".into());
        }
        match action {
            Action::EditCanvas(command, selection, x, y) => {
                self.edit_canvas(command, &selection, x, y, weak)
            }
            Action::Canvas(change) => crate::canvas::apply(&mut self.config, change),
            Action::SaveLayoutLocked(locked) => {
                if self.config.layout_locked != locked {
                    let mut config = self.config.clone();
                    config.layout_locked = locked;
                    config
                        .save()
                        .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                    self.config = config;
                }
                Ok(())
            }
            Action::SaveFrame(id, name, keys, selected) => {
                self.save_frame(&id, name, keys, &selected)
            }
            Action::DeleteFrame(id) => {
                let mut config = self.config.clone();
                let before = config.frames.len();
                config.frames.retain(|frame| frame.id != id);
                if config.frames.len() == before {
                    return Err("Frame is missing".into());
                }
                self.commit(config)
            }
            Action::FitFrame(id, width, height) => self.fit_frame(&id, width, height, weak),
            Action::SaveAnnotation(id, text, kind, style, x, y) => {
                self.save_annotation(&id, text, kind, style, (x, y), weak)
            }
            Action::StyleSelection(ids, style) => {
                if !style.valid() {
                    return Err("Invalid style".into());
                }
                let mut config = self.config.clone();
                for item in &mut config.items {
                    if let Item::Annotation {
                        id, style: saved, ..
                    } = item
                        && ids.contains(id)
                    {
                        *saved = style.clone();
                    }
                }
                if config.items != self.config.items {
                    self.commit(config)?;
                }
                Ok(())
            }
            Action::SaveFontSize(size) => {
                if !(10..=24).contains(&size) {
                    return Err("Font size must be 10–24 px".into());
                }
                if self.config.font_size != size {
                    let mut config = self.config.clone();
                    config.font_size = size;
                    config.save().map_err(|e| e.to_string())?;
                    self.config = config;
                }
                Ok(())
            }
            Action::AddInk(points, style) => self.add_ink(points, style),
            Action::EraseInk(points) => self.erase_ink(&points),
            Action::SaveScript(id, name, path) => {
                crate::scripts::validate(&name, &path)?;
                let mut config = self.config.clone();
                let id = if id.is_empty() { new_id()? } else { id };
                let script = config::Script {
                    id: id.clone(),
                    name,
                    path,
                };
                if let Some(existing) = config.scripts.iter_mut().find(|script| script.id == id) {
                    *existing = script;
                } else {
                    config.scripts.push(script);
                }
                if config.scripts != self.config.scripts {
                    self.commit(config)?;
                }
                let _ = weak.upgrade_in_event_loop(move |ui| ui.set_script_id_draft(id.into()));
                Ok(())
            }
            Action::DeleteScript(id) => {
                let mut config = self.config.clone();
                config.scripts.retain(|script| script.id != id);
                if config.scripts.len() == self.config.scripts.len() {
                    return Err("Script is missing".into());
                }
                self.commit(config)?;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    if ui.get_script_id_draft() == id {
                        ui.invoke_new_script();
                    }
                });
                Ok(())
            }
            Action::ChooseScript => {
                let path = platform::choose_path(false);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    if let Err(error) = platform::restore_after_chooser(&ui) {
                        ui.set_script_error(error.to_string().into());
                    } else if let Some(path) = path {
                        ui.set_script_path_draft(path.to_string_lossy().as_ref().into());
                        ui.set_script_error("".into());
                    }
                    ui.set_sheet_operation("".into());
                });
                Ok(())
            }
            Action::RunScript(id, text) => {
                let script = self
                    .config
                    .scripts
                    .iter()
                    .find(|script| script.id == id)
                    .ok_or("Script is missing")?;
                let (filter, argument) =
                    crate::scripts::query(&text).ok_or("Script query is missing")?;
                if !script.name.to_lowercase().contains(&filter.to_lowercase()) {
                    return Err("Script no longer matches".into());
                }
                platform::launch_script(std::path::Path::new(&script.path), argument)
                    .map_err(|error| format!("Couldn't run '{}': {error}", script.name))
            }
            Action::Search(query) => {
                self.query = query.to_lowercase();
                if !self.query.is_empty() && !self.query.starts_with('/') {
                    self.load_installed_icons_matching_query();
                    self.load_search_paths();
                }
                Ok(())
            }
            Action::SearchWeb(query, engine) => {
                let url = url_action_url(self.config.search_engine_template(&engine), &query)?;
                open_url(&url).map_err(|error| format!("Couldn't open web search: {error}"))
            }
            Action::SaveEnglishModel(
                model,
                suggestions_prompt,
                completion_prompt,
                grammar_prompt,
            ) => {
                let settings = crate::english_assistance::Settings {
                    model,
                    suggestions_prompt,
                    completion_prompt,
                    grammar_prompt,
                    ..self.config.english_assistance.clone()
                };
                if !settings.valid() {
                    return Err(
                        "Choose a local model and nonempty prompts (up to 16000 bytes each)."
                            .into(),
                    );
                }
                if self.config.english_assistance != settings {
                    let mut config = self.config.clone();
                    config.english_assistance = settings;
                    config
                        .save()
                        .map_err(|error| format!("Couldn't save English assistance: {error}"))?;
                    self.config = config;
                }
                Ok(())
            }
            Action::SaveEnglish(suggestions, autocomplete, grammar, address) => {
                let settings = crate::english_assistance::Settings {
                    suggestions,
                    autocomplete,
                    grammar,
                    address,
                    ..self.config.english_assistance.clone()
                };
                if !settings.valid() {
                    return Err("Use a local Ollama address such as http://127.0.0.1:11434".into());
                }
                if self.config.english_assistance != settings {
                    let mut config = self.config.clone();
                    config.english_assistance = settings;
                    config
                        .save()
                        .map_err(|error| format!("Couldn't save English assistance: {error}"))?;
                    self.config = config;
                }
                Ok(())
            }
            Action::SaveSmoothCursor(enabled) => {
                if self.config.smooth_cursor != enabled {
                    let mut config = self.config.clone();
                    config.smooth_cursor = enabled;
                    config
                        .save()
                        .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                    self.config = config;
                }
                Ok(())
            }
            Action::SaveDefaultSearchEngine(index) => {
                let index =
                    usize::try_from(index).map_err(|_| "Choose a supported search engine")?;
                let id =
                    Config::search_engine_id(index).ok_or("Choose a supported search engine")?;
                let mut config = self.config.clone();
                config.default_search_engine = id.into();
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                Ok(())
            }
            Action::SaveNamedCommand(id, name, kind, source, description, tokens) => {
                let kind = match kind.as_str() {
                    "URL" => ActionKind::Url,
                    "TickTick" => ActionKind::TickTick,
                    "AI" => ActionKind::Ai,
                    _ => ActionKind::Shell,
                };
                if name.trim().is_empty()
                    || (kind == ActionKind::Url && !source.contains("%s"))
                    || (kind == ActionKind::Shell && source.trim().is_empty())
                {
                    return Err("Enter a valid Action name and command".into());
                }
                let mut config = self.config.clone();
                let id = if id.is_empty() { new_id()? } else { id };
                let tokens = tokens
                    .split(',')
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                let mut seen = std::collections::HashSet::new();
                if tokens.iter().any(|token| {
                    token.starts_with('/')
                        || token.chars().any(char::is_whitespace)
                        || !seen.insert(token.to_lowercase())
                        || config.prefixes.iter().any(|prefix| {
                            prefix.action_id != id && prefix.token.eq_ignore_ascii_case(token)
                        })
                }) {
                    return Err(
                        "Tokens must be unique, contain no spaces and cannot start with /".into(),
                    );
                }
                let action = UserAction {
                    id: id.clone(),
                    name: name.trim().into(),
                    kind,
                    source,
                    description: description.trim().into(),
                };
                if let Some(existing) = config.user_actions.iter_mut().find(|item| item.id == id) {
                    *existing = action;
                } else {
                    config.user_actions.push(action);
                }
                let insert_at = config
                    .prefixes
                    .iter()
                    .position(|prefix| prefix.action_id == id)
                    .unwrap_or(config.prefixes.len());
                let new_prefixes = tokens
                    .into_iter()
                    .map(|token| {
                        let previous = config
                            .prefixes
                            .iter()
                            .find(|prefix| prefix.action_id == id && prefix.token == token);
                        let prefix_id =
                            previous.map_or_else(new_id, |prefix| Ok(prefix.id.clone()));
                        prefix_id.map(|prefix_id| Prefix {
                            id: prefix_id,
                            name: name.trim().into(),
                            token,
                            action_id: id.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                config.prefixes.retain(|prefix| prefix.action_id != id);
                config.prefixes.splice(
                    insert_at.min(config.prefixes.len())..insert_at.min(config.prefixes.len()),
                    new_prefixes,
                );
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                Ok(())
            }
            Action::DeleteNamedCommand(id) => {
                let mut config = self.config.clone();
                if config.prefixes.iter().any(|prefix| prefix.action_id == id) {
                    return Err("Edit or remove the Prefixes that use this Action first".into());
                }
                config.user_actions.retain(|action| action.id != id);
                if config.user_actions.len() == self.config.user_actions.len() {
                    return Err("Action is missing".into());
                }
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                Ok(())
            }
            Action::SavePrefix(id, name, token, action_id) => {
                let token = token.trim();
                if token.starts_with('/')
                    || token.is_empty()
                    || token.chars().any(char::is_whitespace)
                    || name.trim().is_empty()
                {
                    return Err(
                        "Enter a Prefix name and a Token without spaces or a leading /".into(),
                    );
                }
                let mut config = self.config.clone();
                let id = if id.is_empty() { new_id()? } else { id };
                let prefix = Prefix {
                    id: id.clone(),
                    name: name.trim().into(),
                    token: token.into(),
                    action_id,
                };
                if let Some(existing) = config.prefixes.iter_mut().find(|item| item.id == id) {
                    *existing = prefix;
                } else {
                    config.prefixes.push(prefix);
                }
                if !config.valid() {
                    return Err("That Token is already in use or its Action is missing".into());
                }
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                Ok(())
            }
            Action::DeletePrefix(id) => {
                let mut config = self.config.clone();
                config.prefixes.retain(|prefix| prefix.id != id);
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                Ok(())
            }
            Action::SaveAi(settings) => {
                settings.valid()?;
                if self.config.ai != settings {
                    let mut config = self.config.clone();
                    config.ai = settings;
                    config
                        .save()
                        .map_err(|error| format!("Couldn't save AI settings: {error}"))?;
                    self.config = config;
                }
                Ok(())
            }
            Action::RunPrefix(action_id, argument) => {
                let action = self
                    .config
                    .user_actions
                    .iter()
                    .find(|action| action.id == action_id)
                    .ok_or("Action is missing")?;
                match &action.kind {
                    ActionKind::Url => {
                        let query = argument.trim();
                        let url = if action.id == "browser-search" {
                            browser_search_url(&action.source, query)?
                        } else {
                            url_action_url(&action.source, query)?
                        };
                        open_url(&url)
                            .map_err(|error| format!("Couldn't open {}: {error}", action.name))
                    }
                    ActionKind::Shell => {
                        platform::launch_shell_action(&action.source, argument.trim())
                            .map_err(|error| format!("Couldn't start {}: {error}", action.name))
                    }
                    ActionKind::Ai => Err("Use the AI Prefix from Search anything.".into()),
                    ActionKind::TickTick => create_ticktick_task(argument.trim())
                        .map(|_| ())
                        .map_err(|error| format!("Couldn't create TickTick task: {error}")),
                }
            }
            Action::FilterInstalled(filter) => {
                self.installed_filter = filter;
                self.load_installed_icons();
                Ok(())
            }
            Action::SaveSearchResult(reference, col, row, pin) => {
                let item = self.search_result_item(&reference)?;
                let id = item.id().to_string();
                let mut config = self.config.clone();
                let rect = crate::spatial::Rect {
                    col,
                    row,
                    width: crate::spatial::item_span(1, config.item_size, config.grid_gap),
                    height: crate::spatial::item_span(1, config.item_size, config.grid_gap),
                };
                if !crate::item_drop::place_search_result(&mut config, item, Some(rect), pin) {
                    return Ok(());
                }
                self.commit(config)?;
                if !pin {
                    self.reveal(&id, weak);
                }
                self.load_icons()
            }
            Action::Pin(id) => {
                if self.config.pinned.contains(&id) {
                    return self.unpin(&id, weak);
                }
                let Some(item) = self
                    .config
                    .items
                    .iter()
                    .find(|item| item.id() == id)
                    .cloned()
                else {
                    return Ok(());
                };
                let mut config = self.config.clone();
                if !crate::item_drop::place_search_result(&mut config, item, None, true) {
                    return Err("Item cannot be pinned; the Dock may be full".into());
                }
                self.commit(config)
            }
            Action::Unpin(id) => self.unpin(&id, weak),
            Action::RemoveRecent(id) => {
                self.recent.remove(&id);
                Ok(())
            }
            Action::OpenResult(reference) | Action::OpenDock(reference) => {
                if reference.source < 0 {
                    let app = self.cached_search_result(&reference)?;
                    return self
                        .launch(&app)
                        .map_err(|error| format!("Couldn't open '{}': {error}", app.name));
                }
                if !reference.folder_id.is_empty() {
                    return self.launch_folder_app(&reference.folder_id, &reference.id);
                }
                self.open_item(&reference.id, weak)
            }
            Action::Save(index, name, command, svg, request) => {
                self.save(&index, name, command, svg, requests, request)
            }
            Action::SaveFolder(index, name, request) => {
                self.save_folder(&index, name, requests, request)
            }
            Action::RenameRobot(profile, name) => {
                let mut config = self.config.clone();
                let mut changed = false;
                for item in &mut config.items {
                    if let Item::Robot {
                        profile: saved,
                        name: label,
                        ..
                    } = item
                        && *saved == profile
                        && *label != name
                    {
                        label.clone_from(&name);
                        changed = true;
                    }
                }
                if changed {
                    config.save().map_err(|e| e.to_string())?;
                    self.config = config;
                }
                Ok(())
            }
            Action::AddRobot(profile, name) => {
                let db = crate::robot_data::Database::open(
                    &platform::config_path()
                        .map_err(|e| e.to_string())?
                        .with_file_name("robots.db"),
                )
                .map_err(|e| e.to_string())?;
                db.profile(&profile).map_err(|e| e.to_string())?;
                self.store(
                    "",
                    Item::Robot {
                        id: format!(
                            "robot-{}",
                            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                        ),
                        name,
                        profile,
                    },
                )
            }
            Action::Delete(index) => self.delete(&index, weak),
            Action::Open(index) => self.open_item(&index, weak),
            Action::SelectTickTickList(widget_id, project_id) => {
                let mut config = self.config.clone();
                let widget = config
                    .items
                    .iter_mut()
                    .find(|item| item.id() == widget_id)
                    .ok_or("Widget is missing")?;
                if let Item::Widget {
                    variant: 7,
                    ticktick_project_id,
                    ..
                } = widget
                {
                    *ticktick_project_id = project_id;
                } else {
                    return Err("Widget is missing".into());
                }
                config.save().map_err(|error| error.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::OpenFolder(index) => {
                self.open_folder = Some(index);
                Ok(())
            }
            Action::CloseFolder => {
                self.open_folder = None;
                Ok(())
            }
            Action::LaunchFolder(folder, id) => self.launch_folder_app(&folder, &id),
            Action::AddPosition(col, row) => {
                self.add_position = (col, row);
                Ok(())
            }
            Action::ShowAll(width, height) => {
                let Some(view) = crate::canvas::fit(&self.config.positions, width, height) else {
                    return Ok(());
                };
                crate::canvas::apply(&mut self.config, crate::canvas::Mutation::View(view))?;
                let canvas = self.config.canvas.clone();
                let _ =
                    weak.upgrade_in_event_loop(move |ui| crate::canvas::sync(&ui, &canvas, true));
                Ok(())
            }
            Action::Widget(variant, col, row, auto) => {
                let (col, row) = if auto { self.add_position } else { (col, row) };
                if !matches!(variant, 7..=18 | 20) {
                    return Err("Unknown widget type".into());
                }
                let id = new_id()?;
                let mut config = self.config.clone();
                let (width, height) = crate::spatial::WIDGET_SPANS[variant];
                let rect = crate::spatial::Rect {
                    col,
                    row,
                    width: crate::spatial::widget_span(width, config.item_size, config.grid_gap),
                    height: crate::spatial::widget_span(height, config.item_size, config.grid_gap),
                };
                let occupied = crate::spatial::occupied(&config);
                let rect = if auto {
                    crate::spatial::nearby(&occupied, rect)?
                } else {
                    crate::spatial::place(&occupied, &id, rect)
                        .ok_or("Widget does not fit here")?;
                    rect
                };
                config.positions.insert(id.clone(), rect);
                config.items.push(Item::Widget {
                    id: id.clone(),
                    name: crate::spatial::WIDGETS[variant].into(),
                    text: crate::spatial::WIDGETS[variant].into(),
                    variant,
                    ticktick_project_id: String::new(),
                    min_number: 1,
                    max_number: 100,
                });
                self.commit(config)?;
                self.reveal(&id, weak);
                Ok(())
            }
            Action::DuplicateWidget(id) => {
                let Some(Item::Widget {
                    name,
                    text,
                    variant,
                    ticktick_project_id,
                    min_number,
                    max_number,
                    ..
                }) = self
                    .config
                    .items
                    .iter()
                    .find(|item| item.id() == id)
                    .cloned()
                else {
                    return Ok(());
                };
                let mut config = self.config.clone();
                let rect = config
                    .positions
                    .get(&id)
                    .copied()
                    .ok_or("Widget is missing")?;
                let rect = crate::spatial::nearby(&crate::spatial::occupied(&config), rect)?;
                let id = new_id()?;
                config.positions.insert(id.clone(), rect);
                config.items.push(Item::Widget {
                    id: id.clone(),
                    name,
                    text,
                    variant,
                    ticktick_project_id,
                    min_number,
                    max_number,
                });
                self.commit(config)?;
                self.reveal(&id, weak);
                Ok(())
            }
            Action::ChangeWidget(id, width, height) => {
                let mut config = self.config.clone();
                if let Some(rect) = config.positions.get(&id).copied() {
                    let variant = config
                        .items
                        .iter()
                        .find_map(|item| {
                            if let Item::Widget {
                                id: item_id,
                                variant,
                                ..
                            } = item
                            {
                                (item_id == &id).then_some(*variant)
                            } else {
                                None
                            }
                        })
                        .ok_or("Widget is missing")?;
                    let (min_width, min_height) = crate::spatial::WIDGET_MIN_SPANS[variant];
                    let (max_width, max_height) = crate::spatial::WIDGET_MAX_SPANS[variant];
                    let min_width =
                        crate::spatial::widget_span(min_width, config.item_size, config.grid_gap);
                    let min_height =
                        crate::spatial::widget_span(min_height, config.item_size, config.grid_gap);
                    let max_width =
                        crate::spatial::widget_span(max_width, config.item_size, config.grid_gap);
                    let max_height =
                        crate::spatial::widget_span(max_height, config.item_size, config.grid_gap);
                    if width < min_width
                        || height < min_height
                        || width > max_width
                        || height > max_height
                    {
                        return Err("This widget cannot be resized to that size".into());
                    }
                    let rect = crate::spatial::Rect {
                        width,
                        height,
                        ..rect
                    };
                    let positions =
                        crate::spatial::place(&crate::spatial::occupied(&config), &id, rect)
                            .ok_or("Widget does not fit here")?;
                    config.positions.extend(positions);
                }
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::SaveAppearance(size, gap, speed) => {
                let mut config = self.config.clone();
                let old_gap = config.grid_gap.clamp(0, 64) as u64;
                let new_gap = gap.clamp(0, 64) as u64;
                let old_pitch =
                    crate::spatial::cell_pitch(config.item_size, config.grid_gap) as u64;
                let old_size = config.item_size.max(1) as u64;
                let new_size = size.clamp(80, 176) as u64;
                let new_pitch = crate::spatial::cell_pitch(size.clamp(80, 176), gap) as u64;
                if old_pitch != new_pitch {
                    let widgets: HashSet<_> = config
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            Item::Widget { id, .. } => Some(id.as_str()),
                            _ => None,
                        })
                        .collect();
                    for (id, rect) in &mut config.positions {
                        rect.col = i32::try_from(
                            i64::from(rect.col) * new_pitch as i64 / old_pitch as i64,
                        )
                        .map_err(|_| "Canvas position is too far away for that item size")?;
                        rect.row = i32::try_from(
                            i64::from(rect.row) * new_pitch as i64 / old_pitch as i64,
                        )
                        .map_err(|_| "Canvas position is too far away for that item size")?;
                        if widgets.contains(id.as_str()) {
                            rect.width = (rect.width.saturating_sub(old_gap as usize) as u64
                                * new_size
                                / old_size
                                + new_gap) as usize;
                            rect.height = (rect.height.saturating_sub(old_gap as usize) as u64
                                * new_size
                                / old_size
                                + new_gap) as usize;
                        } else {
                            rect.width = (rect.width as u64 * new_pitch / old_pitch) as usize;
                            rect.height = (rect.height as u64 * new_pitch / old_pitch) as usize;
                        }
                    }
                }
                config.item_size = size.clamp(80, 176);
                config.grid_gap = gap.clamp(0, 64);
                config.canvas_speed = speed.clamp(1, 10);
                crate::spatial::reconcile(&mut config)?;
                config.save().map_err(|error| error.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::SaveBackgroundFolder(path) => {
                let path = PathBuf::from(path.trim());
                if !path.is_dir() {
                    return Err("Choose an existing image folder".into());
                }
                let mut config = self.config.clone();
                config.background_folder = path.to_string_lossy().into_owned();
                config.save().map_err(|error| error.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::ChangeBackground => {
                let mut images: Vec<_> = std::fs::read_dir(&self.config.background_folder)
                    .map_err(|error| format!("Couldn't read image folder: {error}"))?
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.is_file()
                            && path.extension().is_some_and(|ext| {
                                matches!(
                                    ext.to_string_lossy().to_ascii_lowercase().as_str(),
                                    "png" | "jpg" | "jpeg" | "svg"
                                )
                            })
                    })
                    .collect();
                images.sort();
                if images.is_empty() {
                    return Err("No supported images in the image folder".into());
                }
                let mut pick = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize
                    % images.len();
                if images.len() > 1
                    && images[pick].to_string_lossy() == self.config.background_image
                {
                    pick = (pick + 1) % images.len();
                }
                if read_background_image(&images[pick]).is_none() {
                    return Err("Couldn't read background image".into());
                }
                let mut config = self.config.clone();
                config.background_image = images[pick].to_string_lossy().into_owned();
                config.background_color.clear();
                config.save().map_err(|error| error.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::ChooseBackgroundImage => {
                if let Some(path) = platform::choose_background_image() {
                    let (name, data) = platform::read_chosen_icon(&path.to_string_lossy())?;
                    if read_background_image(&path).is_none() {
                        return Err("Couldn't read background image".into());
                    }
                    let saved_path =
                        platform::icon_path(&name).map_err(|error| error.to_string())?;
                    platform::write_icon(&name, &data).map_err(|error| error.to_string())?;
                    let mut config = self.config.clone();
                    config.background_image = saved_path.to_string_lossy().into_owned();
                    config.background_color.clear();
                    config.save().map_err(|error| error.to_string())?;
                    self.config = config;
                }
                Ok(())
            }
            Action::SaveShortcut(index, action, keys) => self.save_shortcut(index, action, keys),
            Action::DeleteShortcut(action, keys) => {
                let mut config = self.config.clone();
                if let Some(index) = config
                    .shortcuts
                    .iter()
                    .position(|binding| binding.action == action && binding.keys == keys)
                {
                    config.shortcuts.remove(index);
                    config.save().map_err(|error| error.to_string())?;
                    self.config = config;
                }
                Ok(())
            }
            Action::Drop(id, col, row, into) => self.drop_item(&id, col, row, &into, weak),
            Action::AddPredefined(action, request) => {
                self.add_system_action(&action, requests, request)
            }
            Action::AddInstalledApp(id, request) => self.add_installed_app(&id, requests, request),
            Action::SaveAppSearchPath(path, remove) => {
                let path = PathBuf::from(path.trim());
                if !remove && !platform::validate_app_search_path(&path) {
                    return Err("Enter an existing absolute directory or executable path".into());
                }
                let path = path.to_string_lossy().into_owned();
                let mut config = self.config.clone();
                if remove {
                    config.app_search_paths.retain(|saved| saved != &path);
                } else if !config.app_search_paths.contains(&path) {
                    config.app_search_paths.push(path);
                }
                if config.app_search_paths != self.config.app_search_paths {
                    config.save().map_err(|error| error.to_string())?;
                    self.config = config;
                }
                Ok(())
            }
            Action::RefreshInstalledApps(request) => self.refresh_installed_apps(requests, request),
            Action::SaveSearchPath(directory, index, path) => {
                self.save_search_path(directory, index, path)
            }
            Action::DeleteSearchPath(directory, index) => self.delete_search_path(directory, index),
            Action::AddFileSearchExclusion(name) => self.edit_file_search_exclusion(&name, false),
            Action::RemoveFileSearchExclusion(name) => self.edit_file_search_exclusion(&name, true),
            Action::SaveWebsite(index, name, url, draft, request) => {
                self.save_website(&index, name, url, draft, requests, request)
            }
            Action::TransferData(import) => {
                let _robots = if import {
                    Some(crate::robot_workbench::before_import()?)
                } else {
                    None
                };
                let mut result = platform::transfer_data(import);
                if import && matches!(&result, Ok(Some(_))) {
                    self.config = config::load().0;
                    self.edits = Default::default();
                    let canvas = self.config.canvas.clone();
                    let _ = weak
                        .upgrade_in_event_loop(move |ui| crate::canvas::sync(&ui, &canvas, true));
                    self.recent = recent::Recent::load().0;
                    self.inventory = Inventory::load().0;
                    self.open_folder = None;
                    self.query.clear();
                    self.installed_filter.clear();
                    self.search_paths = None;
                    self.icons.clear();
                    if let Err(error) = self.load_icons() {
                        result = Err(error);
                    }
                }
                let (status, feedback) = match result {
                    Ok(Some(status)) => (
                        status,
                        Some((
                            if import {
                                "Data imported"
                            } else {
                                "Data exported"
                            },
                            true,
                        )),
                    ),
                    Ok(None) => (String::new(), None),
                    Err(error) => {
                        eprintln!("orbit backup transfer: {error}");
                        (
                            error,
                            Some((
                                if import {
                                    "Couldn't import data"
                                } else {
                                    "Couldn't export data"
                                },
                                false,
                            )),
                        )
                    }
                };
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    if let Err(error) = platform::restore_after_chooser(&launcher) {
                        eprintln!("orbit: {error}");
                        crate::toast::error(&launcher.as_weak(), "Couldn't return to the Launcher");
                    } else if let Some((message, true)) = feedback {
                        crate::toast::success(&launcher.as_weak(), message);
                    } else if let Some((message, false)) = feedback {
                        crate::toast::error(&launcher.as_weak(), message);
                    }
                    launcher.set_data_status(status.into());
                    launcher.set_data_busy(false);
                });
                Ok(())
            }
            Action::ChoosePath(directory, request) => {
                let Some(path) = platform::choose_path(directory) else {
                    return Ok(());
                };
                if !requests.is_current(request) {
                    return Ok(());
                }
                let path = path.to_string_lossy().into_owned();
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    launcher.invoke_path_chosen(path.into());
                });
                Ok(())
            }
            Action::SavePath(index, path, directory, svg, _) => {
                self.save_path(&index, path, directory, svg)
            }
        }
    }

    fn open_item(&mut self, id: &str, _weak: &slint::Weak<Launcher>) -> Result<(), String> {
        let Some(item) = self.config.items.iter().find(|item| item.id() == id) else {
            return Ok(());
        };
        match item {
            Item::App(app) => self
                .launch(app)
                .map_err(|error| format!("Couldn't start '{}': {error}", app.name)),
            Item::Website { name, url, .. } => {
                platform::open(url).map_err(|error| format!("Couldn't open '{}': {error}", name))
            }
            Item::File { name, path, .. } | Item::Directory { name, path, .. } => {
                platform::open(path).map_err(|error| format!("Couldn't open '{}': {error}", name))
            }
            Item::Robot { profile, .. } => {
                let profile = profile.clone();
                let _ = _weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<crate::RobotWorkbench>().set_adding(false);
                    ui.global::<crate::RobotWorkbench>()
                        .invoke_open(profile.into(), false);
                });
                Ok(())
            }
            Item::Widget { variant: 7, .. } => Ok(()),
            Item::Folder { .. } => {
                self.open_folder = Some(id.into());
                Ok(())
            }
            Item::Widget { .. } | Item::Annotation { .. } => Ok(()),
        }
    }

    fn launch_folder_app(&self, folder: &str, id: &str) -> Result<(), String> {
        let Some(Item::Folder { apps, .. }) =
            self.config.items.iter().find(|item| item.id() == folder)
        else {
            return Err("Folder is missing".into());
        };
        let app = apps
            .iter()
            .find(|app| app.id == id)
            .ok_or("Item is missing")?;
        self.launch(app)
            .map_err(|error| format!("Couldn't start '{}': {error}", app.name))
    }

    fn launch(&self, app: &App) -> Result<(), Box<dyn std::error::Error>> {
        match &app.launch {
            Launch::Command(command) => platform::launch(command),
            Launch::Installed(id) => {
                let installed = self
                    .inventory
                    .apps()
                    .iter()
                    .find(|installed| &installed.id == id)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::NotFound, "not found")
                    })?;
                platform::launch(&installed.command)
            }
            Launch::Action(action) => platform::launch_action(action),
            Launch::Website(path) | Launch::File(path) | Launch::Directory(path) => {
                platform::open(path)
            }
        }
    }

    fn add_system_action(
        &mut self,
        action: &str,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        let (_, name, action) = ACTIONS
            .iter()
            .find(|(id, _, _)| *id == action)
            .ok_or("Action is missing")?;
        let app = App {
            id: new_id()?,
            name: (*name).into(),
            icon: Icon::Letter,
            launch: Launch::Action(action.clone()),
        };
        if !requests.is_current(request) {
            return Ok(());
        }
        self.store("", Item::App(app))
    }

    fn add_installed_app(
        &mut self,
        id: &str,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        let Some(installed) = self
            .inventory
            .apps()
            .iter()
            .find(|app| app.id == id)
            .cloned()
        else {
            return Err("Installed app is missing".into());
        };
        let item_id = new_id()?;
        let app = App {
            id: item_id.clone(),
            name: installed.name.clone(),
            icon: installed.icon.clone(),
            launch: Launch::Installed(id.into()),
        };
        if !requests.is_current(request) {
            return Ok(());
        }
        self.store("", Item::App(app))?;
        let _ = self.load_icon(&item_id, &installed.icon);
        Ok(())
    }

    fn search_result_item(&self, reference: &ResultRef) -> Result<Item, String> {
        if reference.source < 0 {
            return self.cached_search_result(reference).map(App::into_item);
        }
        if !reference.folder_id.is_empty() {
            let Some(Item::Folder { apps, .. }) = self
                .config
                .items
                .iter()
                .find(|item| item.id() == reference.folder_id)
            else {
                return Err("Folder is missing".into());
            };
            return apps
                .iter()
                .find(|app| app.id == reference.id)
                .cloned()
                .map(App::into_item)
                .ok_or("Item is missing".into());
        }
        self.config
            .items
            .iter()
            .find(|item| item.id() == reference.id)
            .cloned()
            .ok_or("Item is missing".into())
    }

    fn cached_search_result(&self, reference: &ResultRef) -> Result<App, String> {
        let (name, icon, launch) = match reference.source {
            -1 => {
                let (_, name, action) = ACTIONS
                    .iter()
                    .find(|(id, _, _)| *id == reference.id)
                    .ok_or("Action is missing")?;
                ((*name).into(), Icon::Letter, Launch::Action(action.clone()))
            }
            -2 => {
                let app = self
                    .inventory
                    .apps()
                    .iter()
                    .find(|app| app.id == reference.id)
                    .ok_or("Installed app is missing")?;
                (
                    app.name.clone(),
                    app.icon.clone(),
                    Launch::Installed(app.id.clone()),
                )
            }
            -3 | -4 => {
                let path = self
                    .search_paths
                    .as_ref()
                    .and_then(|paths| paths.iter().find(|path| path.path == reference.id))
                    .ok_or("Path is missing")?;
                let launch = if path.directory {
                    Launch::Directory(path.path.clone())
                } else {
                    Launch::File(path.path.clone())
                };
                (path.name.clone(), Icon::Letter, launch)
            }
            _ => return Err("Item cannot be added".into()),
        };
        Ok(App {
            id: new_id()?,
            name,
            icon,
            launch,
        })
    }

    fn refresh_installed_apps(
        &mut self,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        self.inventory.refreshing();
        let mut installed_apps = platform::scan_installed_apps()
            .map_err(|error| format!("Couldn't refresh installed apps: {error}"))?;
        let extra = platform::scan_app_search_paths(&self.config.app_search_paths)
            .map_err(|error| format!("Couldn't scan app search paths: {error}"))?;
        for app in extra {
            if !installed_apps.iter().any(|saved| saved.id == app.id) {
                installed_apps.push(app);
            }
        }
        installed_apps.sort_by_key(|app| app.name.to_lowercase());
        if !requests.is_current(request) {
            return Ok(());
        }
        platform::write_installed_apps(
            &serde_json::to_vec_pretty(&installed_apps)
                .map_err(|error| format!("Couldn't save installed apps: {error}"))?,
        )
        .map_err(|error| format!("Couldn't save installed apps: {error}"))?;
        self.inventory.replace(installed_apps);
        self.search_paths = Some(index_search_paths(&self.config));
        self.icons.clear();
        let _ = self.load_icons();
        self.load_installed_icons();
        Ok(())
    }

    fn load_installed_icons(&mut self) {
        let icons: Vec<_> = self
            .inventory
            .apps()
            .iter()
            .filter(|app| !self.icons.contains_key(&app.id))
            .map(|app| (app.id.clone(), app.icon.clone()))
            .collect();
        for (id, icon) in icons {
            let _ = self.load_icon(&id, &icon);
        }
    }

    fn load_installed_icons_matching_query(&mut self) {
        let icons: Vec<_> = self
            .inventory
            .apps()
            .iter()
            .filter(|app| app.name.to_lowercase().contains(&self.query))
            .filter(|app| !self.icons.contains_key(&app.id))
            .map(|app| (app.id.clone(), app.icon.clone()))
            .collect();
        for (id, icon) in icons {
            let _ = self.load_icon(&id, &icon);
        }
    }

    fn edit_file_search_exclusion(&mut self, name: &str, remove: bool) -> Result<(), String> {
        let mut config = self.config.clone();
        if !edit_exclusions(&mut config.file_search_exclusions, name, remove) {
            return Ok(());
        }
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.search_paths = None;
        if !self.query.is_empty() {
            self.load_search_paths();
        }
        Ok(())
    }

    fn load_search_paths(&mut self) {
        if self.search_paths.is_none() {
            self.search_paths = Some(index_search_paths(&self.config));
        }
    }

    fn save_search_path(
        &mut self,
        directory: bool,
        index: i32,
        path: String,
    ) -> Result<(), String> {
        let path = PathBuf::from(path.trim());
        if !path.is_absolute() || !path.is_dir() {
            return Err("Enter an existing absolute folder path".into());
        }
        let path = path
            .canonicalize()
            .map_err(|error| format!("Couldn't read path: {error}"))?
            .to_string_lossy()
            .into_owned();
        let mut config = self.config.clone();
        let paths = if directory {
            &mut config.directory_search_paths
        } else {
            &mut config.file_search_paths
        };
        if index >= 0 {
            let Some(saved_path) = paths.get_mut(index as usize) else {
                return Ok(());
            };
            if *saved_path == path {
                return Ok(());
            }
            *saved_path = path;
        } else if !paths.contains(&path) {
            paths.push(path);
        } else {
            return Ok(());
        }
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.search_paths = None;
        if !self.query.is_empty() {
            self.load_search_paths();
        }
        Ok(())
    }

    fn delete_search_path(&mut self, directory: bool, index: usize) -> Result<(), String> {
        let mut config = self.config.clone();
        let paths = if directory {
            &mut config.directory_search_paths
        } else {
            &mut config.file_search_paths
        };
        if index >= paths.len() {
            return Ok(());
        }
        paths.remove(index);
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.search_paths = None;
        if !self.query.is_empty() {
            self.load_search_paths();
        }
        Ok(())
    }

    fn update_installed_apps(&self, weak: &slint::Weak<Launcher>) {
        let installed_apps: Vec<_> = self
            .inventory
            .apps()
            .iter()
            .filter(|app| {
                self.installed_filter.is_empty()
                    || app.name.to_lowercase().starts_with(&self.installed_filter)
            })
            .map(|app| {
                (
                    app.id.clone(),
                    app.name.clone(),
                    app.command.clone(),
                    self.icons.get(&app.id).cloned(),
                )
            })
            .collect();
        let has_match = !installed_apps.is_empty();
        let _ = weak.upgrade_in_event_loop(move |launcher| {
            let installed_apps = installed_apps
                .into_iter()
                .map(|(id, name, command, icon)| {
                    let icon_present = icon.is_some();
                    InstalledAppTile {
                        command: command.into(),
                        letter: name
                            .chars()
                            .next()
                            .map(|letter| letter.to_string())
                            .unwrap_or_default()
                            .into(),
                        id: id.into(),
                        name: name.into(),
                        icon: icon.map(slint::Image::from_rgba8).unwrap_or_default(),
                        icon_present,
                    }
                })
                .collect::<Vec<_>>();
            launcher
                .set_installed_apps(std::rc::Rc::new(slint::VecModel::from(installed_apps)).into());
            launcher.set_installed_apps_match(has_match);
        });
    }

    fn compute_search_results(&self) -> Vec<Tile> {
        let mut results = Vec::new();
        if self.query.is_empty() || self.query.starts_with('/') {
            return results;
        }
        let all = self.config.items.iter();
        let mut seen = HashSet::new();
        for (id, name, _) in ACTIONS.iter() {
            if name.to_lowercase().contains(&self.query) {
                seen.insert(name.trim().to_lowercase());
                results.push(Tile {
                    name: (*name).into(),
                    id: (*id).into(),
                    source: -1,
                    artwork: "action".into(),
                    app: true,
                    ..Tile::default()
                });
            }
        }
        for item in all.clone() {
            let mut push = |tile: Tile| {
                let key = tile.name.trim().to_lowercase();
                if let Some(existing) = results
                    .iter_mut()
                    .find(|t| t.name.trim().eq_ignore_ascii_case(&key))
                {
                    if !existing.pinned && tile.pinned {
                        *existing = tile;
                    }
                } else {
                    seen.insert(key);
                    results.push(tile);
                }
            };
            match item {
                Item::Folder { apps, .. } => {
                    for app in apps {
                        if app.name.to_lowercase().contains(&self.query)
                            && let Some(tile) = self.dock_tile(&app.id)
                        {
                            push(tile);
                        }
                    }
                }
                Item::Annotation { .. } | Item::Widget { .. } | Item::Robot { .. } => {}
                _ if item.name().to_lowercase().contains(&self.query) => {
                    push(self.tile(item));
                }
                _ => {}
            }
        }
        let mut seen_commands = HashSet::new();
        for app in self.inventory.apps() {
            let name = app.name.trim().to_lowercase();
            let cmd = app.command.trim();
            if app.name.to_lowercase().contains(&self.query)
                && seen.insert(name)
                && (cmd.is_empty() || seen_commands.insert(cmd))
            {
                results.push(Tile {
                    id: app.id.clone(),
                    name: app.name.clone(),
                    icon: self.icons.get(&app.id).cloned(),
                    source: -2,
                    command: Some(app.command.clone()),
                    app: true,
                    ..Tile::default()
                });
            }
        }
        for path in self.search_paths.iter().flatten() {
            if path.name.to_lowercase().contains(&self.query)
                && !all.clone().any(|item| match item {
                    Item::File { path: p, .. } | Item::Directory { path: p, .. } => p == &path.path,
                    _ => false,
                })
            {
                results.push(Tile {
                    name: path.name.clone(),
                    path: path.path.clone(),
                    source: if path.directory { -4 } else { -3 },
                    id: path.path.clone(),
                    file: !path.directory,
                    directory: path.directory,
                    ..Tile::default()
                });
            }
        }
        results
    }

    fn update_search_results(&self, weak: &slint::Weak<Launcher>) {
        let results = self.compute_search_results();
        let apply = move |launcher: Launcher| {
            launcher.set_search_results(
                std::rc::Rc::new(slint::VecModel::from(
                    results
                        .into_iter()
                        .map(Tile::into_slint)
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
        };
        if let Some(launcher) = weak.upgrade() {
            apply(launcher);
        } else {
            let _ = weak.upgrade_in_event_loop(apply);
        }
    }

    fn save(
        &mut self,
        id: &str,
        name: String,
        command: String,
        svg: String,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        if !add_item::valid_name(&name) {
            return Err("Name is empty".into());
        }
        let existing = self.existing(id)?;
        let app = prepare_app(existing, name, command, svg)?;
        if matches!(existing, Some(Item::App(old)) if old == &app) {
            return Ok(());
        }
        let item_id = app.id.clone();
        let icon = app.icon.clone();
        self.load_icon(&item_id, &icon)?;
        if !requests.is_current(request) {
            return Ok(());
        }
        self.store(id, Item::App(app))?;
        if matches!(icon, Icon::Letter | Icon::Text(_)) {
            self.icons.remove(&item_id);
        }
        Ok(())
    }

    fn existing(&self, id: &str) -> Result<Option<&Item>, String> {
        if id.is_empty() {
            return Ok(None);
        }
        self.config
            .items
            .iter()
            .find(|item| item.id() == id)
            .map(Some)
            .ok_or("Item changed before it could be saved".into())
    }

    fn edit_canvas(
        &mut self,
        command: i32,
        selection: &[String],
        x: i32,
        y: i32,
        weak: &slint::Weak<Launcher>,
    ) -> Result<(), String> {
        let mut config = self.config.clone();
        let mut reload_icons = false;
        let selected = match command {
            0 | 1 => {
                let Some(target) = self.edits.target(command == 1) else {
                    return Ok(());
                };
                target.restore(&mut config);
                reload_icons = config.items != self.config.items;
                let current = crate::canvas_edit::State::capture(&self.config);
                self.commit(config)?;
                self.edits.finish(current, command == 1);
                Vec::new()
            }
            2 | 3 => {
                let Some(clipboard) =
                    crate::canvas_edit::Clipboard::capture(&config, selection, command == 2)
                else {
                    return Ok(());
                };
                if command == 2 {
                    clipboard.cut(&mut config);
                    self.commit(config)?;
                }
                self.edits.clipboard = Some(clipboard);
                if command == 3 {
                    return Ok(());
                }
                Vec::new()
            }
            4 => {
                let Some(clipboard) = &self.edits.clipboard else {
                    return Ok(());
                };
                let selected = clipboard.paste(&mut config, x, y)?;
                reload_icons = true;
                self.commit(config)?;
                self.edits.clipboard.as_mut().unwrap().pasted();
                selected
            }
            _ => return Err("Unknown Canvas action".into()),
        };
        if reload_icons && let Err(error) = self.load_icons() {
            crate::toast::error(weak, error);
        }
        if command == 4
            && let Some(id) = selected.first()
        {
            self.reveal(id, weak);
        }
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.set_selected_ids(
                std::rc::Rc::new(slint::VecModel::from(
                    selected
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
        });
        Ok(())
    }

    fn commit(&mut self, mut config: Config) -> Result<(), String> {
        crate::spatial::reconcile(&mut config)?;
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        Ok(())
    }

    fn store(&mut self, id: &str, item: Item) -> Result<(), String> {
        let mut config = self.config.clone();
        if id.is_empty() {
            let span = crate::spatial::item_span(1, config.item_size, config.grid_gap);
            let rect = crate::spatial::nearby(
                &crate::spatial::occupied(&config),
                crate::spatial::Rect {
                    col: self.add_position.0,
                    row: self.add_position.1,
                    width: span,
                    height: span,
                },
            )?;
            config.positions.insert(item.id().into(), rect);
            config.items.push(item);
        } else {
            let current = config
                .items
                .iter_mut()
                .find(|item| item.id() == id)
                .ok_or("Item changed before it could be saved")?;
            if current == &item {
                return Ok(());
            }
            *current = item;
        }
        self.commit(config)
    }

    fn delete(&mut self, id: &str, weak: &slint::Weak<Launcher>) -> Result<(), String> {
        let Some(index) = self.config.items.iter().position(|item| item.id() == id) else {
            return Ok(());
        };
        let mut config = self.config.clone();
        let item = config.items.remove(index);
        config.positions.remove(id);
        config.pinned.retain(|pinned| pinned != id);
        let mut revealed = None;
        if let Item::Folder { apps, .. } = &item {
            let anchor = self.config.positions.get(id).copied().unwrap_or_default();
            for app in apps {
                let child = app.clone().into_item();
                if !config.pinned.contains(&app.id) {
                    let rect = crate::spatial::nearby(&crate::spatial::occupied(&config), anchor)?;
                    config.positions.insert(app.id.clone(), rect);
                    revealed = Some(app.id.clone());
                }
                config.items.push(child);
            }
        }
        self.commit(config)?;
        if let Some(id) = revealed {
            self.reveal(&id, weak);
        }
        self.icons.remove(id);
        if self.recent.remove(id)
            && let Err(error) = self.recent.save()
        {
            eprintln!("orbit: {error}");
        }
        // Saved icons may still be referenced by Undo or the Canvas clipboard.
        Ok(())
    }

    fn save_website(
        &mut self,
        id: &str,
        name: String,
        url: String,
        draft: crate::website_icons::Draft,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        if !add_item::valid_name(&name) {
            return Err("Name is empty".into());
        }
        if !add_item::valid_url(&url) {
            return Err("Enter a valid HTTP or HTTPS URL.".into());
        }
        let existing = self.existing(id)?;
        let item_id = match existing {
            Some(Item::Website { id, .. }) => id.clone(),
            Some(_) => return Err("Website is missing".into()),
            None => new_id()?,
        };
        let unchanged_url =
            matches!(existing, Some(Item::Website { url: old, .. }) if old == url.trim());
        let current = match existing {
            Some(Item::Website { icon, .. })
                if unchanged_url
                    && draft.automatic
                    && !draft.attempted
                    && crate::website_icons::automatic(
                        &serde_json::to_string(icon).unwrap_or_default(),
                    ) =>
            {
                Some(icon)
            }
            _ => None,
        };
        let new_file = draft.automatic && draft.fetched.is_some();
        let icon = crate::website_icons::save(draft, current)?;
        if !requests.is_current(request) {
            if new_file && let Icon::File(file) = &icon {
                platform::remove_icon(file);
            }
            return Ok(());
        }
        let item = Item::Website {
            id: item_id.clone(),
            name: name.trim().into(),
            url: url.trim().into(),
            icon: icon.clone(),
        };
        if let Err(error) = self.store(id, item) {
            if new_file && let Icon::File(file) = &icon {
                platform::remove_icon(file);
            }
            return Err(error);
        }
        if matches!(icon, Icon::Letter | Icon::Text(_)) {
            self.icons.remove(&item_id);
        }
        self.load_icon(&item_id, &icon)?;
        Ok(())
    }

    fn save_path(
        &mut self,
        id: &str,
        path: String,
        directory: bool,
        svg: String,
    ) -> Result<(), String> {
        let path = path.trim();
        let location = std::path::Path::new(path);
        if !location.is_absolute() {
            return Err("Enter an absolute Path".into());
        }
        let metadata = std::fs::metadata(location).map_err(|_| "Path does not exist")?;
        if directory && !metadata.is_dir() || !directory && !metadata.is_file() {
            return Err(if directory {
                "Path is not a directory"
            } else {
                "Path is not a file"
            }
            .into());
        }
        let name = location
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or(path)
            .to_string();
        if !add_item::valid_name(&name) {
            return Err("Name is empty".into());
        }
        let existing = self.existing(id)?;
        let item_id = match existing {
            Some(Item::File { id, .. }) if !directory => id.clone(),
            Some(Item::Directory { id, .. }) if directory => id.clone(),
            Some(_) => return Err("Item is missing".into()),
            None => new_id()?,
        };
        let name = name.trim().to_string();
        let path = path.to_string();
        let icon = crate::icons::parse(&svg)?;
        self.load_icon(&item_id, &icon)?;
        if matches!(icon, Icon::Letter | Icon::Text(_)) {
            self.icons.remove(&item_id);
        }
        let item = if directory {
            Item::Directory {
                id: item_id,
                name,
                path,
                icon,
            }
        } else {
            Item::File {
                id: item_id,
                name,
                path,
                icon,
            }
        };
        self.store(id, item)?;
        Ok(())
    }

    fn save_folder(
        &mut self,
        id: &str,
        name: String,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        if !add_item::valid_name(&name) {
            return Err("Name is empty".into());
        }
        let item = match self.existing(id)? {
            Some(Item::Folder { id, apps, .. }) => Item::Folder {
                id: id.clone(),
                name: name.trim().into(),
                apps: apps.clone(),
            },
            Some(_) => return Err("Folder is missing".into()),
            None => Item::Folder {
                id: new_id()?,
                name: name.trim().into(),
                apps: Vec::new(),
            },
        };
        if !requests.is_current(request) {
            return Ok(());
        }
        self.store(id, item)
    }

    fn save_shortcut(&mut self, action: i32, existing: String, keys: String) -> Result<(), String> {
        let mut config = self.config.clone();
        if config
            .frames
            .iter()
            .any(|frame| !keys.is_empty() && frame.keys == keys)
        {
            return Err("Shortcut is already used by a Frame.".into());
        }
        let binding = crate::shortcuts::Shortcut { action, keys };
        if existing.is_empty() {
            config.shortcuts.push(binding);
        } else if let Some(current) = config
            .shortcuts
            .iter_mut()
            .find(|entry| entry.action == action && entry.keys == existing)
        {
            *current = binding;
        } else {
            return Err("The shortcut changed before it could be saved.".into());
        }
        if !crate::shortcuts::valid(&config.shortcuts) {
            return Err("Choose a shortcut that is not already used.".into());
        }
        config.save().map_err(|error| error.to_string())?;
        self.config = config;
        Ok(())
    }

    fn save_frame(
        &mut self,
        id: &str,
        name: String,
        keys: String,
        selected: &[String],
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            return Err("Enter a Frame name (60 characters or fewer).".into());
        }
        let keys = keys.trim();
        let mut config = self.config.clone();
        if config
            .frames
            .iter()
            .any(|frame| frame.id != id && frame.name.trim().to_lowercase() == name.to_lowercase())
        {
            return Err("A Frame with that name already exists.".into());
        }
        crate::shortcuts::validate_frame_keys(keys, id, &config.shortcuts, &config.frames)?;
        if id.is_empty() {
            let existing: std::collections::HashSet<_> = config
                .items
                .iter()
                .map(|item| item.id().to_string())
                .collect();
            let mut members = Vec::new();
            for item in selected {
                if existing.contains(item) && !members.contains(item) {
                    members.push(item.clone());
                }
            }
            if members.is_empty() {
                return Err("Select at least one Canvas item for the Frame.".into());
            }
            config.frames.push(crate::config::CanvasFrame {
                id: new_id()?,
                name: name.into(),
                items: members,
                keys: keys.into(),
            });
        } else {
            let frame = config
                .frames
                .iter_mut()
                .find(|frame| frame.id == id)
                .ok_or("Frame changed before it could be saved.")?;
            frame.name = name.into();
            frame.keys = keys.into();
        }
        self.commit(config)
    }

    fn fit_frame(
        &mut self,
        id: &str,
        width: f32,
        height: f32,
        weak: &slint::Weak<Launcher>,
    ) -> Result<(), String> {
        let frame = self
            .config
            .frames
            .iter()
            .find(|frame| frame.id == id)
            .ok_or("Frame is missing")?;
        let positions = frame
            .items
            .iter()
            .filter_map(|id| {
                self.config
                    .positions
                    .get(id)
                    .map(|rect| (id.clone(), *rect))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let view = crate::canvas::fit(&positions, width, height)
            .ok_or("Frame has no items on the Canvas")?;
        crate::canvas::apply(&mut self.config, crate::canvas::Mutation::View(view))?;
        let canvas = self.config.canvas.clone();
        weak.upgrade_in_event_loop(move |ui| crate::canvas::sync(&ui, &canvas, true))
            .map_err(|error| error.to_string())
    }

    fn save_annotation(
        &mut self,
        id: &str,
        text: String,
        kind: i32,
        style: crate::ink::Style,
        position: (i32, i32),
        weak: &slint::Weak<Launcher>,
    ) -> Result<(), String> {
        let (x, y) = position;
        if !(1..=3).contains(&kind)
            || !style.valid()
            || (kind != 3 && text.trim().is_empty())
            || text.chars().count() > 10_000
        {
            return Err("Enter text (10,000 characters or fewer) and a valid style".into());
        }
        let mut config = self.config.clone();
        if !id.is_empty() {
            let Some(Item::Annotation {
                name, style: saved, ..
            }) = config.items.iter_mut().find(|item| item.id() == id)
            else {
                return Err("Canvas item is missing".into());
            };
            *name = text;
            *saved = style;
        } else {
            let id = new_id()?;
            let mut image = String::new();
            let mut name = text;
            let (width, height) = if kind == 3 {
                let Some(path) = platform::choose_image("Choose image") else {
                    return Ok(());
                };
                let (file, data) = platform::read_chosen_icon(&path.to_string_lossy())?;
                if file.ends_with(".svg")
                    && !add_item::valid_svg(std::str::from_utf8(&data).unwrap_or_default())
                {
                    return Err("Choose a safe image".into());
                }
                let pixels = canvas_image(&data, &file)?;
                let ratio = pixels.height() as f32 / pixels.width().max(1) as f32;
                platform::write_icon(&file, &data).map_err(|e| e.to_string())?;
                self.icons.insert(id.clone(), pixels);
                name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                image = file;
                (400, (400.0 * ratio).clamp(48.0, 1200.0) as usize)
            } else if kind == 1 {
                (240, 240)
            } else {
                (
                    320,
                    (name.lines().count().max(1) as f32 * style.size * 1.5 + 32.0).max(80.0)
                        as usize,
                )
            };
            let rect = crate::spatial::nearby(
                &crate::spatial::occupied(&config),
                crate::spatial::Rect {
                    col: x,
                    row: y,
                    width,
                    height,
                },
            )?;
            config.positions.insert(id.clone(), rect);
            config.items.push(Item::Annotation {
                id: id.clone(),
                name,
                kind,
                style,
                image,
            });
            self.reveal(&id, weak);
        }
        self.commit(config)?;
        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_annotation_editor(false);
            ui.set_tool_mode(0);
        });
        Ok(())
    }

    fn add_ink(
        &mut self,
        mut points: Vec<crate::config::InkPoint>,
        style: crate::ink::Style,
    ) -> Result<(), String> {
        if !style.valid() {
            return Err("Invalid ink style".into());
        }
        crate::ink::normalize(&mut points);
        if points.len() < 2 {
            return Ok(());
        }
        let total = self
            .config
            .ink
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>()
            + points.len();
        if total > crate::ink::MAX_POINTS {
            return Err("Canvas ink limit reached; erase some strokes first.".into());
        }
        let mut config = self.config.clone();
        config.ink.push(crate::config::InkStroke {
            id: new_id()?,
            style,
            points,
        });
        self.commit(config)
    }

    fn erase_ink(&mut self, points: &[crate::config::InkPoint]) -> Result<(), String> {
        let mut config = self.config.clone();
        let before = config.items.len();
        config.items.retain(|item| {
            !matches!(item, Item::Annotation { .. })
                || !config
                    .positions
                    .get(item.id())
                    .is_some_and(|rect| crate::ink::hits_rect(points, *rect))
        });
        let removed_ink = crate::ink::erase(&mut config.ink, points, 12.0);
        if removed_ink || before != config.items.len() {
            self.commit(config)?;
        }
        Ok(())
    }

    fn unpin(&mut self, id: &str, weak: &slint::Weak<Launcher>) -> Result<(), String> {
        if !self.config.pinned.iter().any(|pinned| pinned == id) {
            return Ok(());
        }
        let reference = self.dock_tile(id).ok_or("Item is missing")?;
        let item = self.search_result_item(&ResultRef {
            id: id.into(),
            source: 0,
            folder_id: reference.folder_id,
        })?;
        let mut config = self.config.clone();
        let span = crate::spatial::item_span(1, config.item_size, config.grid_gap);
        let rect = crate::spatial::Rect {
            col: self.add_position.0,
            row: self.add_position.1,
            width: span,
            height: span,
        };
        if !crate::item_drop::place_search_result(&mut config, item, Some(rect), false) {
            return Err("Couldn't return item to Canvas".into());
        }
        self.commit(config)?;
        self.reveal(id, weak);
        Ok(())
    }

    fn drop_item(
        &mut self,
        id: &str,
        col: i32,
        row: i32,
        into: &str,
        weak: &slint::Weak<Launcher>,
    ) -> Result<(), String> {
        let mut config = self.config.clone();
        if !crate::item_drop::apply(&mut config, id, col, row, into, new_id()?) {
            return Ok(());
        }
        self.commit(config)?;
        if self.open_folder.as_deref() != Some(into) {
            self.open_folder = None;
        }
        if into.is_empty() {
            self.reveal(id, weak);
        }
        Ok(())
    }

    fn reveal(&mut self, id: &str, _weak: &slint::Weak<Launcher>) {
        self.pending_reveal = Some(id.into());
    }

    fn publish_reveal(&mut self, weak: &slint::Weak<Launcher>) {
        let Some(id) = self.pending_reveal.take() else {
            return;
        };
        let Some(rect) = self.config.positions.get(&id).copied() else {
            return;
        };
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.invoke_reveal_item(rect.col, rect.row, rect.width as i32, rect.height as i32)
        });
    }

    fn tile(&self, item: &Item) -> Tile {
        let mut tile = Tile::new(
            item,
            &self.icons,
            self.config.item_size,
            self.config.grid_gap,
        );
        tile.rect = self
            .config
            .positions
            .get(item.id())
            .copied()
            .unwrap_or(crate::spatial::Rect {
                width: crate::spatial::item_span(1, self.config.item_size, self.config.grid_gap),
                height: crate::spatial::item_span(1, self.config.item_size, self.config.grid_gap),
                ..Default::default()
            });
        tile.pinned = self.config.pinned.iter().any(|id| id == item.id());
        tile
    }

    fn dock_tile(&self, id: &str) -> Option<Tile> {
        for item in &self.config.items {
            if item.id() == id {
                return Some(self.tile(item));
            }
            if let Item::Folder { apps, .. } = item
                && let Some(app) = apps.iter().find(|app| app.id == id)
            {
                let mut tile = Tile::new(
                    &app.clone().into_item(),
                    &self.icons,
                    self.config.item_size,
                    self.config.grid_gap,
                );
                tile.folder_id = item.id().into();
                tile.pinned = self.config.pinned.iter().any(|pinned| pinned == id);
                return Some(tile);
            }
        }
        None
    }

    fn update(&self, weak: &slint::Weak<Launcher>) {
        let canvas = self.config.canvas.clone();
        let _ = weak
            .upgrade_in_event_loop(move |launcher| crate::canvas::sync(&launcher, &canvas, false));
        let items = self
            .config
            .items
            .iter()
            .map(|item| self.tile(item))
            .collect::<Vec<_>>();
        let size = self.config.item_size;
        let gap = self.config.grid_gap;
        let canvas_speed = self.config.canvas_speed;
        let font_size = self.config.font_size;
        let smooth_cursor = self.config.smooth_cursor;
        let english = self.config.english_assistance.clone();
        let ai = self.config.ai.clone();
        let layout_locked = self.config.layout_locked;
        let search_engine_index = self.config.default_search_engine_index() as i32;
        let app_search_paths = self
            .config
            .app_search_paths
            .iter()
            .map(|path| path.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        let file_search_paths = self
            .config
            .file_search_paths
            .iter()
            .map(|path| path.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        let file_search_exclusions = self
            .config
            .file_search_exclusions
            .iter()
            .map(|name| name.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        let directory_search_paths = self
            .config
            .directory_search_paths
            .iter()
            .map(|path| path.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        let can_undo = self.edits.target(false).is_some();
        let can_redo = self.edits.target(true).is_some();
        let can_paste = self.edits.clipboard.is_some();
        let frames = self
            .config
            .frames
            .iter()
            .filter_map(|frame| {
                let mut bounds: Option<(f32, f32, f32, f32)> = None;
                for id in &frame.items {
                    let Some(rect) = self.config.positions.get(id) else {
                        continue;
                    };
                    let (left, top) = (rect.col as f32, rect.row as f32);
                    let (right, bottom) = (left + rect.width as f32, top + rect.height as f32);
                    bounds = Some(match bounds {
                        Some((l, t, r, b)) => {
                            (l.min(left), t.min(top), r.max(right), b.max(bottom))
                        }
                        None => (left, top, right, bottom),
                    });
                }
                let (x, y, right, bottom) = bounds?;
                Some(CanvasFrameView {
                    id: frame.id.as_str().into(),
                    name: frame.name.as_str().into(),
                    keys: frame.keys.as_str().into(),
                    x,
                    y,
                    width: right - x,
                    height: bottom - y,
                    count: frame.items.len() as i32,
                })
            })
            .collect::<Vec<_>>();
        let ink = self
            .config
            .ink
            .iter()
            .map(|stroke| {
                let ink = crate::ink::styled_view(&stroke.points, &stroke.style);
                (
                    ink.x,
                    ink.y,
                    ink.width,
                    ink.height,
                    ink.path,
                    crate::ink::samples(&stroke.points, &stroke.style),
                    stroke.style.clone(),
                )
            })
            .collect::<Vec<_>>();
        let dock: Vec<_> = self
            .config
            .pinned
            .iter()
            .filter_map(|id| self.dock_tile(id))
            .collect();
        let recent: Vec<_> = self
            .recent
            .ids
            .iter()
            .filter(|id| !self.config.pinned.contains(id))
            .filter_map(|id| self.dock_tile(id))
            .filter(|tile| !tile.folder)
            .take(RECENT_DOCK_LIMIT)
            .collect();
        let results = self.compute_search_results();
        let installed_apps: Vec<_> = self
            .inventory
            .apps()
            .iter()
            .filter(|app| {
                self.installed_filter.is_empty()
                    || app.name.to_lowercase().starts_with(&self.installed_filter)
            })
            .map(|app| {
                (
                    app.id.clone(),
                    app.name.clone(),
                    app.command.clone(),
                    self.icons.get(&app.id).cloned(),
                )
            })
            .collect();
        let shortcuts = self.config.shortcuts.clone();
        let scripts = crate::scripts::sorted(&self.config.scripts);
        let user_actions = self
            .config
            .user_actions
            .iter()
            .map(|action| UserActionChoice {
                id: action.id.clone().into(),
                name: action.name.clone().into(),
                kind: match action.kind {
                    ActionKind::Url => "URL",
                    ActionKind::Shell => "Shell",
                    ActionKind::TickTick => "TickTick",
                    ActionKind::Ai => "AI",
                }
                .into(),
                source: action.source.clone().into(),
                tokens: self
                    .config
                    .prefixes
                    .iter()
                    .filter(|prefix| prefix.action_id == action.id)
                    .map(|prefix| prefix.token.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .into(),
                description: action.description.clone().into(),
                referenced: self
                    .config
                    .prefixes
                    .iter()
                    .any(|prefix| prefix.action_id == action.id),
            })
            .collect::<Vec<_>>();
        let action_names = self
            .config
            .user_actions
            .iter()
            .map(|action| action.name.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        let prefixes = self
            .config
            .prefixes
            .iter()
            .map(|prefix| PrefixChoice {
                id: prefix.id.clone().into(),
                name: prefix.name.clone().into(),
                token: prefix.token.clone().into(),
                action_id: prefix.action_id.clone().into(),
                action_name: self
                    .config
                    .user_actions
                    .iter()
                    .find(|action| action.id == prefix.action_id)
                    .map(|action| action.name.as_str().into())
                    .unwrap_or_default(),
                action_index: self
                    .config
                    .user_actions
                    .iter()
                    .position(|action| action.id == prefix.action_id)
                    .unwrap_or(0) as i32,
            })
            .collect::<Vec<_>>();
        let shortcut_hints: Vec<slint::SharedString> = (0..14)
            .map(|action| {
                shortcuts
                    .iter()
                    .find(|binding| binding.action == action)
                    .map_or_else(slint::SharedString::default, |binding| {
                        binding.keys.as_str().into()
                    })
            })
            .collect();
        let shortcut_groups = (0..25)
            .map(|action| {
                (
                    action,
                    shortcuts
                        .iter()
                        .filter(|binding| binding.action == action)
                        .map(|binding| binding.keys.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let installed_status = self.inventory.status().to_string();
        let installed_apps_loaded = self.inventory.loaded();
        let installed_apps_match = !installed_apps.is_empty();
        let folder = self.open_folder.as_ref().and_then(|id| {
            let Item::Folder { name, apps, .. } =
                self.config.items.iter().find(|item| item.id() == id)?
            else {
                return None;
            };
            Some((
                id.clone(),
                name.clone(),
                apps.iter()
                    .map(|app| {
                        let mut tile = Tile::new(
                            &app.clone().into_item(),
                            &self.icons,
                            self.config.item_size,
                            self.config.grid_gap,
                        );
                        tile.folder_id = id.clone();
                        tile
                    })
                    .collect::<Vec<_>>(),
            ))
        });
        let background_folder = self.config.background_folder.clone();
        let background_image =
            read_background_image(PathBuf::from(&self.config.background_image).as_path());
        let apply = move |launcher: Launcher| {
            let installed_apps: Vec<_> = installed_apps
                .into_iter()
                .map(|(id, name, command, icon)| {
                    let icon_present = icon.is_some();
                    InstalledAppTile {
                        command: command.into(),
                        letter: name
                            .chars()
                            .next()
                            .map(|letter| letter.to_string())
                            .unwrap_or_default()
                            .into(),
                        id: id.into(),
                        name: name.into(),
                        icon: icon.map(slint::Image::from_rgba8).unwrap_or_default(),
                        icon_present,
                    }
                })
                .collect();
            set_items(&launcher, items);
            launcher.set_dock_items(
                std::rc::Rc::new(slint::VecModel::from(
                    dock.into_iter().map(Tile::into_slint).collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher.set_recent_items(
                std::rc::Rc::new(slint::VecModel::from(
                    recent.into_iter().map(Tile::into_slint).collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher.set_search_results(
                std::rc::Rc::new(slint::VecModel::from(
                    results
                        .into_iter()
                        .map(Tile::into_slint)
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher.set_actions(
                std::rc::Rc::new(slint::VecModel::from(
                    ACTIONS
                        .iter()
                        .map(|(id, name, _)| ActionChoice {
                            id: (*id).into(),
                            name: (*name).into(),
                        })
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher
                .set_installed_apps(std::rc::Rc::new(slint::VecModel::from(installed_apps)).into());
            launcher.set_item_size(size);
            launcher.set_grid_gap(gap);
            launcher.set_canvas_speed(canvas_speed);
            launcher.set_font_size(font_size);
            launcher.set_background_folder_input(background_folder.into());
            if let Some(image) = background_image {
                launcher.set_background_image(slint::Image::from_rgba8(image));
                launcher.set_background_image_active(true);
            } else {
                launcher.set_background_image_active(false);
            }
            launcher.set_search_engine_names(
                std::rc::Rc::new(slint::VecModel::from(
                    Config::search_engine_names()
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher.set_smooth_cursor(smooth_cursor);
            ai.update_ui(&launcher);
            launcher.set_english_model(english.model.into());
            launcher.set_english_suggestions_prompt(english.suggestions_prompt.into());
            launcher.set_english_completion_prompt(english.completion_prompt.into());
            launcher.set_english_grammar_prompt(english.grammar_prompt.into());
            launcher.set_english_address(english.address.into());
            launcher.set_english_suggestions_enabled(english.suggestions);
            launcher.set_english_autocomplete_enabled(english.autocomplete);
            launcher.set_english_grammar_enabled(english.grammar);
            launcher.set_layout_locked(layout_locked);
            launcher.set_default_search_engine_index(search_engine_index);
            launcher.invoke_default_search_engine_saved(search_engine_index);
            launcher.set_shortcuts(crate::shortcuts::model(shortcuts));
            launcher
                .set_shortcut_hints(std::rc::Rc::new(slint::VecModel::from(shortcut_hints)).into());
            launcher.set_scripts(std::rc::Rc::new(slint::VecModel::from(scripts)).into());
            launcher.set_user_actions(std::rc::Rc::new(slint::VecModel::from(user_actions)).into());
            launcher.set_action_names(std::rc::Rc::new(slint::VecModel::from(action_names)).into());
            launcher.set_prefixes(std::rc::Rc::new(slint::VecModel::from(prefixes)).into());
            launcher.set_shortcut_groups(
                std::rc::Rc::new(slint::VecModel::from(
                    shortcut_groups
                        .into_iter()
                        .map(|(action, keys)| crate::ShortcutGroup {
                            action,
                            entries: std::rc::Rc::new(slint::VecModel::from(
                                keys.into_iter()
                                    .map(|keys| crate::ShortcutEntry {
                                        action,
                                        keys: keys.into(),
                                    })
                                    .collect::<Vec<_>>(),
                            ))
                            .into(),
                        })
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher.set_installed_status(installed_status.into());
            launcher.set_installed_apps_loaded(installed_apps_loaded);
            launcher.set_installed_apps_match(installed_apps_match);
            launcher.set_app_search_paths(
                std::rc::Rc::new(slint::VecModel::from(app_search_paths)).into(),
            );
            launcher.set_file_search_paths(
                std::rc::Rc::new(slint::VecModel::from(file_search_paths)).into(),
            );
            launcher.set_file_search_exclusions(
                std::rc::Rc::new(slint::VecModel::from(file_search_exclusions)).into(),
            );
            launcher.set_directory_search_paths(
                std::rc::Rc::new(slint::VecModel::from(directory_search_paths)).into(),
            );
            launcher.set_frames(std::rc::Rc::new(slint::VecModel::from(frames)).into());
            launcher.set_can_undo(can_undo);
            launcher.set_can_redo(can_redo);
            launcher.set_can_paste(can_paste);
            launcher.set_ink_strokes(
                std::rc::Rc::new(slint::VecModel::from(
                    ink.into_iter()
                        .map(
                            |(x, y, width, height, path, samples, style)| crate::CanvasInk {
                                x,
                                y,
                                width,
                                height,
                                path,
                                color: parse_color(&style.color),
                                size: if style.highlighter {
                                    style.size * 4.0
                                } else {
                                    style.size
                                },
                                opacity: style.opacity * if style.highlighter { 0.35 } else { 1.0 },
                                dimmed: false,
                                samples: std::rc::Rc::new(slint::VecModel::from(
                                    samples
                                        .into_iter()
                                        .map(|point| crate::InkSample {
                                            x: point.x,
                                            y: point.y,
                                        })
                                        .collect::<Vec<_>>(),
                                ))
                                .into(),
                            },
                        )
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            if let Some((index, name, items)) = folder {
                launcher.set_folder_id(index.into());
                launcher.set_folder_name(name.into());
                let items = items.into_iter().map(Tile::into_slint).collect::<Vec<_>>();
                launcher.set_folder_items(std::rc::Rc::new(slint::VecModel::from(items)).into());
                launcher.set_folder_open(true);
            } else {
                launcher.set_folder_open(false);
            }
        };
        if let Some(launcher) = weak.upgrade() {
            apply(launcher);
        } else {
            let _ = weak.upgrade_in_event_loop(apply);
        }
    }
}

fn set_items(launcher: &Launcher, items: Vec<Tile>) {
    launcher.set_items(
        std::rc::Rc::new(slint::VecModel::from(
            items.into_iter().map(Tile::into_slint).collect::<Vec<_>>(),
        ))
        .into(),
    );
}

fn prepare_app(
    existing: Option<&Item>,
    name: String,
    command: String,
    svg: String,
) -> Result<App, String> {
    let (id, launch) = match existing {
        Some(Item::App(app)) => (
            app.id.clone(),
            match &app.launch {
                Launch::Installed(id) => Launch::Installed(id.clone()),
                Launch::Action(action) => Launch::Action(action.clone()),
                _ => Launch::Command(command),
            },
        ),
        Some(Item::Folder { .. }) => return Err("Open the folder to edit its apps".into()),
        Some(_) => return Err("Item is missing".into()),
        None => (new_id()?, Launch::Command(command)),
    };
    if let Launch::Command(command) = &launch
        && !add_item::valid_command(command)
    {
        return Err("Command is empty or has an unclosed quote".into());
    }
    Ok(App {
        id,
        name: name.trim().into(),
        icon: crate::icons::parse(&svg)?,
        launch,
    })
}

pub(crate) fn new_id() -> Result<String, String> {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    Ok(format!("{}-{}", std::process::id(), time.as_nanos()))
}

pub(crate) fn decode_svg(svg: &str) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
    let image =
        slint::Image::load_from_svg_data(svg.as_bytes()).map_err(|_| "Couldn't read SVG icon")?;
    image
        .to_rgba8()
        .ok_or_else(|| "Couldn't read SVG icon".into())
}

pub(crate) fn decode_icon(
    data: &[u8],
    name: &str,
) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or("Couldn't read website icon")?;
    if extension == "ico" {
        let image = image::load_from_memory_with_format(data, image::ImageFormat::Ico)
            .map_err(|_| "Couldn't read website icon")?
            .to_rgba8();
        return Ok(SharedPixelBuffer::clone_from_slice(
            image.as_raw(),
            image.width(),
            image.height(),
        ));
    }
    let image = slint::Image::load_from_data(data, Some(extension))
        .map_err(|_| "Couldn't read website icon")?;
    if extension == "svg" {
        // Small intrinsic SVG dimensions must not turn a scalable icon into a tiny bitmap.
        i_slint_core::graphics::image_to_rgba8_with_target_size(
            &image,
            i_slint_core::graphics::IntSize::new(80, 80),
        )
    } else {
        image.to_rgba8()
    }
    .ok_or_else(|| "Couldn't read website icon".into())
}

fn read_background_image(path: &std::path::Path) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    if path.as_os_str().is_empty() {
        return None;
    }
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(PathBuf, Option<SharedPixelBuffer<Rgba8Pixel>>)>> =
        Mutex::new(None);
    if let Ok(guard) = CACHE.lock()
        && let Some((cached_path, cached_img)) = &*guard
        && cached_path == path
    {
        return cached_img.clone();
    }
    let data = std::fs::read(path).ok()?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let image = if extension == "svg" {
        slint::Image::load_from_svg_data(&data).ok()?
    } else {
        slint::Image::load_from_data(&data, Some(&extension)).ok()?
    };
    let buf = image.to_rgba8();
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((path.to_path_buf(), buf.clone()));
    }
    buf
}

fn success_message(action: &Action) -> Option<&'static str> {
    match action {
        Action::SaveDefaultSearchEngine(..) => Some("Search engine saved"),
        Action::RunPrefix(..) => Some("Action completed"),
        Action::SaveNamedCommand(..) => Some("Action saved"),
        Action::DeleteNamedCommand(..) => Some("Action deleted"),
        Action::SavePrefix(..) => Some("Prefix saved"),
        Action::DeletePrefix(..) => Some("Prefix removed"),
        Action::Pin(_) => Some("Added to Dock"),
        Action::Unpin(_) => Some("Removed from Dock"),
        Action::RemoveRecent(_) => Some("Removed from Recent"),
        Action::Save(index, ..) => Some(if index.is_empty() {
            "App added"
        } else {
            "App saved"
        }),
        Action::SaveWebsite(index, ..) => Some(if index.is_empty() {
            "Website added"
        } else {
            "Website saved"
        }),
        Action::SavePath(index, ..) => Some(if index.is_empty() {
            "Item added"
        } else {
            "Item saved"
        }),
        Action::SaveFolder(index, ..) => Some(if index.is_empty() {
            "Folder added"
        } else {
            "Folder saved"
        }),
        Action::Delete(_) => Some("Item removed"),
        Action::Drop(..) => Some("Item moved"),
        Action::Widget(..) => Some("Widget added"),
        Action::ChangeWidget(..) => Some("Widget resized"),
        Action::DuplicateWidget(..) => Some("Widget duplicated"),
        Action::SelectTickTickList(..) => Some("Task list changed"),
        Action::AddPredefined(..) | Action::AddInstalledApp(..) => Some("App added"),
        Action::SaveSearchPath(..) => Some("Search path saved"),
        Action::DeleteSearchPath(..) => Some("Search path removed"),
        Action::SaveShortcut(..) => Some("Shortcut saved"),
        Action::SaveAppearance(..) => Some("Appearance saved"),
        Action::SaveBackgroundFolder(..) => Some("Background image folder saved"),
        Action::ChangeBackground => Some("Background changed"),
        _ => None,
    }
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn url_action_url(template: &str, query: &str) -> Result<String, String> {
    if !template.contains("%s") {
        return Err("URL Action template must contain %s".into());
    }
    Ok(template.replace("%s", &url_encode(query)))
}

fn browser_search_url(template: &str, query: &str) -> Result<String, String> {
    let lower = query.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return Ok(query.to_owned());
    }
    let host = query.split(['/', '?', '#']).next().unwrap_or("");
    if !query.is_empty()
        && !query.chars().any(char::is_whitespace)
        && host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
    {
        return Ok(format!("https://{query}"));
    }
    url_action_url(template, query)
}

fn match_prefix(prefixes: &[Prefix], text: &str) -> Option<PrefixMatch> {
    if text.starts_with('/') {
        return None;
    }
    let prefix = prefixes.iter().find(|prefix| {
        text.starts_with(&prefix.token)
            && (text.len() == prefix.token.len()
                || text[prefix.token.len()..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace))
    })?;
    let query = text[prefix.token.len()..].trim_start();
    Some(PrefixMatch {
        action_id: prefix.action_id.clone().into(),
        name: prefix.name.clone().into(),
        query: query.into(),
        query_length: query.len() as i32,
        badge_width: (prefix.name.chars().count() * 7 + 18) as i32,
    })
}

pub(crate) struct TickTickDraft {
    pub title: String,
    pub priority: i32,
    pub tags: Vec<String>,
    pub list: Option<String>,
    pub today: bool,
}

pub(crate) fn parse_ticktick_query(query: &str) -> Result<TickTickDraft, String> {
    let mut title = Vec::new();
    let mut list_words = Vec::new();
    let mut selecting_list = false;
    let mut list_name = None;
    let mut priority = 0;
    let mut tags = Vec::new();
    let mut today = false;
    for word in query.split_whitespace() {
        if let Some(value) = word.strip_prefix('!') {
            selecting_list = false;
            priority = match value.to_ascii_lowercase().as_str() {
                "high" => 5,
                "medium" => 3,
                "low" => 1,
                "none" => 0,
                "today" | "!" => {
                    today = true;
                    continue;
                }
                _ => {
                    title.push(word);
                    continue;
                }
            };
        } else if let Some(tag) = word.strip_prefix('#') {
            selecting_list = false;
            if !tag.is_empty() {
                tags.push(tag.to_owned());
            }
        } else if let Some(list) = word.strip_prefix('^').or_else(|| word.strip_prefix('~')) {
            if !list.is_empty() {
                list_words.push(list.to_owned());
            }
            selecting_list = true;
        } else if selecting_list {
            list_words.push(word.to_owned());
        } else {
            title.push(word);
        }
    }
    if !list_words.is_empty() {
        list_name = Some(list_words.join(" "));
    }
    let title = title.join(" ");
    if title.trim().is_empty() {
        return Err("Enter a task title".into());
    }
    Ok(TickTickDraft {
        title,
        priority,
        tags,
        list: list_name,
        today,
    })
}

fn resolve_ticktick_project<'a>(
    list: Option<&str>,
    projects: &'a [crate::ticktick::Project],
    inbox: &'a crate::ticktick::Project,
) -> Result<&'a crate::ticktick::Project, String> {
    let Some(list) = list else {
        return Ok(inbox);
    };
    let index = crate::ticktick::resolve_list_name(
        projects
            .iter()
            .chain(std::iter::once(inbox))
            .map(|project| &project.name),
        list,
    )?;
    Ok(projects.get(index).unwrap_or(inbox))
}

fn create_ticktick_task(query: &str) -> Result<crate::ticktick::Task, String> {
    let draft = parse_ticktick_query(query)?;
    let token = crate::ticktick::load_token()?.ok_or("Connect TickTick in Settings first")?;
    let projects = crate::ticktick::projects(&token)?;
    let inbox = crate::ticktick::inbox_project(&token)?;
    let project = resolve_ticktick_project(draft.list.as_deref(), &projects, &inbox)?;
    let due_date = draft
        .today
        .then(|| format!("{}T00:00:00+0000", chrono::Local::now().format("%Y-%m-%d")));
    crate::ticktick::create_task_with_details(
        &token,
        &project.id,
        &draft.title,
        "",
        draft.priority,
        &draft.tags,
        due_date.as_deref(),
    )
}

fn finish(
    weak: &slint::Weak<Launcher>,
    banner: String,
    saved: bool,
    launched: bool,
    feedback: Option<&'static str>,
    startup: bool,
) {
    let _ = weak.upgrade_in_event_loop(move |launcher| {
        let operation = launcher.get_sheet_operation();
        let failed = !banner.is_empty();
        let restore_error = if operation == "choosing" {
            platform::restore_after_chooser(&launcher)
                .err()
                .map(|error| error.to_string())
        } else {
            None
        };
        let restore_failed = restore_error.is_some();
        if !operation.is_empty() {
            if let Some(error) = restore_error {
                launcher.set_sheet_status(error.into());
            } else if !banner.is_empty() {
                eprintln!("orbit: {banner}");
                launcher.set_sheet_status(banner.into());
            } else if matches!(operation.as_str(), "choosing" | "choosing-background") {
                launcher.set_sheet_status("".into());
            }
            launcher.set_sheet_operation("".into());
            launcher.set_banner("".into());
        } else if !banner.is_empty() {
            eprintln!("orbit: {banner}");
            launcher.set_banner("".into());
            if startup {
                crate::toast::warning(
                    &launcher.as_weak(),
                    "Some saved information couldn't be loaded",
                );
            } else {
                crate::toast::error(&launcher.as_weak(), "Couldn't complete that action");
            }
        } else {
            launcher.set_banner("".into());
        }
        if !failed
            && !restore_failed
            && let Some(message) = feedback
        {
            crate::toast::success(&launcher.as_weak(), message);
        }
        launcher.set_busy(false);
        if saved {
            launcher.invoke_reset_launcher();
        }
        if launched && let Err(error) = platform::hide(&launcher) {
            eprintln!("orbit: {error}");
            crate::toast::error(&launcher.as_weak(), "Couldn't hide the Launcher");
        }
    });
}

fn edit_exclusions(entries: &mut Vec<String>, name: &str, remove: bool) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    if remove {
        let before = entries.len();
        entries.retain(|entry| entry != name);
        before != entries.len()
    } else if entries.iter().any(|entry| entry == name) {
        false
    } else {
        entries.push(name.to_owned());
        true
    }
}

// Only '*' is special; matching a whole name cannot cross a path separator.
fn exclusion_matches(pattern: &str, name: &str) -> bool {
    let (pattern, name) = (pattern.as_bytes(), name.as_bytes());
    let (mut p, mut n) = (0, 0);
    let mut star = None;
    let mut retry = 0;
    while n < name.len() {
        if p < pattern.len() && pattern[p] != b'*' && pattern[p] == name[n] {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            retry = n;
        } else if let Some(index) = star {
            retry += 1;
            n = retry;
            p = index + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

fn index_search_paths(config: &Config) -> Vec<SearchPathResult> {
    let mut results = Vec::new();
    for root in &config.file_search_paths {
        let mut folders = vec![PathBuf::from(root)];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if config
                    .file_search_exclusions
                    .iter()
                    .any(|pattern| exclusion_matches(pattern, &name))
                {
                    continue;
                }
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    folders.push(entry.path());
                } else if kind.is_file() {
                    results.push(search_path_result(entry.path(), false));
                }
            }
        }
    }
    for root in &config.directory_search_paths {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        results.extend(entries.flatten().filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(std::fs::FileType::is_dir)
                .map(|_| search_path_result(entry.path(), true))
        }));
    }
    results.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.path.cmp(&right.path))
    });
    results.dedup_by(|left, right| left.path == right.path);
    results
}

fn search_path_result(path: PathBuf, directory: bool) -> SearchPathResult {
    SearchPathResult {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        path: path.to_string_lossy().into_owned(),
        directory,
    }
}

// Pixel buffers can cross threads; Slint images stay on the interface thread.

#[derive(Default)]
struct Tile {
    rect: crate::spatial::Rect,
    item_size: i32,
    grid_gap: i32,
    widget: bool,
    widget_kind: i32,
    ticktick: bool,
    ticktick_next: bool,
    ticktick_project_id: String,
    widget_text: String,
    widget_min: i32,
    widget_max: i32,
    min_span_x: i32,
    min_span_y: i32,
    max_span_x: i32,
    max_span_y: i32,
    id: String,
    source: i32,
    folder_id: String,
    pinned: bool,
    artwork: String,
    name: String,
    command: Option<String>,
    url: String,
    path: String,
    icon_value: String,
    letter: String,
    icon: Option<SharedPixelBuffer<Rgba8Pixel>>,
    app: bool,
    website: bool,
    file: bool,
    directory: bool,
    folder: bool,
    preview_icons: Vec<FolderIcon>,
    robot: String,
    annotation: i32,
    annotation_style: crate::ink::Style,
}

#[derive(Default)]
struct FolderIcon {
    icon: Option<SharedPixelBuffer<Rgba8Pixel>>,
    letter: String,
}

impl Tile {
    fn new(
        item: &Item,
        icons: &HashMap<String, SharedPixelBuffer<Rgba8Pixel>>,
        item_size: i32,
        grid_gap: i32,
    ) -> Self {
        let mut tile = Self {
            id: item.id().into(),
            name: item.name().into(),
            item_size,
            grid_gap,
            ..Self::default()
        };
        match item {
            Item::Annotation {
                kind, style, image, ..
            } => {
                tile.annotation = *kind;
                tile.annotation_style = style.clone();
                tile.icon = icons.get(item.id()).cloned();
                tile.path = image.clone();
            }
            Item::Robot { profile, .. } => {
                tile.robot = profile.clone();
            }
            Item::Widget {
                variant,
                ticktick_project_id,
                text,
                min_number,
                max_number,
                ..
            } => {
                tile.widget = true;
                tile.widget_kind = *variant as i32;
                tile.ticktick = matches!(*variant, 7 | 8);
                tile.ticktick_next = *variant == 8;
                tile.ticktick_project_id = ticktick_project_id.clone();
                tile.widget_text = text.clone();
                tile.widget_min = *min_number;
                tile.widget_max = *max_number;
                let (min_x, min_y) = crate::spatial::WIDGET_MIN_SPANS[*variant];
                let (max_x, max_y) = crate::spatial::WIDGET_MAX_SPANS[*variant];
                tile.min_span_x = min_x as i32;
                tile.min_span_y = min_y as i32;
                tile.max_span_x = max_x as i32;
                tile.max_span_y = max_y as i32;
            }
            Item::App(app) => {
                tile.app = true;
                if let Launch::Command(command) = &app.launch {
                    tile.command = Some(command.clone());
                }
                tile.icon = icons.get(&app.id).cloned();
            }
            Item::Website { id, url, .. } => {
                tile.website = true;
                tile.url = url.clone();
                tile.icon = icons.get(id).cloned();
            }
            Item::File { id, path, .. } => {
                tile.file = true;
                tile.path = path.clone();
                tile.icon = icons.get(id).cloned();
            }
            Item::Directory { id, path, .. } => {
                tile.directory = true;
                tile.path = path.clone();
                tile.icon = icons.get(id).cloned();
            }
            Item::Folder { apps, .. } => {
                tile.folder = true;
                for app in apps.iter().take(4) {
                    tile.preview_icons.push(FolderIcon {
                        icon: icons.get(&app.id).cloned(),
                        letter: match &app.icon {
                            Icon::Text(text) => text.clone(),
                            _ => crate::icons::first_letter(&app.name),
                        },
                    });
                }
            }
        }
        let icon = match item {
            Item::App(app) => Some(&app.icon),
            Item::Website { icon, .. } | Item::File { icon, .. } | Item::Directory { icon, .. } => {
                Some(icon)
            }
            Item::Annotation { .. }
            | Item::Folder { .. }
            | Item::Widget { .. }
            | Item::Robot { .. } => None,
        };
        if let Some(icon) = icon {
            tile.icon_value = serde_json::to_string(icon).unwrap_or_default();
            if let Icon::Text(text) = icon {
                tile.letter = text.clone();
            }
        }
        if let Some(Icon::Svg(svg)) = icon {
            for (name, artwork) in [
                ("website", include_str!("../ui/assets/website.svg")),
                ("file", include_str!("../ui/assets/file.svg")),
                ("command", include_str!("../ui/assets/command.svg")),
                ("directory", include_str!("../ui/assets/directory.svg")),
                ("browser", include_str!("../ui/assets/browser.svg")),
            ] {
                if svg == artwork {
                    tile.artwork = name.into();
                    break;
                }
            }
        }
        tile
    }

    fn into_slint(self) -> AppTile {
        let icon_present = self.icon.is_some();
        let min_span_x = crate::spatial::widget_span(
            self.min_span_x.max(1) as usize,
            self.item_size,
            self.grid_gap,
        );
        let min_span_y = crate::spatial::widget_span(
            self.min_span_y.max(1) as usize,
            self.item_size,
            self.grid_gap,
        );
        let max_span_x = crate::spatial::widget_span(
            self.max_span_x.max(1) as usize,
            self.item_size,
            self.grid_gap,
        );
        let max_span_y = crate::spatial::widget_span(
            self.max_span_y.max(1) as usize,
            self.item_size,
            self.grid_gap,
        );
        AppTile {
            annotation: self.annotation,
            annotation_color: parse_color(&self.annotation_style.color),
            annotation_color_name: self.annotation_style.color.clone().into(),
            annotation_size: self.annotation_style.size,
            annotation_opacity: self.annotation_style.opacity,
            row: self.rect.row,
            col: self.rect.col,
            span_x: self.rect.width.max(1) as i32,
            span_y: self.rect.height.max(1) as i32,
            widget: self.widget,
            widget_kind: self.widget_kind,
            ticktick: self.ticktick,
            ticktick_next: self.ticktick_next,
            ticktick_project_id: self.ticktick_project_id.into(),
            widget_text: self.widget_text.into(),
            widget_min: self.widget_min,
            widget_max: self.widget_max,
            min_span_x: min_span_x as i32,
            min_span_y: min_span_y as i32,
            max_span_x: max_span_x as i32,
            max_span_y: max_span_y as i32,
            id: self.id.into(),
            source: self.source,
            folder_id: self.folder_id.into(),
            pinned: self.pinned,
            artwork: self.artwork.into(),
            text_icon: !self.letter.is_empty(),
            letter: if self.letter.is_empty() {
                crate::icons::first_letter(&self.name)
            } else {
                self.letter
            }
            .into(),
            name: self.name.into(),
            custom: self.command.is_some(),
            command: self.command.unwrap_or_default().into(),
            url: self.url.into(),
            parent_path: std::path::Path::new(&self.path)
                .parent()
                .map(|parent| {
                    let mut directories: Vec<_> = parent
                        .components()
                        .rev()
                        .filter_map(|part| match part {
                            std::path::Component::Normal(name) => Some(name.to_string_lossy()),
                            _ => None,
                        })
                        .take(2)
                        .collect();
                    directories.reverse();
                    directories.join("/")
                })
                .unwrap_or_default()
                .into(),
            path: self.path.into(),
            icon_value: self.icon_value.into(),
            icon: self.icon.map(slint::Image::from_rgba8).unwrap_or_default(),
            icon_present,
            app: self.app,
            website: self.website,
            file: self.file,
            directory: self.directory,
            folder: self.folder,
            robot: self.robot.into(),
            preview_icons: std::rc::Rc::new(slint::VecModel::from(
                self.preview_icons
                    .into_iter()
                    .map(|icon| FolderPreview {
                        icon_present: icon.icon.is_some(),
                        icon: icon.icon.map(slint::Image::from_rgba8).unwrap_or_default(),
                        letter: icon.letter.into(),
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_ui_for_test() {
        thread_local! { static INIT: () = i_slint_backend_testing::init_no_event_loop(); }
        INIT.with(|_| ());
    }

    #[test]
    fn script_picker_filters_navigates_and_never_falls_back() {
        init_ui_for_test();
        let ui = Launcher::new().unwrap();
        let (send, receive) = mpsc::channel();
        callbacks(&ui, send, Arc::new(RequestSequence::default()));
        ui.set_scripts(
            std::rc::Rc::new(slint::VecModel::from(crate::scripts::sorted(&[
                config::Script {
                    id: "b".into(),
                    name: "beta".into(),
                    path: "/beta".into(),
                },
                config::Script {
                    id: "a".into(),
                    name: "Alpha".into(),
                    path: "/alpha".into(),
                },
            ])))
            .into(),
        );
        assert!(ui.invoke_script_needs_completion("/al".into(), "Alpha".into()));
        assert!(!ui.invoke_script_needs_completion("/Alpha".into(), "Alpha".into()));
        assert!(!ui.invoke_script_needs_completion("/al argument".into(), "Alpha".into()));
        ui.set_search_text("/".into());
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::ZERO);
        ui.invoke_move_search_selection(3);
        ui.invoke_activate_search_selection();
        assert!(matches!(receive.try_recv().unwrap(), Action::RunScript(id, _) if id == "b"));
        ui.set_busy(false);
        ui.set_search_text("/ argument".into());
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::ZERO);
        ui.invoke_run_activated_prefix();
        assert!(
            matches!(receive.try_recv().unwrap(), Action::RunScript(id, text) if id == "b" && text == "/ argument")
        );
        ui.set_busy(false);
        ui.set_search_text("/AL  argument text".into());
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::ZERO);
        ui.invoke_run_activated_prefix();
        assert!(
            matches!(receive.try_recv().unwrap(), Action::RunScript(id, text) if id == "a" && text == "/AL  argument text")
        );
        ui.set_busy(false);
        ui.set_search_text("/missing".into());
        ui.invoke_run_activated_prefix();
        assert!(receive.try_recv().is_err());
        assert!(!ui.get_busy());
        assert!(
            match_prefix(
                &[Prefix {
                    id: "p".into(),
                    name: "old".into(),
                    token: "/".into(),
                    action_id: "a".into()
                }],
                "/"
            )
            .is_none()
        );
    }

    #[test]
    fn locked_layout_rejects_all_organization_actions_without_changes() {
        let weak = slint::Weak::default();
        let mut config = Config::default();
        config.layout_locked = true;
        let mut apps = Apps {
            edits: Default::default(),
            config,
            add_position: (0, 0),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let before = serde_json::to_value(&apps.config).unwrap();
        let requests = RequestSequence::default();
        for action in [
            Action::Widget(7, -100, 500, true),
            Action::ChangeWidget("widget".into(), 200, 200),
            Action::DuplicateWidget("widget".into()),
            Action::SaveAppearance(80, 0, 5),
            Action::Pin("item".into()),
            Action::Unpin("item".into()),
            Action::Delete("item".into()),
            Action::Drop("item".into(), -100, 0, "".into()),
            Action::AddRobot("robot".into(), "Robot".into()),
            Action::AddPredefined("lock".into(), 0),
            Action::AddInstalledApp("app".into(), 0),
            Action::TransferData(true),
            Action::Save("".into(), "".into(), "".into(), "".into(), 0),
            Action::SaveWebsite(
                "".into(),
                "".into(),
                "".into(),
                crate::website_icons::Draft::default(),
                0,
            ),
            Action::SavePath("".into(), "".into(), false, "".into(), 0),
            Action::SaveFolder("".into(), "".into(), 0),
        ] {
            assert_eq!(
                apps.apply(action, &requests, &weak).unwrap_err(),
                "Unlock layout to make changes"
            );
            assert_eq!(serde_json::to_value(&apps.config).unwrap(), before);
        }
        for action in [
            Action::Search("test".into()),
            Action::Open("missing".into()),
            Action::Save(
                "existing".into(),
                "name".into(),
                "true".into(),
                "".into(),
                0,
            ),
            Action::SaveWebsite(
                "existing".into(),
                "name".into(),
                "https://example.test".into(),
                crate::website_icons::Draft::default(),
                0,
            ),
            Action::SavePath("existing".into(), "/tmp".into(), true, "".into(), 0),
            Action::SaveFolder("existing".into(), "name".into(), 0),
            Action::SelectTickTickList("widget".into(), "list".into()),
            Action::AddPosition(-100, 200),
            Action::ShowAll(800.0, 600.0),
            Action::SaveLayoutLocked(false),
        ] {
            assert!(!action.changes_layout());
        }
    }

    #[test]
    fn no_result_search_uses_the_saved_search_engine() {
        init_ui_for_test();
        let mut config = Config::default();
        config.default_search_engine = "google-search".into();
        let mut apps = Apps {
            edits: Default::default(),
            config,
            add_position: (0, 0),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let launcher = Launcher::new().unwrap();
        let opened = std::cell::RefCell::new(Vec::new());
        apps.apply_with_url_opener(
            Action::SearchWeb("hello world".into(), String::new()),
            &RequestSequence::default(),
            &launcher.as_weak(),
            &|url| {
                opened.borrow_mut().push(url.to_string());
                Ok(())
            },
        )
        .unwrap();
        apps.apply_with_url_opener(
            Action::SearchWeb("Orbit cats & dogs".into(), "perplexity".into()),
            &RequestSequence::default(),
            &launcher.as_weak(),
            &|url| {
                opened.borrow_mut().push(url.to_string());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            opened.into_inner(),
            [
                "https://www.google.com/search?q=hello%20world",
                "https://www.perplexity.ai/search?q=Orbit%20cats%20%26%20dogs"
            ]
        );
    }

    #[test]
    fn folder_apps_can_enter_recent_and_open_from_the_dock() {
        let mut config = Config::default();
        config.items.push(Item::Folder {
            id: "folder".into(),
            name: "Folder".into(),
            apps: vec![App {
                id: "nested".into(),
                name: "Nested".into(),
                icon: Icon::Letter,
                launch: Launch::Command("true".into()),
            }],
        });
        config.pinned.push("nested".into());
        let apps = Apps {
            edits: Default::default(),
            config,
            add_position: (0, 0),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };

        assert_eq!(apps.recent_id(&Action::Open("folder".into())), None);
        assert_eq!(
            apps.recent_id(&Action::LaunchFolder("folder".into(), "nested".into())),
            Some("nested".into())
        );
        assert_eq!(
            apps.recent_id(&Action::OpenDock(ResultRef {
                id: "nested".into(),
                folder_id: "folder".into(),
                source: 0
            })),
            Some("nested".into())
        );
        let tile = apps.dock_tile("nested").unwrap();
        assert_eq!(tile.folder_id, "folder");
        assert!(tile.pinned);
    }

    #[test]
    fn stable_references_survive_reordering_and_missing_edits_do_not_create_items() {
        let item = |id: &str| {
            Item::App(App {
                id: id.into(),
                name: id.into(),
                icon: Icon::Letter,
                launch: Launch::Command("true".into()),
            })
        };
        let mut config = Config::default();
        config.items = vec![item("first"), item("second")];
        let mut apps = Apps {
            edits: Default::default(),
            config,
            add_position: (-1200, 5000),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let reference = ResultRef {
            id: "first".into(),
            ..Default::default()
        };
        apps.config.items.reverse();
        assert_eq!(apps.search_result_item(&reference).unwrap().id(), "first");
        assert_eq!(
            apps.recent_id(&Action::OpenResult(reference.clone())),
            Some("first".into())
        );
        apps.config.items.retain(|item| item.id() != "first");
        let before = serde_json::to_value(&apps.config).unwrap();
        assert!(apps.search_result_item(&reference).is_err());
        assert!(apps.store("first", item("first")).is_err());
        assert_eq!(serde_json::to_value(&apps.config).unwrap(), before);
    }

    #[test]
    fn invalid_resize_and_colliding_moves_leave_saved_state_unchanged() {
        let mut config = Config::default();
        config.items.push(Item::Widget {
            id: "widget".into(),
            name: "Random Quote".into(),
            text: String::new(),
            variant: 9,
            ticktick_project_id: String::new(),
            min_number: 1,
            max_number: 100,
        });
        config.items.push(Item::App(App {
            id: "neighbor".into(),
            name: "Neighbor".into(),
            icon: Icon::Letter,
            launch: Launch::Command("true".into()),
        }));
        let span = crate::spatial::widget_span(2, config.item_size, config.grid_gap);
        config.positions.insert(
            "widget".into(),
            crate::spatial::Rect {
                col: -1000,
                row: 4000,
                width: span,
                height: span,
            },
        );
        config.positions.insert(
            "neighbor".into(),
            crate::spatial::Rect {
                col: -1000 + span as i32,
                row: 4000,
                width: span,
                height: span,
            },
        );
        let mut apps = Apps {
            edits: Default::default(),
            config,
            add_position: (0, 0),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let before = serde_json::to_value(&apps.config).unwrap();
        let weak = slint::Weak::default();
        let requests = RequestSequence::default();
        assert!(
            apps.apply(
                Action::ChangeWidget("widget".into(), span + 1, span),
                &requests,
                &weak
            )
            .is_err()
        );
        assert!(
            apps.apply(
                Action::ChangeWidget("widget".into(), span - 1, span),
                &requests,
                &weak
            )
            .is_err()
        );
        apps.apply(
            Action::Drop("widget".into(), -1000 + span as i32, 4000, String::new()),
            &requests,
            &weak,
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&apps.config).unwrap(), before);
        assert!(apps.pending_reveal.is_none());
    }

    #[test]
    fn cached_result_ids_survive_inventory_reordering() {
        let installed = |id: &str| crate::config::InstalledApp {
            id: id.into(),
            name: id.into(),
            command: "true".into(),
            icon: Icon::Letter,
        };
        let mut inventory = Inventory::default();
        inventory.replace(vec![installed("first"), installed("second")]);
        let mut apps = Apps {
            edits: Default::default(),
            config: Config::default(),
            add_position: (0, 0),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory,
            installed_filter: String::new(),
            query: "first".into(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let tile = apps.compute_search_results().remove(0);
        assert_eq!(tile.source, -2);
        let reference = ResultRef {
            id: tile.id,
            source: tile.source,
            folder_id: String::new(),
        };
        apps.inventory
            .replace(vec![installed("second"), installed("first")]);
        let item = apps.search_result_item(&reference).unwrap();
        assert_eq!(item.name(), "first");
        assert!(
            matches!(item, Item::App(App { launch: Launch::Installed(id), .. }) if id == "first")
        );
    }

    #[test]
    fn cancelled_adds_leave_items_and_canvas_positions_unchanged() {
        let mut apps = Apps {
            edits: Default::default(),
            config: Config::default(),
            add_position: (-1200, 5000),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let requests = RequestSequence::default();
        let request = requests.issue();
        requests.cancel();
        let before = serde_json::to_value(&apps.config).unwrap();
        apps.save(
            "",
            "Cancelled".into(),
            "true".into(),
            "".into(),
            &requests,
            request,
        )
        .unwrap();
        apps.save_folder("", "Cancelled".into(), &requests, request)
            .unwrap();
        assert_eq!(serde_json::to_value(&apps.config).unwrap(), before);
    }

    #[test]
    fn tab_prefix_activation_replaces_token_and_preserves_argument() {
        let config = Config::default();
        let activated = match_prefix(&config.prefixes, "y funny cats & dogs").unwrap();
        assert_eq!(activated.name.as_str(), "YouTube");
        assert_eq!(activated.action_id.as_str(), "youtube");
        assert_eq!(activated.query.as_str(), "funny cats & dogs");
        assert_eq!(activated.query_length as usize, "funny cats & dogs".len());
        assert!(activated.badge_width > 52);
        assert!(match_prefix(&config.prefixes, "youtube cats").is_none());
        assert!(match_prefix(&config.prefixes, "yout cats").is_none());
    }

    #[test]
    fn ai_prefix_uses_the_answer_panel_without_launching_or_hiding() {
        init_ui_for_test();
        let ui = Launcher::new().unwrap();
        ui.set_shortcuts(crate::shortcuts::model(crate::shortcuts::defaults()));
        let (send, receive) = mpsc::channel();
        callbacks(&ui, send, Arc::new(RequestSequence::default()));
        ui.set_user_actions(
            std::rc::Rc::new(slint::VecModel::from(vec![UserActionChoice {
                id: "orbit-ai".into(),
                name: "AI".into(),
                kind: "AI".into(),
                ..Default::default()
            }]))
            .into(),
        );
        ui.set_prefixes(
            std::rc::Rc::new(slint::VecModel::from(vec![PrefixChoice {
                id: "orbit-ai".into(),
                name: "AI".into(),
                token: "ai".into(),
                action_id: "orbit-ai".into(),
                ..Default::default()
            }]))
            .into(),
        );
        let submitted = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = submitted.clone();
        ui.on_ai_submit(move || count.set(count.get() + 1));
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        ui.invoke_focus_search();
        ui.set_search_text("ai".into());
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: "\t".into() });
        assert_eq!(ui.get_activated_prefix_name(), "AI");
        assert!(ui.get_ai_active());
        assert!(!ui.get_home_visible());
        assert!(!ui.get_search_results_visible());
        assert!(!ui.get_grid_navigation());
        assert!(!ui.get_busy());
        ui.set_search_text("hello".into());
        assert!(!ui.get_home_visible());
        assert!(!ui.get_search_results_visible());
        ui.invoke_activate_search_selection();
        use slint::winit_030::winit::keyboard::{Key, ModifiersState, NamedKey};
        assert!(!crate::shortcuts::handle(
            &ui,
            &Key::Named(NamedKey::ArrowLeft),
            ModifiersState::empty()
        ));
        assert!(crate::shortcuts::handle(
            &ui,
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty()
        ));
        assert_eq!(submitted.get(), 1);
        assert!(!ui.get_busy());
        assert!(ui.get_launcher_visible());
        assert!(!receive.try_iter().any(|action| matches!(
            action,
            Action::RunPrefix(..) | Action::SearchWeb(..) | Action::OpenResult(..)
        )));
        ui.set_ai_answer("answer".into());
        ui.invoke_reset_launcher();
        assert!(ui.get_ai_answer().is_empty());
        assert!(ui.get_home_visible());
        ui.set_search_text("ordinary search".into());
        assert!(ui.get_search_results_visible());
    }

    #[test]
    fn slint_tab_and_enter_dispatch_builtin_url_actions_once() {
        use slint::{ComponentHandle, Model};

        fn exercise(token: &str, argument: &str, name: &str, expected_url: &str) {
            use slint::winit_030::winit::keyboard::{Key, ModifiersState, NamedKey};
            let config = Config::default();
            let prefixes = config
                .prefixes
                .iter()
                .map(|prefix| PrefixChoice {
                    id: prefix.id.clone().into(),
                    name: prefix.name.clone().into(),
                    token: prefix.token.clone().into(),
                    action_id: prefix.action_id.clone().into(),
                    action_name: prefix.name.clone().into(),
                    action_index: 0,
                })
                .collect::<Vec<_>>();
            let launcher = Launcher::new().unwrap();
            launcher.set_shortcuts(crate::shortcuts::model(crate::shortcuts::defaults()));
            launcher.set_prefixes(std::rc::Rc::new(slint::VecModel::from(prefixes)).into());
            let (sender, receiver) = mpsc::channel();
            callbacks(&launcher, sender, Arc::new(RequestSequence::default()));
            launcher.set_search_text(token.into());
            launcher.show().unwrap();
            launcher.invoke_focus_search();
            assert!(launcher.get_search_focused());

            let window = launcher.window();
            assert!(crate::shortcuts::handle(
                &launcher,
                &Key::Named(NamedKey::Tab),
                ModifiersState::empty()
            ));
            assert_eq!(launcher.get_activated_prefix_name(), name);
            window.dispatch_event(slint::platform::WindowEvent::KeyPressed {
                text: slint::platform::Key::Backspace.into(),
            });
            window.dispatch_event(slint::platform::WindowEvent::KeyReleased {
                text: slint::platform::Key::Backspace.into(),
            });
            assert_eq!(launcher.get_activated_prefix_name(), "");
            launcher.set_search_text(token.into());
            assert!(crate::shortcuts::handle(
                &launcher,
                &Key::Named(NamedKey::Tab),
                ModifiersState::empty()
            ));
            assert_eq!(launcher.get_activated_prefix_name(), name);
            for ch in argument.chars() {
                let text: slint::SharedString = ch.to_string().into();
                window.dispatch_event(slint::platform::WindowEvent::KeyPressed {
                    text: text.clone(),
                });
                window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
            }
            assert_eq!(launcher.get_search_text(), argument);
            assert_eq!(launcher.get_search_results().row_count(), 0);

            assert!(crate::shortcuts::handle(
                &launcher,
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty()
            ));
            assert!(crate::shortcuts::handle(
                &launcher,
                &Key::Named(NamedKey::Enter),
                ModifiersState::empty()
            ));
            let actions = receiver
                .try_iter()
                .filter_map(|action| match action {
                    Action::RunPrefix(action, argument) => Some((action, argument)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(actions.len(), 1, "Enter must dispatch exactly one action");

            let (action_id, query) = &actions[0];
            assert_eq!(query, argument);
            let mut apps = Apps {
                edits: Default::default(),
                config,
                add_position: (0, 0),
                icons: HashMap::new(),
                open_folder: None,
                pending_reveal: None,
                inventory: Inventory::default(),
                installed_filter: String::new(),
                query: String::new(),
                search_paths: None,
                recent: recent::Recent::default(),
            };
            let requests = RequestSequence::default();
            let opened = std::cell::RefCell::new(Vec::new());
            apps.apply_with_url_opener(
                Action::RunPrefix(action_id.to_string(), query.to_string()),
                &requests,
                &launcher.as_weak(),
                &|url| {
                    opened.borrow_mut().push(url.to_string());
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(opened.into_inner(), [expected_url]);
        }

        init_ui_for_test();
        exercise(
            "y",
            "loda",
            "YouTube",
            "https://www.youtube.com/results?search_query=loda",
        );
        exercise(
            "c",
            "explain orbit integration",
            "Claude",
            "https://claude.ai/new?q=explain%20orbit%20integration",
        );
    }

    #[test]
    fn all_default_url_actions_build_the_requested_destinations() {
        let config = Config::default();
        let expected = [
            (
                "youtube",
                "https://www.youtube.com/results?search_query=Orbit%20cats%20%26%20dogs",
            ),
            (
                "chatgpt",
                "https://chatgpt.com/?q=Orbit%20cats%20%26%20dogs",
            ),
            (
                "image-search",
                "https://www.google.com/search?tbm=isch&q=Orbit%20cats%20%26%20dogs",
            ),
            (
                "perplexity",
                "https://www.perplexity.ai/search?q=Orbit%20cats%20%26%20dogs",
            ),
            (
                "claude",
                "https://claude.ai/new?q=Orbit%20cats%20%26%20dogs",
            ),
            (
                "wikipedia",
                "https://en.wikipedia.org/wiki/Special:Search?search=Orbit%20cats%20%26%20dogs",
            ),
        ];
        for (id, url) in expected {
            let action = config
                .user_actions
                .iter()
                .find(|action| action.id == id)
                .unwrap();
            assert_eq!(
                url_action_url(&action.source, "Orbit cats & dogs").unwrap(),
                url
            );
        }
        let ticktick = config
            .user_actions
            .iter()
            .find(|action| action.id == "ticktick")
            .unwrap();
        assert!(ticktick.kind == ActionKind::TickTick);
        assert!(url_action_url("https://example.test/?q=missing", "query").is_err());
    }

    #[test]
    fn url_encoding_preserves_only_unreserved_bytes() {
        assert_eq!(
            url_encode("'quote' $(echo hi); 雪"),
            "%27quote%27%20%24%28echo%20hi%29%3B%20%E9%9B%AA"
        );
    }

    #[test]
    fn ticktick_metadata_and_multword_lists_parse_and_resolve() {
        let task =
            parse_ticktick_query("Write report !high #work #review ^Personal Projects").unwrap();
        assert_eq!(task.title, "Write report");
        assert_eq!(task.priority, 5);
        assert_eq!(task.tags, ["work", "review"]);
        assert_eq!(task.list.as_deref(), Some("Personal Projects"));

        for (metadata, priority) in [("!high", 5), ("!medium", 3), ("!low", 1), ("!none", 0)] {
            assert_eq!(
                parse_ticktick_query(&format!("Task {metadata}"))
                    .unwrap()
                    .priority,
                priority
            );
        }

        let inbox = crate::ticktick::Project {
            id: "inbox-id".into(),
            name: "Inbox".into(),
            inbox: true,
        };
        let projects = [crate::ticktick::Project {
            id: "personal".into(),
            name: "Personal Projects".into(),
            inbox: false,
        }];
        assert_eq!(
            resolve_ticktick_project(task.list.as_deref(), &projects, &inbox)
                .unwrap()
                .id,
            "personal"
        );
        assert_eq!(
            resolve_ticktick_project(None, &projects, &inbox)
                .unwrap()
                .id,
            "inbox-id"
        );
        assert_eq!(
            resolve_ticktick_project(Some("inbox"), &projects, &inbox)
                .unwrap()
                .id,
            "inbox-id"
        );
    }

    #[test]
    fn ticktick_double_exclamation_sets_today_like_today_metadata() {
        for date in ["!today", "!!"] {
            for query in [
                format!("Buy milk {date} !high #home ~Shopping List"),
                format!("{date} Buy milk !high #home ~Shopping List"),
                format!("Buy milk !high #home ~Shopping List {date}"),
            ] {
                let task = parse_ticktick_query(&query).unwrap();
                assert!(task.today, "{query}");
                assert_eq!(task.title, "Buy milk");
                assert_eq!(task.priority, 5);
                assert_eq!(task.tags, ["home"]);
                assert_eq!(task.list.as_deref(), Some("Shopping List"));
            }
            assert!(parse_ticktick_query(date).is_err());
        }
        let task = parse_ticktick_query("Buy milk!!").unwrap();
        assert_eq!(task.title, "Buy milk!!");
        assert!(!task.today);
    }

    #[test]
    fn ticktick_metadata_failures_are_reported_before_api_calls() {
        assert!(parse_ticktick_query("!high #work ^Inbox").is_err());
        let inbox = crate::ticktick::Project {
            id: "inbox".into(),
            name: "Inbox".into(),
            inbox: true,
        };
        assert!(resolve_ticktick_project(Some("Missing List"), &[], &inbox).is_err());
    }

    #[test]
    #[ignore = "uses the configured TickTick account to create and delete one temporary task"]
    fn live_ticktick_prefix_creates_and_deletes_its_test_task() {
        let token = crate::ticktick::load_token()
            .unwrap()
            .expect("TickTick must be connected for this integration check");
        let projects = crate::ticktick::projects(&token).unwrap();
        let inbox = crate::ticktick::inbox_project(&token).unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let title = format!("Orbit Prefix Integration Test {nonce}");
        let task = parse_ticktick_query(&format!("{title} !low ^Inbox")).unwrap();
        let project = resolve_ticktick_project(task.list.as_deref(), &projects, &inbox).unwrap();
        let created = crate::ticktick::create_task_with_metadata(
            &token,
            &project.id,
            &task.title,
            "",
            task.priority,
            &task.tags,
        )
        .unwrap();
        let cleanup = crate::ticktick::delete_task(&token, &created.project_id, &created.id);
        assert!(
            cleanup.is_ok(),
            "could not remove test task '{title}': {cleanup:?}"
        );
        assert_eq!(created.title, title);
        assert_eq!(created.project_id, inbox.id);
    }

    #[test]
    fn search_results_do_not_duplicate_pinned_or_saved_apps() {
        use slint::Model;
        init_ui_for_test();
        let launcher = Launcher::new().unwrap();

        let mut config = Config::default();
        config.items.push(Item::App(App {
            id: "whatsapp-saved".into(),
            name: "WhatsApp Personal".into(),
            icon: Icon::Letter,
            launch: Launch::Installed("whatsapp-personal.desktop".into()),
        }));
        config.pinned.push("whatsapp-saved".into());

        let mut inventory = Inventory::default();
        inventory.replace(vec![
            crate::config::InstalledApp {
                id: "whatsapp-personal.desktop".into(),
                name: "WhatsApp Personal".into(),
                command: "whatsapp-for-linux".into(),
                icon: Icon::Letter,
            },
            crate::config::InstalledApp {
                id: "telegram.desktop".into(),
                name: "Telegram Desktop".into(),
                command: "telegram-desktop".into(),
                icon: Icon::Letter,
            },
            crate::config::InstalledApp {
                id: "dup1.desktop".into(),
                name: "Duplicate App".into(),
                command: "dup-command".into(),
                icon: Icon::Letter,
            },
            crate::config::InstalledApp {
                id: "dup2.desktop".into(),
                name: "Duplicate App".into(),
                command: "dup-command".into(),
                icon: Icon::Letter,
            },
        ]);

        let mut apps = Apps {
            edits: Default::default(),
            config,
            add_position: (0, 0),
            icons: HashMap::new(),
            open_folder: None,
            pending_reveal: None,
            inventory,
            installed_filter: String::new(),
            query: "whatsapp".into(),
            search_paths: None,
            recent: recent::Recent::default(),
        };

        apps.update(&launcher.as_weak());
        let results = launcher.get_search_results();
        assert_eq!(
            results.row_count(),
            1,
            "WhatsApp Personal must not be duplicated"
        );
        assert!(
            results.row_data(0).unwrap().pinned,
            "The pinned shortcut must be preserved"
        );

        apps.query = "telegram".into();
        apps.update(&launcher.as_weak());
        let results = launcher.get_search_results();
        assert_eq!(results.row_count(), 1, "Telegram Desktop must appear once");
        assert_eq!(
            results.row_data(0).unwrap().source,
            -2,
            "Must be an installed app result"
        );

        apps.query = "duplicate".into();
        apps.update(&launcher.as_weak());
        assert_eq!(
            launcher.get_search_results().row_count(),
            1,
            "Duplicate App in inventory must appear once"
        );
    }
}

#[cfg(test)]
mod file_search_exclusion_tests {
    use super::*;

    #[test]
    fn patterns_match_whole_names_case_sensitively() {
        for (pattern, name, expected) in [
            ("dist", "dist", true),
            ("dist", "distance", false),
            ("dist", "Dist", false),
            (".*", ".git", true),
            (".*", "file", false),
            ("a*b*c", "axybzc", true),
            ("a*b", "ab", true),
            ("*", "", true),
            ("a?", "ab", false),
            ("*.rs", "main.rs", true),
            ("*é", "café", true),
            ("*é", "cafe", false),
        ] {
            assert_eq!(
                exclusion_matches(pattern, name),
                expected,
                "{pattern}: {name}"
            );
        }
    }

    #[test]
    fn edits_trim_ignore_duplicates_and_preserve_empty_lists() {
        let mut entries = Vec::new();
        assert!(!edit_exclusions(&mut entries, "  ", false));
        assert!(edit_exclusions(&mut entries, " dist ", false));
        assert!(!edit_exclusions(&mut entries, "dist", false));
        assert!(edit_exclusions(&mut entries, "Dist", false));
        assert!(edit_exclusions(&mut entries, " dist ", true));
        assert!(edit_exclusions(&mut entries, "Dist", true));
        assert!(entries.is_empty());
    }

    #[test]
    fn file_index_prunes_descendants_but_not_explicit_roots_or_directory_search() {
        let root = std::env::temp_dir().join(format!("orbit-exclusions-{}", std::process::id()));
        std::fs::create_dir_all(root.join("target/target")).unwrap();
        std::fs::create_dir_all(root.join("target/keep")).unwrap();
        std::fs::write(root.join("target/visible.txt"), "").unwrap();
        std::fs::write(root.join("target/.hidden"), "").unwrap();
        std::fs::write(root.join("target/target/skipped.txt"), "").unwrap();
        std::fs::write(root.join("target/keep/kept.txt"), "").unwrap();
        let mut config = Config::default();
        config
            .file_search_paths
            .push(root.join("target").to_string_lossy().into_owned());
        config
            .directory_search_paths
            .push(root.to_string_lossy().into_owned());
        let results = index_search_paths(&config);
        assert!(
            results
                .iter()
                .any(|entry| entry.name == "target" && entry.directory)
        );
        assert!(results.iter().any(|entry| entry.name == "visible.txt"));
        assert!(results.iter().any(|entry| entry.name == "kept.txt"));
        assert!(
            !results
                .iter()
                .any(|entry| entry.name == "skipped.txt" || entry.name == ".hidden")
        );
        config.file_search_exclusions.clear();
        let results = index_search_paths(&config);
        assert!(results.iter().any(|entry| entry.name == "skipped.txt"));
        assert!(results.iter().any(|entry| entry.name == ".hidden"));
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn parse_color(value: &str) -> slint::Color {
    slint::Color::from_argb_encoded(
        0xff000000 | u32::from_str_radix(value.trim_start_matches('#'), 16).unwrap_or(0xe8e5ef),
    )
}

fn canvas_image(data: &[u8], name: &str) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
    if data.len() > 20 * 1024 * 1024 {
        return Err("Choose an image smaller than 20 MB".into());
    }
    let pixels = decode_icon(data, name)?;
    if pixels.width() <= 1600 && pixels.height() <= 1600 {
        return Ok(pixels);
    }
    let source = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        pixels.width(),
        pixels.height(),
        pixels.as_bytes(),
    )
    .ok_or("Couldn't read image")?;
    let resized = image::imageops::thumbnail(&source, 1600, 1600);
    Ok(SharedPixelBuffer::clone_from_slice(
        resized.as_raw(),
        resized.width(),
        resized.height(),
    ))
}
