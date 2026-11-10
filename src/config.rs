use crate::platform;
use serde::{Deserialize, Serialize};

pub const PINNED_LIMIT: usize = 15;

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip)]
    save_blocked: bool,
    #[serde(skip)]
    migration_original: Option<Vec<u8>>,
    #[serde(default)]
    pub canvas: crate::canvas::Canvas,
    #[serde(default = "legacy_version")]
    version: u32,
    #[serde(default)]
    pub positions: std::collections::BTreeMap<String, crate::spatial::Rect>,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default, rename = "pages", skip_serializing)]
    legacy_pages: Vec<LegacyPage>,
    #[serde(default = "default_columns", rename = "columns", skip_serializing)]
    legacy_columns: usize,
    #[serde(default = "default_rows", rename = "rows", skip_serializing)]
    legacy_rows: usize,
    #[serde(default, rename = "active_page", skip_serializing)]
    legacy_active_page: usize,
    #[serde(default)]
    pub pinned: Vec<String>,
    #[serde(default = "default_item_size")]
    pub item_size: i32,
    #[serde(default = "default_grid_gap")]
    pub grid_gap: i32,
    #[serde(default = "default_canvas_speed")]
    pub canvas_speed: i32,
    #[serde(default)]
    pub background_color: String,
    #[serde(default)]
    pub background_image: String,
    #[serde(default = "default_background_folder")]
    pub background_folder: String,
    #[serde(default = "default_search_engine")]
    pub default_search_engine: String,
    #[serde(default = "default_smooth_cursor")]
    pub smooth_cursor: bool,
    #[serde(default)]
    pub english_assistance: crate::english_assistance::Settings,
    #[serde(default)]
    pub ai: crate::ai::Settings,
    #[serde(default)]
    ai_initialized: bool,
    #[serde(default)]
    pub layout_locked: bool,
    #[serde(default = "crate::shortcuts::defaults")]
    pub shortcuts: Vec<crate::shortcuts::Shortcut>,
    #[serde(default)]
    pub file_search_paths: Vec<String>,
    #[serde(default)]
    pub app_search_paths: Vec<String>,
    #[serde(default = "default_file_search_exclusions")]
    pub file_search_exclusions: Vec<String>,
    #[serde(default)]
    pub directory_search_paths: Vec<String>,
    #[serde(default)]
    pub groups: Vec<CanvasGroup>,
    #[serde(default)]
    pub ink: Vec<InkStroke>,
    #[serde(default = "default_actions")]
    pub user_actions: Vec<UserAction>,
    #[serde(default)]
    pub scripts: Vec<Script>,
    #[serde(default = "default_prefixes")]
    pub prefixes: Vec<Prefix>,
    #[serde(default)]
    site_searches_initialized: bool,
    #[serde(default)]
    search_word_shortcut_initialized: bool,
    #[serde(default)]
    search_completion_shortcut_initialized: bool,
    #[serde(default)]
    settings_shortcut_initialized: bool,
    #[serde(default)]
    canvas_shortcuts_initialized: bool,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct UserAction {
    pub id: String,
    pub name: String,
    pub kind: ActionKind,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum ActionKind {
    Url,
    Shell,
    TickTick,
    Ai,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct Prefix {
    pub id: String,
    pub name: String,
    pub token: String,
    pub action_id: String,
}

// A Group names selected Canvas items and fits them on screen; it does not move or contain them.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct CanvasGroup {
    pub id: String,
    pub name: String,
    pub items: Vec<String>,
    #[serde(default)]
    pub keys: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct InkPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct InkStroke {
    pub id: String,
    pub points: Vec<InkPoint>,
}

#[derive(Clone, Deserialize, Serialize)]
struct LegacyPage {
    #[serde(default)]
    vertical: i8,
    items: Vec<Item>,
}

fn legacy_version() -> u32 {
    1
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Item {
    Robot {
        id: String,
        name: String,
        profile: String,
    },
    Widget {
        id: String,
        name: String,
        text: String,
        variant: usize,
        #[serde(default)]
        ticktick_project_id: String,
        #[serde(default = "default_widget_min")]
        min_number: i32,
        #[serde(default = "default_widget_max")]
        max_number: i32,
    },
    App(App),
    Website {
        id: String,
        name: String,
        url: String,
        icon: Icon,
    },
    File {
        id: String,
        name: String,
        path: String,
        #[serde(default)]
        icon: Icon,
    },
    Directory {
        id: String,
        name: String,
        path: String,
        #[serde(default)]
        icon: Icon,
    },
    Folder {
        id: String,
        name: String,
        apps: Vec<App>,
    },
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub icon: Icon,
    pub launch: Launch,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
pub enum Icon {
    #[default]
    Letter,
    Text(String),
    Svg(String),
    File(String),
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Launch {
    Command(String),
    Installed(String),
    Action(SystemAction),
    Website(String),
    File(String),
    Directory(String),
}

impl App {
    // Folder children retain their original shortcut type when moved back out.
    pub fn into_item(self) -> Item {
        let Self {
            id,
            name,
            icon,
            launch,
        } = self;
        match launch {
            Launch::Website(url) => Item::Website {
                id,
                name,
                icon,
                url,
            },
            Launch::File(path) => Item::File {
                id,
                name,
                icon,
                path,
            },
            Launch::Directory(path) => Item::Directory {
                id,
                name,
                icon,
                path,
            },
            launch => Item::App(Self {
                id,
                name,
                icon,
                launch,
            }),
        }
    }
}

impl Item {
    pub fn folder_child(&self) -> Option<App> {
        let (id, name, icon, launch) = match self {
            Self::App(app) => return Some(app.clone()),
            Self::Website {
                id,
                name,
                icon,
                url,
            } => (id, name, icon, Launch::Website(url.clone())),
            Self::File {
                id,
                name,
                icon,
                path,
            } => (id, name, icon, Launch::File(path.clone())),
            Self::Directory {
                id,
                name,
                icon,
                path,
            } => (id, name, icon, Launch::Directory(path.clone())),
            Self::Folder { .. } | Self::Widget { .. } | Self::Robot { .. } => return None,
        };
        Some(App {
            id: id.clone(),
            name: name.clone(),
            icon: icon.clone(),
            launch,
        })
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum SystemAction {
    LockScreen,
    Screenshot,
    Sleep,
    LogOut,
    VolumeUp,
    VolumeDown,
    Mute,
    MuteMicrophone,
    PlayPause,
    NextTrack,
    PreviousTrack,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub icon: Icon,
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Self::App(app) => &app.name,
            Self::Website { name, .. }
            | Self::File { name, .. }
            | Self::Directory { name, .. }
            | Self::Folder { name, .. }
            | Self::Widget { name, .. }
            | Self::Robot { name, .. } => name,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::App(app) => &app.id,
            Self::Website { id, .. }
            | Self::File { id, .. }
            | Self::Directory { id, .. }
            | Self::Folder { id, .. }
            | Self::Widget { id, .. }
            | Self::Robot { id, .. } => id,
        }
    }
}

fn default_columns() -> usize {
    1920
}
fn default_rows() -> usize {
    1080
}

fn default_item_size() -> i32 {
    128
}
fn default_grid_gap() -> i32 {
    16
}
fn default_canvas_speed() -> i32 {
    5
}

fn default_search_engine() -> String {
    "perplexity".into()
}

const SEARCH_ENGINES: [(&str, &str); 8] = [
    ("brave-search", "Brave"),
    ("google-search", "Google"),
    ("duckduckgo-search", "DuckDuckGo"),
    ("qwant-search", "Qwant"),
    ("bing-search", "Bing"),
    ("startpage-search", "Startpage"),
    ("ecosia-search", "Ecosia"),
    ("perplexity", "Perplexity"),
];

const SEARCH_ENGINE_TEMPLATES: [(&str, &str); 8] = [
    (
        "brave-search",
        "https://search.brave.com/search?q=%s&source=desktop",
    ),
    ("google-search", "https://www.google.com/search?q=%s"),
    ("duckduckgo-search", "https://duckduckgo.com/?q=%s&t=brave"),
    (
        "qwant-search",
        "https://www.qwant.com/?q=%s&client=brz-brave",
    ),
    ("bing-search", "https://www.bing.com/search?q=%s"),
    (
        "startpage-search",
        "https://www.startpage.com/do/search?q=%s&segment=startpage.brave",
    ),
    (
        "ecosia-search",
        "https://www.ecosia.org/search?tt=e8eb07a6&q=%s&addon=brave",
    ),
    ("perplexity", "https://www.perplexity.ai/search?q=%s"),
];

fn default_widget_min() -> i32 {
    1
}
fn default_smooth_cursor() -> bool {
    true
}

fn default_widget_max() -> i32 {
    100
}

pub fn default_file_search_exclusions() -> Vec<String> {
    [
        ".git",
        "node_modules",
        ".*",
        ".local",
        ".claude",
        ".agent",
        ".cache",
        ".venv",
        "__pycache__",
        "dist",
        "build",
        ".next",
        "target",
        ".idea",
        ".vscode",
        ".npm",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_blocked: false,
            migration_original: None,
            canvas: Default::default(),
            version: 5,
            positions: Default::default(),
            items: Vec::new(),
            legacy_pages: Vec::new(),
            legacy_columns: default_columns(),
            legacy_rows: default_rows(),
            legacy_active_page: 0,
            pinned: Vec::new(),
            item_size: default_item_size(),
            grid_gap: default_grid_gap(),
            canvas_speed: default_canvas_speed(),
            background_color: String::new(),
            background_image: String::new(),
            background_folder: "/home/sebaq/Pictures/aesthetic-wallpapers/images".into(),
            default_search_engine: default_search_engine(),
            smooth_cursor: true,
            english_assistance: Default::default(),
            ai: Default::default(),
            ai_initialized: true,
            layout_locked: false,
            shortcuts: crate::shortcuts::defaults(),
            file_search_paths: Vec::new(),
            app_search_paths: Vec::new(),
            file_search_exclusions: default_file_search_exclusions(),
            directory_search_paths: Vec::new(),
            groups: Vec::new(),
            ink: Vec::new(),
            user_actions: default_actions(),
            scripts: Vec::new(),
            prefixes: default_prefixes(),
            site_searches_initialized: true,
            search_word_shortcut_initialized: true,
            search_completion_shortcut_initialized: true,
            settings_shortcut_initialized: true,
            canvas_shortcuts_initialized: true,
        }
    }
}

fn default_background_folder() -> String {
    "/home/sebaq/Pictures/aesthetic-wallpapers/images".into()
}

fn builtin_actions() -> Vec<UserAction> {
    let mut actions = [
        (
            "youtube",
            "YouTube",
            ActionKind::Url,
            "https://www.youtube.com/results?search_query=%s",
        ),
        (
            "chatgpt",
            "ChatGPT",
            ActionKind::Url,
            "https://chatgpt.com/?q=%s",
        ),
        (
            "image-search",
            "Image Search",
            ActionKind::Url,
            "https://www.google.com/search?tbm=isch&q=%s",
        ),
        (
            "perplexity",
            "Perplexity",
            ActionKind::Url,
            "https://www.perplexity.ai/search?q=%s",
        ),
        (
            "claude",
            "Claude",
            ActionKind::Url,
            "https://claude.ai/new?q=%s",
        ),
        (
            "wikipedia",
            "Wikipedia",
            ActionKind::Url,
            "https://en.wikipedia.org/wiki/Special:Search?search=%s",
        ),
        ("ticktick", "TickTick", ActionKind::TickTick, ""),
        ("orbit-ai", "AI", ActionKind::Ai, ""),
    ]
    .into_iter()
    .map(|(id, name, kind, source)| UserAction {
        id: id.into(),
        name: name.into(),
        kind,
        source: source.into(),
        description: match id {
            "youtube" => "Search YouTube for the remaining text after the Prefix Token.".into(),
            "chatgpt" => "Open ChatGPT with the remaining text after the Prefix Token.".into(),
            "image-search" => "Search Google Images for the remaining text after the Prefix Token.".into(),
            "perplexity" => "Search Perplexity for the remaining text after the Prefix Token.".into(),
            "claude" => "Open Claude with the remaining text after the Prefix Token.".into(),
            "wikipedia" => "Search Wikipedia for the remaining text after the Prefix Token.".into(),
            "ticktick" => "Enter a title, then optional !today, !high, !medium, !low, #tag, and ^List or ~List. Example: Buy milk !today #home ~Buy. Put a multiword list at the end, such as ~hall of fame. Omit the list to use Inbox.".into(),
            _ => String::new(),
        },
    })
    .collect::<Vec<_>>();
    actions.shrink_to_fit();
    actions
}

fn default_actions() -> Vec<UserAction> {
    let mut actions = builtin_actions();
    actions.extend(
        [
            (
                "brave-search",
                "Brave",
                "https://search.brave.com/search?q=%s&source=desktop",
            ),
            (
                "google-search",
                "Google",
                "https://www.google.com/search?q=%s",
            ),
            (
                "duckduckgo-search",
                "DuckDuckGo",
                "https://duckduckgo.com/?q=%s&t=brave",
            ),
            (
                "qwant-search",
                "Qwant",
                "https://www.qwant.com/?q=%s&client=brz-brave",
            ),
            ("bing-search", "Bing", "https://www.bing.com/search?q=%s"),
            (
                "startpage-search",
                "Startpage",
                "https://www.startpage.com/do/search?q=%s&segment=startpage.brave",
            ),
            (
                "ecosia-search",
                "Ecosia",
                "https://www.ecosia.org/search?tt=e8eb07a6&q=%s&addon=brave",
            ),
            ("github-search", "GitHub", "https://github.com/search?q=%s"),
            (
                "google-maps-search",
                "Google Maps",
                "https://www.google.com/maps/search/%s",
            ),
            (
                "translate-ar-en-search",
                "Translate (ar -> en)",
                "https://translate.google.com/?sl=ar&tl=en&text=%s",
            ),
            (
                "translate-en-ar-search",
                "Translate (en -> ar)",
                "https://translate.google.com/?sl=en&tl=ar&text=%s",
            ),
        ]
        .into_iter()
        .map(|(id, name, source)| UserAction {
            id: id.into(),
            name: name.into(),
            kind: ActionKind::Url,
            source: source.into(),
            description: format!("Search {name} for your query."),
        }),
    );
    actions
}

fn default_prefixes() -> Vec<Prefix> {
    [
        ("youtube", "YouTube", "y", "youtube"),
        ("chatgpt", "ChatGPT", "g", "chatgpt"),
        ("image-search", "Image Search", "i", "image-search"),
        ("perplexity", "Perplexity", "p", "perplexity"),
        ("claude", "Claude", "c", "claude"),
        ("wikipedia", "Wikipedia", "w", "wikipedia"),
        ("ticktick", "TickTick", "t", "ticktick"),
        ("orbit-ai", "AI", "ai", "orbit-ai"),
        ("chatgpt-ar", "ChatGPT", "ل", "chatgpt"),
        ("claude-ar", "Claude", "ؤ", "claude"),
        ("github", "GitHub", "gh", "github-search"),
        ("google-maps", "Google Maps", "m", "google-maps-search"),
        ("image-search-ar", "Image Search", "ه", "image-search"),
        ("perplexity-ar", "Perplexity", "ح", "perplexity"),
        ("wikipedia-ar", "Wikipedia", "ص", "wikipedia"),
        (
            "translate-ar-en",
            "Translate (ar -> en)",
            "ف",
            "translate-ar-en-search",
        ),
        (
            "translate-en-ar",
            "Translate (en -> ar)",
            "tr",
            "translate-en-ar-search",
        ),
        ("youtube-ar", "YouTube", "غ", "youtube"),
    ]
    .into_iter()
    .chain([
        ("brave-search", "Brave", ":br", "brave-search"),
        ("google-search", "Google", ":g", "google-search"),
        ("duckduckgo-search", "DuckDuckGo", ":d", "duckduckgo-search"),
        ("qwant-search", "Qwant", ":q", "qwant-search"),
        ("bing-search", "Bing", ":b", "bing-search"),
        ("startpage-search", "Startpage", ":sp", "startpage-search"),
        ("ecosia-search", "Ecosia", ":e", "ecosia-search"),
    ])
    .map(|(id, name, token, action_id)| Prefix {
        id: id.into(),
        name: name.into(),
        token: token.into(),
        action_id: action_id.into(),
    })
    .collect()
}

impl Config {
    fn initialize_ai(&mut self) -> bool {
        if self.ai_initialized {
            return false;
        }
        self.ai = crate::ai::Settings {
            address: self.english_assistance.address.clone(),
            local_model: self.english_assistance.model.clone(),
            ..Default::default()
        };
        let id = self.add_ai_action();
        if !self
            .prefixes
            .iter()
            .any(|p| p.token.eq_ignore_ascii_case("ai"))
        {
            self.prefixes.push(Prefix {
                id: id.clone(),
                name: "AI".into(),
                token: "ai".into(),
                action_id: id,
            });
        }
        self.ai_initialized = true;
        true
    }
    fn add_ai_action(&mut self) -> String {
        if let Some(action) = self.user_actions.iter().find(|a| a.kind == ActionKind::Ai) {
            return action.id.clone();
        }
        let mut id = "orbit-ai".to_owned();
        let mut suffix = 1;
        while self.user_actions.iter().any(|a| a.id == id)
            || self.prefixes.iter().any(|p| p.id == id)
        {
            id = format!("orbit-ai-{suffix}");
            suffix += 1;
        }
        self.user_actions.push(UserAction {
            id: id.clone(),
            name: "AI".into(),
            kind: ActionKind::Ai,
            source: String::new(),
            description: "Ask a local model, OpenAI or Gemini inside Orbit.".into(),
        });
        id
    }

    fn add_search_completion_shortcut(&mut self) {
        if !self
            .shortcuts
            .iter()
            .any(|entry| entry.action == 13 || entry.keys == "Ctrl+E")
        {
            self.shortcuts.push(crate::shortcuts::Shortcut {
                action: 13,
                keys: "Ctrl+E".into(),
            });
        }
        self.search_completion_shortcut_initialized = true;
    }
    fn add_search_word_shortcut(&mut self) {
        if !self
            .shortcuts
            .iter()
            .any(|entry| entry.action == 12 || entry.keys == "Ctrl+W")
        {
            self.shortcuts.push(crate::shortcuts::Shortcut {
                action: 12,
                keys: "Ctrl+W".into(),
            });
        }
        self.search_word_shortcut_initialized = true;
    }
    fn add_settings_shortcut(&mut self) {
        if !self
            .shortcuts
            .iter()
            .any(|entry| entry.action == 9 || entry.keys == "Ctrl+S")
        {
            self.shortcuts.push(crate::shortcuts::Shortcut {
                action: 9,
                keys: "Ctrl+S".into(),
            });
        }
        self.settings_shortcut_initialized = true;
    }

    fn add_canvas_shortcuts(&mut self) {
        for binding in crate::shortcuts::canvas_defaults() {
            if !self
                .shortcuts
                .iter()
                .any(|entry| entry.keys == binding.keys)
                && !self.groups.iter().any(|group| group.keys == binding.keys)
            {
                self.shortcuts.push(binding);
            }
        }
        self.canvas_shortcuts_initialized = true;
    }

    pub fn search_engine_names() -> Vec<&'static str> {
        SEARCH_ENGINES.iter().map(|(_, name)| *name).collect()
    }

    pub fn search_engine_id(index: usize) -> Option<&'static str> {
        SEARCH_ENGINES.get(index).map(|(id, _)| *id)
    }

    pub fn default_search_engine_index(&self) -> usize {
        SEARCH_ENGINES
            .iter()
            .position(|(id, _)| *id == self.default_search_engine)
            .unwrap_or(0)
    }

    pub fn search_engine_template(&self, engine: &str) -> &str {
        let engine = if engine.is_empty() {
            &self.default_search_engine
        } else {
            engine
        };
        self.user_actions
            .iter()
            .find(|action| action.id == engine && action.kind == ActionKind::Url)
            .map(|action| action.source.as_str())
            .filter(|template| template.contains("%s"))
            .or_else(|| {
                SEARCH_ENGINE_TEMPLATES
                    .iter()
                    .find(|(id, _)| *id == engine)
                    .map(|(_, template)| *template)
            })
            .unwrap_or(SEARCH_ENGINE_TEMPLATES[0].1)
    }

    fn add_default_search_prefixes(&mut self) -> bool {
        let defaults = Config::default();
        let mut changed = false;
        for action in defaults.user_actions {
            if let Some(existing) = self
                .user_actions
                .iter_mut()
                .find(|item| item.id == action.id)
            {
                if existing.description.is_empty() && !action.description.is_empty() {
                    existing.description = action.description;
                    changed = true;
                }
            } else if action.id.ends_with("-search") {
                self.user_actions.push(action);
                changed = true;
            }
        }
        for prefix in defaults.prefixes {
            let token_exists = self
                .prefixes
                .iter()
                .any(|item| item.token.eq_ignore_ascii_case(&prefix.token));
            let id_exists = self.prefixes.iter().any(|item| item.id == prefix.id);
            if !token_exists && !id_exists {
                self.prefixes.push(prefix);
                changed = true;
            }
        }
        changed
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        platform::write_config(&self.saved_bytes(&crate::storage::config_path()?)?)?;
        Ok(())
    }

    fn saved_bytes(&self, path: &std::path::Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if self.save_blocked {
            return Err("Couldn't load config.json; restart Orbit before saving changes".into());
        }
        let mut saved = self.clone();
        saved.version = 5;
        crate::spatial::reconcile(&mut saved).map_err(std::io::Error::other)?;
        if !saved.valid() {
            return Err("Invalid Canvas configuration".into());
        }
        let data = serde_json::to_vec_pretty(&saved)?;
        if let Some(original) = &self.migration_original {
            migration_backup(path, original)?;
        }
        Ok(data)
    }

    pub fn valid(&self) -> bool {
        let mut script_ids = std::collections::HashSet::new();
        if self.scripts.iter().any(|script| {
            script.id.is_empty()
                || !script_ids.insert(&script.id)
                || script.name.is_empty()
                || script.name.chars().any(char::is_whitespace)
                || script.path.trim().is_empty()
        }) {
            return false;
        }
        let action_ids: std::collections::HashSet<_> = self
            .user_actions
            .iter()
            .map(|action| action.id.as_str())
            .collect();
        if action_ids.len() != self.user_actions.len()
            || self.user_actions.iter().any(|action| {
                action.id.is_empty()
                    || action.name.trim().is_empty()
                    || (action.kind == ActionKind::Url && !action.source.contains("%s"))
            })
        {
            return false;
        }
        let mut prefix_ids = std::collections::HashSet::new();
        let mut tokens = std::collections::HashSet::new();
        if self.prefixes.iter().any(|prefix| {
            prefix.id.is_empty()
                || prefix.name.trim().is_empty()
                || prefix.token.trim().is_empty()
                || !prefix_ids.insert(prefix.id.as_str())
                || !tokens.insert(prefix.token.to_lowercase())
                || !action_ids.contains(prefix.action_id.as_str())
        }) {
            return false;
        }
        let mut ids = std::collections::HashSet::new();
        for item in self
            .items
            .iter()
            .chain(self.legacy_pages.iter().flat_map(|page| &page.items))
        {
            if item.id().is_empty() || !ids.insert(item.id()) {
                return false;
            }
            if let Item::Folder { apps, .. } = item {
                for app in apps {
                    if app.id.is_empty() || !ids.insert(&app.id) {
                        return false;
                    }
                }
            }
            if let Item::Widget {
                id,
                variant,
                text,
                min_number,
                max_number,
                ..
            } = item
            {
                if *variant >= crate::spatial::WIDGETS.len()
                    || text.is_empty()
                    || text.chars().count() > 1000
                    || min_number > max_number
                {
                    return false;
                }
                if let Some(rect) = self.positions.get(id) {
                    let (min_width, min_height) = crate::spatial::WIDGET_MIN_SPANS[*variant];
                    let (max_width, max_height) = crate::spatial::WIDGET_MAX_SPANS[*variant];
                    let bounds = if self.version >= 4 {
                        (
                            crate::spatial::widget_span(min_width, self.item_size, self.grid_gap),
                            crate::spatial::widget_span(min_height, self.item_size, self.grid_gap),
                            crate::spatial::widget_span(max_width, self.item_size, self.grid_gap),
                            crate::spatial::widget_span(max_height, self.item_size, self.grid_gap),
                        )
                    } else if self.version >= 3 {
                        let pitch = crate::spatial::cell_pitch(self.item_size, self.grid_gap);
                        (
                            min_width * pitch,
                            min_height * pitch,
                            max_width * pitch,
                            max_height * pitch,
                        )
                    } else {
                        (min_width, min_height, max_width, max_height)
                    };
                    if rect.width < bounds.0
                        || rect.height < bounds.1
                        || rect.width > bounds.2
                        || rect.height > bounds.3
                    {
                        return false;
                    }
                }
            }
        }
        if self.positions.values().any(|rect| !rect.valid()) {
            return false;
        }
        let mut group_ids = std::collections::HashSet::new();
        let mut group_names = std::collections::HashSet::new();
        let mut group_keys = std::collections::HashSet::new();
        if self.groups.iter().any(|group| {
            let mut members = std::collections::HashSet::new();
            group.id.is_empty()
                || group.name.trim().is_empty()
                || group.name.chars().count() > 60
                || !group_names.insert(group.name.trim().to_lowercase())
                || group.items.is_empty()
                || group
                    .items
                    .iter()
                    .any(|id| !ids.contains(id.as_str()) || !members.insert(id.as_str()))
                || !group_ids.insert(group.id.as_str())
                || (!group.keys.is_empty() && !group_keys.insert(group.keys.as_str()))
                || crate::shortcuts::validate_group_keys(
                    &group.keys,
                    &group.id,
                    &self.shortcuts,
                    &self.groups,
                )
                .is_err()
        }) {
            return false;
        }
        let mut ink_ids = std::collections::HashSet::new();
        if self
            .ink
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>()
            > crate::ink::MAX_POINTS
            || self.ink.iter().any(|stroke| {
                stroke.id.is_empty()
                    || !ink_ids.insert(stroke.id.as_str())
                    || !(2..=2000).contains(&stroke.points.len())
                    || stroke.points.iter().any(|point| {
                        !point.x.is_finite()
                            || !point.y.is_finite()
                            || point.x.abs() > 10_000_000.0
                            || point.y.abs() > 10_000_000.0
                    })
            })
        {
            return false;
        }
        self.canvas.valid()
            && (1..=5).contains(&self.version)
            && (self.legacy_pages.is_empty() || self.items.is_empty())
            && (self.version >= 5
                || (!self.legacy_pages.is_empty()
                    && self.legacy_columns > 0
                    && self.legacy_rows > 0))
            && self
                .legacy_pages
                .iter()
                .all(|page| (-1..=1).contains(&page.vertical))
            && crate::shortcuts::valid(
                &self
                    .shortcuts
                    .iter()
                    .filter(|entry| !matches!(entry.action, 10 | 11))
                    .cloned()
                    .collect::<Vec<_>>(),
            )
            && (80..=240).contains(&self.item_size)
            && (0..=64).contains(&self.grid_gap)
            && (1..=10).contains(&self.canvas_speed)
            && SEARCH_ENGINES
                .iter()
                .any(|(id, _)| *id == self.default_search_engine)
    }
}

pub fn load() -> (Config, String) {
    match read() {
        Ok(result) => result,
        // An unreadable or unmigratable original must never be replaced by the empty fallback.
        Err(error) => (
            Config {
                save_blocked: true,
                ..Default::default()
            },
            format!("Couldn't read or save config.json: {error}"),
        ),
    }
}

fn merge_remembrances(
    config: &mut Config,
    progress: Option<&std::path::Path>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut pairs = Vec::new();
    for page in &config.legacy_pages {
        let morning: Vec<_> = page
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Widget {
                    id, variant: 18, ..
                } => Some(id.clone()),
                _ => None,
            })
            .collect();
        let evening = page.items.iter().filter_map(|item| match item {
            Item::Widget {
                id, variant: 19, ..
            } => Some(id.clone()),
            _ => None,
        });
        pairs.extend(morning.into_iter().zip(evening));
    }
    if !pairs.is_empty()
        && let Some(path) = progress
    {
        let db = crate::widget_data::open(path).map_err(std::io::Error::other)?;
        for (keep, removed) in &pairs {
            crate::adhkar::merge_progress(&db, keep, removed).map_err(std::io::Error::other)?;
        }
    }
    let mut changed = !pairs.is_empty();
    for page in &mut config.legacy_pages {
        page.items
            .retain(|item| !pairs.iter().any(|(_, removed)| removed == item.id()));
        for item in &mut page.items {
            if let Item::Widget { variant, name, .. } = item
                && matches!(*variant, 18 | 19)
            {
                if *variant == 19 {
                    *variant = 18;
                    changed = true;
                }
                if matches!(
                    name.as_str(),
                    "Morning Remembrances" | "Evening Remembrances"
                ) {
                    *name = "Remembrances".into();
                    changed = true;
                }
            }
        }
    }
    config
        .positions
        .retain(|id, _| !pairs.iter().any(|(_, removed)| id == removed));
    config
        .pinned
        .retain(|id| !pairs.iter().any(|(_, removed)| id == removed));
    Ok(changed)
}

