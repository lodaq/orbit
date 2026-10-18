use crate::config::{App, Config, Icon, InstalledApp, Item, Launch, SystemAction};
use crate::{AppTile, InstalledAppTile, Launcher, config, platform, recent};
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer};
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};

enum Action {
    Page(usize),
    Search(String),
    OpenResult(usize, usize),
    OpenDock(usize, usize, i32),
    Pin(usize),
    Unpin(String),
    RemoveRecent(String),
    Save(i32, String, String, String),
    SaveWebsite(i32, String, String),
    ChoosePath(i32, bool, String),
    SaveFolder(i32, String),
    Delete(usize),
    Open(usize),
    OpenFolder(usize),
    CloseFolder,
    LaunchFolder(usize, usize),
    AddPage,
    Drop(usize, usize, i32, i32, bool),
    MoveFolderApp(usize, usize, usize),
    ReorderFolderApp(usize, usize, usize),
    AddPredefined(String),
    AddInstalledApp(String),
    RefreshInstalledApps,
}

struct Apps {
    config: Config,
    page: usize,
    icons: HashMap<String, SharedPixelBuffer<Rgba8Pixel>>,
    open_folder: Option<usize>,
    installed_apps: Vec<InstalledApp>,
    installed_status: String,
    query: String,
    recent: recent::Recent,
}

pub fn connect(launcher: &Launcher) {
    let (send, receive) = mpsc::channel();
    callbacks(launcher, send);
    let weak = launcher.as_weak();
    std::thread::spawn(move || {
        let (config, banner) = config::load();
        let (recent, recent_banner) = recent::Recent::load();
        let installed_apps = platform::read_installed_apps()
            .ok()
            .flatten()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        let mut apps = Apps {
            config,
            page: 0,
            icons: HashMap::new(),
            open_folder: None,
            installed_apps,
            installed_status: String::new(),
            query: String::new(),
            recent,
        };
        let banner = apps.load_icons().err().unwrap_or(banner);
        let banner = if banner.is_empty() {
            recent_banner
        } else {
            banner
        };
        apps.update(&weak, false);
        finish(&weak, banner, false, false);
        for action in receive {
            let recent_id = apps.recent_id(&action);
            let remove_recent_id = match &action {
                Action::RemoveRecent(id) => Some(id.clone()),
                _ => None,
            };
            let launching = recent_id.is_some();
            let saving = matches!(
                action,
                Action::Pin(_)
                    | Action::Unpin(_)
                    | Action::Save(..)
                    | Action::SaveWebsite(..)
                    | Action::ChoosePath(..)
                    | Action::SaveFolder(..)
                    | Action::Delete(_)
                    | Action::AddPage
                    | Action::Drop(..)
                    | Action::MoveFolderApp(..)
                    | Action::ReorderFolderApp(..)
                    | Action::AddPredefined(_)
                    | Action::AddInstalledApp(_)
            );
            let update =
                !launching || matches!(action, Action::OpenResult(..) | Action::OpenDock(..));
            let page_only = matches!(action, Action::Page(_))
                && !(apps.config.pages.len() > 1 && apps.config.pages[apps.page].items.is_empty());
            let result = apps.apply(action);
            let success = result.is_ok();
            let recent_changed = success && recent_id.is_some_and(|id| apps.recent.record(&id));
            // RemoveRecent is applied in Apps::apply; save it here without removing twice.
            let recent_removed = success && remove_recent_id.is_some();
            if (recent_changed || recent_removed)
                && let Err(error) = apps.recent.save()
            {
                eprintln!("orbit: {error}");
            }
            if success && (update || recent_changed || recent_removed) {
                apps.update(&weak, page_only);
            }
            finish(
                &weak,
                result.err().unwrap_or_default(),
                saving && success,
                launching && success,
            );
        }
    });
}

