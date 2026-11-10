use crate::robot_data::{Account, McpServer, Model};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, String>;
const LIMIT: usize = 8 * 1024 * 1024;

#[cfg(all(test, unix))]
#[path = "robot-native/tests.rs"]
mod tests;

pub enum Event {
    Check,
    Permission(Value),
    Activity(Value),
}

struct Process {
    child: Child,
    input: mpsc::Sender<Vec<u8>>,
    written: Receiver<Result<()>>,
    output: Receiver<Result<Value>>,
    next: u64,
    text: String,
    secrets: Vec<String>,
}

impl Drop for Process {
    fn drop(&mut self) {
        crate::platform::stop_robot_process(&mut self.child);
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

fn registry_command(root: &Path, config: &Value) -> Result<(String, Vec<String>)> {
    if config["source"] == "local" {
        return Ok((
            config["commandPath"]
                .as_str()
                .ok_or("Missing local agent program.")?
                .into(),
            strings(&config["commandArgs"]),
        ));
    }
    let id = config["agentId"]
        .as_str()
        .ok_or("Missing registry agent ID.")?;
    let registry: Value = serde_json::from_reader(
        std::fs::File::open(root.join("caches/acp-registry/registry.json"))
            .map_err(|_| "Missing cached agent registry.")?,
    )
    .map_err(|_| "Invalid cached agent registry.")?;
    let agent = registry["agents"]
        .as_array()
        .and_then(|a| a.iter().find(|a| a["id"] == id))
        .ok_or("Agent is missing from the cached registry.")?;
    let receipts = root.join("caches/acp-registry/package-installs");
    if let Ok(entries) = std::fs::read_dir(receipts) {
        for entry in entries.flatten() {
            let receipt: Value = std::fs::File::open(entry.path())
                .ok()
                .and_then(|f| serde_json::from_reader(f).ok())
                .unwrap_or(Value::Null);
            if receipt["agentId"] == id
                && receipt["agentVersion"] == agent["version"]
                && let Some(program) = receipt["executablePath"].as_str()
                && Path::new(program).is_file()
            {
                let distribution = receipt["distribution"].as_str().unwrap_or("npx");
                return Ok((
                    program.into(),
                    strings(&agent["distribution"][distribution]["args"]),
                ));
            }
        }
    }
    let target = format!(
        "{}-{}",
        std::env::consts::OS,
        if std::env::consts::ARCH == "x86_64" {
            "x86_64"
        } else {
            "aarch64"
        }
    );
    let binary = &agent["distribution"]["binary"][&target];
    let program = root
        .join("tools")
        .join(id)
        .join(agent["version"].as_str().ok_or("Missing agent version.")?)
        .join(&target)
        .join(
            binary["cmd"]
                .as_str()
                .ok_or("Agent is not installed for this platform.")?
                .trim_start_matches("./"),
        );
    if !program.is_file() {
        return Err("Install this provider in T3 Code first.".into());
    }
    Ok((
        program.to_string_lossy().into_owned(),
        strings(&binary["args"]),
    ))
}

fn command(account: &Account) -> Result<(Command, String, Vec<String>)> {
    let configuration = crate::t3_providers::configuration(account)?;
    let driver = configuration["driver"]
        .as_str()
        .ok_or("Missing native provider driver.")?
        .to_owned();
    let config = &configuration["config"];
    let root = Path::new(
        account
            .home
            .as_deref()
            .ok_or("Missing T3 Code directory.")?,
    );
    let (program, arguments) = match driver.as_str() {
        "codex" => (
            config["binaryPath"].as_str().unwrap_or("codex").into(),
            vec!["app-server".into()],
        ),
        "opencode" => (
            config["binaryPath"].as_str().unwrap_or("opencode").into(),
            vec!["acp".into()],
        ),
        "antigravity" => (
            config["binaryPath"]
                .as_str()
                .ok_or("Missing Antigravity executable.")?
                .into(),
            vec![],
        ),
        "acpRegistry" => registry_command(root, config)?,
        "pi" => (
            config["binaryPath"].as_str().unwrap_or("pi").into(),
            vec!["--mode".into(), "rpc".into(), "--no-session".into()],
        ),
        _ => {
            return Err(format!(
                "The {driver} provider does not expose a connected Orbit protocol yet."
            ));
        }
    };
    let mut command = Command::new(program);
    command.args(arguments).env_clear().env("BROWSER", "false");
    for name in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SystemRoot",
        "TEMP",
        "TMP",
        "LANG",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "DBUS_SESSION_BUS_ADDRESS",
        "SSH_AUTH_SOCK",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut secrets = Vec::new();
    if let Ok(file) = std::fs::File::open(root.join("userdata/shell-environment-cache.json"))
        && let Ok(cache) = serde_json::from_reader::<_, Value>(file)
        && let Some(environment) = cache["environment"].as_object()
    {
        for (name, value) in environment {
            if (name == "PATH"
                || [
                    "OPENCODE_",
                    "CODEX_",
                    "OPENAI_",
                    "ANTHROPIC_",
                    "GOOGLE_",
                    "GEMINI_",
                    "GITHUB_",
                    "GH_",
                    "COPILOT_",
                    "GLM_",
                    "ZAI_",
                    "PI_",
                    "KILO_",
                    "CLINE_",
                    "DEVIN_",
                    "AGORAGENTIC_",
                ]
                .iter()
                .any(|prefix| name.starts_with(prefix)))
                && let Some(value) = value.as_str()
            {
                command.env(name, value);
                collect_secrets(name, value, false, &mut secrets);
            }
        }
    }
    for entry in configuration["environment"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let (Some(name), Some(value)) = (entry["name"].as_str(), entry["value"].as_str()) {
            command.env(name, value);
            collect_secrets(name, value, entry["sensitive"] == true, &mut secrets);
        }
    }
    if driver == "codex"
        && let Some(home) = config["homePath"].as_str()
    {
        command.env("CODEX_HOME", home);
    }
    if driver == "antigravity" {
        let id = account
            .id
            .strip_prefix("t3-")
            .ok_or("Missing Antigravity instance ID.")?;
        let profile = root
            .join("userdata/providers/antigravity")
            .join(format!("{:x}", Sha256::digest(id.as_bytes())));
        if !profile.is_dir() {
            return Err("Missing Antigravity authentication; connect it in T3 Code.".into());
        }
        command
            .env("GEMINI_HOME", profile)
            .env("AGY_ACP_FORCE_FILE_STORAGE", "1")
            .env("PYTHONUNBUFFERED", "1");
        if let Some(key) = config["apiKey"].as_str().filter(|s| !s.is_empty()) {
            command.env(
                if config["authMethod"] == "agent-platform" {
                    "GOOGLE_API_KEY"
                } else {
                    "GEMINI_API_KEY"
                },
                key,
            );
            secrets.push(key.into());
        }
    }
    Ok((command, driver, secrets))
}

fn collect_secrets(name: &str, value: &str, sensitive: bool, secrets: &mut Vec<String>) {
    let name = name.to_ascii_lowercase();
    if (sensitive
        || ["key", "token", "password", "secret"]
            .iter()
            .any(|key| name.contains(key)))
        && !value.is_empty()
    {
        secrets.push(value.into());
    }
    if let Ok(Value::Object(object)) = serde_json::from_str(value) {
        for (key, value) in object {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            collect_secrets(&key, &value, false, secrets);
        }
    }
}

impl Process {
    fn start(mut command: Command, project: &Path, secrets: Vec<String>) -> Result<Self> {
        command
            .current_dir(project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        crate::platform::configure_robot_process(&mut command);
        let mut child = command
            .spawn()
            .map_err(|_| "Couldn't start this provider's configured executable.")?;
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (input, writes) = mpsc::channel::<Vec<u8>>();
        let (done, written) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(bytes) = writes.recv() {
                let result = stdin
                    .write_all(&bytes)
                    .and_then(|_| stdin.flush())
                    .map_err(|_| "Provider input closed.".into());
                let failed = result.is_err();
                if done.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let (sender, output) = mpsc::sync_channel(16);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                let result = reader
                    .by_ref()
                    .take(LIMIT as u64 + 1)
                    .read_until(b'\n', &mut bytes)
                    .map_err(|_| "Couldn't read provider output.".to_owned())
                    .and_then(|n| {
                        if n == 0 || n > LIMIT {
                            Err("Provider closed or exceeded the message limit.".into())
                        } else {
                            serde_json::from_slice(&bytes)
                                .map_err(|_| "Invalid provider JSON.".into())
                        }
                    });
                let failed = result.is_err();
                if sender.send(result).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            input,
            written,
            output,
            next: 0,
            text: String::new(),
            secrets,
        })
    }
    fn send(&mut self, value: Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
        if bytes.len() > LIMIT {
            return Err("Native provider request exceeds 8 MiB.".into());
        }
        bytes.push(b'\n');
        self.input
            .send(bytes)
            .map_err(|_| "Provider input closed.")?;
        self.written
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Provider input timed out.")?
    }
    fn receive(
        &self,
        callback: &mut impl FnMut(Event) -> Result<bool>,
        deadline: Instant,
    ) -> Result<Value> {
        loop {
            callback(Event::Check)?;
            if Instant::now() >= deadline {
                return Err(
                    "Provider timed out; work may have occurred. It was not retried.".into(),
                );
            }
            match self.output.recv_timeout(Duration::from_millis(50)) {
                Ok(result) => {
                    return result;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err("Provider output closed.".into()),
            }
        }
    }
    fn request(
        &mut self,
        method: &str,
        params: Value,
        callback: &mut impl FnMut(Event) -> Result<bool>,
    ) -> Result<Value> {
        self.next += 1;
        let id = self.next;
        self.send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}))?;
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let value = self.receive(callback, deadline)?;
            if value["id"] == id && value.get("method").is_none() {
                if value.get("error").is_some() {
                    if value["error"]["code"] == -32601 {
                        return Err(format!(
                            "This provider does not implement {method}; its native protocol is unsupported."
                        ));
                    }
                    return Err(format!(
                        "Provider rejected {method} (code {}). Check its authentication and selected model in T3 Code.",
                        value["error"]["code"].as_i64().unwrap_or(-1)
                    ));
                }
                return Ok(value["result"].clone());
            }
            self.event(value, callback)?;
        }
    }
    fn event(
        &mut self,
        value: Value,
        callback: &mut impl FnMut(Event) -> Result<bool>,
    ) -> Result<()> {
        let method = value["method"].as_str().unwrap_or_default();
        let params = &value["params"];
        if let Some(id) = value.get("id") {
            let permission =
                method == "session/request_permission" || method.ends_with("/requestApproval");
            if !permission {
                self.send(json!({"jsonrpc":"2.0", "id":id,"error":{"code":-32601,"message":"Unsupported client operation"}}))?;
                return Ok(());
            }
            let mut visible = params.clone();
            self.redact(&mut visible);
            let approved = callback(Event::Permission(visible))?;
            let result = if method == "session/request_permission" {
                let kind = if approved {
                    "allow_once"
                } else {
                    "reject_once"
                };
                let option = params["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|o| o["kind"] == kind)
                    .and_then(|o| o["optionId"].as_str());
                option.map_or_else(
                    || json!({"outcome":{"outcome":"cancelled"}}),
                    |id| json!({"outcome":{"outcome":"selected","optionId":id}}),
                )
            } else if method == "item/permissions/requestApproval" {
                json!({"permissions":if approved { params["permissions"].clone() } else { json!({}) },"scope":"turn"})
            } else {
                json!({"decision":if approved { "accept" } else { "decline" }})
            };
            self.send(json!({"jsonrpc":"2.0","id":id,"result":result}))?;
        } else if method == "item/agentMessage/delta" {
            self.append(params["delta"].as_str().unwrap_or_default())?;
        } else if method == "session/update" {
            let update = &params["update"];
            if update["sessionUpdate"] == "agent_message_chunk"
                && update["content"]["type"] == "text"
            {
                self.append(update["content"]["text"].as_str().unwrap_or_default())?;
            } else if update["sessionUpdate"] == "tool_call"
                || update["sessionUpdate"] == "tool_call_update"
            {
                let mut update = update.clone();
                self.redact(&mut update);
                callback(Event::Activity(update))?;
            }
        } else if method == "item/completed"
            && params["item"]["type"] != "agentMessage"
            && params["item"]["type"] != "reasoning"
        {
            let mut item = params["item"].clone();
            self.redact(&mut item);
            callback(Event::Activity(item))?;
        }
        Ok(())
    }
    fn append(&mut self, text: &str) -> Result<()> {
        if self.text.len() + text.len() > LIMIT {
            return Err("Provider answer exceeds 8 MiB.".into());
        }
        self.text.push_str(text);
        Ok(())
    }
    fn redact(&self, value: &mut Value) {
        for secret in &self.secrets {
            crate::robot_tools::redact(value, secret);
        }
    }
}

