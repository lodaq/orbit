use crate::platform;
use serde::{Deserialize, Serialize};

const HISTORY_LIMIT: usize = 16;

#[derive(Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Recent {
    pub ids: Vec<String>,
}

impl Recent {
    pub fn load() -> (Self, String) {
        match platform::read_recent() {
            Ok(Some(data)) => match serde_json::from_slice::<Self>(&data) {
                Ok(recent) => (recent, String::new()),
                Err(_) => match platform::backup_recent() {
                    Ok(()) => (
                        Self::default(),
                        "Couldn't read recent.json. Saved as recent.json.bad; Recent starts empty."
                            .into(),
                    ),
                    Err(error) => (
                        Self::default(),
                        format!("Couldn't save recent.json.bad: {error}"),
                    ),
                },
            },
            Ok(None) => (Self::default(), String::new()),
            Err(error) => (
                Self::default(),
                format!("Couldn't read recent.json: {error}"),
            ),
        }
    }

    pub fn record(&mut self, id: &str) -> bool {
        if self.ids.first().is_some_and(|first| first == id) {
            return false;
        }
        self.ids.retain(|saved| saved != id);
        self.ids.insert(0, id.into());
        self.ids.truncate(HISTORY_LIMIT);
        true
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let length = self.ids.len();
        self.ids.retain(|saved| saved != id);
        self.ids.len() != length
    }

    pub fn save(&self) -> Result<(), String> {
        platform::write_recent(
            &serde_json::to_vec_pretty(self)
                .map_err(|error| format!("Couldn't save recent.json: {error}"))?,
        )
        .map_err(|error| format!("Couldn't save recent.json: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_is_unique_newest_first_and_bounded() {
        let mut recent = Recent::default();
        for index in 0..20 {
            assert!(recent.record(&index.to_string()));
        }
        assert_eq!(recent.ids.len(), HISTORY_LIMIT);
        assert_eq!(recent.ids[0], "19");
        assert_eq!(recent.ids[15], "4");
        assert!(!recent.record("19"));
        assert!(recent.record("10"));
        assert_eq!(recent.ids[0], "10");
        assert_eq!(recent.ids.len(), HISTORY_LIMIT);
        assert!(!recent.remove("missing"));
        assert!(recent.remove("10"));
        assert!(!recent.ids.contains(&"10".to_string()));
    }
}