pub fn decode(data: &[u8]) -> Result<Config, Box<dyn std::error::Error>> {
    let mut config: Config = serde_json::from_slice(data)?;
    if !config.valid() {
        return Err("Invalid Canvas configuration".into());
    }
    migrate(&mut config, None)?;
    Ok(config)
}

fn migrate(
    config: &mut Config,
    progress: Option<&std::path::Path>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let legacy = config.version < 5 || !config.legacy_pages.is_empty();
    let mut changed = config.initialize_ai();
    changed |= merge_remembrances(config, progress)?;
    changed |= config.english_assistance.migrate_prompts();
    if config.version < 3 {
        let pitch = crate::spatial::cell_pitch(config.item_size, config.grid_gap);
        for rect in config.positions.values_mut() {
            rect.row = rect
                .row
                .checked_mul(pitch as i32)
                .ok_or("Canvas coordinate overflow")?;
            rect.col = rect
                .col
                .checked_mul(pitch as i32)
                .ok_or("Canvas coordinate overflow")?;
            rect.width = rect
                .width
                .checked_mul(pitch)
                .ok_or("Canvas size overflow")?;
            rect.height = rect
                .height
                .checked_mul(pitch)
                .ok_or("Canvas size overflow")?;
        }
        config.legacy_columns = config
            .legacy_columns
            .checked_mul(pitch)
            .ok_or("Canvas size overflow")?;
        config.legacy_rows = config
            .legacy_rows
            .checked_mul(pitch)
            .ok_or("Canvas size overflow")?;
        config.version = 3;
    }
    if config.version < 4 {
        for item in config.legacy_pages.iter().flat_map(|page| &page.items) {
            if matches!(item, Item::Widget { .. })
                && let Some(rect) = config.positions.get_mut(item.id())
            {
                let pitch = crate::spatial::cell_pitch(config.item_size, config.grid_gap);
                rect.width = crate::spatial::widget_span(
                    (rect.width / pitch).max(1),
                    config.item_size,
                    config.grid_gap,
                );
                rect.height = crate::spatial::widget_span(
                    (rect.height / pitch).max(1),
                    config.item_size,
                    config.grid_gap,
                );
            }
        }
    }
    if legacy {
        flatten(config)?;
        config.version = 5;
        changed = true;
    }
    if !config.search_completion_shortcut_initialized {
        config.add_search_completion_shortcut();
        changed = true;
    }
    if !config.search_word_shortcut_initialized {
        config.add_search_word_shortcut();
        changed = true;
    }
    if !config.settings_shortcut_initialized {
        config.add_settings_shortcut();
        changed = true;
    }
    if !config.canvas_shortcuts_initialized {
        config.add_canvas_shortcuts();
        changed = true;
    }
    if !config.site_searches_initialized {
        config.add_default_search_prefixes();
        config.site_searches_initialized = true;
        changed = true;
    }
    let previous_shortcuts = config.shortcuts.len();
    config
        .shortcuts
        .retain(|entry| !matches!(entry.action, 10 | 11));
    changed |= config.shortcuts.len() != previous_shortcuts;
    let before = config.positions.clone();
    crate::spatial::reconcile(config).map_err(std::io::Error::other)?;
    changed |= before != config.positions;
    if !config.valid() {
        return Err("Invalid migrated Canvas configuration".into());
    }
    Ok(changed)
}

