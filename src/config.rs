use crate::platform;
use serde::{Deserialize, Serialize};

pub const PINNED_LIMIT: usize = 10;

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub canvas: crate::canvas::Canvas,
    version: u32,
    #[serde(default)]
    pub positions: std::collections::BTreeMap<String, crate::spatial::Rect>,
    #[serde(default = "default_columns")]
    pub columns: usize,
    #[serde(default = "default_rows")]
    pub rows: usize,
    #[serde(default)]
    pub active_page: usize,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub pinned: Vec<String>,
    #[serde(default = "default_page_margin")]
    pub page_margin: i32,
    #[serde(default = "default_item_size")]
    pub item_size: i32,
    #[serde(default = "default_grid_gap")]
    pub grid_gap: i32,
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
    #[serde(default = "default_actions")]
    pub user_actions: Vec<UserAction>,
    #[serde(default = "default_prefixes")]
    pub prefixes: Vec<Prefix>,
    #[serde(default)]
    site_searches_initialized: bool,
    #[serde(default)]
    search_word_shortcut_initialized: bool,
    #[serde(default)]
    search_completion_shortcut_initialized: bool,
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

fn is_horizontal(vertical: &i8) -> bool {
    *vertical == 0
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Page {
    #[serde(default, skip_serializing_if = "is_horizontal")]
    pub vertical: i8,
    #[serde(default = "new_page_id")]
    pub id: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Item {
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
            Self::Folder { .. } | Self::Widget { .. } => return None,
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
            | Self::Widget { name, .. } => name,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::App(app) => &app.id,
            Self::Website { id, .. }
            | Self::File { id, .. }
            | Self::Directory { id, .. }
            | Self::Folder { id, .. }
            | Self::Widget { id, .. } => id,
        }
    }
}

fn default_columns() -> usize {
    1920
}
fn default_rows() -> usize {
    1080
}