fn mcp_servers(servers: &[McpServer]) -> Result<Vec<Value>> {
    servers.iter().filter(|s| !s.enabled_tools.is_empty()).map(|s| {
        // Native agents cannot enforce Orbit's individual MCP tool allow-list.
        Err(format!("Native provider MCP attachment '{}' needs a provider-side tool allow-list. Remove it from this Robot or use its existing native MCP configuration.", s.id))
    }).collect()
}

pub fn run(
    model: &Model,
    project: &Path,
    prompt: &str,
    servers: &[McpServer],
    mut callback: impl FnMut(Event) -> Result<bool>,
) -> Result<String> {
    model.validate().map_err(|e| e.to_string())?;
    let account = model.account.as_ref().ok_or("Choose a provider account.")?;
    if account.needs_reconnection {
        return Err("Reconnect this imported provider account before running work.".into());
    }
    let mcp = mcp_servers(servers)?;
    let (command, driver, secrets) = command(account)?;
    if !callback(Event::Permission(
        json!({"provider":account.name,"scope":format!("Run this command with the native agent.\nExecutable: {}\n\nIt may read and change files, execute programs, access the network and use its configured MCP servers with your account. Native permissions may cover multiple operations. Access lasts until this command finishes or is cancelled; completed effects are not rolled back.",command.get_program().to_string_lossy())}),
    ))? {
        return Err("Native provider command denied. No provider process was started.".into());
    }
    let mut process = Process::start(command, project, secrets)?;
    match driver.as_str() {
        "pi" => run_pi(&mut process, model, project, prompt, &mut callback)?,
        "codex" => run_codex(&mut process, model, project, prompt, &mut callback)?,
        _ => run_acp(&mut process, model, project, prompt, &mut callback, &mcp)?,
    }
    let mut answer = Value::String(std::mem::take(&mut process.text));
    process.redact(&mut answer);
    let answer = answer.as_str().unwrap_or_default();
    if answer.trim().is_empty() {
        return Err("Provider returned no visible answer.".into());
    }
    Ok(answer.into())
}

