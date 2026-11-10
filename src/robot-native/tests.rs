use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    model: Model,
}
impl Fixture {
    fn new(driver: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "orbit-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("userdata")).unwrap();
        let script = root.join("agent");
        std::fs::write(
            &script,
            include_str!("../../tests/native-provider-fixture.py"),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let configuration = json!({"driver":driver,"enabled":true,"config":{
            "binaryPath":script,"source":"local","commandPath":script,"commandArgs":[]
        },"environment":[{"name":"ORBIT_NATIVE_FIXTURE","value":root,"sensitive":false}]});
        std::fs::write(
            root.join("userdata/settings.json"),
            json!({"providerInstances":{"fixture":configuration}}).to_string(),
        )
        .unwrap();
        let account = crate::t3_providers::instances(&root)
            .unwrap()
            .remove(0)
            .account;
        let model = Model {
            provider: account.provider.clone(),
            id: "default".into(),
            account: Some(account),
            ollama_address: None,
        };
        Self { root, model }
    }
    fn run(&self, callback: impl FnMut(Event) -> Result<bool>) -> Result<String> {
        run(&self.model, &self.root, "Reply briefly", &[], callback)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn denial_does_not_start_a_native_process() {
    let f = Fixture::new("acpRegistry");
    assert!(f.run(|_| Ok(false)).unwrap_err().contains("denied"));
    assert!(!f.root.join("started").exists());
}

#[test]
fn acp_and_codex_permissions_are_single_use_and_visible() {
    for driver in ["acpRegistry", "codex"] {
        for approve_action in [false, true] {
            let f = Fixture::new(driver);
            let mut permissions = 0;
            let answer = f
                .run(|event| {
                    if let Event::Permission(scope) = event {
                        assert!(scope.is_object());
                        permissions += 1;
                        return Ok(permissions == 1 || approve_action);
                    }
                    Ok(true)
                })
                .unwrap();
            assert_eq!(permissions, 2);
            assert_eq!(f.root.join("effect").exists(), approve_action);
            assert_eq!(
                answer,
                if approve_action {
                    "مرحبا Orbit"
                } else {
                    "Denied safely"
                }
            );
        }
    }
}

#[test]
fn pi_requires_command_scope_permission() {
    let f = Fixture::new("pi");
    let mut permissions = 0;
    assert_eq!(
        f.run(|event| {
            if matches!(event, Event::Permission(_)) {
                permissions += 1;
            }
            Ok(true)
        })
        .unwrap(),
        "مرحبا Orbit"
    );
    assert_eq!(permissions, 1);
}

#[test]
fn cancellation_terminates_an_active_native_process() {
    let f = Fixture::new("acpRegistry");
    std::fs::write(f.root.join("hang"), "").unwrap();
    let mut checks = 0;
    let error = f
        .run(|event| {
            if matches!(event, Event::Check) {
                checks += 1;
                if checks > 12 {
                    return Err("cancelled".into());
                }
            }
            Ok(true)
        })
        .unwrap_err();
    assert_eq!(error, "cancelled");
    let pid = std::fs::read_to_string(f.root.join("started")).unwrap();
    assert!(!Path::new("/proc").join(pid).exists());
    assert!(!f.root.join("effect").exists());
}

#[test]
fn nested_environment_secrets_are_redacted() {
    let mut secrets = Vec::new();
    collect_secrets(
        "OPENCODE_CONFIG_CONTENT",
        r#"{"provider":{"options":{"apiKey":"fixture-secret"}}}"#,
        false,
        &mut secrets,
    );
    assert!(secrets.iter().any(|s| s == "fixture-secret"));
}

#[test]
fn acp_uses_the_exposed_model_configuration() {
    let mut f = Fixture::new("acpRegistry");
    f.model.id = "fixture/model".into();
    std::fs::write(f.root.join("config-model"), "").unwrap();
    f.run(|_| Ok(true)).unwrap();
    assert_eq!(
        std::fs::read_to_string(f.root.join("selected-model")).unwrap(),
        "fixture/model"
    );
}

#[test]
fn pi_model_errors_are_explicit_and_do_not_expose_provider_bodies() {
    let f = Fixture::new("pi");
    std::fs::write(f.root.join("pi-error"), "").unwrap();
    let error = f.run(|_| Ok(true)).unwrap_err();
    assert!(error.contains("model provider"), "{error}");
    assert!(!error.contains("seeded-secret"));
}

#[test]
#[ignore = "sends one tool-free authentication smoke prompt through each configured T3 Code account"]
fn live_configured_providers() {
    let root = crate::t3_providers::directory().unwrap();
    let instances = crate::t3_providers::instances(&root).unwrap();
    let project = std::env::temp_dir().join(format!("orbit-provider-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&project).unwrap();
    let mut report = Vec::new();
    let selected = std::env::var("ORBIT_PROVIDER_SMOKE_FILTER").ok();
    for instance in instances.into_iter().filter(|i| {
        i.enabled
            && selected
                .as_ref()
                .is_none_or(|selected| selected.split(',').any(|name| name == i.account.name))
    }) {
        let model = Model {
            provider: instance.account.provider.clone(),
            id: instance
                .models
                .first()
                .cloned()
                .unwrap_or_else(|| "default".into()),
            ollama_address: None,
            account: Some(instance.account.clone()),
        };
        let mut permissions = 0;
        let result = run(
            &model,
            &project,
            "Reply exactly ORBIT_PROVIDER_OK. Do not use any tools, read files, execute programs, or change anything.",
            &[],
            |event| {
                if matches!(event, Event::Permission(_)) {
                    permissions += 1;
                    return Ok(permissions == 1);
                }
                Ok(true)
            },
        );
        let error = match result {
            Ok(text) if text.contains("ORBIT_PROVIDER_OK") => None,
            Ok(_) => Some("Unexpected authentication smoke reply.".to_owned()),
            Err(error) => Some(error),
        };
        println!(
            "{}: {}",
            instance.account.name,
            error.as_deref().unwrap_or("connected")
        );
        report.push(json!({"name":instance.account.name,"driver":instance.driver,"error":error}));
    }
    let path = Path::new(if selected.is_some() {
        "target/ui-check/providers-live-filtered.json"
    } else {
        "target/ui-check/providers-live.json"
    });
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(project);
    assert!(
        report.iter().all(|r| r["error"].is_null()),
        "See target/ui-check/providers-live.json for connection failures."
    );
}
