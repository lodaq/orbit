use super::*;

fn role() -> Role {
    Role {
        id: "researcher".into(),
        name: "باحث".into(),
        instructions: "Cite every source. اكتب بالعربية".into(),
    }
}

fn profile() -> Profile {
    Profile {
        id: "robot-1".into(),
        name: "Research Robot".into(),
        project: "/tmp/project".into(),
        model: Model {
            provider: Provider::Ollama,
            id: "qwen3:4b".into(),
            ollama_address: Some("http://127.0.0.1:11434".into()),
        },
        role_id: Some(role().id),
        skills: vec!["/tmp/skills/research/SKILL.md".into()],
        mcp_servers: vec![McpServer {
            id: "research".into(),
            command: "research-mcp".into(),
            arguments: vec!["--stdio".into()],
            secret_env: BTreeMap::from([("API_KEY".into(), "research-key".into())]),
            enabled_tools: vec!["search".into()],
        }],
    }
}

fn database() -> Database {
    let db = Database::open(Path::new(":memory:")).unwrap();
    db.save_role(&role()).unwrap();
    db.save_profile(&profile()).unwrap();
    db
}

#[test]
fn resumed_conversation_keeps_starting_context_after_profile_and_role_edits() {
    let mut db = database();
    db.start(
        "first",
        "robot-1",
        "  ابحث عن Rust\nInclude linked sources.",
        10,
    )
    .unwrap();
    let original = db.conversation("first").unwrap().unwrap();
    assert_eq!(original.title, "ابحث عن Rust");
    let mut changed = profile();
    changed.model = Model {
        provider: Provider::Gemini,
        id: "gemini-test".into(),
        ollama_address: None,
    };
    changed.project = "C:\\projects\\different".into();
    db.save_profile(&changed).unwrap();
    let mut changed_role = role();
    changed_role.instructions = "Changed instructions".into();
    db.save_role(&changed_role).unwrap();
    db.append(
        "first",
        &Entry::Reply {
            text: "نتيجة البحث [Source](https://example.com)".into(),
        },
        20,
    )
    .unwrap();
    db.rate("first", 5).unwrap();
    let resumed = db.conversation("first").unwrap().unwrap();
    assert_eq!(resumed.context, original.context);
    assert_eq!(resumed.rating, Some(5));
    assert_eq!(resumed.updated_at, 20);
    assert_eq!(resumed.entries.len(), 2);
    db.start("second", "robot-1", "New conversation", 30)
        .unwrap();
    let new = db.conversation("second").unwrap().unwrap();
    assert_eq!(new.context.model, changed.model);
    assert_eq!(new.context.role, Some(changed_role));
}