fn run_pi(
    process: &mut Process,
    model: &Model,
    _project: &Path,
    prompt: &str,
    callback: &mut impl FnMut(Event) -> Result<bool>,
) -> Result<()> {
    if model.id != "default" {
        let (provider, id) = model
            .id
            .split_once('/')
            .ok_or("Pi models use provider/model names.")?;
        process.send(json!({"id":"model","type":"set_model","provider":provider,"modelId":id}))?;
    }
    process.send(json!({"id":"prompt","type":"prompt","message":prompt}))?;
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let message = process.receive(callback, deadline)?;
        if message["type"] == "message_end"
            && message["message"]["role"] == "assistant"
            && message["message"]["stopReason"] == "error"
        {
            return Err(
                "Pi model provider failed. Check its account and selected model in T3 Code.".into(),
            );
        }
        if message["type"] == "response" && message["success"] == false {
            return Err("Pi rejected the selected model or command.".into());
        }
        if message["type"] == "tool_execution_end" {
            let mut visible = message.clone();
            process.redact(&mut visible);
            callback(Event::Activity(visible))?;
        }
        if message["type"] == "agent_end" {
            for message in message["messages"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|m| m["role"] == "assistant")
            {
                for content in message["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|c| c["type"] == "text")
                {
                    process.append(content["text"].as_str().unwrap_or_default())?;
                }
            }
            break;
        }
    }
    Ok(())
}

