use crate::add_item::{Inventory, RequestSequence};
use crate::config::{App, Config, Icon, Item, Launch, SystemAction};
use crate::{
    ActionChoice, AppTile, FolderPreview, InstalledAppTile, Launcher, add_item, config, platform,
    recent,
};
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer};
use std::collections::HashMap;
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

enum Action {
    Move(String, usize),
    ReorderPage(i32),
    Geometry(usize, usize),
    Widget(usize),
    ChangeWidget(String, bool, usize, usize),
    TransferData(bool),
    SaveAppearance(i32, i32, usize, usize),
    DeletePage,
    SaveShortcut(i32, String, String),
    DeleteShortcut(i32, String),
    Page(usize),
    Search(String),
    FilterInstalled(String),
    OpenResult(i32, usize),
    OpenDock(usize, usize, i32),
    Pin(usize),
    Unpin(String),
    RemoveRecent(String),
    Save(i32, String, String, String, u64),
    SaveWebsite(i32, String, String, u64),
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
    SaveSearchPath(bool, i32, String),
    DeleteSearchPath(bool, usize),
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
    callbacks(launcher, send, requests.clone());
    crate::icons::connect(launcher);
    let weak = launcher.as_weak();
    std::thread::spawn(move || {
        let (config, banner) = config::load();
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
        finish(&weak, banner, false, false);
        for action in receive {
            let request = action.request_id();
            if request.is_some_and(|request| !requests.is_current(request)) {
                continue;
            }
            let recent_id = apps.recent_id(&action);
            let remove_recent_id = match &action {
                Action::RemoveRecent(id) => Some(id.clone()),
                _ => None,
            };
            let launching =
                recent_id.is_some() || matches!(action, Action::OpenResult(page, _) if page < 0);
            let saving = matches!(
                action,
                Action::Pin(_)
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
            );
            let drag_drop = matches!(action, Action::Drop(..));
            let save_shortcut = matches!(action, Action::SaveShortcut(..));
            let save_search_path = matches!(action, Action::SaveSearchPath(..));
            let reset_after_save = saving
                && !matches!(
                    action,
                    Action::AddPage
                        | Action::Drop(..)
                        | Action::SaveSearchPath(..)
                        | Action::DeleteSearchPath(..)
                );
            let choosing = matches!(action, Action::ChoosePath(..));
            let update = !choosing
                && (!launching || matches!(action, Action::OpenResult(..) | Action::OpenDock(..)));
            let page_only = matches!(action, Action::Page(_));
            let refreshing = matches!(action, Action::RefreshInstalledApps(_));
            let filtering = matches!(action, Action::FilterInstalled(_));
            let result = apps
                .apply(action, &requests, &weak)
                .and_then(|()| crate::spatial::reconcile(&mut apps.config));
            if request.is_some_and(|request| !requests.is_current(request)) {
                continue;
            }
            let success = result.is_ok();
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
                eprintln!("orbit: {error}");
            }
            if success && filtering {
                apps.update_installed_apps(&weak);
            } else if success && (update || recent_changed || recent_removed) {
                apps.update(&weak, page_only);
            }
            if drag_drop && !success {
                let _ = weak.upgrade_in_event_loop(|launcher| launcher.invoke_finish_drag());
            }
            if save_shortcut && success {
                let _ = weak.upgrade_in_event_loop(|launcher| launcher.invoke_shortcut_saved());
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
            );
        }
    });
}

