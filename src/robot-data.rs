use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Provider {
    Ollama,
    OpenAi,
    Gemini,
    OpenCode,
    Antigravity,
    Copilot,
    Codex,
    Agent,
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini",
            Self::OpenCode => "OpenCode",
            Self::Antigravity => "Antigravity",
            Self::Copilot => "Copilot",
            Self::Codex => "Codex / ChatGPT",
            Self::Agent => "Agent",
        }
    }

    pub fn index(&self) -> i32 {
        match self {
            Self::Ollama => 0,
            Self::OpenAi => 1,
            Self::Gemini => 2,
            Self::OpenCode => 3,
            Self::Antigravity => 4,
            Self::Copilot => 5,
            Self::Codex => 6,
            Self::Agent => 7,
        }
    }

    pub fn from_index(index: i32) -> Option<Self> {
        Some(match index {
            0 => Self::Ollama,
            1 => Self::OpenAi,
            2 => Self::Gemini,
            3 => Self::OpenCode,
            4 => Self::Antigravity,
            5 => Self::Copilot,
            6 => Self::Codex,
            7 => Self::Agent,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub provider: Provider,
    pub address: Option<String>,
    pub credential: Option<String>,
    pub home: Option<String>,
    #[serde(default)]
    pub needs_reconnection: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub provider: Provider,
    pub id: String,
    pub ollama_address: Option<String>,
    #[serde(default)]
    pub account: Option<Account>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub instructions: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct McpServer {
    pub id: String,
    pub command: String,
    pub arguments: Vec<String>,
    // Values identify OS keyring entries; credential values never enter this structure.
    pub secret_env: BTreeMap<String, String>,
    pub enabled_tools: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub project: String,
    pub model: Model,
    pub role_id: Option<String>,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<McpServer>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub model: Model,
    pub role: Option<Role>,
    pub project: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum ToolStatus {
    Running,
    AwaitingApproval,
    Denied,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Entry {
    Queued {
        id: String,
        text: String,
    },
    Run {
        id: String,
        status: RunStatus,
    },
    Command {
        text: String,
    },
    Reply {
        text: String,
    },
    Progress {
        text: String,
    },
    Tool {
        name: String,
        input: serde_json::Value,
        output: String,
        status: ToolStatus,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Conversation {
    pub id: String,
    pub robot_id: String,
    pub title: String,
    pub context: Context,
    pub created_at: i64,
    pub updated_at: i64,
    pub rating: Option<u8>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, PartialEq)]
pub struct History {
    pub id: String,
    pub title: String,
    pub updated_at: i64,
    pub rating: Option<u8>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(default)]
    pub catalog: Vec<CatalogEntry>,
    #[serde(default)]
    pub accounts: Vec<Account>,
    pub roles: Vec<Role>,
    pub profiles: Vec<Profile>,
    pub conversations: Vec<Conversation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub content: String,
}
impl CatalogEntry {
    fn validate(&self) -> Result<()> {
        if !matches!(
            self.kind.as_str(),
            "skill" | "prompt" | "shell" | "project" | "credential" | "mcp"
        ) || self.id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.id.len() > 512
            || self.name.len() > 512
            || self.content.len() > 131_072
            || self.content.contains('\0')
            || (self.kind != "credential" && self.content.trim().is_empty())
        {
            return Err("Enter a name and content for this collection.".into());
        }
        if self.kind == "credential" && !self.content.is_empty() {
            return Err("Credential values belong only in the system store.".into());
        }
        if self.kind == "skill"
            && Path::new(&self.content)
                .file_name()
                .is_none_or(|name| name != "SKILL.md")
        {
            return Err("Choose an installed SKILL.md file.".into());
        }
        if self.kind == "mcp" {
            serde_json::from_str::<McpServer>(&self.content)?.validate()?;
        }
        Ok(())
    }
}

pub struct Database {
    connection: Connection,
}

impl Account {
    pub(crate) fn validate(&self) -> Result<()> {
        if self
            .binding
            .as_ref()
            .is_some_and(|s| s.len() != 64 || !s.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("Invalid provider connection binding.".into());
        }
        identifier(&self.id)?;
        required(&self.name, 512)?;
        if let Some(reference) = &self.credential {
            identifier(reference)?;
        }
        match self.provider {
            Provider::Ollama => {
                loopback(self.address.as_deref().ok_or("Enter an Ollama address.")?)?;
                if self.home.is_some() || self.credential.is_some() {
                    return Err("Ollama accounts use only a local address.".into());
                }
            }
            Provider::OpenAi | Provider::Gemini => {
                if self.address.is_some() || self.home.is_some() {
                    return Err("API accounts use a credential reference.".into());
                }
            }
            _ => {
                if self.address.is_some() || self.credential.is_some() {
                    return Err("Agent accounts use a separate account directory.".into());
                }
                if let Some(home) = &self.home {
                    absolute_path(home)?;
                }
            }
        }
        Ok(())
    }
}

fn loopback(address: &str) -> Result<()> {
    let port = address
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| address.strip_prefix("http://localhost:"))
        .and_then(|port| port.parse::<u16>().ok());
    if !port.is_some_and(|port| port > 0) {
        return Err("Choose a loopback Ollama address with a port.".into());
    }
    Ok(())
}

impl Model {
    pub(crate) fn validate(&self) -> Result<()> {
        required(&self.id, 256)?;
        if !self.id.bytes().all(|c| {
            c.is_ascii_alphanumeric()
                || b"-_.:".contains(&c)
                || (matches!(
                    self.provider,
                    Provider::Ollama | Provider::OpenCode | Provider::Agent
                ) && c == b'/')
        }) {
            return Err("Invalid Robot model ID.".into());
        }
        match (&self.provider, &self.ollama_address) {
            (Provider::Ollama, Some(address)) => {
                loopback(address)?;
            }
            (_, None) if self.provider != Provider::Ollama => {}
            _ => return Err("Only Ollama uses a local model address.".into()),
        }
        if let Some(account) = &self.account {
            account.validate()?;
            if account.provider != self.provider || account.address != self.ollama_address {
                return Err("Robot model and provider account do not match.".into());
            }
        } else if matches!(
            self.provider,
            Provider::OpenCode
                | Provider::Antigravity
                | Provider::Copilot
                | Provider::Codex
                | Provider::Agent
        ) {
            return Err("Choose a saved provider account.".into());
        }
        Ok(())
    }
}

fn required(value: &str, limit: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > limit || value.contains('\0') {
        return Err("Missing, oversized or invalid Robot field.".into());
    }
    Ok(())
}

pub(crate) fn identifier(value: &str) -> Result<()> {
    required(value, 128)?;
    if !value
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    {
        return Err("Invalid Robot data ID.".into());
    }
    Ok(())
}

fn absolute_path(value: &str) -> Result<()> {
    required(value, 32_768)?;
    // Backups can move between Windows and Linux before their paths are reassigned.
    let windows = value.as_bytes().get(0..3).is_some_and(|p| {
        p[0].is_ascii_alphabetic() && p[1] == b':' && matches!(p[2], b'/' | b'\\')
    });
    if !value.starts_with('/') && !value.starts_with("\\\\") && !windows {
        return Err("Choose an absolute Robot project or Skill path.".into());
    }
    Ok(())
}

impl Role {
    fn validate(&self) -> Result<()> {
        identifier(&self.id)?;
        required(&self.name, 512)?;
        required(&self.instructions, 1024 * 1024)
    }
}

impl McpServer {
    pub(crate) fn validate(&self) -> Result<()> {
        identifier(&self.id)?;
        required(&self.command, 32_768)?;
        if self.arguments.len() > 256
            || self
                .arguments
                .iter()
                .any(|arg| arg.contains('\0') || arg.len() > 32_768)
        {
            return Err("Invalid MCP server arguments.".into());
        }
        for (name, reference) in &self.secret_env {
            if name.is_empty()
                || !name.bytes().enumerate().all(|(i, c)| {
                    c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                })
            {
                return Err("Invalid MCP environment variable name.".into());
            }
            identifier(reference)?;
        }
        let mut tools = HashSet::new();
        for name in &self.enabled_tools {
            required(name, 256)?;
            if !tools.insert(name) {
                return Err("Duplicate enabled MCP tool.".into());
            }
        }
        Ok(())
    }
}

impl Profile {
    fn validate(&self) -> Result<()> {
        identifier(&self.id)?;
        required(&self.name, 512)?;
        absolute_path(&self.project)?;
        self.model.validate()?;
        if let Some(id) = &self.role_id {
            identifier(id)?;
        }
        let mut skills = HashSet::new();
        for skill in &self.skills {
            absolute_path(skill)?;
            if skill.rsplit(['/', '\\']).next() != Some("SKILL.md") || !skills.insert(skill) {
                return Err("Choose distinct installed SKILL.md files.".into());
            }
        }
        let mut servers = HashSet::new();
        for server in &self.mcp_servers {
            server.validate()?;
            if !servers.insert(&server.id) {
                return Err("Duplicate MCP server ID.".into());
            }
        }
        Ok(())
    }
}

impl Entry {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Queued { id, text } => {
                identifier(id)?;
                required(text, 1024 * 1024)
            }
            Self::Run { id, .. } => identifier(id),
            Self::Command { text } | Self::Reply { text } | Self::Progress { text } => {
                required(text, 1024 * 1024)
            }
            Self::Tool { name, .. } => {
                required(name, 256)?;
                if serde_json::to_vec(self)?.len() > 2 * 1024 * 1024 {
                    return Err("Robot tool activity exceeds 2 MB.".into());
                }
                Ok(())
            }
        }
    }
}

impl Conversation {
    fn validate(&self) -> Result<()> {
        identifier(&self.id)?;
        identifier(&self.robot_id)?;
        required(&self.title, 32_768)?;
        absolute_path(&self.context.project)?;
        self.context.model.validate()?;
        if let Some(role) = &self.context.role {
            role.validate()?;
        }
        if self.created_at < 0
            || self.updated_at < self.created_at
            || self.rating.is_some_and(|r| !(1..=5).contains(&r))
        {
            return Err("Invalid conversation date or rating.".into());
        }
        if !matches!(
            self.entries.first(),
            Some(Entry::Command { .. } | Entry::Queued { .. })
        ) {
            return Err("A conversation must start with a command.".into());
        }
        let mut runs = BTreeMap::new();
        for entry in &self.entries {
            entry.validate()?;
            match entry {
                Entry::Queued { id, .. } => {
                    if runs.insert(id, None).is_some() {
                        return Err("Duplicate queued command.".into());
                    }
                }
                Entry::Run { id, status } => {
                    let previous = runs.get_mut(id).ok_or("Run has no queued command.")?;
                    if previous.is_some_and(|s| s != RunStatus::Running)
                        || (*previous == Some(RunStatus::Running) && *status == RunStatus::Running)
                        || (previous.is_none() && *status == RunStatus::Completed)
                    {
                        return Err("Invalid command state transition.".into());
                    }
                    *previous = Some(*status);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        let mut catalog = HashSet::new();
        for entry in &self.catalog {
            entry.validate()?;
            if !catalog.insert(&entry.id) {
                return Err("Duplicate collection entry ID.".into());
            }
        }
        let mut accounts = HashSet::new();
        for account in &self.accounts {
            account.validate()?;
            if !accounts.insert(&account.id) {
                return Err("Duplicate provider account ID.".into());
            }
        }
        let mut roles = HashSet::new();
        for role in &self.roles {
            role.validate()?;
            if !roles.insert(&role.id) {
                return Err("Duplicate Role ID.".into());
            }
        }
        let mut profiles = HashSet::new();
        for profile in &self.profiles {
            profile.validate()?;
            if profile.model.account.as_ref().is_some_and(|a| {
                !self
                    .accounts
                    .iter()
                    .any(|saved| saved.id == a.id && saved.provider == a.provider)
            }) {
                return Err("Missing Robot provider account.".into());
            }
            if !profiles.insert(&profile.id)
                || profile
                    .role_id
                    .as_ref()
                    .is_some_and(|id| !roles.contains(id))
            {
                return Err("Duplicate Robot ID or missing Role.".into());
            }
        }
        let mut conversations = HashSet::new();
        for conversation in &self.conversations {
            conversation.validate()?;
            if conversation
                .context
                .model
                .account
                .as_ref()
                .is_some_and(|a| {
                    !self
                        .accounts
                        .iter()
                        .any(|saved| saved.id == a.id && saved.provider == a.provider)
                })
            {
                return Err("Missing conversation provider account.".into());
            }
            if !conversations.insert(&conversation.id) || !profiles.contains(&conversation.robot_id)
            {
                return Err("Duplicate conversation ID or missing Robot.".into());
            }
        }
        Ok(())
    }

    pub fn disable_tools(&mut self) {
        for entry in &mut self.catalog {
            if entry.kind == "mcp"
                && let Ok(serde_json::Value::Object(mut server)) =
                    serde_json::from_str(&entry.content)
            {
                server.insert("enabled_tools".into(), serde_json::json!([]));
                entry.content = serde_json::Value::Object(server).to_string();
            }
        }
        for profile in &mut self.profiles {
            for server in &mut profile.mcp_servers {
                server.enabled_tools.clear();
            }
        }
    }
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let existing = path.try_exists()?;
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA secure_delete=ON;")?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 4 {
            return Err("This Robot database needs a newer version of Orbit.".into());
        }
        if version == 0 {
            if existing {
                return Err("Unrecognized Robot database; the original file was preserved.".into());
            }
            connection.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE roles(id TEXT PRIMARY KEY NOT NULL, data TEXT NOT NULL);
                CREATE TABLE profiles(id TEXT PRIMARY KEY NOT NULL, role_id TEXT REFERENCES roles(id), data TEXT NOT NULL);
                CREATE TABLE conversations(id TEXT PRIMARY KEY NOT NULL, robot_id TEXT NOT NULL REFERENCES profiles(id),
                    title TEXT NOT NULL, context TEXT NOT NULL, created_at INTEGER NOT NULL CHECK(created_at>=0),
                    updated_at INTEGER NOT NULL CHECK(updated_at>=created_at), rating INTEGER CHECK(rating BETWEEN 1 AND 5));
                CREATE INDEX conversation_history ON conversations(robot_id,updated_at DESC,id);
                CREATE TABLE entries(conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                    sequence INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(conversation_id,sequence));
                PRAGMA user_version=1; COMMIT;")?;
        }
        if version < 2 {
            connection.pragma_update(None, "user_version", 2)?;
        }
        if version < 3 {
            connection.execute_batch("BEGIN IMMEDIATE; CREATE TABLE accounts(id TEXT PRIMARY KEY NOT NULL, data TEXT NOT NULL); PRAGMA user_version=3; COMMIT;")?;
        }
        if version < 4 {
            connection.execute_batch("BEGIN IMMEDIATE; CREATE TABLE catalog(id TEXT PRIMARY KEY NOT NULL, data TEXT NOT NULL); PRAGMA user_version=4; COMMIT;")?;
        }
        Ok(Self { connection })
    }

    pub fn catalog(&self) -> Result<Vec<CatalogEntry>> {
        read_json_rows(
            &self.connection,
            "SELECT data FROM catalog ORDER BY json_extract(data,'$.name'),id",
        )
    }
    pub fn save_catalog(&self, entry: &CatalogEntry) -> Result<()> {
        entry.validate()?;
        self.connection.execute("INSERT INTO catalog(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data WHERE data<>excluded.data", params![entry.id, serde_json::to_string(entry)?])?;
        Ok(())
    }
    pub fn delete_catalog(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM catalog WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn delete_role(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM roles WHERE id=?1", [id])
            .map_err(|_| "This Role is assigned to a Robot. Remove that assignment first.")?;
        Ok(())
    }
    pub fn delete_profile(&mut self, id: &str) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM conversations WHERE robot_id=?1", [id])?;
        tx.execute("DELETE FROM profiles WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }
    pub fn common_commands(&self, robot: &str) -> Result<Vec<String>> {
        let mut query = self.connection.prepare("SELECT json_extract(e.data,'$.text') AS text FROM entries e JOIN conversations c ON c.id=e.conversation_id WHERE c.robot_id=?1 AND json_extract(e.data,'$.kind') IN ('Command','Queued') GROUP BY text ORDER BY COUNT(*) DESC,MAX(c.updated_at) DESC,text LIMIT 3")?;
        Ok(query
            .query_map([robot], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn accounts(&self) -> Result<Vec<Account>> {
        read_json_rows(
            &self.connection,
            "SELECT data FROM accounts ORDER BY json_extract(data,'$.name'),id",
        )
    }

    pub fn account(&self, id: &str) -> Result<Account> {
        read_account(&self.connection, id)
    }

    pub fn save_account(&self, account: &Account) -> Result<()> {
        account.validate()?;
        let previous: Option<String> = self
            .connection
            .query_row(
                "SELECT data FROM accounts WHERE id=?1",
                [&account.id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(previous) = previous {
            let previous: Account = serde_json::from_str(&previous)?;
            if previous.provider != account.provider {
                return Err("Create a new account to change its provider.".into());
            }
        }
        self.connection.execute("INSERT INTO accounts(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data WHERE data<>excluded.data", params![account.id, serde_json::to_string(account)?])?;
        Ok(())
    }

    pub fn check_account(&self, model: &mut Model) -> Result<()> {
        if let Some(account) = &mut model.account {
            let saved = self.account(&account.id)?;
            if account.id.starts_with("t3-")
                && (saved.home != account.home || saved.binding != account.binding)
            {
                return Err("This T3 Code connection changed. Select the provider again for a new conversation.".into());
            }
            if saved.provider != account.provider || saved.needs_reconnection {
                return Err(
                    "Reconnect this provider account in Robot settings before running work.".into(),
                );
            }
            // Reconnection authorizes the saved account, without retargeting its conversation snapshot.
            account.needs_reconnection = false;
        }
        Ok(())
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        let used: bool = self.connection.query_row("SELECT EXISTS(SELECT 1 FROM profiles WHERE json_extract(data,'$.model.account.id')=?1 UNION ALL SELECT 1 FROM conversations WHERE json_extract(context,'$.model.account.id')=?1)", [id], |r| r.get(0))?;
        if used {
            return Err("This account is used by a Robot or conversation. Keep it to preserve its account binding.".into());
        }
        self.connection
            .execute("DELETE FROM accounts WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn profiles(&self) -> Result<Vec<Profile>> {
        read_json_rows(
            &self.connection,
            "SELECT data FROM profiles ORDER BY json_extract(data,'$.name'),id",
        )
    }
    pub fn roles(&self) -> Result<Vec<Role>> {
        read_json_rows(
            &self.connection,
            "SELECT data FROM roles ORDER BY json_extract(data,'$.name'),id",
        )
    }
    pub fn save_role(&self, role: &Role) -> Result<()> {
        role.validate()?;
        self.connection.execute(
            "INSERT INTO roles(id,data) VALUES(?1,?2)
            ON CONFLICT(id) DO UPDATE SET data=excluded.data WHERE data<>excluded.data",
            params![role.id, serde_json::to_string(role)?],
        )?;
        Ok(())
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        profile.validate()?;
        if let Some(account) = &profile.model.account {
            let saved = self.account(&account.id)?;
            if saved.provider != account.provider {
                return Err("Provider account does not match the Robot.".into());
            }
        }
        self.connection.execute("INSERT INTO profiles(id,role_id,data) VALUES(?1,?2,?3)
            ON CONFLICT(id) DO UPDATE SET role_id=excluded.role_id,data=excluded.data WHERE data<>excluded.data",
            params![profile.id, profile.role_id, serde_json::to_string(profile)?])?;
        Ok(())
    }

    pub fn profile(&self, id: &str) -> Result<Profile> {
        let data: String =
            self.connection
                .query_row("SELECT data FROM profiles WHERE id=?1", [id], |r| r.get(0))?;
        let profile: Profile = serde_json::from_str(&data)?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn start(&mut self, id: &str, robot_id: &str, command: &str, now: i64) -> Result<()> {
        let tx = self.connection.transaction()?;
        create_conversation(
            &tx,
            id,
            robot_id,
            Entry::Command {
                text: command.into(),
            },
            now,
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn enqueue(
        &mut self,
        conversation: &str,
        robot: &str,
        run: &str,
        text: &str,
        now: i64,
    ) -> Result<()> {
        let entry = Entry::Queued {
            id: run.into(),
            text: text.into(),
        };
        entry.validate()?;
        identifier(conversation)?;
        identifier(robot)?;
        if now < 0 {
            return Err("Invalid conversation date.".into());
        }
        let tx = self.connection.transaction()?;
        let owner: Option<String> = tx
            .query_row(
                "SELECT robot_id FROM conversations WHERE id=?1",
                [conversation],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(owner) = owner {
            if owner != robot {
                return Err("Conversation belongs to another Robot.".into());
            }
            let duplicate: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM entries WHERE conversation_id=?1 AND json_extract(data,'$.kind')='Queued' AND json_extract(data,'$.id')=?2)", params![conversation,run], |r| r.get(0))?;
            if duplicate {
                return Err("Command already queued.".into());
            }
            append_entry(&tx, conversation, &entry, now)?;
        } else {
            create_conversation(&tx, conversation, robot, entry, now)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn interrupt_runs(&mut self, now: i64) -> Result<()> {
        let tx = self.connection.transaction()?;
        let pending = tx.prepare("SELECT q.conversation_id,json_extract(q.data,'$.id') FROM entries q
            WHERE json_extract(q.data,'$.kind')='Queued' AND NOT EXISTS(
                SELECT 1 FROM entries r WHERE r.conversation_id=q.conversation_id
                AND json_extract(r.data,'$.kind')='Run' AND json_extract(r.data,'$.id')=json_extract(q.data,'$.id')
                AND json_extract(r.data,'$.status')<>'Running')")?
            .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for (conversation, id) in pending {
            append_entry(
                &tx,
                &conversation,
                &Entry::Run {
                    id,
                    status: RunStatus::Interrupted,
                },
                now,
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn append(&mut self, id: &str, entry: &Entry, now: i64) -> Result<()> {
        entry.validate()?;
        if now < 0 {
            return Err("Invalid conversation date.".into());
        }
        let tx = self.connection.transaction()?;
        append_entry(&tx, id, entry, now)?;
        tx.commit()?;
        Ok(())
    }

    pub fn rate(&self, id: &str, rating: u8) -> Result<()> {
        if !(1..=5).contains(&rating) {
            return Err("Choose a rating from one to five stars.".into());
        }
        if self.connection.execute(
            "UPDATE conversations SET rating=?2 WHERE id=?1",
            params![id, rating],
        )? != 1
        {
            return Err("Conversation no longer exists.".into());
        }
        Ok(())
    }

    pub fn history(&self, robot_id: &str, offset: u32) -> Result<Vec<History>> {
        let mut query = self.connection.prepare(
            "SELECT id,title,updated_at,rating FROM conversations
            WHERE robot_id=?1 ORDER BY updated_at DESC,id LIMIT 50 OFFSET ?2",
        )?;
        Ok(query
            .query_map(params![robot_id, offset], |r| {
                Ok(History {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    updated_at: r.get(2)?,
                    rating: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn conversation(&self, id: &str) -> Result<Option<Conversation>> {
        read_conversation(&self.connection, id)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM conversations WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn snapshot(&mut self) -> Result<Snapshot> {
        let tx = self.connection.transaction()?;
        let catalog = read_json_rows(&tx, "SELECT data FROM catalog ORDER BY id")?;
        let accounts = read_json_rows(&tx, "SELECT data FROM accounts ORDER BY id")?;
        let roles = read_json_rows(&tx, "SELECT data FROM roles ORDER BY id")?;
        let profiles = read_json_rows(&tx, "SELECT data FROM profiles ORDER BY id")?;
        let ids = tx
            .prepare("SELECT id FROM conversations ORDER BY id")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let conversations = ids
            .iter()
            .map(|id| {
                read_conversation(&tx, id)?.ok_or_else(|| "Conversation no longer exists.".into())
            })
            .collect::<Result<Vec<_>>>()?;
        tx.commit()?;
        let snapshot = Snapshot {
            catalog,
            accounts,
            roles,
            profiles,
            conversations,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn restore(&mut self, snapshot: &Snapshot) -> Result<()> {
        snapshot.validate()?;
        let tx = self.connection.transaction()?;
        tx.execute_batch("DELETE FROM conversations; DELETE FROM profiles; DELETE FROM roles; DELETE FROM accounts; DELETE FROM catalog;")?;
        for entry in &snapshot.catalog {
            let mut entry = entry.clone();
            if entry.kind == "mcp" {
                let mut server: McpServer = serde_json::from_str(&entry.content)?;
                server.enabled_tools.clear();
                entry.content = serde_json::to_string(&server)?;
            }
            tx.execute(
                "INSERT INTO catalog(id,data) VALUES(?1,?2)",
                params![entry.id, serde_json::to_string(&entry)?],
            )?;
        }
        for account in &snapshot.accounts {
            let mut account = account.clone();
            account.needs_reconnection = true;
            tx.execute(
                "INSERT INTO accounts(id,data) VALUES(?1,?2)",
                params![account.id, serde_json::to_string(&account)?],
            )?;
        }
        for role in &snapshot.roles {
            tx.execute(
                "INSERT INTO roles(id,data) VALUES(?1,?2)",
                params![role.id, serde_json::to_string(role)?],
            )?;
        }
        for profile in &snapshot.profiles {
            let mut profile = profile.clone();
            if let Some(account) = &mut profile.model.account {
                account.needs_reconnection = true;
            }
            for server in &mut profile.mcp_servers {
                server.enabled_tools.clear();
            }
            tx.execute(
                "INSERT INTO profiles(id,role_id,data) VALUES(?1,?2,?3)",
                params![
                    profile.id,
                    profile.role_id,
                    serde_json::to_string(&profile)?
                ],
            )?;
        }
        for conversation in &snapshot.conversations {
            let mut conversation = conversation.clone();
            if let Some(account) = &mut conversation.context.model.account {
                account.needs_reconnection = true;
            }
            insert_conversation(&tx, &conversation)?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn read_account(db: &Connection, id: &str) -> Result<Account> {
    let data: String = db
        .query_row("SELECT data FROM accounts WHERE id=?1", [id], |r| r.get(0))
        .optional()?
        .ok_or("Provider account is missing. Choose an account in Robot settings.")?;
    let account: Account = serde_json::from_str(&data)?;
    account.validate()?;
    Ok(account)
}

fn title(command: &str) -> String {
    command
        .trim()
        .lines()
        .next()
        .unwrap_or_default()
        .graphemes(true)
        .take(80)
        .collect()
}

fn create_conversation(
    db: &Connection,
    id: &str,
    robot: &str,
    first: Entry,
    now: i64,
) -> Result<()> {
    let data: String = db.query_row("SELECT data FROM profiles WHERE id=?1", [robot], |r| {
        r.get(0)
    })?;
    let mut profile: Profile = serde_json::from_str(&data)?;
    if let Some(account) = &profile.model.account {
        let account = read_account(db, &account.id)?;
        if account.provider != profile.model.provider {
            return Err("Provider account does not match the Robot.".into());
        }
        profile.model.ollama_address = account.address.clone();
        profile.model.account = Some(account);
    }
    let role = profile
        .role_id
        .as_ref()
        .map(|id| -> Result<Role> {
            let data: String =
                db.query_row("SELECT data FROM roles WHERE id=?1", [id], |r| r.get(0))?;
            Ok(serde_json::from_str(&data)?)
        })
        .transpose()?;
    let title = match &first {
        Entry::Command { text } | Entry::Queued { text, .. } => title(text),
        _ => return Err("Missing first command.".into()),
    };
    let conversation = Conversation {
        id: id.into(),
        robot_id: robot.into(),
        title,
        context: Context {
            model: profile.model,
            role,
            project: profile.project,
        },
        created_at: now,
        updated_at: now,
        rating: None,
        entries: vec![first],
    };
    conversation.validate()?;
    insert_conversation(db, &conversation)
}

fn append_entry(db: &Connection, id: &str, entry: &Entry, now: i64) -> Result<()> {
    if db.execute(
        "UPDATE conversations SET updated_at=max(updated_at,?2) WHERE id=?1",
        params![id, now],
    )? != 1
    {
        return Err("Conversation no longer exists.".into());
    }
    db.execute(
        "INSERT INTO entries(conversation_id,sequence,data)
        SELECT ?1,coalesce(max(sequence)+1,0),?2 FROM entries WHERE conversation_id=?1",
        params![id, serde_json::to_string(entry)?],
    )?;
    Ok(())
}

fn insert_conversation(db: &Connection, conversation: &Conversation) -> Result<()> {
    db.execute(
        "INSERT INTO conversations(id,robot_id,title,context,created_at,updated_at,rating)
        VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            conversation.id,
            conversation.robot_id,
            conversation.title,
            serde_json::to_string(&conversation.context)?,
            conversation.created_at,
            conversation.updated_at,
            conversation.rating
        ],
    )?;
    let mut insert =
        db.prepare("INSERT INTO entries(conversation_id,sequence,data) VALUES(?1,?2,?3)")?;
    for (sequence, entry) in conversation.entries.iter().enumerate() {
        insert.execute(params![
            conversation.id,
            sequence as i64,
            serde_json::to_string(entry)?
        ])?;
    }
    Ok(())
}

fn read_json_rows<T: serde::de::DeserializeOwned>(db: &Connection, sql: &str) -> Result<Vec<T>> {
    let mut query = db.prepare(sql)?;
    query
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|row| Ok(serde_json::from_str(&row?)?))
        .collect()
}

fn read_conversation(db: &Connection, id: &str) -> Result<Option<Conversation>> {
    let record = db.query_row("SELECT robot_id,title,context,created_at,updated_at,rating FROM conversations WHERE id=?1", [id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, Option<u8>>(5)?))
    }).optional()?;
    let Some((robot_id, title, context, created_at, updated_at, rating)) = record else {
        return Ok(None);
    };
    let mut query =
        db.prepare("SELECT data FROM entries WHERE conversation_id=?1 ORDER BY sequence")?;
    let entries = query
        .query_map([id], |r| r.get::<_, String>(0))?
        .map(|row| Ok(serde_json::from_str(&row?)?))
        .collect::<Result<_>>()?;
    Ok(Some(Conversation {
        id: id.into(),
        robot_id,
        title,
        context: serde_json::from_str(&context)?,
        created_at,
        updated_at,
        rating,
        entries,
    }))
}

pub fn export(path: &Path) -> Result<Snapshot> {
    if !path.try_exists()? {
        return Ok(Snapshot::default());
    }
    Database::open(path)?.snapshot()
}

#[cfg(test)]
#[path = "robot-data/tests.rs"]
mod tests;
