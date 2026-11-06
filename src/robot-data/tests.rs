use super::*;

#[test]
fn catalog_round_trip_validation_and_removal() {
    let mut db = database();
    let mut entry = CatalogEntry {
        id: "action-1".into(),
        kind: "shell".into(),
        name: "Check project".into(),
        content: "git status --short".into(),
    };
    db.save_catalog(&entry).unwrap();
    let saved = db.snapshot().unwrap();
    assert_eq!(saved.catalog, vec![entry.clone()]);
    let mut restored = database();
    restored.restore(&saved).unwrap();
    assert_eq!(restored.catalog().unwrap(), vec![entry.clone()]);
    entry.kind = "credential".into();
    assert!(db.save_catalog(&entry).is_err());
    assert!(db.delete_role("researcher").is_err());
    db.delete_catalog("action-1").unwrap();
    assert!(db.catalog().unwrap().is_empty());
}

#[test]
fn common_commands_are_ranked_per_robot_and_deleted_with_history() {
    let mut db = database();
    for (index, text) in ["A", "B", "A", "C", "B", "A", "D"].into_iter().enumerate() {
        db.start(&format!("chat-{index}"), "robot-1", text, index as i64 + 1)
            .unwrap();
    }
    assert_eq!(db.common_commands("robot-1").unwrap(), vec!["A", "B", "D"]);
    assert!(db.common_commands("other").unwrap().is_empty());
    db.delete_profile("robot-1").unwrap();
    assert!(db.common_commands("robot-1").unwrap().is_empty());
    assert!(db.snapshot().unwrap().conversations.is_empty());
}

#[test]
fn imported_shared_mcp_definitions_require_tools_to_be_enabled_again() {
    let mut db = database();
    let server = profile().mcp_servers.remove(0);
    db.save_catalog(&CatalogEntry {
        id: "mcp-research".into(),
        kind: "mcp".into(),
        name: server.id.clone(),
        content: serde_json::to_string(&server).unwrap(),
    })
    .unwrap();
    let snapshot = db.snapshot().unwrap();
    db.restore(&snapshot).unwrap();
    let imported: McpServer = serde_json::from_str(&db.catalog().unwrap()[0].content).unwrap();
    assert!(imported.enabled_tools.is_empty());
    assert_eq!(imported.secret_env, server.secret_env);
}

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
            account: None,
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
        account: None,
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
        .execute_batch("PRAGMA user_version=5")
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

fn account(id: &str, provider: Provider) -> Account {
    let local = provider == Provider::Ollama;
    Account {
        id: id.into(),
        name: format!("حساب {id}"),
        provider,
        address: local.then(|| "http://127.0.0.1:11434".into()),
        credential: None,
        home: None,
        needs_reconnection: false,
        binding: None,
    }
}

#[test]
fn account_selection_is_captured_and_new_conversations_use_saved_connection() {
    let mut db = database();
    let personal = account("personal", Provider::Ollama);
    let work = account("work", Provider::Ollama);
    db.save_account(&personal).unwrap();
    db.save_account(&work).unwrap();
    let mut robot = profile();
    robot.model.account = Some(personal.clone());
    db.save_profile(&robot).unwrap();
    db.start("first", &robot.id, "Hello", 10).unwrap();
    robot.model.account = Some(work.clone());
    db.save_profile(&robot).unwrap();
    let mut changed_work = work.clone();
    changed_work.address = Some("http://localhost:11435".into());
    db.save_account(&changed_work).unwrap();
    db.start("second", &robot.id, "مرحبا", 20).unwrap();
    assert_eq!(
        db.conversation("first")
            .unwrap()
            .unwrap()
            .context
            .model
            .account,
        Some(personal)
    );
    let second = db.conversation("second").unwrap().unwrap();
    assert_eq!(second.context.model.account, Some(changed_work.clone()));
    assert_eq!(second.context.model.ollama_address, changed_work.address);
    assert!(db.delete_account("personal").is_err());
    assert!(db.delete_account("work").is_err());
    assert_eq!(db.accounts().unwrap().len(), 2);
}

