use crate::platform;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
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
    #[serde(default = "crate::shortcuts::defaults")]
    pub shortcuts: Vec<crate::shortcuts::Shortcut>,
    #[serde(default)]
    pub file_search_paths: Vec<String>,
    #[serde(default)]
    pub directory_search_paths: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Page {
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
    9
}
fn default_rows() -> usize {
    5
}

fn default_page_margin() -> i32 {
    10
}
fn default_item_size() -> i32 {
    128
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            positions: Default::default(),
            columns: default_columns(),
            rows: default_rows(),
            active_page: 0,
            pages: vec![Page {
                id: crate::config::new_page_id(),
                items: Vec::new(),
            }],
            pinned: Vec::new(),
            page_margin: default_page_margin(),
            item_size: default_item_size(),
            shortcuts: crate::shortcuts::defaults(),
            file_search_paths: Vec::new(),
            directory_search_paths: Vec::new(),
        }
    }
}

impl Config {
    pub fn remove_page(&mut self, index: usize) -> Option<usize> {
        if self.pages.len() <= 1 || index >= self.pages.len() {
            return None;
        }
        let removed = self.pages.remove(index);
        let destination = index.saturating_sub(1);
        self.pages[destination].items.extend(removed.items);
        Some(destination)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut saved = self.clone();
        saved.version = 2;
        crate::spatial::reconcile(&mut saved).map_err(std::io::Error::other)?;
        platform::write_config(&serde_json::to_vec_pretty(&saved)?)?;
        Ok(())
    }

    pub fn valid(&self) -> bool {
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
            if let Item::Widget { variant, text, .. } = item
                && (*variant >= 5 || text.is_empty())
            {
                return false;
            }
        }
        if self.positions.values().any(|rect| {
            rect.row > 128
                || rect.col > 128
                || !(1..=4).contains(&rect.width)
                || !(1..=4).contains(&rect.height)
        }) {
            return false;
        }
        (1..=2).contains(&self.version)
            && (1..=128).contains(&self.columns)
            && (1..=128).contains(&self.rows)
            && (1..=9).contains(&self.pages.len())
            && crate::shortcuts::valid(&self.shortcuts)
            && (0..=64).contains(&self.page_margin)
            && (80..=240).contains(&self.item_size)
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

fn read() -> Result<(Config, String), Box<dyn std::error::Error>> {
    let mut banner = String::new();
    if let Some(text) = platform::read_config()? {
        if let Ok(mut config) = serde_json::from_slice::<Config>(&text)
            && config.valid()
        {
            crate::spatial::reconcile(&mut config).map_err(std::io::Error::other)?;
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
    fn first_run_has_one_empty_page() {
        let config = Config::default();
        assert!(config.valid());
        assert_eq!(config.pages.len(), 1);
        assert!(config.pages[0].items.is_empty());
    }

    #[test]
    fn saved_page_count_is_between_one_and_nine() {
        for count in 0..=10 {
            let config = Config {
                version: 1,
                pages: (0..count)
                    .map(|_| Page {
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
    fn deleting_pages_keeps_items_and_the_last_page() {
        for index in 0..3 {
            let mut config = Config::default();
            config.pages = (0..3)
                .map(|number| Page {
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
