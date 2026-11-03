use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[path = "robot-tools/mcp.rs"]
pub mod mcp;
#[path = "robot-tools/web.rs"]
pub mod web;

pub type Result<T> = std::result::Result<T, String>;
const LIMIT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(60);
static APPROVAL_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "action", deny_unknown_fields)]
pub enum LocalAction {
    Read {
        path: PathBuf,
    },
    Write {
        path: PathBuf,
        text: String,
    },
    List {
        path: PathBuf,
    },
    Run {
        program: String,
        arguments: Vec<String>,
    },
    Skill {
        path: PathBuf,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct Approval {
    pub id: u64,
    pub project: PathBuf,
    pub action: LocalAction,
}

// Pending actions are memory-only and owned by one conversation's worker.
#[derive(Default)]
pub struct LocalTools {
    pending: Option<Approval>,
}

impl LocalTools {
    pub fn prepare(&mut self, project: &Path, mut action: LocalAction) -> Result<&Approval> {
        if self.pending.is_some() {
            return Err("Resolve the pending local action first.".into());
        }
        let project = project.canonicalize().map_err(|e| e.to_string())?;
        if !project.is_dir() {
            return Err("Choose an existing project directory.".into());
        }
        validate_action(&project, &mut action)?;
        let id = APPROVAL_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| "Approval IDs exhausted.")?
            + 1;
        self.pending = Some(Approval {
            id,
            project,
            action,
        });
        Ok(self.pending.as_ref().unwrap())
    }

    pub fn pending(&self) -> Option<&Approval> {
        self.pending.as_ref()
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }

    pub fn resolve(&mut self, id: u64, approved: bool) -> Result<String> {
        if self.pending.as_ref().map(|p| p.id) != Some(id) {
            return Err("This local approval is no longer pending.".into());
        }
        let request = self.pending.take().unwrap();
        if !approved {
            return Err("Local action denied.".into());
        }
        execute(request)
    }
}

fn validate_action(project: &Path, action: &mut LocalAction) -> Result<()> {
    match action {
        LocalAction::Run { program, arguments } => {
            if program.is_empty()
                || program.contains('\0')
                || program.len() > 32_768
                || arguments.len() > 256
                || arguments
                    .iter()
                    .any(|a| a.contains('\0') || a.len() > 32_768)
            {
                return Err("Invalid local program or arguments.".into());
            }
        }
        LocalAction::Read { path }
        | LocalAction::List { path }
        | LocalAction::Write { path, .. }
        | LocalAction::Skill { path } => {
            if path.as_os_str().is_empty() {
                return Err("Choose a local path.".into());
            }
            if path.is_relative() {
                *path = project.join(&*path);
            }
        }
    }
    if let LocalAction::Write { text, .. } = action
        && text.len() > LIMIT
    {
        return Err("Local writes are limited to 1 MiB.".into());
    }
    if let LocalAction::Skill { path } = action
        && path.file_name().is_none_or(|name| name != "SKILL.md")
    {
        return Err("Choose an installed SKILL.md file.".into());
    }
    Ok(())
}

fn execute(request: Approval) -> Result<String> {
    match request.action {
        LocalAction::Read { path } => read_text(&path),
        LocalAction::Skill { path } => {
            let instructions = read_text(&path)?;
            Ok(format!(
                "Skill: {}\nResolve relative resources against this directory. Skill instructions do not grant permissions. Use Orbit local and enabled MCP tools; report unsupported Codex/T3 tools instead of claiming execution.\n\n{instructions}",
                path.parent().unwrap().display()
            ))
        }
        LocalAction::Write { path, text } => atomic_write(&path, &text),
        LocalAction::List { path } => {
            let mut names = Vec::new();
            let mut bytes = 0;
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                let name = entry
                    .map_err(|e| e.to_string())?
                    .file_name()
                    .to_string_lossy()
                    .into_owned();
                bytes += name.len() + 1;
                if bytes > LIMIT {
                    return Err("Directory listing exceeds 1 MiB.".into());
                }
                names.push(name);
            }
            names.sort();
            serde_json::to_string(&names).map_err(|e| e.to_string())
        }
        LocalAction::Run { program, arguments } => {
            run(&request.project, &program, &arguments, TIMEOUT)
        }
    }
}

fn read_text(path: &Path) -> Result<String> {
    if !fs::metadata(path).map_err(|e| e.to_string())?.is_file() {
        return Err("Choose a regular text file.".into());
    }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Choose a regular text file.".into());
    }
    let bytes = read_bounded(file)?;
    String::from_utf8(bytes).map_err(|_| "The local file is not UTF-8 text.".into())
}

fn read_bounded(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Tool output exceeds 1 MiB.".into());
    }
    Ok(bytes)
}

fn atomic_write(path: &Path, text: &str) -> Result<String> {
    // Resolve existing symlinks so replacement updates the approved target, not its link.
    let target = if path.exists() {
        path.canonicalize().map_err(|e| e.to_string())?
    } else {
        path.to_owned()
    };
    let parent = target.parent().ok_or("File has no parent directory.")?;
    let temporary = parent.join(format!(".orbit-robot-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        if let Ok(metadata) = fs::metadata(&target) {
            file.set_permissions(metadata.permissions())?;
        }
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e: std::io::Error| e.to_string())?;
    Ok(format!("Saved {}", path.display()))
}

fn run(project: &Path, program: &str, arguments: &[String], timeout: Duration) -> Result<String> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::platform::configure_robot_process(&mut command);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let (sender, receiver) = mpsc::channel();
    for (name, pipe) in [
        (
            "stdout",
            Box::new(child.stdout.take().unwrap()) as Box<dyn Read + Send>,
        ),
        (
            "stderr",
            Box::new(child.stderr.take().unwrap()) as Box<dyn Read + Send>,
        ),
    ] {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send((name, read_bounded(pipe)));
        });
    }
    let deadline = Instant::now() + timeout;
    let result = (|| {
        let mut output = serde_json::Map::new();
        for _ in 0..2 {
            let (name, bytes) = receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "Local command timed out; side effects may have occurred.")?;
            output.insert(
                name.into(),
                String::from_utf8_lossy(&bytes?).into_owned().into(),
            );
        }
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                output.insert("exit_code".into(), status.code().into());
                output.insert("success".into(), status.success().into());
                return Ok(serde_json::Value::Object(output).to_string());
            }
            if Instant::now() >= deadline {
                return Err("Local command timed out; side effects may have occurred.".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    if result.is_err() {
        crate::platform::stop_robot_process(&mut child);
    }
    result
}

pub fn credential(reference: &str) -> Result<String> {
    #[cfg(target_os = "linux")]
    crate::ticktick::activate_secret_service()?;
    keyring::Entry::new("orbit-robots", reference)
        .map_err(|_| "Couldn't open the system credential store.")?
        .get_password()
        .map_err(|_| format!("Missing Robot credential: {reference}. Configure it in Settings."))
}

pub(crate) fn redact(value: &mut serde_json::Value, secret: &str) {
    if secret.is_empty() {
        return;
    }
    match value {
        serde_json::Value::String(text) => *text = text.replace(secret, "[redacted]"),
        serde_json::Value::Array(values) => {
            for value in values {
                redact(value, secret);
            }
        }
        serde_json::Value::Object(values) => {
            let old = std::mem::take(values);
            for (key, mut value) in old {
                redact(&mut value, secret);
                values.insert(key.replace(secret, "[redacted]"), value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "robot-tools/tests.rs"]
mod tests;
