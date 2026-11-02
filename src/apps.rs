use crate::add_item::{Inventory, RequestSequence};
use crate::config::{
    ActionKind, App, Config, Icon, Item, Launch, Prefix, SystemAction, UserAction,
};
use crate::{
    ActionChoice, AppTile, FolderPreview, InstalledAppTile, Launcher, add_item, config, platform,
    recent,
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

enum Action {
    Canvas(crate::canvas::Mutation),
    Move(String, usize),
    ReorderPage(i32),
    Geometry(usize, usize),
    Widget(usize, Option<usize>),
    ChangeWidget(String, usize, usize),
    DuplicateWidget(String),
    SelectTickTickList(String, String),
    TransferData(bool),
    SaveAppearance(i32, i32, i32, usize, usize),
    SaveBackgroundFolder(String),
    ChooseBackgroundImage,
    ChangeBackground,
    SaveDefaultSearchEngine(i32),
    SaveSmoothCursor(bool),
    SaveEnglish(bool, bool, bool, String),
    SaveAi(crate::ai::Settings),
    SaveEnglishModel(String, String, String, String),
    SaveLayoutLocked(bool),
    DeletePage,
    SaveShortcut(i32, String, String),
    DeleteShortcut(i32, String),
    Page(usize),
    Search(String),
    SearchWeb(String, String),
    RunPrefix(String, String),
    SaveNamedCommand(String, String, String, String, String, String),
    DeleteNamedCommand(String),
    SavePrefix(String, String, String, String),
    DeletePrefix(String),
    FilterInstalled(String),
    OpenResult(i32, usize),
    SaveSearchResult(i32, usize, i32, usize, i32, bool),
    OpenDock(usize, usize, i32),
    Pin(usize),
    Unpin(String),
    RemoveRecent(String),
    Save(i32, String, String, String, u64),
    SaveWebsite(i32, String, String, crate::website_icons::Draft, u64),
    ChoosePath(bool, u64),
    SavePath(i32, String, bool, String, u64),
    SaveFolder(i32, String, u64),
    Delete(usize),
    Open(usize),
    OpenFolder(usize),
    CloseFolder,
    LaunchFolder(usize, usize),
    AddPage,
    Drop(String, usize, usize, String),
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
    config: Config,
    page: usize,
    icons: HashMap<String, SharedPixelBuffer<Rgba8Pixel>>,
    open_folder: Option<usize>,
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
            Self::Move(..)
            | Self::ReorderPage(..)
            | Self::Geometry(..)
            | Self::Widget(..)
            | Self::ChangeWidget(..)
            | Self::DuplicateWidget(..)
            | Self::SaveAppearance(..)
            | Self::DeletePage
            | Self::Pin(..)
            | Self::Unpin(..)
            | Self::Delete(..)
            | Self::AddPage
            | Self::SaveSearchResult(..)
            | Self::Drop(..)
            | Self::AddPredefined(..)
            | Self::AddInstalledApp(..)
            | Self::TransferData(true) => true,
            Self::Save(index, ..)
            | Self::SaveWebsite(index, ..)
            | Self::SavePath(index, ..)
            | Self::SaveFolder(index, ..) => *index < 0,
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
    let canvas_sender = send.clone();
    crate::canvas::connect(launcher, move |change| {
        let _ = canvas_sender.send(Action::Canvas(change));
    });
    callbacks(launcher, send, requests.clone());
    crate::icons::connect(launcher);
    crate::ticktick_ui::connect(launcher);
    let weak = launcher.as_weak();
    std::thread::spawn(move || {
        let (mut config, banner) = config::load();
        let mut next = config.clone();
        let canvas_warning = crate::canvas::prepare(&mut next)
            .and_then(|changed| {
                if changed {
                    next.save()
                        .map_err(|error| format!("Couldn't save canvas: {error}"))?;
                    config = next;
                }
                Ok(())
            })
            .err();
        let (recent, recent_banner) = recent::Recent::load();
        let (inventory, inventory_banner) = Inventory::load();
        let mut apps = Apps {
            page: config.active_page,
            config,
            icons: HashMap::new(),
            open_folder: None,
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
        apps.update(&weak, false);
        finish(&weak, banner, false, false, None, true);
        if let Some(message) = canvas_warning {
            let _ = weak
                .upgrade_in_event_loop(move |ui| crate::toast::warning(&ui.as_weak(), &message));
        }
        for action in receive {
            if let Action::Canvas(change) = action {
                let adding = matches!(change, crate::canvas::Mutation::Add(..));
                let result = crate::canvas::apply(&mut apps.config, change);
                let canvas = apps.config.canvas.clone();
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    let error = result.err().unwrap_or_default();
                    crate::canvas::sync(&launcher, &canvas, !error.is_empty());
                    let state = launcher.global::<crate::CanvasState>();
                    state.set_saving(false);
                    state.set_error(error.clone().into());
                    if adding && error.is_empty() {
                        launcher.set_sheet(false);
                    }
                    if !adding && !error.is_empty() {
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
                || matches!(action, Action::OpenResult(page, _) if page < 0)
                || matches!(action, Action::SearchWeb(..))
                || matches!(action, Action::RunPrefix(..));
            let saving = matches!(
                action,
                Action::Pin(_)
                    | Action::SaveSearchResult(..)
                    | Action::Unpin(_)
                    | Action::Save(..)
                    | Action::SaveWebsite(..)
                    | Action::SavePath(..)
                    | Action::SaveFolder(..)
                    | Action::Delete(_)
                    | Action::AddPage
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
            );
            let save_search_result = matches!(action, Action::SaveSearchResult(..));
            let drag_drop = matches!(action, Action::Drop(..) | Action::SaveSearchResult(..));
            let save_shortcut = matches!(action, Action::SaveShortcut(..));
            let save_action = matches!(
                action,
                Action::SaveNamedCommand(..) | Action::DeleteNamedCommand(..)
            );
            let add_exclusion = matches!(action, Action::AddFileSearchExclusion(..));
            let save_search_path = matches!(action, Action::SaveSearchPath(..));
            let reset_after_save = saving
                && !matches!(
                    action,
                    Action::AddPage
                        | Action::Drop(..)
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
                );
            let choosing = matches!(action, Action::ChoosePath(..));
            let update = !choosing
                && (!launching || matches!(action, Action::OpenResult(..) | Action::OpenDock(..)));
            let page_only = matches!(action, Action::Page(_));
            let refreshing = matches!(action, Action::RefreshInstalledApps(_));
            let filtering = matches!(action, Action::FilterInstalled(_));
            let searching = matches!(action, Action::Search(..));
            let appearance = match &action {
                Action::SaveAppearance(margin, size, gap, ..) => Some((*margin, *size, *gap)),
                _ => None,
            };
            let action_feedback = success_message(&action);
            let result = apps
                .apply(action, &requests, &weak)
                .and_then(|()| crate::spatial::reconcile(&mut apps.config));
            if request.is_some_and(|request| !requests.is_current(request)) {
                continue;
            }
            let success = result.is_ok();
            if success && let Some((margin, size, gap)) = appearance {
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    launcher.invoke_appearance_saved(margin, size, gap);
                });
            }
            if refreshing && !success {
                apps.inventory
                    .fail(result.as_ref().err().cloned().unwrap_or_default());
                apps.update(&weak, false);
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
                apps.update(&weak, page_only);
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
            finish(
                &weak,
                result.err().unwrap_or_default(),
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
    launcher.on_save_appearance(move |margin, size, gap, columns, rows| {
        let _ = sender.send(Action::SaveAppearance(
            margin,
            size,
            gap,
            columns as usize,
            rows as usize,
        ));
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
    launcher.on_open_result(move |page, index| {
        let _ = sender.send(Action::OpenResult(page, index as usize));
    });
    let sender = send.clone();
    launcher.on_open_dock_item(move |page, index, folder_app| {
        let _ = sender.send(Action::OpenDock(page as usize, index as usize, folder_app));
    });
    let sender = send.clone();
    launcher.on_pin_item(move |index| {
        let _ = sender.send(Action::Pin(index as usize));
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
    launcher.on_open_page(move |index| {
        let _ = sender.send(Action::Page(index as usize));
    });
    let sender = send.clone();
    let requests_for_save = requests.clone();
    launcher.on_save_app(move |index, name, command, svg| {
        let request = requests_for_save.issue();
        let _ = sender.send(Action::Save(
            index,
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
            index,
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
            index,
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
        let _ = sender.send(Action::SaveFolder(index, name.into(), request));
    });
    let requests_for_cancel = requests;
    launcher.on_cancel_add(move || requests_for_cancel.cancel());
    let sender = send.clone();
    launcher.on_delete_item(move |index| {
        let _ = sender.send(Action::Delete(index as usize));
    });
    let sender = send.clone();
    launcher.on_open_item(move |index| {
        let _ = sender.send(Action::Open(index as usize));
    });
    let sender = send.clone();
    launcher.on_delete_page(move || {
        let _ = sender.send(Action::DeletePage);
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
    launcher.on_add_page(move || {
        let _ = sender.send(Action::AddPage);
    });
    let sender = send.clone();
    launcher.on_move_item(move |id, page| {
        let _ = sender.send(Action::Move(id.into(), page as usize));
    });
    let sender = send.clone();
    launcher.on_reorder_page(move |direction| {
        let _ = sender.send(Action::ReorderPage(direction));
    });
    let sender = send.clone();
    launcher.on_geometry(move |columns, rows| {
        let _ = sender.send(Action::Geometry(columns as usize, rows as usize));
    });
    let sender = send.clone();
    launcher.on_add_widget(move |variant, slot| {
        let _ = sender.send(Action::Widget(
            variant as usize,
            (slot >= 0).then_some(slot as usize),
        ));
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
    launcher.on_save_search_result(move |source, index, child, page, target, pin| {
        let _ = sender.send(Action::SaveSearchResult(
            source,
            index as usize,
            child,
            page as usize,
            target,
            pin,
        ));
    });
    let sender = send.clone();
    launcher.on_drop_item(move |id, page, target, into| {
        let _ = sender.send(Action::Drop(
            id.into(),
            page as usize,
            target.max(0) as usize,
            into.into(),
        ));
    });
    let sender = send.clone();
    launcher.on_open_folder(move |index| {
        let _ = sender.send(Action::OpenFolder(index as usize));
    });
    let sender = send.clone();
    launcher.on_close_folder(move || {
        let _ = sender.send(Action::CloseFolder);
    });
    let sender = send.clone();
    launcher.on_launch_folder_app(move |folder, app| {
        let _ = sender.send(Action::LaunchFolder(folder as usize, app as usize));
    });
}

impl Apps {
    fn recent_id(&self, action: &Action) -> Option<String> {
        let (page, index, folder_app) = match action {
            Action::Open(index) => (self.page, *index, -1),
            Action::OpenResult(page, index) => ((*page).try_into().ok()?, *index, -1),
            Action::OpenDock(page, index, folder_app) => (*page, *index, *folder_app),
            Action::LaunchFolder(folder, index) => (self.page, *folder, *index as i32),
            _ => return None,
        };
        let item = self.config.pages.get(page)?.items.get(index)?;
        if folder_app >= 0 {
            let Item::Folder { apps, .. } = item else {
                return None;
            };
            return Some(apps.get(folder_app as usize)?.id.clone());
        }
        (!matches!(item, Item::Folder { .. } | Item::Widget { .. })).then(|| item.id().into())
    }

    fn load_icons(&mut self) -> Result<(), String> {
        let mut error = None;
        let mut icons = Vec::new();
        for page in &self.config.pages {
            for item in &page.items {
                match item {
                    Item::Widget { .. } => {}
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
        if crate::canvas::is_page(&self.config, self.page)
            && matches!(
                &action,
                Action::Widget(..)
                    | Action::DuplicateWidget(..)
                    | Action::Save(-1, ..)
                    | Action::SaveWebsite(-1, ..)
                    | Action::SavePath(-1, ..)
                    | Action::SaveFolder(-1, ..)
                    | Action::AddPredefined(..)
                    | Action::AddInstalledApp(..)
            )
        {
            return Err("Use Add box on the Canvas".into());
        }
        match action {
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
            Action::Search(query) => {
                self.query = query.to_lowercase();
                if !self.query.is_empty() {
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
                    token.chars().any(char::is_whitespace)
                        || !seen.insert(token.to_lowercase())
                        || config.prefixes.iter().any(|prefix| {
                            prefix.action_id != id && prefix.token.eq_ignore_ascii_case(token)
                        })
                }) {
                    return Err("Tokens must be unique and contain no spaces".into());
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
                if token.is_empty()
                    || token.chars().any(char::is_whitespace)
                    || name.trim().is_empty()
                {
                    return Err("Enter a Prefix name and a Token without spaces".into());
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
                        let url = url_action_url(&action.source, argument.trim())?;
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
            Action::SaveSearchResult(source, index, child, page, target, pin) => {
                let item = self.search_result_item(source, index, child)?;
                let page = if !pin
                    && self.page < self.config.horizontal_pages()
                    && page == self.config.horizontal_pages()
                {
                    self.config.pages.len()
                } else {
                    page
                };
                let mut config = self.config.clone();
                if !crate::item_drop::place_search_result(&mut config, item, page, target, pin)? {
                    return Ok(());
                }
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                self.page = self.config.active_page;
                self.load_icons()?;
                Ok(())
            }
            Action::Pin(index) => {
                let Some(item) = self.config.pages[self.page].items.get(index) else {
                    return Ok(());
                };
                let mut config = self.config.clone();
                if config.pinned.iter().any(|id| id == item.id()) {
                    config.pinned.retain(|id| id != item.id());
                } else if config.pinned.len() < crate::config::PINNED_LIMIT {
                    config.pinned.push(item.id().into());
                } else {
                    return Err("Dock is full".into());
                }
                config
                    .save()
                    .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                self.config = config;
                Ok(())
            }
            Action::Unpin(id) => {
                let mut config = self.config.clone();
                config.pinned.retain(|pinned| pinned != &id);
                if config.pinned.len() != self.config.pinned.len() {
                    config
                        .save()
                        .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                    self.config = config;
                }
                Ok(())
            }
            Action::RemoveRecent(id) => {
                self.recent.remove(&id);
                Ok(())
            }
            Action::OpenResult(page, index) => {
                if page == -1 {
                    let (_, name, action) = ACTIONS.get(index).ok_or("Action is missing")?;
                    return platform::launch_action(action)
                        .map_err(|error| format!("Couldn't start '{name}': {error}"));
                }
                if page == -2 {
                    let app = self
                        .inventory
                        .apps()
                        .get(index)
                        .ok_or("Installed app is missing")?;
                    return platform::launch(&app.command)
                        .map_err(|error| format!("Couldn't start '{}': {error}", app.name));
                }
                if page == -3 || page == -4 {
                    let Some(path) = self
                        .search_paths
                        .as_ref()
                        .and_then(|paths| paths.get(index))
                    else {
                        return Ok(());
                    };
                    return platform::open(&path.path)
                        .map_err(|error| format!("Couldn't open '{}': {error}", path.name));
                }
                let Some(item) = self
                    .config
                    .pages
                    .get(page as usize)
                    .and_then(|page| page.items.get(index))
                else {
                    return Ok(());
                };
                let folder = matches!(item, Item::Folder { .. });
                let previous = self.page;
                self.page = page as usize;
                if folder {
                    self.open_folder = Some(index);
                    Ok(())
                } else {
                    let result = self.open_item(index, weak);
                    if result.is_err() {
                        self.page = previous;
                    }
                    result
                }
            }
            Action::OpenDock(page, index, folder_app) => {
                if folder_app < 0 {
                    self.apply_with_url_opener(
                        Action::OpenResult(page as i32, index),
                        requests,
                        weak,
                        open_url,
                    )
                } else {
                    self.launch_folder_app(page, index, folder_app as usize)
                }
            }
            Action::Page(mut index) => {
                if index == self.config.horizontal_pages() && !self.config.canvas.enabled {
                    let mut next = self.config.clone();
                    match crate::canvas::prepare(&mut next) {
                        Ok(_) => {
                            next.save().map_err(|error| error.to_string())?;
                            self.config = next;
                            index = self.config.horizontal_pages();
                        }
                        Err(message) => {
                            let _ = weak.upgrade_in_event_loop(move |ui| {
                                crate::toast::warning(&ui.as_weak(), &message)
                            });
                        }
                    }
                }
                if index < self.config.horizontal_pages() + 2 {
                    self.config.ensure_vertical_pages();
                }
                if index < self.config.pages.len() && index != self.page {
                    self.page = index;
                    self.config.active_page = index;
                    self.config.save().map_err(|error| error.to_string())?;
                }
                Ok(())
            }
            Action::Save(index, name, command, svg, request) => {
                self.save(index, name, command, svg, requests, request)
            }
            Action::SaveFolder(index, name, request) => {
                self.save_folder(index, name, requests, request)
            }
            Action::Delete(index) => self.delete(index),
            Action::Open(index) => self.open_item(index, weak),
            Action::SelectTickTickList(widget_id, project_id) => {
                let mut config = self.config.clone();
                let widget = config
                    .pages
                    .iter_mut()
                    .flat_map(|page| &mut page.items)
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
            Action::LaunchFolder(folder, index) => self.launch_folder_app(self.page, folder, index),
            Action::AddPage => self.add_page(),
            Action::DeletePage => self.delete_page(),
            Action::Move(id, page) => {
                let page = if page == self.config.horizontal_pages() {
                    self.config.pages.len()
                } else {
                    page
                };
                let rect =
                    self.config
                        .positions
                        .get(&id)
                        .copied()
                        .unwrap_or(crate::spatial::Rect {
                            width: crate::spatial::item_span(
                                1,
                                self.config.item_size,
                                self.config.grid_gap,
                            ),
                            height: crate::spatial::item_span(
                                1,
                                self.config.item_size,
                                self.config.grid_gap,
                            ),
                            ..crate::spatial::Rect::cell(0, self.config.columns)
                        });
                let mut occupied = crate::spatial::occupied(&self.config, page);
                occupied.remove(&id);
                let target = crate::spatial::find_first_fit(
                    &occupied,
                    rect.width,
                    rect.height,
                    self.config.columns,
                    self.config.rows,
                )
                .ok_or("Page is full")?;
                let target = target.row * self.config.columns + target.col;
                self.drop_item(&id, page, target, "")
            }
            Action::ReorderPage(direction) => {
                let destination = self.page as i32 + direction;
                if self.page >= self.config.horizontal_pages()
                    || destination < 0
                    || destination as usize >= self.config.horizontal_pages()
                {
                    return Ok(());
                }
                let mut config = self.config.clone();
                config.pages.swap(self.page, destination as usize);
                config.active_page = destination as usize;
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                self.page = destination as usize;
                Ok(())
            }
            Action::Geometry(columns, rows) => {
                if self.config.columns == columns && self.config.rows == rows {
                    return Ok(());
                }
                let mut config = self.config.clone();
                config.columns = columns.max(1);
                config.rows = rows.max(1);
                config.active_page = self.page;
                crate::spatial::reconcile(&mut config)?;
                self.page = config.active_page;
                self.config = config;
                Ok(())
            }
            Action::Widget(variant, slot) => {
                if !(7..crate::spatial::WIDGETS.len()).contains(&variant) {
                    return Err("Unknown widget type".into());
                }
                let id = new_id()?;
                let text = crate::spatial::WIDGETS[variant].to_owned();
                let mut config = self.config.clone();
                let (width, height) = crate::spatial::WIDGET_SPANS[variant];
                let width = crate::spatial::widget_span(width, config.item_size, config.grid_gap);
                let height = crate::spatial::widget_span(height, config.item_size, config.grid_gap);
                let destination = if let Some(slot) = slot {
                    let rect = crate::spatial::Rect {
                        row: slot / config.columns.max(1),
                        col: slot % config.columns.max(1),
                        width,
                        height,
                    };
                    crate::spatial::can_place(
                        rect,
                        &crate::spatial::occupied(&config, self.page),
                        config.columns,
                        config.rows,
                    )
                    .then_some((self.page, rect))
                } else {
                    (self.page..config.pages.len())
                        .chain(0..self.page)
                        .filter(|page| {
                            *page == self.page
                                || (self.page < config.horizontal_pages()
                                    && *page < config.horizontal_pages())
                        })
                        .find_map(|page| {
                            crate::spatial::find_first_fit(
                                &crate::spatial::occupied(&config, page),
                                width,
                                height,
                                config.columns,
                                config.rows,
                            )
                            .map(|rect| (page, rect))
                        })
                }
                .or_else(|| {
                    if self.page >= config.horizontal_pages() || config.horizontal_pages() >= 9 {
                        return None;
                    }
                    let rect = crate::spatial::find_first_fit(
                        &Default::default(),
                        width,
                        height,
                        config.columns,
                        config.rows,
                    )?;
                    let new_page = config.insert_page();
                    Some((new_page, rect))
                })
                .ok_or("Not enough free space")?;
                let (destination_page, rect) = destination;
                config.positions.insert(id.clone(), rect);
                config.pages[destination_page].items.push(Item::Widget {
                    id,
                    name: crate::spatial::WIDGETS[variant].into(),
                    text,
                    variant,
                    ticktick_project_id: String::new(),
                    min_number: 1,
                    max_number: 100,
                });
                crate::spatial::reconcile(&mut config)?;
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                self.page = destination_page;
                Ok(())
            }
            Action::DuplicateWidget(id) => {
                let Some(source) = self
                    .config
                    .pages
                    .iter()
                    .flat_map(|page| &page.items)
                    .find(|item| item.id() == id)
                    .cloned()
                else {
                    return Ok(());
                };
                let Item::Widget {
                    name,
                    text,
                    variant,
                    ticktick_project_id,
                    min_number,
                    max_number,
                    ..
                } = source
                else {
                    return Ok(());
                };
                let new_id = new_id()?;
                let mut config = self.config.clone();
                let (width, height) = crate::spatial::WIDGET_SPANS[variant];
                let source_rect =
                    config
                        .positions
                        .get(&id)
                        .copied()
                        .unwrap_or(crate::spatial::Rect {
                            width: crate::spatial::widget_span(
                                width,
                                config.item_size,
                                config.grid_gap,
                            ),
                            height: crate::spatial::widget_span(
                                height,
                                config.item_size,
                                config.grid_gap,
                            ),
                            ..Default::default()
                        });
                let source_page = config
                    .pages
                    .iter()
                    .position(|page| page.items.iter().any(|item| item.id() == id))
                    .unwrap_or(self.page);
                let mut destination = None;
                for page in source_page..config.pages.len() {
                    if page != source_page
                        && (source_page >= config.horizontal_pages()
                            || page >= config.horizontal_pages())
                    {
                        continue;
                    }
                    let occupied = crate::spatial::occupied(&config, page);
                    if let Some(rect) = crate::spatial::find_first_fit(
                        &occupied,
                        source_rect.width,
                        source_rect.height,
                        config.columns,
                        config.rows,
                    ) {
                        destination = Some((page, rect));
                        break;
                    }
                }
                if destination.is_none()
                    && source_page < config.horizontal_pages()
                    && config.horizontal_pages() < 9
                {
                    let new_page = config.insert_page();
                    destination = crate::spatial::find_first_fit(
                        &Default::default(),
                        source_rect.width,
                        source_rect.height,
                        config.columns,
                        config.rows,
                    )
                    .map(|rect| (new_page, rect));
                }
                let (page, rect) = destination.ok_or("Not enough free space")?;
                config.positions.insert(new_id.clone(), rect);
                config.pages[page].items.push(Item::Widget {
                    id: new_id,
                    name,
                    text,
                    variant,
                    ticktick_project_id,
                    min_number,
                    max_number,
                });
                config.active_page = page;
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                self.page = page;
                Ok(())
            }
            Action::ChangeWidget(id, width, height) => {
                let mut config = self.config.clone();
                if let Some(rect) = config.positions.get(&id).copied() {
                    let variant = config
                        .pages
                        .iter()
                        .flat_map(|page| &page.items)
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
                    let page = config
                        .pages
                        .iter()
                        .position(|page| page.items.iter().any(|item| item.id() == id))
                        .unwrap_or(self.page);
                    let positions = crate::spatial::place(
                        &crate::spatial::occupied(&config, page),
                        &id,
                        rect,
                        config.columns,
                        config.rows,
                    )
                    .ok_or("Widget does not fit here")?;
                    config.positions.extend(positions);
                }
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::SaveAppearance(margin, size, gap, columns, rows) => {
                let mut config = self.config.clone();
                config.page_margin = margin.clamp(0, 64);
                let old_gap = config.grid_gap.clamp(0, 64) as u64;
                let new_gap = gap.clamp(0, 64) as u64;
                let old_pitch =
                    crate::spatial::cell_pitch(config.item_size, config.grid_gap) as u64;
                let old_size = config.item_size.max(1) as u64;
                let new_size = size.clamp(80, 176) as u64;
                let new_pitch = crate::spatial::cell_pitch(size.clamp(80, 176), gap) as u64;
                if old_pitch != new_pitch {
                    let widgets: HashSet<_> = config
                        .pages
                        .iter()
                        .flat_map(|page| &page.items)
                        .filter_map(|item| match item {
                            Item::Widget { id, .. } => Some(id.as_str()),
                            _ => None,
                        })
                        .collect();
                    for (id, rect) in &mut config.positions {
                        rect.col = (rect.col as u64 * new_pitch / old_pitch) as usize;
                        rect.row = (rect.row as u64 * new_pitch / old_pitch) as usize;
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
                config.columns = columns.max(1);
                config.rows = rows.max(1);
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
            Action::Drop(id, page, target, into) => self.drop_item(&id, page, target, &into),
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
                self.save_website(index, name, url, draft, requests, request)
            }
            Action::TransferData(import) => {
                let mut result = platform::transfer_data(import);
                if import && matches!(&result, Ok(Some(_))) {
                    self.config = config::load().0;
                    let mut next = self.config.clone();
                    match crate::canvas::prepare(&mut next) {
                        Ok(true) => match next.save() {
                            Ok(()) => self.config = next,
                            Err(error) => result = Err(error.to_string()),
                        },
                        Ok(false) => {}
                        Err(error) => result = Err(error),
                    }
                    let canvas = self.config.canvas.clone();
                    let _ = weak
                        .upgrade_in_event_loop(move |ui| crate::canvas::sync(&ui, &canvas, true));
                    self.recent = recent::Recent::load().0;
                    self.inventory = Inventory::load().0;
                    self.page = 0;
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
                self.save_path(index, path, directory, svg)
            }
        }
    }

    fn open_item(&mut self, index: usize, _weak: &slint::Weak<Launcher>) -> Result<(), String> {
        let Some(item) = self.config.pages[self.page].items.get(index) else {
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
            Item::Widget { variant: 7, .. } => Ok(()),
            Item::Folder { .. } | Item::Widget { .. } => Ok(()),
        }
    }

    fn launch_folder_app(
        &mut self,
        page: usize,
        folder: usize,
        index: usize,
    ) -> Result<(), String> {
        let Some(Item::Folder { apps, .. }) = self
            .config
            .pages
            .get(page)
            .and_then(|contents| contents.items.get(folder))
        else {
            return Ok(());
        };
        let Some(app) = apps.get(index) else {
            return Ok(());
        };
        let result = self.launch(app);
        if result.is_ok() {
            self.page = page;
        }
        result.map_err(|error| format!("Couldn't start '{}': {error}", app.name))
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
        self.store(-1, Item::App(app))
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
        self.store(-1, Item::App(app))?;
        let _ = self.load_icon(&item_id, &installed.icon);
        Ok(())
    }

    fn search_result_item(&self, page: i32, index: usize, child: i32) -> Result<Item, String> {
        if page >= 0 {
            let item = self
                .config
                .pages
                .get(page as usize)
                .and_then(|page| page.items.get(index))
                .ok_or("Item is missing")?;
            return match item {
                Item::Folder { apps, .. } if child >= 0 => apps
                    .get(child as usize)
                    .cloned()
                    .map(App::into_item)
                    .ok_or("Item is missing".into()),
                _ => Ok(item.clone()),
            };
        }
        self.cached_search_result(page, index).map(App::into_item)
    }

    fn cached_search_result(&self, page: i32, index: usize) -> Result<App, String> {
        let (name, icon, launch) = match page {
            -1 => {
                let (_, name, action) = ACTIONS.get(index).ok_or("Action is missing")?;
                ((*name).into(), Icon::Letter, Launch::Action(action.clone()))
            }
            -2 => {
                let app = self
                    .inventory
                    .apps()
                    .get(index)
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
                    .and_then(|paths| paths.get(index))
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
        if self.query.is_empty() {
            return results;
        }
        let all = self
            .config
            .pages
            .iter()
            .enumerate()
            .flat_map(|(page, contents)| {
                contents
                    .items
                    .iter()
                    .enumerate()
                    .map(move |(index, item)| (page, index, item))
            });
        let mut seen = HashSet::new();
        for (index, (_, name, _)) in ACTIONS.iter().enumerate() {
            if name.to_lowercase().contains(&self.query) {
                seen.insert(name.trim().to_lowercase());
                results.push(Tile {
                    name: (*name).into(),
                    page: -1,
                    index: index as i32,
                    artwork: "action".into(),
                    app: true,
                    ..Tile::default()
                });
            }
        }
        for (page, index, item) in all.clone() {
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
                Item::Widget { .. } => {}
                _ if item.name().to_lowercase().contains(&self.query) => {
                    push(self.tile(item, page, index));
                }
                _ => {}
            }
        }
        let mut seen_commands = HashSet::new();
        for (index, app) in self.inventory.apps().iter().enumerate() {
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
                    page: -2,
                    index: index as i32,
                    command: Some(app.command.clone()),
                    app: true,
                    ..Tile::default()
                });
            }
        }
        for (index, path) in self.search_paths.iter().flatten().enumerate() {
            if path.name.to_lowercase().contains(&self.query)
                && !all.clone().any(|(_, _, item)| match item {
                    Item::File { path: p, .. } | Item::Directory { path: p, .. } => p == &path.path,
                    _ => false,
                })
            {
                results.push(Tile {
                    name: path.name.clone(),
                    path: path.path.clone(),
                    page: if path.directory { -4 } else { -3 },
                    index: index as i32,
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
        index: i32,
        name: String,
        command: String,
        svg: String,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        if !add_item::valid_name(&name) {
            return Err("Name is empty".into());
        }
        let existing = if index >= 0 {
            self.config.pages[self.page].items.get(index as usize)
        } else {
            None
        };
        let app = prepare_app(existing, name, command, svg)?;
        if matches!(existing, Some(Item::App(old)) if old == &app) {
            return Ok(());
        }
        let id = app.id.clone();
        let icon = app.icon.clone();
        self.load_icon(&id, &icon)?;
        if !requests.is_current(request) {
            return Ok(());
        }
        self.store(index, Item::App(app))?;
        if matches!(icon, Icon::Letter | Icon::Text(_)) {
            self.icons.remove(&id);
        }
        Ok(())
    }

    fn store(&mut self, index: i32, app: Item) -> Result<(), String> {
        let items = &mut self.config.pages[self.page].items;
        if index >= 0 && items.get(index as usize) == Some(&app) {
            return Ok(());
        }
        let previous = if index >= 0 {
            Some(std::mem::replace(&mut items[index as usize], app))
        } else {
            items.push(app);
            None
        };
        if let Err(error) = self.config.save() {
            let items = &mut self.config.pages[self.page].items;
            if let Some(previous) = previous {
                items[index as usize] = previous;
            } else {
                items.pop();
            }
            return Err(format!("Couldn't save config.json: {error}"));
        }
        Ok(())
    }

    fn delete(&mut self, index: usize) -> Result<(), String> {
        if self.config.pages[self.page].items.get(index).is_none() {
            return Ok(());
        }
        let mut config = self.config.clone();
        let item = config.pages[self.page].items.remove(index);
        let item_id = item.id().to_string();
        config.pinned.retain(|id| id != &item_id);
        let icon = match &item {
            Item::Website {
                icon: Icon::File(name),
                ..
            } => Some(name.clone()),
            _ => None,
        };
        if let Item::Folder { apps, .. } = &item {
            for (offset, app) in apps.iter().enumerate() {
                config.pages[self.page]
                    .items
                    .insert(index + offset, app.clone().into_item());
            }
        }
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.icons.remove(&item_id);
        if self.recent.remove(&item_id)
            && let Err(error) = self.recent.save()
        {
            eprintln!("orbit: {error}");
        }
        if let Some(name) = icon {
            platform::remove_icon(&name);
        }
        Ok(())
    }

    fn save_website(
        &mut self,
        index: i32,
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
        let existing = (index >= 0)
            .then(|| self.config.pages[self.page].items.get(index as usize))
            .flatten();
        let id = match existing {
            Some(Item::Website { id, .. }) => id.clone(),
            Some(_) => return Err("Website is missing".into()),
            None => new_id()?,
        };
        let unchanged_url =
            matches!(existing, Some(Item::Website { url: old, .. }) if old == url.trim());
        let old_icon = match existing {
            Some(Item::Website {
                icon: Icon::File(name),
                ..
            }) => Some(name.clone()),
            _ => None,
        };
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
            id: id.clone(),
            name: name.trim().into(),
            url: url.trim().into(),
            icon: icon.clone(),
        };
        if let Err(error) = self.store(index, item) {
            if new_file && let Icon::File(file) = &icon {
                platform::remove_icon(file);
            }
            return Err(error);
        }
        if let Some(old_icon) = old_icon
            && !old_icon.starts_with("chosen-")
            && !matches!(&icon, Icon::File(file) if file == &old_icon)
        {
            platform::remove_icon(&old_icon);
        }
        if matches!(icon, Icon::Letter | Icon::Text(_)) {
            self.icons.remove(&id);
        }
        self.load_icon(&id, &icon)?;
        Ok(())
    }

    fn save_path(
        &mut self,
        index: i32,
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
        let existing = (index >= 0)
            .then(|| self.config.pages[self.page].items.get(index as usize))
            .flatten();
        let id = match existing {
            Some(Item::File { id, .. }) if !directory => id.clone(),
            Some(Item::Directory { id, .. }) if directory => id.clone(),
            Some(_) => return Err("Item is missing".into()),
            None => new_id()?,
        };
        let name = name.trim().to_string();
        let path = path.to_string();
        let icon = crate::icons::parse(&svg)?;
        self.load_icon(&id, &icon)?;
        if matches!(icon, Icon::Letter | Icon::Text(_)) {
            self.icons.remove(&id);
        }
        let item = if directory {
            Item::Directory {
                id,
                name,
                path,
                icon,
            }
        } else {
            Item::File {
                id,
                name,
                path,
                icon,
            }
        };
        self.store(index, item)?;
        Ok(())
    }

    fn save_folder(
        &mut self,
        index: i32,
        name: String,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        if !add_item::valid_name(&name) {
            return Err("Name is empty".into());
        }
        let mut config = self.config.clone();
        if index >= 0 {
            let Some(Item::Folder { name: old_name, .. }) =
                config.pages[self.page].items.get_mut(index as usize)
            else {
                return Err("Folder is missing".into());
            };
            *old_name = name.trim().into();
        } else {
            config.pages[self.page].items.push(Item::Folder {
                id: new_id()?,
                name: name.trim().into(),
                apps: Vec::new(),
            });
        }
        if !requests.is_current(request) {
            return Ok(());
        }
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        Ok(())
    }

    fn delete_page(&mut self) -> Result<(), String> {
        if self.config.pages.len() == 1 {
            return Ok(());
        }
        let mut config = self.config.clone();
        let Some(destination) = config.remove_page(self.page) else {
            return Ok(());
        };
        config.active_page = destination;
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.page = destination;
        self.open_folder = None;
        Ok(())
    }

    fn save_shortcut(&mut self, action: i32, existing: String, keys: String) -> Result<(), String> {
        let mut config = self.config.clone();
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

    fn add_page(&mut self) -> Result<(), String> {
        if self.page >= self.config.horizontal_pages() || self.config.horizontal_pages() >= 9 {
            return Ok(());
        }
        let mut config = self.config.clone();
        let page = config.insert_page();
        config.active_page = page;
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.page = page;
        Ok(())
    }

    fn drop_item(
        &mut self,
        id: &str,
        page: usize,
        target: usize,
        into: &str,
    ) -> Result<(), String> {
        let page = if page == self.config.horizontal_pages()
            && self.page < self.config.horizontal_pages()
            && into != "@dock"
        {
            self.config.pages.len()
        } else {
            page
        };
        let final_page = if page == self.config.pages.len() {
            self.config.horizontal_pages()
        } else {
            page
        };
        let keep_open = self
            .open_folder
            .and_then(|index| self.config.pages[self.page].items.get(index))
            .is_some_and(|item| self.page == page && item.id() == into);
        let mut config = self.config.clone();
        if !crate::item_drop::apply(&mut config, id, page, target, into, new_id()?) {
            return Ok(());
        }
        config.active_page = final_page.min(config.pages.len() - 1);
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.page = final_page.min(self.config.pages.len() - 1);
        self.config.active_page = self.page;
        self.open_folder = self.config.pages[self.page]
            .items
            .iter()
            .position(|item| item.id() == into && matches!(item, Item::Folder { .. }))
            .filter(|_| keep_open);
        Ok(())
    }

    fn tile(&self, item: &Item, page: usize, index: usize) -> Tile {
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
                ..crate::spatial::Rect::cell(index, self.config.columns)
            });
        tile.page = page as i32;
        tile.index = index as i32;
        tile.pinned = self.config.pinned.iter().any(|id| id == item.id());
        tile
    }

    fn dock_tile(&self, id: &str) -> Option<Tile> {
        for (page, contents) in self.config.pages.iter().enumerate() {
            for (index, item) in contents.items.iter().enumerate() {
                if item.id() == id {
                    return Some(self.tile(item, page, index));
                }
                if let Item::Folder { apps, .. } = item
                    && let Some(folder_app) = apps.iter().position(|app| app.id == id)
                {
                    let mut tile = Tile::new(
                        &apps[folder_app].clone().into_item(),
                        &self.icons,
                        self.config.item_size,
                        self.config.grid_gap,
                    );
                    tile.page = page as i32;
                    tile.index = index as i32;
                    tile.folder_app_index = Some(folder_app as i32);
                    tile.pinned = self.config.pinned.iter().any(|pinned| pinned == id);
                    return Some(tile);
                }
            }
        }
        None
    }

    fn update(&self, weak: &slint::Weak<Launcher>, page_only: bool) {
        let canvas = self.config.canvas.clone();
        let _ = weak
            .upgrade_in_event_loop(move |launcher| crate::canvas::sync(&launcher, &canvas, false));
        let page_items = |page: usize| {
            self.config
                .pages
                .get(page)
                .map_or_else(Vec::new, |contents| {
                    contents
                        .items
                        .iter()
                        .enumerate()
                        .map(|(index, item)| self.tile(item, page, index))
                        .collect::<Vec<_>>()
                })
        };
        let previews: Vec<_> = (0..self.config.pages.len()).map(page_items).collect();
        let margin = self.config.page_margin;
        let size = self.config.item_size;
        let gap = self.config.grid_gap;
        let smooth_cursor = self.config.smooth_cursor;
        let english = self.config.english_assistance.clone();
        let ai = self.config.ai.clone();
        let layout_locked = self.config.layout_locked;
        let search_engine_index = self.config.default_search_engine_index() as i32;
        let saved_columns = self.config.columns as i32;
        let saved_rows = self.config.rows as i32;
        let items = page_items(self.page);
        let horizontal = self.config.horizontal_pages();
        let previous = self
            .page
            .checked_sub(1)
            .filter(|_| self.page < horizontal)
            .map(page_items)
            .unwrap_or_default();
        let next = if self.page + 1 < horizontal {
            page_items(self.page + 1)
        } else {
            Vec::new()
        };
        let vertical = (page_items(horizontal), page_items(horizontal + 1));
        let page = self.page as i32;
        let count = horizontal as i32;
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
        if page_only {
            let _ = weak.upgrade_in_event_loop(move |launcher| {
                set_page_items(&launcher, items, previous, next, vertical, page, count);
            });
            return;
        }
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
        let shortcut_groups = (0..14)
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
        let folder = self.open_folder.and_then(|index| {
            let Item::Folder { name, apps, .. } = self.config.pages[self.page].items.get(index)?
            else {
                return None;
            };
            Some((
                index as i32,
                name.clone(),
                apps.iter()
                    .map(|app| {
                        let item = app.clone().into_item();
                        Tile::new(
                            &item,
                            &self.icons,
                            self.config.item_size,
                            self.config.grid_gap,
                        )
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
            set_page_items(&launcher, items, previous, next, vertical, page, count);
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
            launcher.set_page_margin(margin);
            launcher.set_item_size(size);
            launcher.set_grid_gap(gap);
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
            launcher.set_saved_columns(saved_columns);
            launcher.set_saved_rows(saved_rows);
            launcher.set_page_previews(
                std::rc::Rc::new(slint::VecModel::from(
                    previews
                        .into_iter()
                        .map(|items| crate::PageItems {
                            items: std::rc::Rc::new(slint::VecModel::from(
                                items.into_iter().map(Tile::into_slint).collect::<Vec<_>>(),
                            ))
                            .into(),
                        })
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            launcher.set_shortcuts(crate::shortcuts::model(shortcuts));
            launcher
                .set_shortcut_hints(std::rc::Rc::new(slint::VecModel::from(shortcut_hints)).into());
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
            if let Some((index, name, items)) = folder {
                launcher.set_folder_index(index);
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

fn set_page_items(
    launcher: &Launcher,
    items: Vec<Tile>,
    previous: Vec<Tile>,
    next: Vec<Tile>,
    vertical: (Vec<Tile>, Vec<Tile>),
    page: i32,
    count: i32,
) {
    let model = |items: Vec<Tile>| {
        std::rc::Rc::new(slint::VecModel::from(
            items.into_iter().map(Tile::into_slint).collect::<Vec<_>>(),
        ))
        .into()
    };
    launcher.set_items(model(items));
    launcher.set_previous_items(model(previous));
    launcher.set_next_items(model(next));
    launcher.set_top_items(model(vertical.0));
    launcher.set_bottom_items(model(vertical.1));
    launcher.set_current_page(page);
    launcher.set_page_names(slint::ModelRc::new(slint::VecModel::from(
        (0..count)
            .map(|index| format!("Page {}", index + 1).into())
            .chain((count < 9).then(|| "New page".into()))
            .collect::<Vec<slint::SharedString>>(),
    )));
    launcher.set_page_count(count);
    launcher.invoke_finish_drag();
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
    slint::Image::load_from_data(data, Some(extension))
        .map_err(|_| "Couldn't read website icon")?
        .to_rgba8()
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
        Action::Save(index, ..) => Some(if *index < 0 { "App added" } else { "App saved" }),
        Action::SaveWebsite(index, ..) => Some(if *index < 0 {
            "Website added"
        } else {
            "Website saved"
        }),
        Action::SavePath(index, ..) => Some(if *index < 0 {
            "Item added"
        } else {
            "Item saved"
        }),
        Action::SaveFolder(index, ..) => Some(if *index < 0 {
            "Folder added"
        } else {
            "Folder saved"
        }),
        Action::Delete(_) => Some("Item removed"),
        Action::AddPage => Some("Page added"),
        Action::DeletePage => Some("Page deleted"),
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

fn match_prefix(prefixes: &[Prefix], text: &str) -> Option<PrefixMatch> {
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
    page: i32,
    index: i32,
    folder_app_index: Option<i32>,
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
            Item::Folder { .. } | Item::Widget { .. } => None,
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
            row: self.rect.row as i32,
            col: self.rect.col as i32,
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
            page: self.page,
            index: self.index,
            folder_app_index: self.folder_app_index.unwrap_or(-1),
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
    fn locked_layout_rejects_all_organization_actions_without_changes() {
        let weak = slint::Weak::default();
        let mut config = Config::default();
        config.layout_locked = true;
        let mut apps = Apps {
            config,
            page: 0,
            icons: HashMap::new(),
            open_folder: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };
        let before = serde_json::to_value(&apps.config).unwrap();
        let requests = RequestSequence::default();
        for action in [
            Action::Move("item".into(), 1),
            Action::ReorderPage(1),
            Action::Geometry(800, 600),
            Action::Widget(7, None),
            Action::ChangeWidget("widget".into(), 200, 200),
            Action::DuplicateWidget("widget".into()),
            Action::SaveAppearance(10, 80, 0, 800, 600),
            Action::DeletePage,
            Action::Pin(0),
            Action::Unpin("item".into()),
            Action::Delete(0),
            Action::AddPage,
            Action::Drop("item".into(), 1, 0, "".into()),
            Action::AddPredefined("lock".into(), 0),
            Action::AddInstalledApp("app".into(), 0),
            Action::TransferData(true),
            Action::Save(-1, "".into(), "".into(), "".into(), 0),
            Action::SaveWebsite(
                -1,
                "".into(),
                "".into(),
                crate::website_icons::Draft::default(),
                0,
            ),
            Action::SavePath(-1, "".into(), false, "".into(), 0),
            Action::SaveFolder(-1, "".into(), 0),
        ] {
            assert_eq!(
                apps.apply(action, &requests, &weak).unwrap_err(),
                "Unlock layout to make changes"
            );
            assert_eq!(serde_json::to_value(&apps.config).unwrap(), before);
        }
        for action in [
            Action::Search("test".into()),
            Action::Page(0),
            Action::Open(0),
            Action::SelectTickTickList("widget".into(), "list".into()),
            Action::Save(0, "".into(), "".into(), "".into(), 0),
            Action::SaveWebsite(
                0,
                "".into(),
                "".into(),
                crate::website_icons::Draft::default(),
                0,
            ),
            Action::SavePath(0, "".into(), false, "".into(), 0),
            Action::SaveFolder(0, "".into(), 0),
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
            config,
            page: 0,
            icons: HashMap::new(),
            open_folder: None,
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
        config.pages[0].items.push(Item::Folder {
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
            config,
            page: 0,
            icons: HashMap::new(),
            open_folder: None,
            inventory: Inventory::default(),
            installed_filter: String::new(),
            query: String::new(),
            search_paths: None,
            recent: recent::Recent::default(),
        };

        assert_eq!(apps.recent_id(&Action::Open(0)), None);
        assert_eq!(
            apps.recent_id(&Action::LaunchFolder(0, 0)),
            Some("nested".into())
        );
        assert_eq!(
            apps.recent_id(&Action::OpenDock(0, 0, 0)),
            Some("nested".into())
        );
        let tile = apps.dock_tile("nested").unwrap();
        assert_eq!(tile.folder_app_index, Some(0));
        assert!(tile.pinned);
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
    fn touchpad_page_swipes_use_the_dominant_axis_after_initial_drift() {
        init_ui_for_test();
        for (first, movement, expected) in [
            ((0.0, 0.0), (-65.0, 0.0), 1),
            ((0.0, 1.0), (-65.0, 2.0), 1),
            ((0.0, -1.0), (65.0, -2.0), 0),
            ((1.0, 0.0), (2.0, 65.0), 2),
            ((-1.0, 0.0), (-2.0, -65.0), 3),
        ] {
            let ui = Launcher::new().unwrap();
            ui.set_page_count(2);
            ui.set_current_page(if expected == 0 { 1 } else { 0 });
            let opened = std::rc::Rc::new(std::cell::Cell::new(-1));
            let result = opened.clone();
            ui.on_open_page(move |page| result.set(page));
            ui.show().unwrap();
            ui.set_launcher_visible(true);
            for (delta_x, delta_y) in [first, movement] {
                ui.window()
                    .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                        position: slint::LogicalPosition::new(400.0, 300.0),
                        delta_x,
                        delta_y,
                    });
            }
            assert_eq!(opened.get(), expected);
        }
    }

    #[test]
    fn fresh_touchpad_swipes_do_not_wait_for_the_previous_momentum_timeout() {
        init_ui_for_test();
        let ui = Launcher::new().unwrap();
        ui.set_page_count(4);
        let weak = ui.as_weak();
        ui.on_open_page(move |page| {
            let ui = weak.upgrade().unwrap();
            ui.set_current_page(page);
            ui.set_busy(false);
            ui.invoke_finish_drag();
        });
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        let scroll = |delta_x| {
            ui.window()
                .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                    position: slint::LogicalPosition::new(400.0, 300.0),
                    delta_x,
                    delta_y: 0.0,
                })
        };
        let advance = |milliseconds| {
            for _ in 0..milliseconds / 10 {
                i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
            }
        };
        ui.invoke_start_page_scroll();
        scroll(-65.0);
        assert_eq!(ui.get_current_page(), 1);
        advance(250);
        scroll(-65.0);
        assert_eq!(ui.get_current_page(), 1);
        ui.invoke_start_page_scroll();
        scroll(-65.0);
        assert_eq!(ui.get_current_page(), 2);
        advance(50);
        scroll(-65.0);
        assert_eq!(ui.get_current_page(), 2);
        ui.invoke_start_page_scroll();
        scroll(65.0);
        assert_eq!(ui.get_current_page(), 1);
        advance(400);
        scroll(-65.0);
        assert_eq!(ui.get_current_page(), 2);
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
        ui.set_page_count(2);
        ui.invoke_page_key(1);
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
            Action::RunPrefix(..)
                | Action::SearchWeb(..)
                | Action::OpenResult(..)
                | Action::Page(..)
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
                config,
                page: 0,
                icons: HashMap::new(),
                open_folder: None,
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
        config.pages[0].items.push(Item::App(App {
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
            config,
            page: 0,
            icons: HashMap::new(),
            open_folder: None,
            inventory,
            installed_filter: String::new(),
            query: "whatsapp".into(),
            search_paths: None,
            recent: recent::Recent::default(),
        };

        apps.update(&launcher.as_weak(), false);
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
        apps.update(&launcher.as_weak(), false);
        let results = launcher.get_search_results();
        assert_eq!(results.row_count(), 1, "Telegram Desktop must appear once");
        assert_eq!(
            results.row_data(0).unwrap().page,
            -2,
            "Must be an installed app result"
        );

        apps.query = "duplicate".into();
        apps.update(&launcher.as_weak(), false);
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
