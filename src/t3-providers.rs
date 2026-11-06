use crate::robot_data::{Account, Database, Provider};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, String>;

pub struct Instance {
    pub account: Account,
    pub driver: String,
    pub enabled: bool,
    pub status: String,
    pub models: Vec<String>,
}

pub fn directory() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or("Home directory is unavailable.")?;
    Ok(PathBuf::from(home).join(".t3"))
}

fn read(path: &Path) -> Result<Value> {
    let file = std::fs::File::open(path).map_err(|_| "Couldn't read T3 Code provider settings.")?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
        return Err("T3 Code provider settings exceed 16 MiB.".into());
    }
    serde_json::from_reader(file).map_err(|_| "Couldn't decode T3 Code provider settings.".into())
}

fn binding(configuration: &Value) -> String {
    let connection = serde_json::json!({"driver":configuration["driver"],"config":configuration["config"],"environment":configuration["environment"]});
    format!("{:x}", Sha256::digest(connection.to_string().as_bytes()))
}

pub fn instances(root: &Path) -> Result<Vec<Instance>> {
    let settings = read(&root.join("userdata/settings.json"))?;
    let providers = settings["providerInstances"]
        .as_object()
        .ok_or("Missing T3 Code providers.")?;
    let mut instances = Vec::new();
    for (id, configuration) in providers {
        crate::robot_data::identifier(&format!("t3-{id}")).map_err(|e| e.to_string())?;
        let driver = configuration["driver"]
            .as_str()
            .ok_or("Missing provider driver.")?;
        let cache = read(&root.join("caches").join(format!("{id}.json"))).unwrap_or(Value::Null);
        let provider = match driver {
            "codex" => Provider::Codex,
            "opencode" => Provider::OpenCode,
            "antigravity" => Provider::Antigravity,
            "acpRegistry" if configuration["config"]["agentId"] == "github-copilot-cli" => {
                Provider::Copilot
            }
            _ => Provider::Agent,
        };
        let enabled = configuration["enabled"].as_bool().unwrap_or(true);
        let account = Account {
            id: format!("t3-{id}"),
            name: configuration["displayName"]
                .as_str()
                .or_else(|| cache["displayName"].as_str())
                .unwrap_or(id)
                .into(),
            provider,
            address: None,
            credential: None,
            home: Some(root.to_string_lossy().into_owned()),
            needs_reconnection: false,
            binding: Some(binding(configuration)),
        };
        account.validate().map_err(|e| e.to_string())?;
        let preferences = &settings["providerModelPreferences"][id];
        let hidden = preferences["hiddenModels"].as_array();
        let mut models: Vec<String> = cache["models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|model| model["slug"].as_str().map(str::to_owned))
            .filter(|id| hidden.is_none_or(|hidden| !hidden.iter().any(|v| v == id)))
            .collect();
        if let Some(order) = preferences["modelOrder"].as_array() {
            models.sort_by_key(|id| order.iter().position(|v| v == id).unwrap_or(usize::MAX));
        }
        instances.push(Instance {
            account,
            driver: driver.into(),
            enabled,
            status: if enabled {
                cache["status"].as_str().unwrap_or("Not checked").into()
            } else {
                "Disabled in T3 Code".into()
            },
            models,
        });
    }
    instances.sort_by_key(|i| i.account.name.to_lowercase());
    Ok(instances)
}

pub fn sync(db: &Database, root: &Path) -> Result<Vec<Instance>> {
    let instances = instances(root)?;
    for instance in &instances {
        if instance.enabled {
            db.save_account(&instance.account)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(instances)
}

pub fn configuration(account: &Account) -> Result<Value> {
    let id = account
        .id
        .strip_prefix("t3-")
        .ok_or("This is not a T3 Code provider reference.")?;
    let root = Path::new(
        account
            .home
            .as_deref()
            .ok_or("Missing provider configuration directory.")?,
    );
    let settings = read(&root.join("userdata/settings.json"))?;
    let value = settings["providerInstances"]
        .get(id)
        .ok_or("Provider was removed from T3 Code.")?;
    if value["enabled"] == false {
        return Err("This provider is disabled in T3 Code.".into());
    }
    if account
        .binding
        .as_ref()
        .is_none_or(|expected| *expected != binding(value))
    {
        return Err("This provider's T3 Code connection changed. Select it again for a new conversation; existing conversations cannot switch accounts.".into());
    }
    Ok(value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_references_without_copying_secrets() {
        let root = std::env::temp_dir().join(format!("orbit-t3-{}", std::process::id()));
        std::fs::create_dir_all(root.join("userdata")).unwrap();
        std::fs::write(root.join("userdata/settings.json"), r#"{"providerInstances":{"first":{"driver":"codex","displayName":"عمل Work","enabled":true,"environment":[{"name":"API_KEY","value":"never-export-this"}]},"off":{"driver":"pi","enabled":false}}}"#).unwrap();
        let db = Database::open(&root.join("robots.db")).unwrap();
        let instances = sync(&db, &root).unwrap();
        assert_eq!(instances.len(), 2);
        let accounts = db.accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "عمل Work");
        assert!(
            !serde_json::to_string(&accounts)
                .unwrap()
                .contains("never-export-this")
        );
        assert!(configuration(&instances.iter().find(|i| !i.enabled).unwrap().account).is_err());
        let original = accounts[0].clone();
        assert!(configuration(&original).is_ok());
        let path = root.join("userdata/settings.json");
        let mut changed: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        changed["providerInstances"]["first"]["config"] =
            serde_json::json!({"homePath":"/different/account"});
        std::fs::write(&path, changed.to_string()).unwrap();
        assert!(configuration(&original).unwrap_err().contains("changed"));
        sync(&db, &root).unwrap();
        let mut old_model = crate::robot_data::Model {
            provider: original.provider.clone(),
            id: "default".into(),
            ollama_address: None,
            account: Some(original),
        };
        assert!(db.check_account(&mut old_model).is_err());
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