#[test]
fn accounts_round_trip_without_secrets_and_import_requires_reconnection() {
    let mut db = database();
    for provider in [
        Provider::Ollama,
        Provider::OpenAi,
        Provider::Gemini,
        Provider::OpenCode,
        Provider::Antigravity,
        Provider::Copilot,
        Provider::Codex,
    ] {
        for suffix in ["personal", "work"] {
            let id = format!("account-{}-{suffix}", provider.index());
            db.save_account(&account(&id, provider.clone())).unwrap();
        }
    }
    let mut robot = profile();
    robot.model.account = Some(db.account("account-0-personal").unwrap());
    db.save_profile(&robot).unwrap();
    db.start("first", &robot.id, "Hello", 10).unwrap();
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.accounts.len(), 14);
    let encoded = serde_json::to_vec(&snapshot).unwrap();
    let decoded: Snapshot = serde_json::from_slice(&encoded).unwrap();
    let mut restored = Database::open(Path::new(":memory:")).unwrap();
    restored.restore(&decoded).unwrap();
    assert!(
        restored
            .accounts()
            .unwrap()
            .iter()
            .all(|a| a.needs_reconnection)
    );
    assert!(
        restored
            .profile(&robot.id)
            .unwrap()
            .model
            .account
            .unwrap()
            .needs_reconnection
    );
    assert!(
        restored
            .conversation("first")
            .unwrap()
            .unwrap()
            .context
            .model
            .account
            .unwrap()
            .needs_reconnection
    );
    assert_eq!(
        restored.conversation("first").unwrap().unwrap().entries,
        snapshot.conversations[0].entries
    );
    let mut malformed = serde_json::to_value(&snapshot).unwrap();
    malformed["accounts"][0]["access_token"] = "secret".into();
    assert!(serde_json::from_value::<Snapshot>(malformed).is_err());
}

#[test]
fn account_validation_and_deletion_preserve_bindings() {
    let db = database();
    let mut a = account("personal", Provider::Ollama);
    a.address = Some("https://example.org".into());
    assert!(db.save_account(&a).is_err());
    a = account("personal", Provider::Ollama);
    db.save_account(&a).unwrap();
    let mut changed = account("personal", Provider::Codex);
    assert!(db.save_account(&changed).is_err());
    changed.id = "codex".into();
    changed.home = Some("relative/home".into());
    assert!(db.save_account(&changed).is_err());
    let mut robot = profile();
    robot.model.account = Some(account("missing", Provider::Ollama));
    assert!(db.save_profile(&robot).is_err());
    robot.model.account = Some(account("personal", Provider::OpenAi));
    assert!(db.save_profile(&robot).is_err());
    db.delete_account("personal").unwrap();
    assert!(db.accounts().unwrap().is_empty());
}

#[test]
fn schema_two_and_legacy_json_migrate_without_account_changes() {
    let mut db = database();
    db.start("first", "robot-1", "Hello", 10).unwrap();
    let before = db.snapshot().unwrap();
    let mut legacy = serde_json::to_value(&before).unwrap();
    legacy.as_object_mut().unwrap().remove("accounts");
    legacy["profiles"][0]["model"]
        .as_object_mut()
        .unwrap()
        .remove("account");
    legacy["conversations"][0]["context"]["model"]
        .as_object_mut()
        .unwrap()
        .remove("account");
    let decoded: Snapshot = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded, before);
    db.connection
        .execute_batch("DROP TABLE accounts; DROP TABLE catalog; PRAGMA user_version=2")
        .unwrap();
    let path =
        std::env::temp_dir().join(format!("orbit-account-migration-{}.db", std::process::id()));
    // Use a fresh file to exercise Database::open's actual schema migration.
    let old = Database::open(&path).unwrap();
    old.connection
        .execute_batch("DROP TABLE accounts; DROP TABLE catalog; PRAGMA user_version=2")
        .unwrap();
    drop(old);
    let migrated = Database::open(&path).unwrap();
    assert!(migrated.accounts().unwrap().is_empty());
    assert_eq!(
        migrated
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    drop(migrated);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn reconnecting_an_imported_account_unblocks_its_original_conversation_only() {
    let mut db = database();
    let original = account("personal", Provider::Ollama);
    db.save_account(&original).unwrap();
    let mut robot = profile();
    robot.model.account = Some(original.clone());
    db.save_profile(&robot).unwrap();
    db.start("first", &robot.id, "Hello", 10).unwrap();
    let snapshot = db.snapshot().unwrap();
    db.restore(&snapshot).unwrap();
    let mut model = db.conversation("first").unwrap().unwrap().context.model;
    assert!(db.check_account(&mut model).is_err());
    let mut reconnected = original.clone();
    reconnected.address = Some("http://localhost:11435".into());
    db.save_account(&reconnected).unwrap();
    db.check_account(&mut model).unwrap();
    assert_eq!(model.account, Some(original));
    assert_eq!(
        model.ollama_address.as_deref(),
        Some("http://127.0.0.1:11434")
    );
    db.connection
        .execute("DELETE FROM accounts WHERE id='personal'", [])
        .unwrap();
    assert!(db.check_account(&mut model).is_err());
}
