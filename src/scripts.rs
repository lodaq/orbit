use crate::config::Script;

pub fn query(text: &str) -> Option<(&str, Option<&str>)> {
    let text = text.strip_prefix('/')?;
    Some(match text.split_once(' ') {
        Some((filter, argument)) => (filter, Some(argument)),
        None => (text, None),
    })
}

pub fn validate(name: &str, path: &str) -> Result<(), String> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err("Enter a script name without spaces".into());
    }
    if !std::path::Path::new(path).is_file() {
        return Err("Choose an existing script file".into());
    }
    Ok(())
}

pub fn sorted(scripts: &[Script]) -> Vec<crate::ScriptChoice> {
    let mut scripts: Vec<_> = scripts
        .iter()
        .map(|script| crate::ScriptChoice {
            id: script.id.clone().into(),
            name: script.name.clone().into(),
            path: script.path.clone().into(),
        })
        .collect();
    scripts.sort_by_cached_key(|script| script.name.to_lowercase());
    scripts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_one_unchanged_string() {
        assert_eq!(
            query("/Deploy  a '$HOME'; b "),
            Some(("Deploy", Some(" a '$HOME'; b ")))
        );
        assert_eq!(query("/"), Some(("", None)));
        assert_eq!(query("/deploy"), Some(("deploy", None)));
        assert_eq!(query("deploy"), None);
    }

    #[cfg(unix)]
    #[test]
    fn direct_launch_preserves_argument_and_working_directory() {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::env::temp_dir().join(format!("orbit script test {}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("test script");
        std::fs::write(
            &path,
            "#!/bin/sh\nprintf '%s\\n' \"$#\" \"$PWD\" \"$1\" > result\n",
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let argument = "a '$HOME'; touch injected";
        crate::platform::launch_script(&path, Some(argument)).unwrap();
        let output = folder.join("result");
        let expected = format!("1\n{}\n{argument}\n", folder.display());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::fs::read_to_string(&output).ok().as_deref() != Some(expected.as_str()) {
            assert!(
                std::time::Instant::now() < deadline,
                "script output missing or changed"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!folder.join("injected").exists());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(crate::platform::launch_script(&path, None).is_err());
        std::fs::write(&path, "touch implicit-shell\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(crate::platform::launch_script(&path, None).is_err());
        assert!(!folder.join("implicit-shell").exists());
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn config_defaults_and_roundtrip_preserve_scripts() {
        let old: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert!(old.scripts.is_empty());
        let mut config = crate::config::Config::default();
        assert!(config.scripts.is_empty());
        config.scripts.push(Script {
            id: "s".into(),
            name: "نشر".into(),
            path: "/missing/script".into(),
        });
        let restored: crate::config::Config =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert!(restored.scripts == config.scripts);
        assert!(restored.valid()); // Missing files remain editable after restore.
        assert!(validate("two words", "/missing").is_err());
        assert!(validate("deploy", "/missing").is_err());
    }
}
