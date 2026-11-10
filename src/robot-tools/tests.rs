use super::*;

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "orbit-tools-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn absent_denied_stale_and_cross_conversation_approvals_do_not_write() {
    let project = Project::new();
    let action = LocalAction::Write {
        path: "report.md".into(),
        text: "# بحث\n[Source](https://example.org)".into(),
    };
    let mut local = LocalTools::default();
    assert!(local.resolve(1, true).is_err());
    let first = local.prepare(&project.0, action.clone()).unwrap().id;
    assert!(!project.0.join("report.md").exists());
    assert!(local.resolve(first, false).is_err());
    assert!(!project.0.join("report.md").exists());
    let second = local.prepare(&project.0, action.clone()).unwrap().id;
    assert!(local.resolve(first, true).is_err());
    assert_eq!(local.pending().unwrap().id, second);
    let mut other = LocalTools::default();
    other.prepare(&project.0, action).unwrap();
    assert!(other.resolve(second, true).is_err());
    local.resolve(second, true).unwrap();
    assert!(
        fs::read_to_string(project.0.join("report.md"))
            .unwrap()
            .contains("https://example.org")
    );
    assert!(local.resolve(second, true).is_err());
    other.cancel();
    assert!(other.pending().is_none());
}

#[test]
fn approved_reads_skills_and_writes_are_bounded_and_preserve_content_on_failure() {
    let project = Project::new();
    let mut local = LocalTools::default();
    fs::write(project.0.join("SKILL.md"), "# بحث\nUse a local report.").unwrap();
    let id = local
        .prepare(
            &project.0,
            LocalAction::Skill {
                path: "SKILL.md".into(),
            },
        )
        .unwrap()
        .id;
    assert!(
        local
            .resolve(id, true)
            .unwrap()
            .contains("do not grant permissions")
    );
    fs::write(project.0.join("large"), vec![b'x'; LIMIT + 1]).unwrap();
    let id = local
        .prepare(
            &project.0,
            LocalAction::Read {
                path: "large".into(),
            },
        )
        .unwrap()
        .id;
    assert!(local.resolve(id, true).unwrap_err().contains("1 MiB"));
    assert!(
        local
            .prepare(
                &project.0,
                LocalAction::Write {
                    path: "SKILL.md".into(),
                    text: "x".repeat(LIMIT + 1)
                }
            )
            .is_err()
    );
    assert!(
        fs::read_to_string(project.0.join("SKILL.md"))
            .unwrap()
            .starts_with("# بحث")
    );
    let id = local
        .prepare(&project.0, LocalAction::List { path: ".".into() })
        .unwrap()
        .id;
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&local.resolve(id, true).unwrap()).unwrap(),
        ["SKILL.md", "large"]
    );
    assert!(atomic_write(&project.0.join("missing/report.md"), "report").is_err());
}

#[cfg(unix)]
#[test]
fn shell_requires_approval_preserves_arguments_and_reports_exit_and_timeout() {
    let project = Project::new();
    let mut local = LocalTools::default();
    let action = LocalAction::Run {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".into(),
            "printf '%s' \"$1\" > result; printf problem >&2; exit 7".into(),
            "orbit".into(),
            "$(touch unexpected); مرحبا".into(),
        ],
    };
    let id = local.prepare(&project.0, action.clone()).unwrap().id;
    assert!(local.resolve(id, false).is_err());
    assert!(!project.0.join("result").exists());
    let id = local.prepare(&project.0, action).unwrap().id;
    let output: serde_json::Value =
        serde_json::from_str(&local.resolve(id, true).unwrap()).unwrap();
    assert_eq!(output["exit_code"], 7);
    assert_eq!(output["stderr"], "problem");
    assert!(!project.0.join("unexpected").exists());
    assert_eq!(
        fs::read_to_string(project.0.join("result")).unwrap(),
        "$(touch unexpected); مرحبا"
    );
    let start = Instant::now();
    assert!(
        run(
            &project.0,
            "/bin/sh",
            &["-c".into(), "sleep 30".into()],
            Duration::from_millis(50)
        )
        .unwrap_err()
        .contains("timed out")
    );
    assert!(start.elapsed() < Duration::from_secs(3));
}