#[test]
fn activity_orders_history_and_deletion_removes_all_entries() {
    let mut db = database();
    db.start("first", "robot-1", "First", 10).unwrap();
    db.start("second", "robot-1", "Second", 20).unwrap();
    db.append(
        "first",
        &Entry::Tool {
            name: "read_file".into(),
            input: serde_json::json!({"path":"/tmp/project/a.md"}),
            output: "Denied by user".into(),
            status: ToolStatus::Denied,
        },
        30,
    )
    .unwrap();
    db.append(
        "first",
        &Entry::Progress {
            text: "Waiting".into(),
        },
        5,
    )
    .unwrap();
    assert_eq!(
        db.history("robot-1", 0)
            .unwrap()
            .iter()
            .map(|h| h.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    assert_eq!(db.history("robot-1", 0).unwrap()[0].updated_at, 30);
    db.delete_conversation("first").unwrap();
    assert!(db.conversation("first").unwrap().is_none());
    assert!(
        db.append(
            "first",
            &Entry::Reply {
                text: "Late result".into()
            },
            40
        )
        .is_err()
    );
    assert!(db.rate("first", 3).is_err());
    let count: i64 = db
        .connection
        .query_row(
            "SELECT count(*) FROM entries WHERE conversation_id='first'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(db.history("robot-1", 0).unwrap().len(), 1);
}

#[test]
fn invalid_mutations_and_imports_leave_saved_data_unchanged() {
    let mut db = database();
    db.start("first", "robot-1", "First", 10).unwrap();
    let before = db.snapshot().unwrap();
    assert!(db.start("first", "robot-1", "Overwrite", 20).is_err());
    assert!(db.start("empty", "robot-1", "  \n", 20).is_err());
    assert!(db.start("missing", "missing", "Command", 20).is_err());
    assert!(db.rate("first", 0).is_err());
    assert!(db.rate("first", 6).is_err());
    let mut invalid = before.clone();
    invalid.conversations[0].rating = Some(6);
    assert!(db.restore(&invalid).is_err());
    invalid = before.clone();
    invalid.profiles[0].role_id = Some("missing".into());
    assert!(db.restore(&invalid).is_err());
    assert!(db.save_profile(&invalid.profiles[0]).is_err());
    invalid = before.clone();
    invalid.conversations.push(invalid.conversations[0].clone());
    assert!(db.restore(&invalid).is_err());
    assert_eq!(db.snapshot().unwrap(), before);
}

#[test]
fn import_retains_history_and_keyring_references_but_not_tool_authority() {
    let mut db = database();
    db.start("first", "robot-1", "Report", 10).unwrap();
    db.rate("first", 4).unwrap();
    let snapshot = db.snapshot().unwrap();
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(json.contains("research-key"));
    let mut imported = Database::open(Path::new(":memory:")).unwrap();
    imported
        .restore(&serde_json::from_str(&json).unwrap())
        .unwrap();
    let mut expected = snapshot;
    expected.disable_tools();
    assert_eq!(imported.snapshot().unwrap(), expected);
    let mut with_secret = serde_json::to_value(profile()).unwrap();
    with_secret["model"]["api_key"] = "secret-must-not-be-accepted".into();
    assert!(serde_json::from_value::<Profile>(with_secret).is_err());
    let mut with_secret = serde_json::to_value(profile()).unwrap();
    with_secret["mcp_servers"][0]["env"] =
        serde_json::json!({"API_KEY":"secret-must-not-be-accepted"});
    assert!(serde_json::from_value::<Profile>(with_secret).is_err());
}

#[test]
fn reopen_preserves_unicode_profiles_and_conversations() {
    let path = std::env::temp_dir().join(format!(
        "orbit-robots-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let expected = {
        let mut db = Database::open(&path).unwrap();
        db.save_role(&role()).unwrap();
        db.save_profile(&profile()).unwrap();
        db.start("first", "robot-1", "👩🏽‍💻 مرحباً Rust", 10).unwrap();
        db.rate("first", 5).unwrap();
        db.snapshot().unwrap()
    };
    assert_eq!(export(&path).unwrap(), expected);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(export(&path).unwrap(), Snapshot::default());
    assert!(!path.exists());
    assert_eq!(title(&"👩🏽‍💻".repeat(90)), "👩🏽‍💻".repeat(80));
}

#[test]
fn rejects_invalid_models_paths_and_mcp_configuration() {
    for address in [
        "https://example.com",
        "http://127.0.0.1:0",
        "http://localhost:11434/private",
    ] {
        let mut profile = profile();
        profile.model.ollama_address = Some(address.into());
        assert!(profile.validate().is_err());
    }
    let mut profile = profile();
    profile.project = "relative/path".into();
    assert!(profile.validate().is_err());
    profile.project = "/tmp/project".into();
    profile.mcp_servers[0]
        .secret_env
        .insert("BAD=NAME".into(), "key".into());
    assert!(profile.validate().is_err());
    assert!(absolute_path("C:\\projects\\robot").is_ok());
    assert!(absolute_path("C:relative").is_err());
}

#[test]
fn sql_failure_rolls_back_entire_restore() {
    let mut db = database();
    db.start("first", "robot-1", "First", 10).unwrap();
    let before = db.snapshot().unwrap();
    db.connection.execute_batch("CREATE TRIGGER fail_restore BEFORE INSERT ON entries BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(db.restore(&before).is_err());
    assert_eq!(db.snapshot().unwrap(), before);
}

#[test]
fn export_preserves_unrecognized_and_future_database_files() {
    let path = std::env::temp_dir().join(format!(
        "orbit-robots-invalid-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for data in [b"".as_slice(), b"damaged database"] {
        std::fs::write(&path, data).unwrap();
        assert!(export(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), data);
    }
    std::fs::remove_file(&path).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version=3")
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(export(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn queued_commands_survive_backup_but_restart_never_replays_them() {
    let mut db = database();
    db.enqueue("conversation", "robot-1", "first", "First command", 10)
        .unwrap();
    db.append(
        "conversation",
        &Entry::Run {
            id: "first".into(),
            status: RunStatus::Running,
        },
        11,
    )
    .unwrap();
    db.enqueue("conversation", "robot-1", "second", "Second command", 12)
        .unwrap();
    let before = db.snapshot().unwrap();
    assert!(
        db.enqueue("conversation", "robot-1", "second", "Duplicate", 12)
            .is_err()
    );
    let mut restored = Database::open(Path::new(":memory:")).unwrap();
    restored.restore(&before).unwrap();
    restored.interrupt_runs(13).unwrap();
    let interrupted = restored.snapshot().unwrap();
    assert_eq!(
        interrupted.conversations[0]
            .entries
            .iter()
            .filter(|e| matches!(
                e,
                Entry::Run {
                    status: RunStatus::Interrupted,
                    ..
                }
            ))
            .count(),
        2
    );
    restored.interrupt_runs(14).unwrap();
    assert_eq!(restored.snapshot().unwrap(), interrupted);
    let mut invalid = interrupted;
    invalid.conversations[0].entries.push(Entry::Run {
        id: "first".into(),
        status: RunStatus::Running,
    });
    assert!(invalid.validate().is_err());
}
