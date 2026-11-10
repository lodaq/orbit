use super::{LIMIT, Result, TIMEOUT, credential};
use crate::robot_data::McpServer;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

pub struct Session {
    child: Child,
    configuration: McpServer,
    input: Sender<Vec<u8>>,
    written: Receiver<Result<()>>,
    output: Receiver<Result<Value>>,
    secrets: Vec<String>,
    next_id: u64,
    failed: bool,
}

impl Session {
    // Call only from an explicit configuration/discovery action or an enabled tool run.
    pub fn connect(server: &McpServer, project: &Path) -> Result<Self> {
        server.validate().map_err(|e| e.to_string())?;
        let mut secrets = BTreeMap::new();
        for (name, reference) in &server.secret_env {
            secrets.insert(name.clone(), credential(reference)?);
        }
        Self::start(server, project, secrets)
    }

    fn start(
        server: &McpServer,
        project: &Path,
        secrets: BTreeMap<String, String>,
    ) -> Result<Self> {
        let mut command = Command::new(&server.command);
        // Inherited provider keys must not leak into a configured MCP process.
        command
            .args(&server.arguments)
            .current_dir(project)
            .env_clear();
        for name in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SystemRoot",
            "TEMP",
            "TMP",
            "LANG",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command
            .envs(&secrets)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        crate::platform::configure_robot_process(&mut command);
        let mut child = command
            .spawn()
            .map_err(|_| "Couldn't start the configured MCP server.")?;
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (input, writes) = mpsc::channel::<Vec<u8>>();
        let (done, written) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(bytes) = writes.recv() {
                let result = stdin
                    .write_all(&bytes)
                    .and_then(|_| stdin.flush())
                    .map_err(|_| "Couldn't write to MCP server.".into());
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
                let message = read_message(&mut reader);
                let failed = message.is_err();
                if sender.send(message).is_err() || failed {
                    break;
                }
            }
        });
        let mut session = Self {
            child,
            configuration: server.clone(),
            input,
            written,
            output,
            secrets: secrets.into_values().filter(|s| !s.is_empty()).collect(),
            next_id: 0,
            failed: false,
        };
        session
            .secrets
            .sort_by_key(|secret| std::cmp::Reverse(secret.len()));
        let init = session.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "Orbit", "version": env!("CARGO_PKG_VERSION")}
            }),
        )?;
        if init["protocolVersion"] != "2025-06-18" || !init["capabilities"]["tools"].is_object() {
            return Err("MCP server must support protocol 2025-06-18 and tools.".into());
        }
        session.send(
            json!({"jsonrpc":"2.0", "method":"notifications/initialized"}),
            Instant::now() + TIMEOUT,
        )?;
        Ok(session)
    }

    pub fn discover(&mut self) -> Result<Vec<Value>> {
        let mut tools = Vec::new();
        let mut bytes = 0;
        let mut cursor = None;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            let result = self.request(
                "tools/list",
                cursor.as_ref().map_or(json!({}), |c| json!({"cursor":c})),
            )?;
            bytes += result.to_string().len();
            if bytes > LIMIT {
                return Err("MCP tool definitions exceed 1 MiB.".into());
            }
            let page = result["tools"].as_array().ok_or("Invalid MCP tool list.")?;
            for tool in page {
                let name = tool["name"]
                    .as_str()
                    .filter(|name| !name.is_empty())
                    .ok_or("Invalid MCP tool name.")?;
                if !tool["inputSchema"].is_object() || !seen.insert(name.to_owned()) {
                    return Err("Invalid or duplicate MCP tool definition.".into());
                }
                tools.push(tool.clone());
            }
            if tools.len() > 1024 {
                return Err("Too many MCP tools.".into());
            }
            match result.get("nextCursor") {
                None => return Ok(tools),
                Some(Value::String(next)) if !next.is_empty() && cursor.as_ref() != Some(next) => {
                    cursor = Some(next.clone())
                }
                _ => return Err("Invalid MCP pagination cursor.".into()),
            }
        }
        Err("MCP discovery exceeded its page limit.".into())
    }

    pub fn call(&mut self, current: &McpServer, name: &str, arguments: Value) -> Result<Value> {
        if current != &self.configuration {
            return Err("MCP settings changed; reconnect before calling a tool.".into());
        }
        if !current.enabled_tools.iter().any(|enabled| enabled == name) {
            return Err("Enable this MCP tool in Settings before running it.".into());
        }
        if !arguments.is_object() {
            return Err("MCP tool arguments must be an object.".into());
        }
        self.request("tools/call", json!({"name":name, "arguments":arguments}))
    }

    pub fn redact(&self, value: &mut Value) {
        for secret in &self.secrets {
            super::redact(value, secret);
        }
    }

    fn send(&mut self, value: Value, deadline: Instant) -> Result<()> {
        let mut bytes = serde_json::to_vec(&value).map_err(|_| "Invalid MCP request.")?;
        if bytes.len() > LIMIT {
            return Err("MCP request exceeds 1 MiB.".into());
        }
        bytes.push(b'\n');
        self.input
            .send(bytes)
            .map_err(|_| "MCP server closed its input.")?;
        self.written
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| "MCP write timed out; the action may have occurred.")?
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        if self.failed {
            return Err(
                "MCP session failed; reconnect explicitly. Do not replay a tool call.".into(),
            );
        }
        self.next_id += 1;
        let deadline = Instant::now() + TIMEOUT;
        let result = self.exchange(method, params, deadline);
        if result.is_err() {
            self.failed = true;
            crate::platform::stop_robot_process(&mut self.child);
        }
        result
    }

    fn exchange(&mut self, method: &str, params: Value, deadline: Instant) -> Result<Value> {
        self.send(
            json!({"jsonrpc":"2.0", "id":self.next_id, "method":method, "params":params}),
            deadline,
        )?;
        loop {
            if Instant::now() >= deadline {
                return Err("MCP response timed out; the action may have occurred.".into());
            }
            let mut message = self
                .output
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "MCP response timed out; the action may have occurred.")??;
            self.redact(&mut message);
            if message["jsonrpc"] != "2.0" {
                return Err("Invalid MCP JSON-RPC version.".into());
            }
            if message.get("method").is_some() {
                if let Some(id) = message.get("id") {
                    // Sampling, roots and elicitation are not advertised or authorized.
                    self.send(json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32601,"message":"Unsupported client method"}}), deadline)?;
                }
                continue;
            }
            if message["id"] != self.next_id {
                return Err("MCP response ID does not match the request.".into());
            }
            if let Some(error) = message.get("error") {
                return Err(format!("MCP error: {error}"));
            }
            return message
                .get("result")
                .cloned()
                .ok_or("MCP response has no result.".into());
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> Result<Value> {
    let mut bytes = Vec::new();
    loop {
        let buffer = reader.fill_buf().map_err(|_| "Couldn't read MCP output.")?;
        if buffer.is_empty() {
            return Err("MCP server closed its output.".into());
        }
        let count = buffer
            .iter()
            .position(|b| *b == b'\n')
            .map_or(buffer.len(), |n| n + 1);
        if bytes.len() + count > LIMIT {
            return Err("MCP response exceeds 1 MiB.".into());
        }
        bytes.extend_from_slice(&buffer[..count]);
        reader.consume(count);
        if bytes.last() == Some(&b'\n') {
            break;
        }
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid MCP JSON response.".into())
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.failed {
            crate::platform::stop_robot_process(&mut self.child);
        }
    }
}

#[cfg(test)]
#[path = "mcp-tests.rs"]
mod tests;
