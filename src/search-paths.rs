use crate::PrefixMatch;
use crate::config::{Config, Prefix};
use std::path::PathBuf;

pub(crate) struct SearchPathResult {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) directory: bool,
}

pub(crate) fn url_encode(value: &str) -> String {
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

pub(crate) fn url_action_url(template: &str, query: &str) -> Result<String, String> {
    if !template.contains("%s") {
        return Err("URL Action template must contain %s".into());
    }
    Ok(template.replace("%s", &url_encode(query)))
}

pub(crate) fn browser_search_url(template: &str, query: &str) -> Result<String, String> {
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

pub(crate) fn match_prefix(prefixes: &[Prefix], text: &str) -> Option<PrefixMatch> {
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

pub(crate) fn edit_exclusions(entries: &mut Vec<String>, name: &str, remove: bool) -> bool {
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
pub(crate) fn exclusion_matches(pattern: &str, name: &str) -> bool {
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

pub(crate) fn index_search_paths(config: &Config) -> Vec<SearchPathResult> {
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

#[cfg(test)]
mod tests {
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