fn default_page_margin() -> i32 {
    10
}
fn default_item_size() -> i32 {
    128
}
fn default_grid_gap() -> i32 {
    16
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
            canvas: Default::default(),
            version: 5,
            positions: Default::default(),
            columns: default_columns(),
            rows: default_rows(),
            active_page: 0,
            pages: vec![Page {
                vertical: 0,
                id: crate::config::new_page_id(),
                items: Vec::new(),
            }],
            pinned: Vec::new(),
            page_margin: default_page_margin(),
            item_size: default_item_size(),
            grid_gap: default_grid_gap(),
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
            user_actions: default_actions(),
            prefixes: default_prefixes(),
            site_searches_initialized: true,
            search_word_shortcut_initialized: true,
            search_completion_shortcut_initialized: true,
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

    pub fn horizontal_pages(&self) -> usize {
        self.pages
            .iter()
            .take_while(|page| page.vertical == 0)
            .count()
    }

    pub fn insert_page(&mut self) -> usize {
        let index = self.horizontal_pages();
        self.pages.insert(
            index,
            Page {
                vertical: 0,
                id: new_page_id(),
                items: Vec::new(),
            },
        );
        if self.active_page >= index {
            self.active_page += 1;
        }
        index
    }

    pub fn ensure_vertical_pages(&mut self) {
        if self.pages.iter().all(|page| page.vertical == 0) {
            for vertical in [-1, 1] {
                self.pages.push(Page {
                    vertical,
                    id: new_page_id(),
                    items: Vec::new(),
                });
            }
        }
    }

    pub fn remove_page(&mut self, index: usize) -> Option<usize> {
        if self.horizontal_pages() <= 1 || index >= self.horizontal_pages() {
            return None;
        }
        let removed = self.pages.remove(index);
        let destination = index.saturating_sub(1);
        self.pages[destination].items.extend(removed.items);
        Some(destination)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut saved = self.clone();
        saved.version = 4;
        crate::spatial::reconcile(&mut saved).map_err(std::io::Error::other)?;
        platform::write_config(&serde_json::to_vec_pretty(&saved)?)?;
        Ok(())
    }

    pub fn valid(&self) -> bool {
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
        for item in self.pages.iter().flat_map(|page| &page.items) {
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
        let limit = if self.version >= 3 { 32768 } else { 128 };
        if self.positions.values().any(|rect| {
            rect.row > limit
                || rect.col > limit
                || !(1..=limit).contains(&rect.width)
                || !(1..=limit).contains(&rect.height)
        }) {
            return false;
        }
        self.canvas.valid()
            && (1..=5).contains(&self.version)
            && (1..=limit).contains(&self.columns)
            && (1..=limit).contains(&self.rows)
            && (1..=9).contains(&self.horizontal_pages())
            && (self.pages.len() == self.horizontal_pages()
                || (self.pages.len() == self.horizontal_pages() + 2
                    && self.pages[self.horizontal_pages()].vertical == -1
                    && self.pages[self.horizontal_pages() + 1].vertical == 1))
            && crate::shortcuts::valid(&self.shortcuts)
            && (0..=64).contains(&self.page_margin)
            && (80..=240).contains(&self.item_size)
            && (0..=64).contains(&self.grid_gap)
            && SEARCH_ENGINES
                .iter()
                .any(|(id, _)| *id == self.default_search_engine)
    }
}

pub fn load() -> (Config, String) {
    match read() {
        Ok(result) => result,
        Err(error) => (
            Config::default(),
            format!("Couldn't read or save config.json: {error}"),
        ),
    }
}

fn merge_remembrances(config: &mut Config) -> Result<bool, Box<dyn std::error::Error>> {
    let mut pairs = Vec::new();
    for page in &config.pages {
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
    if !pairs.is_empty() {
        let path = crate::storage::config_path()?.with_file_name("widgets.db");
        let db = crate::widget_data::open(&path).map_err(std::io::Error::other)?;
        for (keep, removed) in &pairs {
            crate::adhkar::merge_progress(&db, keep, removed).map_err(std::io::Error::other)?;
        }
    }
    let mut changed = !pairs.is_empty();
    for page in &mut config.pages {
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

fn read() -> Result<(Config, String), Box<dyn std::error::Error>> {
    let mut banner = String::new();
    if let Some(text) = platform::read_config()? {
        if let Ok(mut config) = serde_json::from_slice::<Config>(&text)
            && config.valid()
        {
            let mut changed = config.initialize_ai();
            changed |= merge_remembrances(&mut config)?;
            changed |= config.english_assistance.migrate_prompts();
            if config.version < 3 {
                let pitch = crate::spatial::cell_pitch(config.item_size, config.grid_gap);
                for rect in config.positions.values_mut() {
                    rect.row *= pitch;
                    rect.col *= pitch;
                    rect.width *= pitch;
                    rect.height *= pitch;
                }
                config.columns *= pitch;
                config.rows *= pitch;
                config.version = 3;
                changed = true;
            }
            if config.version < 4 {
                let pitch = crate::spatial::cell_pitch(config.item_size, config.grid_gap);
                let widgets: Vec<_> = config
                    .pages
                    .iter()
                    .flat_map(|page| &page.items)
                    .filter_map(|item| match item {
                        Item::Widget { id, .. } => Some(id.clone()),
                        _ => None,
                    })
                    .collect();
                for id in widgets {
                    if let Some(rect) = config.positions.get_mut(&id) {
                        let width = (rect.width / pitch).max(1);
                        let height = (rect.height / pitch).max(1);
                        rect.width =
                            crate::spatial::widget_span(width, config.item_size, config.grid_gap);
                        rect.height =
                            crate::spatial::widget_span(height, config.item_size, config.grid_gap);
                    }
                }
                config.version = 4;
                changed = true;
            }
            if config.version < 5 {
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
            if !config.site_searches_initialized {
                config.add_default_search_prefixes();
                config.site_searches_initialized = true;
                changed = true;
            }
            // Retired widget kinds are removed; shortcut and TickTick IDs remain intact.
            let mut removed = Vec::new();
            for page in &mut config.pages {
                page.items.retain(|item| {
                    let keep =
                        !matches!(item, Item::Widget { variant, .. } if !(7..crate::spatial::WIDGETS.len()).contains(variant));
                    if !keep {
                        removed.push(item.id().to_owned());
                    }
                    keep
                });
            }
            config.positions.retain(|id, _| !removed.contains(id));
            config.pinned.retain(|id| !removed.contains(id));
            crate::spatial::reconcile(&mut config).map_err(std::io::Error::other)?;
            if !removed.is_empty() {
                changed = true;
            }
            if changed {
                config.save()?;
            }
            return Ok((config, banner));
        }
        platform::backup_config()?;
        banner = "Couldn't read config.json. Saved as config.json.bad; Orbit starts empty.".into();
    }
    let config = Config::default();
    platform::write_config(&serde_json::to_vec_pretty(&config)?)?;
    Ok((config, banner))
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
    fn first_run_has_one_empty_page() {
        let config = Config::default();
        assert!(config.valid());
        assert_eq!(config.pages.len(), 1);
        assert!(config.pages[0].items.is_empty());
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
    fn saved_page_count_is_between_one_and_nine() {
        for count in 0..=10 {
            let config = Config {
                version: 3,
                pages: (0..count)
                    .map(|_| Page {
                        vertical: 0,
                        id: crate::config::new_page_id(),
                        items: Vec::new(),
                    })
                    .collect(),
                ..Config::default()
            };
            assert_eq!(config.valid(), (1..=9).contains(&count));
        }
    }
    #[test]
    fn vertical_pages_are_fixed_and_do_not_use_the_horizontal_limit() {
        let mut config = Config::default();
        config.pages = (0..9)
            .map(|_| serde_json::from_value(serde_json::json!({"items": []})).unwrap())
            .collect();
        for vertical in [-1, 1] {
            config.pages.push(
                serde_json::from_value(serde_json::json!({
                    "vertical": vertical, "items": []
                }))
                .unwrap(),
            );
        }
        assert!(config.valid());
        assert_eq!(config.remove_page(9), None);
        assert_eq!(config.remove_page(10), None);
        let saved = serde_json::to_value(&config).unwrap();
        assert_eq!(saved["pages"][9]["vertical"], -1);
        config.pages.push(
            serde_json::from_value(serde_json::json!({
                "vertical": -1, "items": []
            }))
            .unwrap(),
        );
        assert!(!config.valid());
    }

    #[test]
    fn vertical_page_items_survive_horizontal_overflow_and_drops() {
        let mut config = Config::default();
        config.ensure_vertical_pages();
        config.ensure_vertical_pages();
        assert_eq!(config.pages.len(), 3);
        config.columns = 144;
        config.rows = 144;
        let item = |id: &str| Item::Folder {
            id: id.into(),
            name: id.into(),
            apps: Vec::new(),
        };
        config.pages[0].items = vec![item("a"), item("b")];
        config.pages[1].items = vec![item("top")];
        config.active_page = 1;
        crate::spatial::reconcile(&mut config).unwrap();
        assert_eq!(config.horizontal_pages(), 2);
        assert_eq!(config.pages[2].vertical, -1);
        assert_eq!(config.pages[2].items[0].id(), "top");
        assert_eq!(config.active_page, 2);
        assert!(config.valid());
        let new_page = config.pages.len();
        assert!(crate::item_drop::apply(
            &mut config,
            "top",
            new_page,
            0,
            "",
            "unused".into()
        ));
        assert_eq!(config.horizontal_pages(), 3);
        assert_eq!(config.pages[2].items[0].id(), "top");
        assert_eq!(config.pages[3].vertical, -1);
        assert!(config.valid());
        assert!(crate::item_drop::apply(
            &mut config,
            "top",
            4,
            0,
            "",
            "unused".into()
        ));
        assert_eq!(config.pages[4].items[0].id(), "top");
        assert!(config.valid());
    }

    #[test]
    fn deleting_pages_keeps_items_and_the_last_page() {
        for index in 0..3 {
            let mut config = Config::default();
            config.pages = (0..3)
                .map(|number| Page {
                    vertical: 0,
                    id: crate::config::new_page_id(),
                    items: vec![Item::Folder {
                        id: number.to_string(),
                        name: number.to_string(),
                        apps: Vec::new(),
                    }],
                })
                .collect();
            assert_eq!(config.remove_page(index), Some(index.saturating_sub(1)));
            assert_eq!(config.pages.len(), 2);
            let mut ids: Vec<_> = config
                .pages
                .iter()
                .flat_map(|page| page.items.iter().map(Item::id))
                .collect();
            ids.sort();
            assert_eq!(ids, ["0", "1", "2"]);
            config.remove_page(0);
            assert_eq!(config.remove_page(0), None);
            assert_eq!(config.pages[0].items.len(), 3);
        }
    }
}

pub fn new_page_id() -> String {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "page-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
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
        assert_eq!(loaded.pages.len(), config.pages.len());
    }
}