fn callbacks(launcher: &Launcher, send: Sender<Action>) {
    let sender = send.clone();
    launcher.on_search(move |query| {
        let _ = sender.send(Action::Search(query.into()));
    });
    let sender = send.clone();
    launcher.on_open_result(move |page, index| {
        let _ = sender.send(Action::OpenResult(page as usize, index as usize));
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
    launcher.on_save_app(move |index, name, command, svg| {
        let _ = sender.send(Action::Save(index, name.into(), command.into(), svg.into()));
    });
    let sender = send.clone();
    launcher.on_add_system_action(move |action| {
        let _ = sender.send(Action::AddPredefined(action.into()));
    });
    let sender = send.clone();
    launcher.on_add_installed_app(move |id| {
        let _ = sender.send(Action::AddInstalledApp(id.into()));
    });
    let sender = send.clone();
    launcher.on_refresh_installed_apps(move || {
        let _ = sender.send(Action::RefreshInstalledApps);
    });
    let sender = send.clone();
    launcher.on_save_website(move |index, name, url| {
        let _ = sender.send(Action::SaveWebsite(index, name.into(), url.into()));
    });
    let sender = send.clone();
    launcher.on_choose_path(move |index, directory, svg| {
        let _ = sender.send(Action::ChoosePath(index, directory, svg.into()));
    });
    let sender = send.clone();
    launcher.on_save_folder(move |index, name| {
        let _ = sender.send(Action::SaveFolder(index, name.into()));
    });
    let sender = send.clone();
    launcher.on_delete_item(move |index| {
        let _ = sender.send(Action::Delete(index as usize));
    });
    let sender = send.clone();
    launcher.on_open_item(move |index| {
        let _ = sender.send(Action::Open(index as usize));
    });
    let sender = send.clone();
    launcher.on_add_page(move || {
        let _ = sender.send(Action::AddPage);
    });
    let sender = send.clone();
    launcher.on_drop_item(move |source, target, origin, page, make_folder| {
        let _ = sender.send(Action::Drop(
            source as usize,
            target as usize,
            origin,
            page,
            make_folder,
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
    let sender = send.clone();
    launcher.on_move_folder_app(move |folder, app, target| {
        let _ = sender.send(Action::MoveFolderApp(
            folder as usize,
            app as usize,
            target as usize,
        ));
    });
    let sender = send.clone();
    launcher.on_reorder_folder_app(move |folder, source, target| {
        let _ = sender.send(Action::ReorderFolderApp(
            folder as usize,
            source as usize,
            target as usize,
        ));
    });
}

impl Apps {
    fn recent_id(&self, action: &Action) -> Option<String> {
        let (page, index, folder_app) = match action {
            Action::Open(index) => (self.page, *index, -1),
            Action::OpenResult(page, index) => (*page, *index, -1),
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
        (!matches!(item, Item::Folder { .. })).then(|| item.id().into())
    }

    fn load_icons(&mut self) -> Result<(), String> {
        let mut error = None;
        let mut icons = Vec::new();
        for page in &self.config.pages {
            for item in &page.items {
                match item {
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
            Icon::Letter => None,
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

    fn apply(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::Search(query) => {
                self.query = query.to_lowercase();
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
                let Some(item) = self
                    .config
                    .pages
                    .get(page)
                    .and_then(|page| page.items.get(index))
                else {
                    return Ok(());
                };
                let folder = matches!(item, Item::Folder { .. });
                let previous = self.page;
                self.page = page;
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
                    self.apply(Action::OpenResult(page, index))
                } else {
                    self.launch_folder_app(page, index, folder_app as usize)
                }
            }
            Action::Page(index) => {
                if index < self.config.pages.len() && index != self.page {
                    let mut config = self.config.clone();
                    let mut next = index;
                    if config.pages.len() > 1 && config.pages[self.page].items.is_empty() {
                        config.pages.remove(self.page);
                        if next > self.page {
                            next -= 1;
                        }
                    }
                    if config.pages.len() != self.config.pages.len() {
                        config
                            .save()
                            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
                    }
                    self.config = config;
                    self.page = next;
                }
                Ok(())
            }
            Action::Save(index, name, command, svg) => self.save(index, name, command, svg),
            Action::SaveFolder(index, name) => self.save_folder(index, name),
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
            Action::Drop(source, target, origin, page, make_folder) => {
                self.drop_item(source, target, origin, page, make_folder)
            }
            Action::MoveFolderApp(folder, app, target) => self.move_folder_app(folder, app, target),
            Action::ReorderFolderApp(folder, source, target) => {
                self.reorder_folder_app(folder, source, target)
            }
            Action::AddPredefined(action) => self.add_system_action(&action),
            Action::AddInstalledApp(id) => self.add_installed_app(&id),
            Action::RefreshInstalledApps => self.refresh_installed_apps(),
            Action::SaveWebsite(index, name, url) => self.save_website(index, name, url),
            Action::ChoosePath(index, directory, svg) => {
                let Some(path) = platform::choose_path(directory) else {
                    return Err(String::new());
                };
                let name = path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or_default()
                    .to_string();
                self.save_path(
                    index,
                    name,
                    path.to_string_lossy().into_owned(),
                    directory,
                    svg,
                )
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
            Item::Folder { .. } => Ok(()),
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
                    .installed_apps
                    .iter()
                    .find(|installed| &installed.id == id)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::NotFound, "not found")
                    })?;
                platform::launch(&installed.command)
            }
            Launch::Action(action) => platform::launch_action(action),
        }
    }

    fn add_system_action(&mut self, action: &str) -> Result<(), String> {
        let (name, action) = match action {
            "lock-screen" => ("Lock screen", SystemAction::LockScreen),
            "screenshot" => ("Take screenshot", SystemAction::Screenshot),
            "sleep" => ("Sleep", SystemAction::Sleep),
            "log-out" => ("Log out", SystemAction::LogOut),
            _ => return Err("Action is missing".into()),
        };
        let app = App {
            id: new_id()?,
            name: name.into(),
            icon: Icon::Letter,
            launch: Launch::Action(action),
        };
        self.store(-1, Item::App(app))
    }

    fn add_installed_app(&mut self, id: &str) -> Result<(), String> {
        let Some(installed) = self.installed_apps.iter().find(|app| app.id == id).cloned() else {
            return Err("Installed app is missing".into());
        };
        let item_id = new_id()?;
        let app = App {
            id: item_id.clone(),
            name: installed.name.clone(),
            icon: installed.icon.clone(),
            launch: Launch::Installed(id.into()),
        };
        self.store(-1, Item::App(app))?;
        let _ = self.load_icon(&item_id, &installed.icon);
        Ok(())
    }

    fn refresh_installed_apps(&mut self) -> Result<(), String> {
        let installed_apps = platform::scan_installed_apps()
            .map_err(|error| format!("Couldn't refresh installed apps: {error}"))?;
        platform::write_installed_apps(
            &serde_json::to_vec_pretty(&installed_apps)
                .map_err(|error| format!("Couldn't save installed apps: {error}"))?,
        )
        .map_err(|error| format!("Couldn't save installed apps: {error}"))?;
        self.installed_status = format!("{} installed apps", installed_apps.len());
        self.installed_apps = installed_apps;
        Ok(())
    }

    fn save(
        &mut self,
        index: i32,
        name: String,
        command: String,
        svg: String,
    ) -> Result<(), String> {
        if name.trim().is_empty() {
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
        let icon = match &app.icon {
            Icon::Svg(svg) if matches!(existing, Some(Item::App(old)) if matches!(&old.icon, Icon::Svg(old_svg) if old_svg == svg)) => {
                self.icons.get(&id).cloned()
            }
            Icon::Svg(svg) => Some(decode_svg(svg)?),
            _ => None,
        };
        self.store(index, Item::App(app))?;
        if let Some(icon) = icon {
            self.icons.insert(id, icon);
        } else {
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
                    .insert(index + offset, Item::App(app.clone()));
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

    fn save_website(&mut self, index: i32, name: String, url: String) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Name is empty".into());
        }
        if !valid_url(url.trim()) {
            return Err("URL must start with http:// or https://".into());
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
        name: String,
        path: String,
        directory: bool,
        svg: String,
    ) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Name is empty".into());
        }
        if path.trim().is_empty() {
            return Err("Path is empty".into());
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
        let path = path.trim().to_string();
        let (icon, image) = if svg.is_empty() {
            (Icon::Letter, None)
        } else {
            (Icon::Svg(svg.clone()), Some(decode_svg(&svg)?))
        };
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
        let id = item.id().to_string();
        self.store(index, item)?;
        if let Some(image) = image {
            self.icons.insert(id, image);
        } else {
            self.icons.remove(&id);
        }
        Ok(())
    }

    fn save_folder(&mut self, index: i32, name: String) -> Result<(), String> {
        if name.trim().is_empty() {
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
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        Ok(())
    }

    fn add_page(&mut self) -> Result<(), String> {
        if self.config.pages.len() >= 9 {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.pages.push(crate::config::Page { items: Vec::new() });
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.page = self.config.pages.len() - 1;
        Ok(())
    }

    fn drop_item(
        &mut self,
        source: usize,
        target: usize,
        origin: i32,
        destination: i32,
        make_folder: bool,
    ) -> Result<(), String> {
        let origin = if origin >= 0 {
            origin as usize
        } else {
            self.page
        };
        if origin >= self.config.pages.len() {
            return Ok(());
        }
        let items = &self.config.pages[origin].items;
        if source >= items.len() || target > items.len() {
            return Ok(());
        }
        let mut config = self.config.clone();
        let destination = if destination >= 0 {
            destination as usize
        } else {
            origin
        };
        if destination != origin {
            if destination == config.pages.len() && config.pages.len() < 9 {
                config.pages.push(crate::config::Page { items: Vec::new() });
            }
            if destination >= config.pages.len() {
                return Ok(());
            }
            let item = config.pages[origin].items.remove(source);
            config.pages[destination].items.push(item);
            let next_page = if config.pages.len() > 1 && config.pages[origin].items.is_empty() {
                config.pages.remove(origin);
                destination.saturating_sub(usize::from(destination > origin))
            } else {
                destination
            };
            config
                .save()
                .map_err(|error| format!("Couldn't save config.json: {error}"))?;
            self.config = config;
            self.page = next_page;
            self.open_folder = None;
            return Ok(());
        }
        if source == target {
            return Ok(());
        }
        let items = &mut config.pages[origin].items;
        let source_item = items.remove(source);
        let insert_after = source < target;
        let target = if source < target {
            target.saturating_sub(1)
        } else {
            target
        };
        match (source_item, items.get_mut(target)) {
            (Item::App(app), Some(Item::App(other))) if make_folder => {
                let other = std::mem::replace(other, app.clone());
                items[target] = Item::Folder {
                    id: new_id()?,
                    name: other.name.clone(),
                    apps: vec![other, app],
                };
            }
            (Item::App(app), Some(Item::Folder { apps, .. })) => apps.push(app),
            (item, _) => items.insert((target + usize::from(insert_after)).min(items.len()), item),
        }
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        Ok(())
    }

    fn move_folder_app(
        &mut self,
        folder: usize,
        index: usize,
        target: usize,
    ) -> Result<(), String> {
        let mut config = self.config.clone();
        let removes_folder = matches!(
            config.pages[self.page].items.get(folder),
            Some(Item::Folder { apps, .. }) if apps.len() == 1
        );
        let Some(Item::Folder { apps, .. }) = config.pages[self.page].items.get_mut(folder) else {
            return Ok(());
        };
        if index >= apps.len() {
            return Ok(());
        }
        let app = apps.remove(index);
        if removes_folder {
            config.pages[self.page].items.remove(folder);
        }
        let items = &mut config.pages[self.page].items;
        let target = target.min(items.len());
        let target = target.saturating_sub(usize::from(removes_folder && target > folder));
        items.insert(target.min(items.len()), Item::App(app));
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        self.open_folder = None;
        Ok(())
    }

    fn reorder_folder_app(
        &mut self,
        folder: usize,
        source: usize,
        target: usize,
    ) -> Result<(), String> {
        let mut config = self.config.clone();
        let Some(Item::Folder { apps, .. }) = config.pages[self.page].items.get_mut(folder) else {
            return Ok(());
        };
        if source >= apps.len() || target >= apps.len() || source == target {
            return Ok(());
        }
        let app = apps.remove(source);
        apps.insert(target.min(apps.len()), app);
        config
            .save()
            .map_err(|error| format!("Couldn't save config.json: {error}"))?;
        self.config = config;
        Ok(())
    }

    fn tile(&self, item: &Item, page: usize, index: usize) -> Tile {
        let mut tile = Tile::new(item, &self.icons);
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
                    let mut tile = Tile::new(&Item::App(apps[folder_app].clone()), &self.icons);
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
        let items = page_items(self.page);
        let previous = self.page.checked_sub(1).map(page_items).unwrap_or_default();
        let next = page_items(self.page + 1);
        let page = self.page as i32;
        let count = self.config.pages.len() as i32;
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
        let results: Vec<_> = all
            .iter()
            .filter(|(_, _, item)| {
                !self.query.is_empty() && item.name().to_lowercase().contains(&self.query)
            })
            .map(|(page, index, item)| self.tile(item, *page, *index))
            .collect();
        let installed_apps: Vec<_> = self
            .installed_apps
            .iter()
            .map(|app| {
                (
                    app.id.clone(),
                    app.name.clone(),
                    self.icons.get(&app.id).cloned(),
                )
            })
            .collect();
        let installed_status = self.installed_status.clone();
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
                        let item = Item::App(app.clone());
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
            launcher
                .set_installed_apps(std::rc::Rc::new(slint::VecModel::from(installed_apps)).into());
            launcher.set_installed_status(installed_status.into());
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
    launcher.set_page_count(count);
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
        && shlex::split(command).is_none_or(|args| args.is_empty() || args[0].is_empty())
    {
        return Err("Command is empty or has an unclosed quote".into());
    }
    Ok(App {
        id,
        name: name.trim().into(),
        icon: if svg.trim().is_empty() {
            Icon::Letter
        } else {
            Icon::Svg(svg)
        },
        launch,
    })
}

fn new_id() -> Result<String, String> {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    Ok(format!("{}-{}", std::process::id(), time.as_nanos()))
}

fn decode_svg(svg: &str) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
    let image =
        slint::Image::load_from_svg_data(svg.as_bytes()).map_err(|_| "Couldn't read SVG icon")?;
    image
        .to_rgba8()
        .ok_or_else(|| "Couldn't read SVG icon".into())
}

fn decode_icon(data: &[u8], name: &str) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
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

fn valid_url(url: &str) -> bool {
    let Some((scheme, address)) = url.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https")
        && !address.split('/').next().unwrap_or_default().is_empty()
        && !url.chars().any(char::is_whitespace)
}

fn finish(weak: &slint::Weak<Launcher>, banner: String, saved: bool, launched: bool) {
    let _ = weak.upgrade_in_event_loop(move |launcher| {
        launcher.set_banner(banner.into());
        launcher.set_busy(false);
        if saved {
            launcher.invoke_reset_launcher();
        }
        if launched && let Err(error) = platform::hide(&launcher) {
            launcher.set_banner(error.to_string().into());
        }
    });
}

// Pixel buffers can cross threads; Slint images stay on the interface thread.
#[derive(Default)]
struct Tile {
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
    svg: String,
    icon: Option<SharedPixelBuffer<Rgba8Pixel>>,
    app: bool,
    website: bool,
    file: bool,
    directory: bool,
    folder: bool,
    preview: String,
}

impl Tile {
    fn new(item: &Item, icons: &HashMap<String, SharedPixelBuffer<Rgba8Pixel>>) -> Self {
        let mut tile = Self {
            id: item.id().into(),
            name: item.name().into(),
            ..Self::default()
        };
        match item {
            Item::App(app) => {
                tile.app = true;
                if let Launch::Command(command) = &app.launch {
                    tile.command = Some(command.clone());
                }
                if let Icon::Svg(svg) = &app.icon {
                    tile.svg = svg.clone();
                }
                tile.icon = icons.get(&app.id).cloned();
            }
            Item::Website { id, url, .. } => {
                tile.website = true;
                tile.url = url.clone();
                tile.icon = icons.get(id).cloned();
            }
            Item::File { id, path, icon, .. } => {
                tile.file = true;
                tile.path = path.clone();
                if let Icon::Svg(svg) = icon {
                    tile.svg = svg.clone();
                }
                tile.icon = icons.get(id).cloned();
            }
            Item::Directory { id, path, icon, .. } => {
                tile.directory = true;
                tile.path = path.clone();
                if let Icon::Svg(svg) = icon {
                    tile.svg = svg.clone();
                }
                tile.icon = icons.get(id).cloned();
            }
            Item::Folder { apps, .. } => {
                tile.folder = true;
                for app in apps.iter().take(4) {
                    if !tile.preview.is_empty() {
                        tile.preview.push(' ');
                    }
                    tile.preview
                        .push(app.name.chars().next().unwrap_or_default());
                }
            }
        }
        let icon = match item {
            Item::App(app) => Some(&app.icon),
            Item::Website { icon, .. } | Item::File { icon, .. } | Item::Directory { icon, .. } => {
                Some(icon)
            }
            Item::Folder { .. } => None,
        };
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
        let icon_present = self.icon.is_some() || !self.svg.is_empty();
        AppTile {
            id: self.id.into(),
            page: self.page,
            index: self.index,
            folder_app_index: self.folder_app_index.unwrap_or(-1),
            pinned: self.pinned,
            artwork: self.artwork.into(),
            letter: self
                .name
                .chars()
                .next()
                .map(|letter| letter.to_string())
                .unwrap_or_default()
                .into(),
            name: self.name.into(),
            custom: self.command.is_some(),
            command: self.command.unwrap_or_default().into(),
            url: self.url.into(),
            path: self.path.into(),
            svg: self.svg.into(),
            icon: self.icon.map(slint::Image::from_rgba8).unwrap_or_default(),
            icon_present,
            app: self.app,
            website: self.website,
            file: self.file,
            directory: self.directory,
            folder: self.folder,
            preview: self.preview.into(),
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
            installed_apps: Vec::new(),
            installed_status: String::new(),
            query: String::new(),
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