fn flatten(config: &mut Config) -> Result<(), String> {
    let pitch = crate::spatial::cell_pitch(config.item_size, config.grid_gap);
    let mut offset = 0i64;
    for (index, page) in std::mem::take(&mut config.legacy_pages)
        .into_iter()
        .enumerate()
    {
        let mut local = std::collections::BTreeMap::new();
        for (slot, item) in page.items.iter().enumerate() {
            let rect = config.positions.get(item.id()).copied().unwrap_or_else(|| {
                let (width, height) = if let Item::Widget { variant, .. } = item {
                    let (w, h) = crate::spatial::WIDGET_SPANS[*variant];
                    (
                        crate::spatial::widget_span(w, config.item_size, config.grid_gap),
                        crate::spatial::widget_span(h, config.item_size, config.grid_gap),
                    )
                } else {
                    (pitch, pitch)
                };
                let columns = (config.legacy_columns / pitch).max(1);
                crate::spatial::Rect {
                    col: ((slot % columns) * pitch) as i32,
                    row: ((slot / columns) * pitch) as i32,
                    width,
                    height,
                }
            });
            local.insert(item.id().to_owned(), rect);
        }
        let left = local
            .values()
            .map(|r| i64::from(r.col))
            .min()
            .unwrap_or(0)
            .min(0);
        let right = local
            .values()
            .map(|r| i64::from(r.col) + r.width as i64)
            .max()
            .unwrap_or(0)
            .max(config.legacy_columns as i64);
        let translation = offset - left;
        if index == config.legacy_active_page {
            config.canvas.view = crate::canvas::View {
                x: -(translation as f32) + 24.0,
                y: 110.0,
                zoom: 1.0,
            };
        }
        for (id, mut rect) in local {
            rect.col = i32::try_from(i64::from(rect.col) + translation)
                .map_err(|_| "Canvas coordinate overflow")?;
            config.positions.insert(id, rect);
        }
        offset += right - left + pitch as i64;
        config.items.extend(page.items);
    }
    Ok(())
}

