use crate::platform;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    version: u32,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub pinned: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Page {
    pub items: Vec<Item>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Item {
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
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum SystemAction {
    LockScreen,
    Screenshot,
    Sleep,
    LogOut,
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
            | Self::Folder { name, .. } => name,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::App(app) => &app.id,
            Self::Website { id, .. }
            | Self::File { id, .. }
            | Self::Directory { id, .. }
            | Self::Folder { id, .. } => id,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            pages: vec![Page { items: Vec::new() }],
            pinned: Vec::new(),
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        platform::write_config(&serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    fn valid(&self) -> bool {
        self.version == 1 && (1..=9).contains(&self.pages.len())
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
        if let Ok(config) = serde_json::from_slice::<Config>(&text)
            && config.valid()
        {
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
                pages: (0..count).map(|_| Page { items: Vec::new() }).collect(),
                pinned: Vec::new(),
            };
            assert_eq!(config.valid(), (1..=9).contains(&count));
        }
    }
}
