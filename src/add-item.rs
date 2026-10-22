use crate::{config::InstalledApp, platform};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Inventory {
    apps: Vec<InstalledApp>,
    loaded: bool,
    status: String,
}

impl Inventory {
    pub fn load() -> (Self, String) {
        match platform::read_installed_apps() {
            Ok(Some(data)) => match serde_json::from_slice::<Vec<InstalledApp>>(&data) {
                Ok(apps) => {
                    let status = format!("{} apps · Cached inventory", apps.len());
                    (
                        Self {
                            apps,
                            loaded: true,
                            status,
                        },
                        String::new(),
                    )
                }
                Err(_) => match platform::backup_installed_apps() {
                    Ok(()) => (
                        Self::default(),
                        "Couldn't read installed-apps.json. Saved as installed-apps.json.bad; Installed apps starts empty.".into(),
                    ),
                    Err(error) => (
                        Self::default(),
                        format!("Couldn't save installed-apps.json.bad: {error}"),
                    ),
                },
            },
            Ok(None) => (Self::default(), String::new()),
            Err(error) => (
                Self::default(),
                format!("Couldn't read installed-apps.json: {error}"),
            ),
        }
    }

    pub fn apps(&self) -> &[InstalledApp] {
        &self.apps
    }

    pub fn loaded(&self) -> bool {
        self.loaded
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn refreshing(&mut self) {
        self.status = "Refreshing installed apps…".into();
    }

    pub fn replace(&mut self, apps: Vec<InstalledApp>) {
        self.status = format!("{} apps · Cached inventory", apps.len());
        self.apps = apps;
        self.loaded = true;
    }

    pub fn fail(&mut self, error: String) {
        self.status = error;
    }
}

#[derive(Default)]
pub struct RequestSequence(AtomicU64);

impl RequestSequence {
    pub fn issue(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn cancel(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn is_current(&self, request: u64) -> bool {
        self.0.load(Ordering::Relaxed) == request
    }
}

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
}

pub fn valid_command(command: &str) -> bool {
    shlex::split(command).is_some_and(|parts| !parts.is_empty() && !parts[0].is_empty())
}

pub fn valid_url(url: &str) -> bool {
    let Some((scheme, rest)) = url.trim().split_once("://") else {
        return false;
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
        || url.chars().any(char::is_whitespace)
    {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let host = if let Some(address) = authority.strip_prefix('[') {
        let Some((address, suffix)) = address.split_once(']') else {
            return false;
        };
        if !suffix.is_empty() && !valid_port(suffix) {
            return false;
        }
        return address.parse::<std::net::Ipv6Addr>().is_ok();
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if !valid_port(port) {
            return false;
        }
        host
    } else {
        authority
    };
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label
                    .chars()
                    .all(|character| character.is_alphanumeric() || character == '-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

fn valid_port(suffix: &str) -> bool {
    suffix
        .strip_prefix(':')
        .unwrap_or(suffix)
        .parse::<u16>()
        .ok()
        .is_some_and(|port| port > 0)
}

pub fn valid_svg(svg: &str) -> bool {
    let trimmed = svg.trim();
    let lower = trimmed.to_ascii_lowercase();
    let root = lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg"));
    root && trimmed.len() <= 65_536
        && lower.contains("</svg>")
        && ![
            "<script",
            "<foreignobject",
            "javascript:",
            "<!doctype",
            "<!entity",
            " onload=",
            " onclick=",
            " onerror=",
            "href=\"http",
            "href='http",
            "url(http",
            "url(//",
        ]
        .iter()
        .any(|blocked| lower.contains(blocked))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_sequence_invalidates_stale_work() {
        let requests = RequestSequence::default();
        let request = requests.issue();
        assert!(requests.is_current(request));
        requests.cancel();
        assert!(!requests.is_current(request));
    }

    #[test]
    fn validates_names_commands_and_urls() {
        assert!(valid_name(" Terminal "));
        assert!(!valid_name("  "));
        assert!(valid_command("foot --title 'my terminal'"));
        assert!(!valid_command("foot 'unfinished"));
        assert!(valid_url("https://example.com/path"));
        assert!(valid_url("http://[::1]:8080"));
        assert!(valid_url("http://127.0.0.1:8080/"));
        assert!(!valid_url("http://example.com:0"));
        assert!(!valid_url("http://example.com:65536"));
        assert!(!valid_url("file:///tmp/document"));
        assert!(!valid_url("https://"));
        assert!(!valid_url("https://example.com/path with spaces"));
        assert!(!valid_url("https://user@example.com"));
    }

    #[test]
    fn rejects_active_or_external_svg_content() {
        assert!(valid_svg("<svg xmlns='http://www.w3.org/2000/svg'></svg>"));
        assert!(!valid_svg("<svg><script>alert(1)</script></svg>"));
        assert!(!valid_svg(
            "<svg><image href='https://example.com/x.png'/></svg>"
        ));
        assert!(!valid_svg("<svg"));
    }
}