fn migration_backup(
    path: &std::path::Path,
    original: &[u8],
) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_nanos();
    let backup = path.with_file_name(format!("config-before-canvas-{stamp}.json"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)?;
    file.write_all(original)?;
    file.sync_all()?;
    Ok(backup)
}

fn read() -> Result<(Config, String), Box<dyn std::error::Error>> {
    let Some(text) = platform::read_config()? else {
        let config = Config::default();
        config.save()?;
        return Ok((config, String::new()));
    };
    let Ok(mut config) = serde_json::from_slice::<Config>(&text) else {
        platform::backup_config()?;
        return Ok((
            Config::default(),
            "Couldn't read config.json. Saved as config.json.bad; Orbit starts empty.".into(),
        ));
    };
    if !config.valid() {
        platform::backup_config()?;
        return Ok((
            Config::default(),
            "Couldn't read config.json. Saved as config.json.bad; Orbit starts empty.".into(),
        ));
    }
    let original = config.clone();
    let path = crate::storage::config_path()?;
    let legacy = config.version < 5 || !config.legacy_pages.is_empty();
    // Keep the exact legacy bytes before progress or layout migration changes saved data.
    if legacy && let Err(error) = migration_backup(&path, &text) {
        migrate(&mut config, None)?;
        config.migration_original = Some(text.clone());
        return Ok((
            config,
            format!("Couldn't back up config.json; migration was not saved: {error}"),
        ));
    }
    match migrate(&mut config, Some(&path.with_file_name("widgets.db"))) {
        Ok(changed) => {
            if changed && let Err(error) = config.save() {
                return Ok((
                    config,
                    format!("Couldn't save migrated config.json; original retained: {error}"),
                ));
            }
            Ok((config, String::new()))
        }
        Err(error) => {
            let mut usable = original;
            migrate(&mut usable, None)?;
            usable.migration_original = Some(text.clone());
            Ok((
                usable,
                format!("Couldn't migrate config.json; original retained: {error}"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_migration_of_omitted_action_fields_does_not_duplicate_defaults() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        for field in ["ai_initialized", "ai", "user_actions", "prefixes"] {
            value.as_object_mut().unwrap().remove(field);
        }
        let mut config: Config = serde_json::from_value(value).unwrap();
        assert!(config.initialize_ai());
        assert_eq!(
            config
                .user_actions
                .iter()
                .filter(|a| a.kind == ActionKind::Ai)
                .count(),
            1
        );
        assert_eq!(
            config.prefixes.iter().filter(|p| p.token == "ai").count(),
            1
        );
        assert!(config.valid());
    }

    #[test]
    fn ai_migration_preserves_user_tokens_and_is_one_time() {
        let mut config = Config::default();
        config.ai_initialized = false;
        config.user_actions.retain(|a| a.kind != ActionKind::Ai);
        config.prefixes.retain(|p| p.token != "ai");
        config.prefixes.push(Prefix {
            id: "custom-ai".into(),
            name: "Custom".into(),
            token: "AI".into(),
            action_id: "youtube".into(),
        });
        config.english_assistance.model = "my-local-model".into();
        assert!(config.initialize_ai());
        assert_eq!(config.ai.local_model, "my-local-model");
        assert_eq!(
            config
                .prefixes
                .iter()
                .filter(|p| p.token.eq_ignore_ascii_case("ai"))
                .count(),
            1
        );
        assert_eq!(config.prefixes.last().unwrap().action_id, "youtube");
        assert!(config.valid());
        let count = config.user_actions.len();
        assert!(!config.initialize_ai());
        assert_eq!(config.user_actions.len(), count);
        let saved = serde_json::to_value(&config).unwrap();
        assert!(saved["ai"].get("api_key").is_none());
    }

    #[test]
    fn ai_migration_installs_prefix_without_action_id_collisions() {
        let mut config = Config::default();
        config.ai_initialized = false;
        config
            .user_actions
            .iter_mut()
            .find(|a| a.id == "orbit-ai")
            .unwrap()
            .kind = ActionKind::Shell;
        config.prefixes.retain(|p| p.token != "ai");
        assert!(config.initialize_ai());
        assert!(config.valid());
        let prefix = config.prefixes.iter().find(|p| p.token == "ai").unwrap();
        assert_eq!(prefix.action_id, "orbit-ai-1");
    }

    #[test]
    fn completion_shortcut_migration_preserves_conflicts_and_removal() {
        let mut config = Config::default();
        config.shortcuts.retain(|entry| entry.action != 13);
        config.shortcuts[0].keys = "Ctrl+E".into();
        config.add_search_completion_shortcut();
        assert!(!config.shortcuts.iter().any(|entry| entry.action == 13));
        config.shortcuts[0].keys = "Ctrl+K".into();
        config.add_search_completion_shortcut();
        assert!(
            config
                .shortcuts
                .iter()
                .any(|entry| entry.action == 13 && entry.keys == "Ctrl+E")
        );
        config.shortcuts.retain(|entry| entry.action != 13);
        let loaded: Config = serde_json::from_value(serde_json::to_value(config).unwrap()).unwrap();
        assert!(loaded.search_completion_shortcut_initialized);
        assert!(!loaded.shortcuts.iter().any(|entry| entry.action == 13));
    }

    #[test]
    fn layout_lock_defaults_to_unlocked_and_round_trips() {
        let mut saved = serde_json::to_value(Config::default()).unwrap();
        saved.as_object_mut().unwrap().remove("layout_locked");
        let mut config: Config = serde_json::from_value(saved).unwrap();
        assert!(!config.layout_locked);
        for locked in [true, false] {
            config.layout_locked = locked;
            let restored: Config =
                serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
            assert_eq!(restored.layout_locked, locked);
        }
    }

    #[test]
    fn first_run_has_an_empty_canvas() {
        let config = Config::default();
        assert!(config.valid());
        assert!(config.items.is_empty());
    }

    #[test]
    fn missing_search_engine_setting_defaults_to_perplexity_and_selected_engine_persists() {
        let mut legacy = serde_json::to_value(Config::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("default_search_engine");
        let mut config: Config = serde_json::from_value(legacy).unwrap();
        assert_eq!(config.default_search_engine, "perplexity");
        config.default_search_engine = "google-search".into();
        let restored: Config =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(restored.default_search_engine_index(), 1);
        assert_eq!(
            restored.search_engine_template(""),
            "https://www.google.com/search?q=%s"
        );
    }

    #[test]
    fn default_prefixes_and_actions_include_builtin_and_web_search_destinations() {
        let config = Config::default();
        let expected = [
            ("y", "YouTube", "youtube"),
            ("g", "ChatGPT", "chatgpt"),
            ("i", "Image Search", "image-search"),
            ("p", "Perplexity", "perplexity"),
            ("c", "Claude", "claude"),
            ("w", "Wikipedia", "wikipedia"),
            ("t", "TickTick", "ticktick"),
        ];
        for (token, name, action_id) in expected {
            let prefix = config
                .prefixes
                .iter()
                .find(|prefix| prefix.token == token)
                .unwrap();
            let action = config
                .user_actions
                .iter()
                .find(|action| action.id == action_id)
                .unwrap();
            assert_eq!(prefix.name, name);
            assert_eq!(prefix.action_id, action_id);
            assert_eq!(action.name, name);
        }
        for (token, name, action_id) in [
            (":br", "Brave", "brave-search"),
            (":g", "Google", "google-search"),
            (":d", "DuckDuckGo", "duckduckgo-search"),
            (":q", "Qwant", "qwant-search"),
            (":b", "Bing", "bing-search"),
            (":sp", "Startpage", "startpage-search"),
            (":e", "Ecosia", "ecosia-search"),
        ] {
            let prefix = config
                .prefixes
                .iter()
                .find(|prefix| prefix.token == token)
                .unwrap();
            let action = config
                .user_actions
                .iter()
                .find(|action| action.id == action_id)
                .unwrap();
            assert_eq!(prefix.name, name);
            assert_eq!(prefix.action_id, action_id);
            assert_eq!(action.name, name);
        }
        for (token, name, action_id) in [
            ("ل", "ChatGPT", "chatgpt"),
            ("ؤ", "Claude", "claude"),
            ("gh", "GitHub", "github-search"),
            ("m", "Google Maps", "google-maps-search"),
            ("ه", "Image Search", "image-search"),
            ("ح", "Perplexity", "perplexity"),
            ("ص", "Wikipedia", "wikipedia"),
            ("ف", "Translate (ar -> en)", "translate-ar-en-search"),
            ("tr", "Translate (en -> ar)", "translate-en-ar-search"),
            ("غ", "YouTube", "youtube"),
        ] {
            let prefix = config
                .prefixes
                .iter()
                .find(|prefix| prefix.token == token)
                .unwrap();
            assert_eq!(prefix.name, name);
            assert_eq!(prefix.action_id, action_id);
            assert!(
                config
                    .user_actions
                    .iter()
                    .any(|action| action.id == action_id)
            );
        }
        assert_eq!(config.prefixes.len(), 25);
        assert_eq!(config.user_actions.len(), 19);
    }

    #[test]
    fn prefix_and_action_config_round_trips_without_losing_custom_entries() {
        let mut config = Config::default();
        config.user_actions.push(UserAction {
            id: "custom-shell".into(),
            name: "Custom shell".into(),
            kind: ActionKind::Shell,
            source: "printf '%s' \"$1\"".into(),
            description: "Print the query".into(),
        });
        config.prefixes.push(Prefix {
            id: "custom-prefix".into(),
            name: "Custom".into(),
            token: "z".into(),
            action_id: "custom-shell".into(),
        });
        let saved = serde_json::to_vec(&config).unwrap();
        let restored: Config = serde_json::from_slice(&saved).unwrap();
        assert!(restored.valid());
        assert!(restored.user_actions == config.user_actions);
        assert!(restored.prefixes == config.prefixes);
    }

    #[test]
    fn invalid_prefix_tokens_and_missing_actions_are_rejected() {
        let mut config = Config::default();
        config.prefixes[1].token = config.prefixes[0].token.clone();
        assert!(!config.valid());

        let mut config = Config::default();
        config.prefixes[0].action_id = "missing".into();
        assert!(!config.valid());
    }

    #[test]
    fn legacy_regions_preserve_all_items_ids_sizes_and_active_view_once() {
        for version in 1..=4 {
            let mut value = serde_json::to_value(Config::default()).unwrap();
            value["version"] = version.into();
            value.as_object_mut().unwrap().remove("items");
            value["columns"] = if version < 3 { 8 } else { 1152 }.into();
            value["rows"] = if version < 3 { 6 } else { 864 }.into();
            value["active_page"] = 2.into();
            value["pages"] = serde_json::json!([
                {"items":[{"Website":{"id":"a","name":"Site","url":"https://example.com","icon":"Letter"}}]},
                {"vertical":-1,"items":[{"Folder":{"id":"folder","name":"Folder","apps":[{"id":"child","name":"Child","icon":"Letter","launch":{"File":"/tmp/saved"}}]}}]},
                {"vertical":1,"items":[{"Widget":{"id":"widget","name":"Quote","text":"Saved state","variant":9,"min_number":1,"max_number":7}}]}
            ]);
            value["positions"] = if version < 3 {
                serde_json::json!({"a":{"col":2,"row":1,"width":1,"height":1},"widget":{"col":1,"row":2,"width":2,"height":2}})
            } else {
                serde_json::json!({"a":{"col":288,"row":144,"width":144,"height":144},"widget":{"col":144,"row":288,"width":288,"height":288}})
            };
            value["pinned"] = serde_json::json!(["child"]);
            value["canvas"] = serde_json::json!({"enabled":false,"boxes":[{"id":"discard","name":"Box","x":0,"y":0}]});
            let config = decode(&serde_json::to_vec(&value).unwrap()).unwrap();
            assert_eq!(config.version, 5);
            assert_eq!(config.items.len(), 4);
            assert_eq!(config.pinned, ["child"]);
            assert_eq!(config.positions["a"].row, 144);
            assert_eq!(
                config.positions["widget"].height,
                if version < 4 { 272 } else { 288 }
            );
            assert!(config.canvas.view.x < -2000.0);
            assert!(config.items.iter().any(
                |item| matches!(item,Item::File {id,path,..} if id=="child" && path=="/tmp/saved")
            ));
            let saved = serde_json::to_vec(&config).unwrap();
            let saved_value: serde_json::Value = serde_json::from_slice(&saved).unwrap();
            for retired in ["pages", "columns", "rows", "active_page", "page_margin"] {
                assert!(saved_value.get(retired).is_none());
            }
            assert!(saved_value["canvas"].get("boxes").is_none());
            let again = decode(&saved).unwrap();
            assert_eq!(config.positions, again.positions);
            assert_eq!(config.canvas, again.canvas);
        }
    }
    #[test]
    fn failed_migration_backup_blocks_subsequent_saves() {
        let mut config = Config::default();
        config.migration_original = Some(b"original".to_vec());
        config.canvas.view.x = 500.0;
        let root =
            std::env::temp_dir().join(format!("orbit-blocked-migration-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let original = root.join("config.json");
        std::fs::write(&original, b"original").unwrap();
        // A non-directory parent deterministically prevents backup creation on every platform.
        let blocked = original.join("config.json");
        assert!(config.saved_bytes(&blocked).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        assert!(config.saved_bytes(&blocked).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        assert!(config.saved_bytes(&original).is_ok());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn backup_failure_does_not_change_the_original_file() {
        let root =
            std::env::temp_dir().join(format!("orbit-migration-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("config.json");
        std::fs::write(&path, b"original").unwrap();
        assert!(migration_backup(&root.join("missing/config.json"), b"original").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        let backup = migration_backup(&path, b"original").unwrap();
        assert_eq!(std::fs::read(backup).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod cursor_tests {
    use super::Config;
    #[test]
    fn old_config_enables_smooth_cursor_and_false_round_trips() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value.as_object_mut().unwrap().remove("smooth_cursor");
        let mut config: Config = serde_json::from_value(value).unwrap();
        assert!(config.smooth_cursor);
        config.smooth_cursor = false;
        let config: Config = serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert!(!config.smooth_cursor);
    }
}

#[cfg(test)]
mod search_shortcut_tests {
    use super::Config;

    #[test]
    fn adds_settings_shortcut_to_old_settings_without_restoring_removed_bindings() {
        let mut config = Config::default();
        config.shortcuts.retain(|entry| entry.action != 9);
        config.settings_shortcut_initialized = false;
        config.add_settings_shortcut();
        assert!(
            config
                .shortcuts
                .iter()
                .any(|entry| entry.action == 9 && entry.keys == "Ctrl+S")
        );
        config.shortcuts.retain(|entry| entry.action != 9);
        let saved = serde_json::to_vec(&config).unwrap();
        let loaded: Config = serde_json::from_slice(&saved).unwrap();
        assert!(loaded.settings_shortcut_initialized);
        assert!(!loaded.shortcuts.iter().any(|entry| entry.action == 9));
    }

    #[test]
    fn migration_preserves_an_existing_settings_or_ctrl_s_binding() {
        let mut config = Config::default();
        config.shortcuts.retain(|entry| entry.action != 9);
        config.shortcuts[0].keys = "Ctrl+S".into();
        config.settings_shortcut_initialized = false;
        config.add_settings_shortcut();
        assert!(!config.shortcuts.iter().any(|entry| entry.action == 9));
        assert_eq!(config.shortcuts[0].keys, "Ctrl+S");

        let mut config = Config::default();
        config.shortcuts.retain(|entry| entry.action != 9);
        config.shortcuts.push(crate::shortcuts::Shortcut {
            action: 9,
            keys: "Alt+S".into(),
        });
        config.settings_shortcut_initialized = false;
        config.add_settings_shortcut();
        assert!(!config.shortcuts.iter().any(|entry| entry.keys == "Ctrl+S"));
    }

    #[test]
    fn adds_word_shortcut_to_old_settings_without_restoring_removed_bindings() {
        let mut config = Config::default();
        config.shortcuts.retain(|entry| entry.action != 12);
        config.search_word_shortcut_initialized = false;
        config.add_search_word_shortcut();
        assert!(
            config
                .shortcuts
                .iter()
                .any(|entry| entry.action == 12 && entry.keys == "Ctrl+W")
        );
        config.shortcuts.retain(|entry| entry.action != 12);
        let saved = serde_json::to_vec(&config).unwrap();
        let loaded: Config = serde_json::from_slice(&saved).unwrap();
        assert!(loaded.search_word_shortcut_initialized);
        assert!(!loaded.shortcuts.iter().any(|entry| entry.action == 12));
    }

    #[test]
    fn migration_preserves_an_existing_ctrl_w_binding() {
        let mut config = Config::default();
        config.shortcuts.retain(|entry| entry.action != 12);
        config.shortcuts[0].keys = "Ctrl+W".into();
        config.search_word_shortcut_initialized = false;
        config.add_search_word_shortcut();
        assert!(!config.shortcuts.iter().any(|entry| entry.action == 12));
        assert_eq!(config.shortcuts[0].keys, "Ctrl+W");
    }
}

#[cfg(test)]
mod file_search_exclusion_tests {
    use super::*;

    #[test]
    fn missing_exclusions_get_defaults_but_empty_lists_stay_empty() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("file_search_exclusions");
        let migrated: Config = serde_json::from_value(value).unwrap();
        assert_eq!(
            migrated.file_search_exclusions,
            default_file_search_exclusions()
        );
        let config = Config {
            file_search_exclusions: Vec::new(),
            ..Config::default()
        };
        let loaded: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert!(loaded.file_search_exclusions.is_empty());
        assert_eq!(loaded.version, config.version);
    }
}

#[cfg(test)]
mod exclusion_persistence_tests {
    use super::*;

    #[test]
    fn custom_exclusions_round_trip_without_changing_search_roots_or_items() {
        let mut config = Config {
            file_search_exclusions: vec!["*.tmp".into(), "Build".into()],
            ..Config::default()
        };
        config.file_search_paths.push("/documents".into());
        config.directory_search_paths.push("/projects".into());
        let saved = serde_json::to_vec(&config).unwrap();
        let loaded: Config = serde_json::from_slice(&saved).unwrap();
        assert_eq!(loaded.file_search_exclusions, ["*.tmp", "Build"]);
        assert_eq!(loaded.file_search_paths, config.file_search_paths);
        assert_eq!(loaded.directory_search_paths, config.directory_search_paths);
        assert_eq!(loaded.items.len(), config.items.len());
    }
}

#[cfg(test)]
mod canvas_group_tests {
    use super::*;

    #[test]
    fn canvas_shortcuts_preserve_existing_bindings_and_deleted_defaults() {
        let mut config = config_with_group();
        config.shortcuts.retain(|entry| entry.action < 14);
        config.shortcuts.push(crate::shortcuts::Shortcut {
            action: 0,
            keys: "Ctrl+Z".into(),
        });
        config.groups[0].keys = "Ctrl+V".into();
        config.canvas_shortcuts_initialized = false;
        config.add_canvas_shortcuts();
        assert!(config.canvas_shortcuts_initialized);
        assert!(
            !config
                .shortcuts
                .iter()
                .any(|entry| entry.action == 14 || entry.action == 18)
        );
        assert!(config.valid());
        config.shortcuts.retain(|entry| entry.action != 21);
        let loaded: Config = serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert!(loaded.canvas_shortcuts_initialized);
        assert!(!loaded.shortcuts.iter().any(|entry| entry.action == 21));
    }

    fn config_with_group() -> Config {
        let mut config = Config::default();
        config.items.push(Item::App(App {
            id: "a".into(),
            name: "A".into(),
            icon: Icon::Letter,
            launch: Launch::Command("true".into()),
        }));
        config.positions.insert(
            "a".into(),
            crate::spatial::Rect {
                col: 0,
                row: 0,
                width: 144,
                height: 144,
            },
        );
        config.groups.push(CanvasGroup {
            id: "g".into(),
            name: "Work".into(),
            items: vec!["a".into()],
            keys: "Ctrl+1".into(),
        });
        config
    }

    #[test]
    fn groups_require_members_unique_names_and_modified_unique_keys() {
        let mut config = config_with_group();
        assert!(config.valid());
        let saved = serde_json::to_vec(&config).unwrap();
        let loaded: Config = serde_json::from_slice(&saved).unwrap();
        assert_eq!(loaded.groups.len(), 1);

        config.groups[0].items = vec!["missing".into()];
        assert!(!config.valid());
        config.groups[0].items = vec!["a".into()];

        config.groups.push(CanvasGroup {
            id: "h".into(),
            name: " work ".into(),
            items: vec!["a".into()],
            keys: String::new(),
        });
        assert!(!config.valid());
        config.groups[1].name = "Personal".into();
        assert!(config.valid());

        config.groups[1].keys = "A".into();
        assert!(!config.valid());
        config.groups[1].keys = "1".into();
        assert!(!config.valid());
        config.groups[1].keys = "Ctrl+1".into();
        assert!(!config.valid());
    }

    #[test]
    fn groups_drop_removed_members_and_disappear_when_empty() {
        let mut config = config_with_group();
        config.groups[0].items.push("missing".into());
        crate::spatial::reconcile(&mut config).unwrap();
        assert_eq!(config.groups[0].items, ["a"]);
        config.items.clear();
        crate::spatial::reconcile(&mut config).unwrap();
        assert!(config.groups.is_empty());
    }

    #[test]
    fn ink_requires_two_to_two_thousand_points_and_unique_ids() {
        let mut config = Config::default();
        config.ink.push(InkStroke {
            id: "s".into(),
            points: vec![InkPoint { x: 0.0, y: 0.0 }, InkPoint { x: 1.0, y: 1.0 }],
        });
        assert!(config.valid());
        config.ink[0].points.pop();
        assert!(!config.valid());
        config.ink[0].points = vec![InkPoint { x: 0.0, y: 0.0 }, InkPoint { x: 1.0, y: 1.0 }];
        config.ink.push(InkStroke {
            id: "s".into(),
            points: vec![InkPoint { x: 0.0, y: 0.0 }, InkPoint { x: 1.0, y: 1.0 }],
        });
        assert!(!config.valid());
    }
}