fn callbacks(launcher: &Launcher, send: Sender<Action>, requests: Arc<RequestSequence>) {
    let sender = send.clone();
    launcher.on_save_appearance(move |margin, size, columns, rows| {
        let _ = sender.send(Action::SaveAppearance(
            margin,
            size,
            columns as usize,
            rows as usize,
        ));
    });
    let sender = send.clone();
    launcher.on_transfer_data(move |import| {
        let _ = sender.send(Action::TransferData(import));
    });
    let sender = send.clone();
    launcher.on_first_letter(|name| crate::icons::first_letter(&name).into());
    launcher.on_valid_name(|name| add_item::valid_name(&name));
    launcher.on_valid_command(|command| add_item::valid_command(&command));
    launcher.on_valid_url(|url| add_item::valid_url(&url));
    launcher.on_search(move |query| {
        let _ = sender.send(Action::Search(query.into()));
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
    launcher.on_save_search_path(move |directory, index, path| {
        let _ = sender.send(Action::SaveSearchPath(directory, index, path.into()));
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
    launcher.on_save_website(move |index, name, url| {
        let request = requests_for_website.issue();
        let _ = sender.send(Action::SaveWebsite(index, name.into(), url.into(), request));
    });
    let sender = send.clone();
    let requests_for_path = requests.clone();
    let weak_for_path = launcher.as_weak();
    launcher.on_choose_path(move |directory| {
        if let Some(launcher) = weak_for_path.upgrade()
            && let Err(error) = platform::hide(&launcher)
        {
            launcher.set_sheet_status(error.to_string().into());
            launcher.set_sheet_operation("".into());
            launcher.set_busy(false);
            return;
        }
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
    launcher.on_add_widget(move |variant| {
        let _ = sender.send(Action::Widget(variant as usize));
    });
    let sender = send.clone();
    launcher.on_widget_action(move |id, refresh, width, height| {
        let _ = sender.send(Action::ChangeWidget(
            id.into(),
            refresh,
            width as usize,
            height as usize,
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
        match action {
            Action::Search(query) => {
                self.query = query.to_lowercase();
                if !self.query.is_empty() {
                    self.load_installed_icons_matching_query();
                    self.load_search_paths();
                }
                Ok(())
            }
            Action::FilterInstalled(filter) => {
                self.installed_filter = filter;
                self.load_installed_icons();
                Ok(())
            }
            Action::Pin(index) => {
                let Some(item) = self.config.pages[self.page].items.get(index) else {
                    return Ok(());
                };
                let mut config = self.config.clone();
                if config.pinned.iter().any(|id| id == item.id()) {
                    config.pinned.retain(|id| id != item.id());
                } else if config.pinned.len() < 8 {
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
                    let result = self.open_item(index);
                    if result.is_err() {
                        self.page = previous;
                    }
                    result
                }
            }
            Action::OpenDock(page, index, folder_app) => {
                if folder_app < 0 {
                    self.apply(Action::OpenResult(page as i32, index), requests, weak)
                } else {
                    self.launch_folder_app(page, index, folder_app as usize)
                }
            }
            Action::Page(index) => {
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
            Action::Open(index) => self.open_item(index),
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
                let rect = self
                    .config
                    .positions
                    .get(&id)
                    .copied()
                    .unwrap_or(crate::spatial::Rect::cell(0, self.config.columns));
                let occupied = crate::spatial::occupied(&self.config, page);
                let target = (0..self.config.columns * self.config.rows)
                    .find(|slot| {
                        let candidate = crate::spatial::Rect {
                            row: slot / self.config.columns,
                            col: slot % self.config.columns,
                            ..rect
                        };
                        candidate.fits(self.config.columns, self.config.rows)
                            && !occupied
                                .iter()
                                .any(|(other, r)| *other != id && candidate.overlaps(*r))
                    })
                    .ok_or("Page is full")?;
                self.drop_item(&id, page, target, "")
            }
            Action::ReorderPage(direction) => {
                let destination = self.page as i32 + direction;
                if destination < 0 || destination as usize >= self.config.pages.len() {
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
                crate::spatial::reconcile(&mut config)?;
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::Widget(variant) => {
                let id = new_id()?;
                let text = crate::spatial::widget_text(variant, sample());
                let mut config = self.config.clone();
                config.positions.insert(
                    id.clone(),
                    crate::spatial::Rect {
                        row: 0,
                        col: 0,
                        width: 2,
                        height: if variant < 2 { 2 } else { 1 },
                    },
                );
                config.pages[self.page].items.push(Item::Widget {
                    id,
                    name: crate::spatial::WIDGETS[variant.min(4)].into(),
                    text,
                    variant,
                });
                crate::spatial::reconcile(&mut config)?;
                config.save().map_err(|e| e.to_string())?;
                self.config = config;
                Ok(())
            }
            Action::ChangeWidget(id, refresh, width, height) => {
                let mut config = self.config.clone();
                if refresh {
                    for item in config.pages.iter_mut().flat_map(|page| &mut page.items) {
                        if let Item::Widget {
                            id: item_id,
                            text,
                            variant,
                            ..
                        } = item
                            && *item_id == id
                        {
                            let mut next = crate::spatial::widget_text(*variant, sample());
                            if next == *text {
                                next = crate::spatial::widget_text(*variant, sample() + 1);
                            }
                            *text = next;
                        }
                    }
                } else if let Some(rect) = config.positions.get(&id).copied() {
                    let rect = crate::spatial::Rect {
                        width: width.clamp(1, 4),
                        height: height.clamp(1, 4),
                        ..rect
                    };
                    let positions = crate::spatial::place(
                        &crate::spatial::occupied(&config, self.page),
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
            Action::SaveAppearance(margin, size, columns, rows) => {
                let mut config = self.config.clone();
                config.page_margin = margin.clamp(0, 64);
                config.item_size = size.clamp(80, 240);
                config.columns = columns.max(1);
                config.rows = rows.max(1);
                crate::spatial::reconcile(&mut config)?;
                config.save().map_err(|error| error.to_string())?;
                self.config = config;
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
            Action::RefreshInstalledApps(request) => self.refresh_installed_apps(requests, request),
            Action::SaveSearchPath(directory, index, path) => {
                self.save_search_path(directory, index, path)
            }
            Action::DeleteSearchPath(directory, index) => self.delete_search_path(directory, index),
            Action::SaveWebsite(index, name, url, request) => {
                self.save_website(index, name, url, requests, request)
            }
            Action::TransferData(import) => {
                let mut result = platform::transfer_data(import);
                if import && matches!(&result, Ok(Some(_))) {
                    self.config = config::load().0;
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
                let status = match result {
                    Ok(Some(status)) => status,
                    Ok(None) => String::new(),
                    Err(error) => error,
                };
                let _ = weak.upgrade_in_event_loop(move |launcher| {
                    let _ = platform::restore_after_chooser(&launcher);
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

    fn open_item(&self, index: usize) -> Result<(), String> {
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

    fn refresh_installed_apps(
        &mut self,
        requests: &RequestSequence,
        request: u64,
    ) -> Result<(), String> {
        self.inventory.refreshing();
        let installed_apps = platform::scan_installed_apps()
            .map_err(|error| format!("Couldn't refresh installed apps: {error}"))?;
        if !requests.is_current(request) {
            return Ok(());
        }
        platform::write_installed_apps(
            &serde_json::to_vec_pretty(&installed_apps)
                .map_err(|error| format!("Couldn't save installed apps: {error}"))?,
        )
        .map_err(|error| format!("Couldn't save installed apps: {error}"))?;
        self.inventory.replace(installed_apps);
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
                    self.icons.get(&app.id).cloned(),
                )
            })
            .collect();
        let has_match = !installed_apps.is_empty();
        let _ = weak.upgrade_in_event_loop(move |launcher| {
            let installed_apps = installed_apps
                .into_iter()
                .map(|(id, name, icon)| {
                    let icon_present = icon.is_some();
                    InstalledAppTile {
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
        let keep_icon = unchanged_url
            && matches!(existing, Some(Item::Website { icon, .. }) if !matches!(icon, Icon::Letter));
        let icon = if keep_icon {
            match existing {
                Some(Item::Website { icon, .. }) => icon.clone(),
                _ => Icon::Letter,
            }
        } else if let Some((extension, data)) = platform::fetch_icon(url.trim()) {
            let file = format!("{}.{extension}", new_id()?);
            if decode_icon(&data, &file).is_ok() && platform::write_icon(&file, &data).is_ok() {
                Icon::File(file)
            } else {
                Icon::Letter
            }
        } else {
            Icon::Letter
        };
        if !requests.is_current(request) {
            if let Icon::File(file) = &icon {
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
            if let Icon::File(file) = &icon {
                platform::remove_icon(file);
            }
            return Err(error);
        }
        if let Some(old_icon) = old_icon
            && !matches!(&icon, Icon::File(file) if file == &old_icon)
        {
            platform::remove_icon(&old_icon);
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
        if self.config.pages.len() >= 9 {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.pages.push(crate::config::Page {
            id: crate::config::new_page_id(),
            items: Vec::new(),
        });
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.page = self.config.pages.len() - 1;
        Ok(())
    }

    fn drop_item(
        &mut self,
        id: &str,
        page: usize,
        target: usize,
        into: &str,
    ) -> Result<(), String> {
        let keep_open = self
            .open_folder
            .and_then(|index| self.config.pages[self.page].items.get(index))
            .is_some_and(|item| self.page == page && item.id() == into);
        let mut config = self.config.clone();
        if !crate::item_drop::apply(&mut config, id, page, target, into, new_id()?) {
            return Ok(());
        }
        config.active_page = page.min(config.pages.len() - 1);
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.page = page.min(self.config.pages.len() - 1);
        self.config.active_page = self.page;
        self.open_folder = self.config.pages[page]
            .items
            .iter()
            .position(|item| item.id() == into && matches!(item, Item::Folder { .. }))
            .filter(|_| keep_open);
        Ok(())
    }

    fn tile(&self, item: &Item, page: usize, index: usize) -> Tile {
        let mut tile = Tile::new(item, &self.icons);
        tile.rect = self
            .config
            .positions
            .get(item.id())
            .copied()
            .unwrap_or(crate::spatial::Rect::cell(index, self.config.columns));
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
                    let mut tile = Tile::new(&apps[folder_app].clone().into_item(), &self.icons);
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
        let items = page_items(self.page);
        let previous = self.page.checked_sub(1).map(page_items).unwrap_or_default();
        let next = page_items(self.page + 1);
        let page = self.page as i32;
        let count = self.config.pages.len() as i32;
        let file_search_paths = self
            .config
            .file_search_paths
            .iter()
            .map(|path| path.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        let directory_search_paths = self
            .config
            .directory_search_paths
            .iter()
            .map(|path| path.as_str().into())
            .collect::<Vec<slint::SharedString>>();
        if page_only {
            let _ = weak.upgrade_in_event_loop(move |launcher| {
                set_page_items(&launcher, items, previous, next, page, count);
            });
            return;
        }
        let all: Vec<_> = self
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
            })
            .collect();
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
            .take(2)
            .collect();
        let mut results: Vec<_> = all
            .iter()
            .filter(|(_, _, item)| {
                !self.query.is_empty() && item.name().to_lowercase().contains(&self.query)
            })
            .map(|(page, index, item)| self.tile(item, *page, *index))
            .collect();
        if !self.query.is_empty() {
            results.extend(
                ACTIONS
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, name, _))| name.to_lowercase().contains(&self.query))
                    .map(|(index, (_, name, _))| Tile {
                        name: (*name).into(),
                        page: -1,
                        index: index as i32,
                        artwork: "action".into(),
                        app: true,
                        ..Tile::default()
                    }),
            );
            results.extend(
                self.inventory
                    .apps()
                    .iter()
                    .enumerate()
                    .filter(|(_, app)| app.name.to_lowercase().contains(&self.query))
                    .map(|(index, app)| Tile {
                        id: app.id.clone(),
                        name: app.name.clone(),
                        icon: self.icons.get(&app.id).cloned(),
                        page: -2,
                        index: index as i32,
                        app: true,
                        ..Tile::default()
                    }),
            );
            results.extend(
                self.search_paths
                    .iter()
                    .flatten()
                    .enumerate()
                    .filter(|(_, path)| path.name.to_lowercase().contains(&self.query))
                    .map(|(index, path)| Tile {
                        name: path.name.clone(),
                        path: path.path.clone(),
                        page: if path.directory { -4 } else { -3 },
                        index: index as i32,
                        file: !path.directory,
                        directory: path.directory,
                        ..Tile::default()
                    }),
            );
        }
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
                    self.icons.get(&app.id).cloned(),
                )
            })
            .collect();
        let shortcuts = self.config.shortcuts.clone();
        let shortcut_groups = (0..12)
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
                        Tile::new(&item, &self.icons)
                    })
                    .collect::<Vec<_>>(),
            ))
        });
        let _ = weak.upgrade_in_event_loop(move |launcher| {
            let installed_apps: Vec<_> = installed_apps
                .into_iter()
                .map(|(id, name, icon)| {
                    let icon_present = icon.is_some();
                    InstalledAppTile {
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
            set_page_items(&launcher, items, previous, next, page, count);
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
            launcher.set_file_search_paths(
                std::rc::Rc::new(slint::VecModel::from(file_search_paths)).into(),
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
        });
    }
}

fn set_page_items(
    launcher: &Launcher,
    items: Vec<Tile>,
    previous: Vec<Tile>,
    next: Vec<Tile>,
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

fn new_id() -> Result<String, String> {
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

fn finish(weak: &slint::Weak<Launcher>, banner: String, saved: bool, launched: bool) {
    let _ = weak.upgrade_in_event_loop(move |launcher| {
        let operation = launcher.get_sheet_operation();
        let restore_error = if operation == "choosing" {
            platform::restore_after_chooser(&launcher)
                .err()
                .map(|error| error.to_string())
        } else {
            None
        };
        if !operation.is_empty() {
            if let Some(error) = restore_error {
                launcher.set_sheet_status(error.into());
            } else if !banner.is_empty() {
                launcher.set_sheet_status(banner.into());
            } else if operation == "choosing" {
                launcher.set_sheet_status("".into());
            }
            launcher.set_sheet_operation("".into());
            launcher.set_banner("".into());
        } else {
            launcher.set_banner(banner.into());
        }
        launcher.set_busy(false);
        if saved {
            launcher.invoke_reset_launcher();
        }
        if launched && let Err(error) = platform::hide(&launcher) {
            launcher.set_banner(error.to_string().into());
        }
    });
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
    widget: bool,
    widget_text: String,
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
    fn new(item: &Item, icons: &HashMap<String, SharedPixelBuffer<Rgba8Pixel>>) -> Self {
        let mut tile = Self {
            id: item.id().into(),
            name: item.name().into(),
            ..Self::default()
        };
        match item {
            Item::Widget { text, .. } => {
                tile.widget = true;
                tile.widget_text = text.clone();
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
        AppTile {
            row: self.rect.row as i32,
            col: self.rect.col as i32,
            span_x: self.rect.width.max(1) as i32,
            span_y: self.rect.height.max(1) as i32,
            widget: self.widget,
            widget_text: self.widget_text.into(),
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
}

fn sample() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