fn run_codex(
    process: &mut Process,
    model: &Model,
    project: &Path,
    prompt: &str,
    callback: &mut impl FnMut(Event) -> Result<bool>,
) -> Result<()> {
    process.request("initialize", json!({"clientInfo":{"name":"orbit","version":"0.1.0"},"capabilities":{"experimentalApi":true}}), callback)?;
    process.send(json!({"method":"initialized","params":{}}))?;
    let thread = process.request("thread/start", json!({"model":model.id,"cwd":project,"approvalPolicy":"untrusted","sandbox":"read-only","ephemeral":true}), callback)?;
    let id = thread["thread"]["id"]
        .as_str()
        .ok_or("Missing Codex thread ID.")?;
    process.request("turn/start", json!({"threadId":id,"input":[{"type":"text","text":prompt}],"approvalPolicy":"untrusted","sandboxPolicy":{"type":"readOnly"}}), callback)?;
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let value = process.receive(callback, deadline)?;
        if value["method"] == "turn/completed" && value["params"]["threadId"] == id {
            if value["params"]["turn"]["status"] != "completed" {
                return Err("Codex did not complete this command.".into());
            }
            break;
        }
        process.event(value, callback)?;
    }
    Ok(())
}

fn run_acp(
    process: &mut Process,
    model: &Model,
    project: &Path,
    prompt: &str,
    callback: &mut impl FnMut(Event) -> Result<bool>,
    mcp: &[Value],
) -> Result<()> {
    process.request("initialize", json!({"protocolVersion":1,"clientCapabilities":{"fs":{"readTextFile":false,"writeTextFile":false},"terminal":false},"clientInfo":{"name":"orbit","version":"0.1.0"}}), callback)?;
    let session = process.request(
        "session/new",
        json!({"cwd":project,"mcpServers":mcp}),
        callback,
    )?;
    let id = session["sessionId"]
        .as_str()
        .ok_or("Missing native session ID.")?;
    if model.id != "default" {
        let option = session["configOptions"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|o| o["category"] == "model");
        if let Some(option) = option {
            process.request(
                "session/set_config_option",
                json!({"sessionId":id,"configId":option["id"],"value":model.id}),
                callback,
            )?;
        } else {
            process.request(
                "session/set_model",
                json!({"sessionId":id,"modelId":model.id}),
                callback,
            )?;
        }
    }
    let result = process.request(
        "session/prompt",
        json!({"sessionId":id,"prompt":[{"type":"text","text":prompt}]}),
        callback,
    )?;
    if result["stopReason"] != "end_turn" {
        return Err("Native provider did not complete this command.".into());
    }
    Ok(())
}
