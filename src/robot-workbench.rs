use crate::robot_data::{Account, Database, Entry, McpServer, Model, Profile, Provider, Role};
use crate::robot_runtime::{self, Event, Handle};
use crate::{Launcher, RobotActivity, RobotChoice, RobotProvider, RobotResource, RobotWorkbench};
use slint::{ComponentHandle, Model as _, ModelRc, VecModel};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

type Result<T> = std::result::Result<T, String>;
static IMPORTING: AtomicBool = AtomicBool::new(false);
static WORKER: OnceLock<mpsc::Sender<Work>> = OnceLock::new();
pub struct ImportGuard;
impl Drop for ImportGuard {
    fn drop(&mut self) {
        IMPORTING.store(false, Ordering::Release);
    }
}
pub fn before_import() -> Result<ImportGuard> {
    IMPORTING.store(true, Ordering::Release);
    let guard = ImportGuard;
    if let Some(sender) = WORKER.get() {
        let (reply, receiver) = mpsc::sync_channel(1);
        sender
            .send(Work::Suspend(reply))
            .map_err(|_| "Robot worker is unavailable.")?;
        receiver
            .recv()
            .map_err(|_| "Robot worker is unavailable.")??;
    }
    Ok(guard)
}

enum Work {
    Skills,
    Resource(crate::robot_data::CatalogEntry),
    Remove(String, String),
    Common(String),
    Action(String, String),
    Duplicate(String),
    Providers,
    Open(String, bool),
    Save(Box<Profile>, bool, String),
    Account(String),
    SaveAccount(Account),
    Role(String),
    SaveRole(Role),
    Server(String),
    SaveServer(McpServer),
    Discover(McpServer, String),
    Credential(String, String),
    Conversation(String),
    History(i32),
    Submit(String, String, String),
    Cancel(String),
    Approve(String, u64, bool),
    Rate(String, i32),
    Delete(String),
    Suspend(mpsc::SyncSender<Result<()>>),
}
struct Worker {
    weak: slint::Weak<Launcher>,
    path: PathBuf,
    profile: Option<Profile>,
    conversation: String,
    page: u32,
    servers: Vec<McpServer>,
    runtime: Option<(Handle, mpsc::Receiver<Event>)>,
    jobs: HashMap<String, String>,
    approvals: HashMap<String, crate::robot_tools::Approval>,
    models: HashMap<String, Vec<String>>,
}
fn id() -> String {
    format!(
        "robot-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}
fn choices(values: Vec<(String, String)>) -> ModelRc<RobotChoice> {
    ModelRc::new(VecModel::from(
        values
            .into_iter()
            .map(|(id, name)| RobotChoice {
                id: id.into(),
                name: name.into(),
            })
            .collect::<Vec<_>>(),
    ))
}
fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
fn resources_of_kind(resources: &[RobotResource], kinds: &[&str]) -> ModelRc<RobotResource> {
    ModelRc::new(VecModel::from(
        resources
            .iter()
            .filter(|r| kinds.contains(&r.kind.as_str()))
            .cloned()
            .collect::<Vec<_>>(),
    ))
}
fn server(ui: &RobotWorkbench<'_>) -> Result<McpServer> {
    let mut secret_env = BTreeMap::new();
    for line in lines(ui.get_server_environment().as_str()) {
        let (key, value) = line
            .split_once('=')
            .ok_or("Use NAME=credential-reference for secret environment fields.")?;
        if secret_env
            .insert(key.trim().into(), value.trim().into())
            .is_some()
        {
            return Err("Duplicate secret environment name.".into());
        }
    }
    let server = McpServer {
        id: ui.get_server_id().into(),
        command: ui.get_server_command().into(),
        arguments: ui
            .get_server_arguments()
            .lines()
            .map(str::to_owned)
            .collect(),
        secret_env,
        enabled_tools: lines(ui.get_server_tools().as_str()),
    };
    server.validate().map_err(|e| e.to_string())?;
    Ok(server)
}

pub fn connect(ui: &Launcher) {
    let (sender, receiver) = mpsc::channel();
    let _ = WORKER.set(sender.clone());
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let path = match crate::storage::config_path() {
            Ok(p) => p.with_file_name("robots.db"),
            Err(_) => return,
        };
        let mut worker = Worker {
            weak,
            path,
            profile: None,
            conversation: String::new(),
            page: 0,
            servers: vec![],
            runtime: None,
            jobs: HashMap::new(),
            approvals: HashMap::new(),
            models: HashMap::new(),
        };
        loop {
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(Work::Suspend(reply)) => {
                    let result = if worker.jobs.is_empty() {
                        worker.stop();
                        Ok(())
                    } else {
                        Err("Cancel Robot work and wait for it to finish before importing.".into())
                    };
                    let _ = reply.send(result);
                }
                Ok(work) => {
                    let result = if IMPORTING.load(Ordering::Acquire) {
                        Err("Wait for the data import to finish.".into())
                    } else {
                        worker.apply(work)
                    };
                    if let Err(error) = result {
                        crate::toast::error(&worker.weak, &error);
                        worker.update(move |g| {
                            g.set_error(error.into());
                            g.set_busy(false);
                        });
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
            let events: Vec<_> = worker
                .runtime
                .as_ref()
                .map(|(_, events)| events.try_iter().collect())
                .unwrap_or_default();
            for event in events {
                worker.event(event);
            }
        }
    });
    let g = ui.global::<RobotWorkbench>();
    let weak = ui.as_weak();
    g.on_add_to_canvas(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            if !g.get_profile_id().is_empty() {
                ui.invoke_add_robot(g.get_profile_id(), g.get_name());
            }
        }
    });
    let send = sender.clone();
    g.on_refresh_skills(move || {
        let _ = send.send(Work::Skills);
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_context_actions(move |profile| {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            g.set_common_profile(profile.clone());
            g.set_common_actions(ModelRc::new(VecModel::from(Vec::<RobotChoice>::new())));
        }
        let _ = send.send(Work::Common(profile.into()));
    });
    let send = sender.clone();
    g.on_run_action(move |profile, text| {
        let _ = send.send(Work::Action(profile.into(), text.into()));
    });
    let send = sender.clone();
    g.on_duplicate_robot(move |profile| {
        let _ = send.send(Work::Duplicate(profile.into()));
    });
    let send = sender.clone();
    g.on_remove_resource(move |kind, id| {
        let _ = send.send(Work::Remove(kind.into(), id.into()));
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_save_resource(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            let kind = match g.get_section() {
                1 => "skill",
                3 => {
                    if g.get_shell_action() {
                        "shell"
                    } else {
                        "prompt"
                    }
                }
                6 => "project",
                _ => return,
            };
            let _ = send.send(Work::Resource(crate::robot_data::CatalogEntry {
                id: if g.get_resource_id().is_empty() {
                    id()
                } else {
                    g.get_resource_id().into()
                },
                kind: kind.into(),
                name: g.get_resource_name().into(),
                content: g.get_resource_content().into(),
            }));
        }
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_load_providers(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            g.set_providers_loading(true);
            g.set_providers_error("".into());
        }
        let _ = send.send(Work::Providers);
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_open(move |profile, editing| {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            g.set_visible(true);
            g.set_busy(true);
            g.set_error("".into());
            g.set_status("".into());
            if editing {
                g.set_section(0);
            }
        }
        let _ = send.send(Work::Open(profile.into(), editing));
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_save_profile(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            let Some(provider) = Provider::from_index(g.get_provider()) else {
                return;
            };
            let profile = Profile {
                id: if g.get_profile_id().is_empty() {
                    id()
                } else {
                    g.get_profile_id().into()
                },
                name: g.get_name().into(),
                project: g.get_project().into(),
                model: Model {
                    account: None,
                    ollama_address: (provider == Provider::Ollama).then(|| g.get_address().into()),
                    provider,
                    id: g.get_model().into(),
                },
                role_id: (!g.get_role_id().is_empty()).then(|| g.get_role_id().into()),
                skills: lines(g.get_skills().as_str()),
                mcp_servers: vec![],
            };
            g.set_busy(true);
            let _ = send.send(Work::Save(
                Box::new(profile),
                g.get_adding(),
                g.get_account_id().into(),
            ));
        }
    });
    let send = sender.clone();
    g.on_select_account(move |id| {
        let _ = send.send(Work::Account(id.into()));
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_save_account(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            let Some(provider) = Provider::from_index(g.get_provider()) else {
                return;
            };
            let account = Account {
                id: if g.get_account_id().is_empty() {
                    id()
                } else {
                    g.get_account_id().into()
                },
                name: g.get_account_name().into(),
                address: (provider == Provider::Ollama).then(|| g.get_address().into()),
                credential: matches!(provider, Provider::OpenAi | Provider::Gemini)
                    .then(|| g.get_account_credential().to_string())
                    .filter(|s| !s.is_empty()),
                home: (provider.index() >= 3)
                    .then(|| g.get_account_home().to_string())
                    .filter(|s| !s.is_empty()),
                provider,
                needs_reconnection: false,
                binding: None,
            };
            let _ = send.send(Work::SaveAccount(account));
        }
    });
    let send = sender.clone();
    g.on_select_role(move |id| {
        let _ = send.send(Work::Role(id.into()));
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_save_role(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            let _ = send.send(Work::SaveRole(Role {
                id: if g.get_role_id().is_empty() {
                    id()
                } else {
                    g.get_role_id().into()
                },
                name: g.get_role_name().into(),
                instructions: g.get_instructions().into(),
            }));
        }
    });
    let send = sender.clone();
    g.on_select_server(move |id| {
        let _ = send.send(Work::Server(id.into()));
    });
    for discover in [false, true] {
        let send = sender.clone();
        let weak = ui.as_weak();
        let callback = move || {
            if let Some(ui) = weak.upgrade() {
                let g = ui.global::<RobotWorkbench>();
                match server(&g) {
                    Ok(server) => {
                        g.set_busy(true);
                        let _ = send.send(if discover {
                            Work::Discover(server, g.get_project().into())
                        } else {
                            Work::SaveServer(server)
                        });
                    }
                    Err(error) => g.set_error(error.into()),
                }
            }
        };
        if discover {
            g.on_discover(callback);
        } else {
            g.on_save_server(callback);
        }
    }
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_save_credential(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            let _ = send.send(Work::Credential(
                g.get_credential_name().into(),
                g.get_credential_value().into(),
            ));
            g.set_credential_value("".into());
        }
    });
    let send = sender.clone();
    g.on_select_conversation(move |id| {
        let _ = send.send(Work::Conversation(id.into()));
    });
    let send = sender.clone();
    g.on_load_history(move |page| {
        let _ = send.send(Work::History(page));
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_submit(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            if g.get_conversation().is_empty() {
                g.set_conversation(id().into());
            }
            let _ = send.send(Work::Submit(
                g.get_profile_id().into(),
                g.get_conversation().into(),
                g.get_command().into(),
            ));
        }
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_cancel(move || {
        if let Some(ui) = weak.upgrade() {
            let _ = send.send(Work::Cancel(
                ui.global::<RobotWorkbench>().get_conversation().into(),
            ));
        }
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_approve(move |approved| {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            if let Ok(id) = g.get_approval_id().parse() {
                let _ = send.send(Work::Approve(g.get_conversation().into(), id, approved));
                g.set_approval("".into());
                g.set_approval_id("".into());
            }
        }
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_rate(move |rating| {
        if let Some(ui) = weak.upgrade() {
            let _ = send.send(Work::Rate(
                ui.global::<RobotWorkbench>().get_conversation().into(),
                rating,
            ));
        }
    });
    let weak = ui.as_weak();
    g.on_delete_conversation(move || {
        if let Some(ui) = weak.upgrade() {
            let _ = sender.send(Work::Delete(
                ui.global::<RobotWorkbench>().get_conversation().into(),
            ));
        }
    });
}

impl Worker {
    fn providers(&mut self) -> Result<()> {
        let instances = crate::t3_providers::sync(&self.db()?, &crate::t3_providers::directory()?)?;
        self.models = instances
            .iter()
            .filter(|i| i.enabled)
            .map(|i| (i.account.id.clone(), i.models.clone()))
            .collect();
        self.update(move |g| {
            g.set_providers(ModelRc::new(VecModel::from(
                instances
                    .into_iter()
                    .map(|i| RobotProvider {
                        id: i.account.id.into(),
                        name: i.account.name.into(),
                        driver: match i.driver.as_str() {
                            "pi" => "Pi",
                            "acpRegistry" => "Native agent",
                            _ => i.account.provider.name(),
                        }
                        .into(),
                        status: format!("T3 Code: {}", i.status).into(),
                        models: format!(
                            "{} {}{}",
                            i.models.len(),
                            if i.models.len() == 1 {
                                "model"
                            } else {
                                "models"
                            },
                            if i.models.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    " · {}{}",
                                    i.models
                                        .iter()
                                        .take(3)
                                        .cloned()
                                        .collect::<Vec<_>>()
                                        .join(" · "),
                                    if i.models.len() > 3 { " …" } else { "" }
                                )
                            }
                        )
                        .into(),
                    })
                    .collect::<Vec<_>>(),
            )));
            g.set_providers_loading(false);
            g.set_status("Providers fetched from T3 Code.".into());
        });
        self.lists()
    }
    fn update(&self, callback: impl FnOnce(RobotWorkbench<'_>) + Send + 'static) {
        let _ = self
            .weak
            .upgrade_in_event_loop(move |ui| callback(ui.global::<RobotWorkbench>()));
    }
    fn db(&self) -> Result<Database> {
        Database::open(&self.path).map_err(|e| e.to_string())
    }
    fn stop(&mut self) {
        if let Some((handle, _)) = self.runtime.take() {
            handle.shutdown();
        }
    }
    fn lists(&self) -> Result<()> {
        let db = self.db()?;
        let mut resources = db.catalog().map_err(|e| e.to_string())?;
        let profiles = db.profiles().map_err(|e| e.to_string())?;
        let roles = db.roles().map_err(|e| e.to_string())?;
        let accounts: Vec<_> = db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        for reference in accounts
            .iter()
            .filter_map(|a| a.credential.as_ref())
            .chain(self.servers.iter().flat_map(|s| s.secret_env.values()))
        {
            if !resources
                .iter()
                .any(|r| r.kind == "credential" && &r.name == reference)
            {
                resources.push(crate::robot_data::CatalogEntry {
                    id: format!("credential-{reference}"),
                    kind: "credential".into(),
                    name: reference.clone(),
                    content: String::new(),
                });
            }
        }
        self.update(move |g| {
            let resources: Vec<_> = resources
                .into_iter()
                .map(|r| RobotResource {
                    id: r.id.into(),
                    kind: r.kind.into(),
                    name: r.name.into(),
                    content: r.content.into(),
                })
                .collect();
            g.set_actions(resources_of_kind(&resources, &["prompt", "shell"]));
            g.set_custom_skills(resources_of_kind(&resources, &["skill"]));
            g.set_projects(resources_of_kind(&resources, &["project"]));
            g.set_mcps(resources_of_kind(&resources, &["mcp"]));
            g.set_credentials(resources_of_kind(&resources, &["credential"]));
            g.set_profiles(choices(
                profiles.into_iter().map(|p| (p.id, p.name)).collect(),
            ));
            g.set_roles(choices(roles.into_iter().map(|r| (r.id, r.name)).collect()));
            let mut names = vec!["Choose provider".into()];
            names.extend(
                accounts
                    .iter()
                    .map(|a| format!("{} · {}", a.provider.name(), a.name).into()),
            );
            g.set_account_index(
                accounts
                    .iter()
                    .position(|a| a.id == g.get_account_id().as_str())
                    .map_or(0, |i| i as i32 + 1),
            );
            g.set_account_names(ModelRc::new(VecModel::from(names)));
            g.set_accounts(choices(
                accounts
                    .into_iter()
                    .map(|a| (a.id, format!("{} · {}", a.provider.name(), a.name)))
                    .collect(),
            ));
        });
        Ok(())
    }
    fn show_profile(&mut self, profile: Profile, editing: bool) -> Result<()> {
        let models = profile
            .model
            .account
            .as_ref()
            .and_then(|a| self.models.get(&a.id))
            .cloned()
            .unwrap_or_default();
        self.servers = profile.mcp_servers.clone();
        self.profile = Some(profile.clone());
        self.conversation.clear();
        self.page = 0;
        let role = self
            .db()?
            .roles()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|r| Some(&r.id) == profile.role_id.as_ref());
        self.update(move |g| {
            g.set_provider_model_index(
                models
                    .iter()
                    .position(|id| *id == profile.model.id)
                    .unwrap_or(0) as i32,
            );
            g.set_provider_models(ModelRc::new(VecModel::from(
                models.into_iter().map(Into::into).collect::<Vec<_>>(),
            )));
            g.set_profile_id(profile.id.into());
            g.set_name(profile.name.into());
            g.set_project(profile.project.into());
            g.set_provider(profile.model.provider.index());
            show_account(&g, profile.model.account.as_ref());
            g.set_model(profile.model.id.into());
            g.set_address(
                profile
                    .model
                    .ollama_address
                    .unwrap_or_else(|| "http://127.0.0.1:11434".into())
                    .into(),
            );
            g.set_role_id(
                role.as_ref()
                    .map(|r| r.id.clone())
                    .unwrap_or_default()
                    .into(),
            );
            g.set_role_name(
                role.as_ref()
                    .map(|r| r.name.clone())
                    .unwrap_or_default()
                    .into(),
            );
            g.set_instructions(role.map(|r| r.instructions).unwrap_or_default().into());
            g.set_skills(profile.skills.join("\n").into());
            g.set_servers(choices(
                profile
                    .mcp_servers
                    .iter()
                    .map(|s| (s.id.clone(), s.id.clone()))
                    .collect(),
            ));
            g.set_editing(editing);
            g.set_busy(false);
            g.set_error("".into());
        });
        self.refresh()
    }
    fn apply(&mut self, work: Work) -> Result<()> {
        self.update(|g| g.set_error("".into()));
        match work {
            Work::Skills => {
                let home = std::env::var_os("HOME").ok_or("Home directory is unavailable.")?;
                let root = PathBuf::from(home).join(".agents/skills");
                let mut skills = Vec::new();
                if root.try_exists().map_err(|e| e.to_string())? {
                    for entry in std::fs::read_dir(&root).map_err(|e| e.to_string())? {
                        let entry = entry.map_err(|e| e.to_string())?;
                        let path = entry.path().join("SKILL.md");
                        if path.is_file() {
                            skills.push((
                                path.to_string_lossy().into_owned(),
                                entry.file_name().to_string_lossy().into_owned(),
                            ));
                        }
                    }
                }
                skills.sort_by(|a, b| a.1.cmp(&b.1));
                self.update(move |g| {
                    g.set_installed_skills(choices(skills));
                    g.set_status("Skills refreshed from ~/.agents/skills.".into());
                });
            }
            Work::Resource(entry) => {
                if entry.kind == "skill"
                    && (!std::path::Path::new(&entry.content).is_absolute()
                        || !std::path::Path::new(&entry.content).is_file())
                {
                    return Err("Choose an existing absolute SKILL.md path.".into());
                }
                if entry.kind == "project"
                    && (!std::path::Path::new(&entry.content).is_absolute()
                        || !std::path::Path::new(&entry.content).is_dir())
                {
                    return Err("Choose an existing absolute project directory.".into());
                }
                self.db()?.save_catalog(&entry).map_err(|e| e.to_string())?;
                self.lists()?;
                self.update(|g| {
                    g.set_resource_id("".into());
                    g.set_resource_name("".into());
                    g.set_resource_content("".into());
                    g.set_status("Saved.".into());
                });
            }
            Work::Common(profile) => {
                let commands = self
                    .db()?
                    .common_commands(&profile)
                    .map_err(|e| e.to_string())?;
                let catalog = self.db()?.catalog().map_err(|e| e.to_string())?;
                let commands = commands.into_iter().map(|text| {
                    let name = catalog.iter().find(|entry| {
                        (entry.kind == "prompt" && entry.content == text) || (entry.kind == "shell" && format!("Run this exact shell command in the project directory using your shell tool. Request permission before execution.\n\n{}", entry.content) == text)
                    }).map(|entry| entry.name.clone()).unwrap_or_else(|| text.lines().next().unwrap_or_default().chars().take(60).collect());
                    (text, name)
                }).collect();
                self.update(move |g| {
                    if g.get_common_profile().as_str() == profile {
                        g.set_common_actions(choices(commands));
                    }
                });
            }
            Work::Action(profile, text) => {
                let profile = self.db()?.profile(&profile).map_err(|e| e.to_string())?;
                let profile_id = profile.id.clone();
                if self.profile.as_ref().is_none_or(|p| p.id != profile.id) {
                    self.show_profile(profile, false)?;
                }
                self.update(|g| {
                    g.set_visible(true);
                    g.set_editing(false);
                });
                let conversation = if self.conversation.is_empty() {
                    id()
                } else {
                    self.conversation.clone()
                };
                self.apply(Work::Submit(profile_id, conversation, text))?;
            }
            Work::Duplicate(profile) => {
                let mut profile = self.db()?.profile(&profile).map_err(|e| e.to_string())?;
                profile.id = id();
                profile.name = format!("{} copy", profile.name);
                self.db()?
                    .save_profile(&profile)
                    .map_err(|e| e.to_string())?;
                let _ = self.weak.upgrade_in_event_loop(move |ui| {
                    ui.invoke_add_robot(profile.id.into(), profile.name.into())
                });
                self.lists()?;
            }
            Work::Remove(kind, id) => {
                match kind.as_str() {
                    "account" => self.db()?.delete_account(&id).map_err(|e| e.to_string())?,
                    "role" => self.db()?.delete_role(&id).map_err(|e| e.to_string())?,
                    "robot" => {
                        if !self.jobs.is_empty() {
                            return Err(
                                "Wait for Robot work to finish before deleting a Robot.".into()
                            );
                        }
                        // Canvas references must be removed before the profile and its history.
                        let config = crate::config::load().0;
                        if config.items.iter().any(|item| matches!(item, crate::config::Item::Robot { profile, .. } if profile == &id)) {
                            return Err("Delete this Robot's Canvas items first, then remove its profile and history.".into());
                        }
                        self.db()?.delete_profile(&id).map_err(|e| e.to_string())?;
                    }
                    "server" => {
                        self.servers.retain(|s| s.id != id);
                        self.db()?
                            .delete_catalog(&format!("mcp-{id}"))
                            .map_err(|e| e.to_string())?;
                        let servers = self
                            .servers
                            .iter()
                            .map(|s| (s.id.clone(), s.id.clone()))
                            .collect();
                        self.update(move |g| {
                            g.set_servers(choices(servers));
                            g.set_server_id("".into());
                        });
                    }
                    "credential" => {
                        keyring::Entry::new("orbit-robots", &id)
                            .and_then(|e| e.delete_credential())
                            .map_err(|_| "Couldn't remove the credential from the system store.")?;
                        for entry in self.db()?.catalog().map_err(|e| e.to_string())? {
                            if entry.kind == "credential" && entry.name == id {
                                self.db()?
                                    .delete_catalog(&entry.id)
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    _ => self.db()?.delete_catalog(&id).map_err(|e| e.to_string())?,
                }
                self.lists()?;
                self.update(move |g| {
                    if kind == "account" && g.get_account_id().as_str() == id {
                        show_account(&g, None);
                    }
                    if kind == "role" && g.get_role_id().as_str() == id {
                        g.set_role_id("".into());
                        g.set_role_name("".into());
                        g.set_instructions("".into());
                    }
                    if g.get_resource_id().as_str() == id {
                        g.set_resource_id("".into());
                        g.set_resource_name("".into());
                        g.set_resource_content("".into());
                    }
                    g.set_status("Removed.".into());
                    if kind == "robot" && g.get_profile_id().as_str() == id {
                        g.invoke_open("".into(), true);
                    }
                });
            }
            Work::Providers => {
                if let Err(error) = self.providers() {
                    self.update(move |g| {
                        g.set_providers_loading(false);
                        g.set_providers_error(error.into());
                    });
                }
            }
            Work::Open(id, editing) => {
                self.lists()?;
                let profile = if id.is_empty() {
                    let ai = crate::config::load().0.ai;
                    Profile {
                        id: String::new(),
                        name: String::new(),
                        project: String::new(),
                        model: Model {
                            account: None,
                            provider: match ai.provider {
                                1 => Provider::OpenAi,
                                2 => Provider::Gemini,
                                _ => Provider::Ollama,
                            },
                            id: match ai.provider {
                                1 => ai.openai_model,
                                2 => ai.gemini_model,
                                _ => ai.local_model,
                            },
                            ollama_address: Some(ai.address),
                        },
                        role_id: None,
                        skills: vec![],
                        mcp_servers: vec![],
                    }
                } else {
                    self.db()?.profile(&id).map_err(|e| e.to_string())?
                };
                self.show_profile(profile, editing)?;
            }
            Work::Save(profile, adding, account_id) => {
                let mut profile = *profile;
                if !account_id.is_empty() {
                    let account = self.db()?.account(&account_id).map_err(|e| e.to_string())?;
                    if account.provider != profile.model.provider {
                        return Err("Choose an account for the selected provider.".into());
                    }
                    profile.model.ollama_address = account.address.clone();
                    profile.model.account = Some(account);
                }
                if !std::path::Path::new(&profile.project).is_dir() {
                    return Err("Choose an existing project directory.".into());
                }
                for skill in &profile.skills {
                    if !std::path::Path::new(skill).is_file() {
                        return Err(format!("Skill is missing: {skill}"));
                    }
                }
                profile.mcp_servers = self.servers.clone();
                self.db()?
                    .save_profile(&profile)
                    .map_err(|e| e.to_string())?;
                let saved = profile.clone();
                let renamed = profile.clone();
                let _ = self.weak.upgrade_in_event_loop(move |ui| {
                    ui.invoke_rename_robot(renamed.id.into(), renamed.name.into())
                });
                self.show_profile(profile, true)?;
                self.lists()?;
                self.update(|g| {
                    g.set_adding(false);
                    g.set_status("Profile saved. New conversations use these settings.".into());
                });
                if adding {
                    let _ = self.weak.upgrade_in_event_loop(move |ui| {
                        ui.invoke_add_robot(saved.id.into(), saved.name.into())
                    });
                }
            }
            Work::Account(id) => {
                let account = if id.is_empty() {
                    None
                } else {
                    Some(self.db()?.account(&id).map_err(|e| e.to_string())?)
                };
                let models = account
                    .as_ref()
                    .and_then(|a| self.models.get(&a.id))
                    .cloned()
                    .unwrap_or_default();
                self.update(move |g| {
                    show_account(&g, account.as_ref());
                    g.set_model(models.first().cloned().unwrap_or_default().into());
                    g.set_provider_model_index(0);
                    g.set_provider_models(ModelRc::new(VecModel::from(
                        models.into_iter().map(Into::into).collect::<Vec<_>>(),
                    )));
                    if let Some(account) = account {
                        g.set_provider(account.provider.index());
                        if let Some(address) = account.address {
                            g.set_address(address.into());
                        }
                    }
                });
            }
            Work::SaveAccount(account) => {
                self.db()?
                    .save_account(&account)
                    .map_err(|e| e.to_string())?;
                self.lists()?;
                self.update(move |g| {
                    show_account(&g, Some(&account));
                    g.set_status("Account saved. Save the Robot profile to assign it.".into());
                });
            }
            Work::Role(id) => {
                let role = self
                    .db()?
                    .roles()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .find(|r| r.id == id);
                self.update(move |g| {
                    g.set_role_id(id.into());
                    g.set_role_name(
                        role.as_ref()
                            .map(|r| r.name.clone())
                            .unwrap_or_default()
                            .into(),
                    );
                    g.set_instructions(role.map(|r| r.instructions).unwrap_or_default().into());
                });
            }
            Work::SaveRole(role) => {
                self.db()?.save_role(&role).map_err(|e| e.to_string())?;
                self.lists()?;
                self.update(move |g| {
                    g.set_role_id(role.id.into());
                    g.set_status("Role saved. Save the profile to assign it.".into());
                });
            }
            Work::Server(id) => {
                let server = self
                    .servers
                    .iter()
                    .find(|s| s.id == id)
                    .cloned()
                    .or_else(|| {
                        self.db()
                            .ok()?
                            .catalog()
                            .ok()?
                            .into_iter()
                            .find(|r| r.kind == "mcp" && r.name == id)
                            .and_then(|r| serde_json::from_str(&r.content).ok())
                    });
                self.update(move |g| {
                    g.set_server_id(id.into());
                    g.set_server_command(
                        server
                            .as_ref()
                            .map(|s| s.command.clone())
                            .unwrap_or_default()
                            .into(),
                    );
                    g.set_server_arguments(
                        server
                            .as_ref()
                            .map(|s| s.arguments.join("\n"))
                            .unwrap_or_default()
                            .into(),
                    );
                    g.set_server_environment(
                        server
                            .as_ref()
                            .map(|s| {
                                s.secret_env
                                    .iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            })
                            .unwrap_or_default()
                            .into(),
                    );
                    g.set_server_tools(
                        server
                            .map(|s| s.enabled_tools.join("\n"))
                            .unwrap_or_default()
                            .into(),
                    );
                    g.set_discovered("".into());
                });
            }
            Work::SaveServer(server) => {
                self.db()?
                    .save_catalog(&crate::robot_data::CatalogEntry {
                        id: format!("mcp-{}", server.id),
                        kind: "mcp".into(),
                        name: server.id.clone(),
                        content: serde_json::to_string(&server).map_err(|e| e.to_string())?,
                    })
                    .map_err(|e| e.to_string())?;
                self.servers.retain(|s| s.id != server.id);
                self.servers.push(server);
                let servers = self
                    .servers
                    .iter()
                    .map(|s| (s.id.clone(), s.id.clone()))
                    .collect();
                self.update(move |g| {
                    g.set_servers(choices(servers));
                    g.set_busy(false);
                    g.set_status("Server saved in draft. Save the profile to apply it.".into());
                });
                self.lists()?;
            }
            Work::Discover(server, project) => {
                let mut session = crate::robot_tools::mcp::Session::connect(
                    &server,
                    std::path::Path::new(&project),
                )?;
                let tools = session
                    .discover()?
                    .iter()
                    .filter_map(|t| t["name"].as_str())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
                    .join("\n");
                self.update(move |g| {
                    g.set_discovered(tools.into());
                    g.set_busy(false);
                });
            }
            Work::Credential(reference, value) => {
                if reference.is_empty()
                    || !reference
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
                    || value.trim().is_empty()
                {
                    return Err("Enter a credential reference and a nonempty value.".into());
                }
                #[cfg(target_os = "linux")]
                crate::ticktick::activate_secret_service()?;
                keyring::Entry::new("orbit-robots", &reference)
                    .and_then(|e| e.set_password(&value))
                    .map_err(|_| "Couldn't save the credential in the system store.")?;
                self.db()?
                    .save_catalog(&crate::robot_data::CatalogEntry {
                        id: format!("credential-{reference}"),
                        kind: "credential".into(),
                        name: reference,
                        content: String::new(),
                    })
                    .map_err(|e| e.to_string())?;
                self.lists()?;
                self.update(|g| g.set_status("Credential saved in the system store.".into()));
            }
            Work::Conversation(id) => {
                self.conversation = id;
                self.refresh()?;
            }
            Work::History(page) => {
                self.page = page.max(0) as u32;
                self.refresh()?;
            }
            Work::Submit(profile, conversation, text) => {
                if profile.is_empty() {
                    return Err("Save a Robot profile first.".into());
                }
                if self.runtime.is_none() {
                    self.runtime = Some(robot_runtime::start(self.path.clone()));
                }
                let run =
                    self.runtime
                        .as_ref()
                        .unwrap()
                        .0
                        .submit(&conversation, &profile, &text)?;
                self.jobs.insert(run, conversation.clone());
                self.conversation = conversation;
                self.update(|g| {
                    g.set_command("".into());
                    g.set_running(true);
                    g.set_status("Submitting command…".into());
                });
            }
            Work::Cancel(conversation) => {
                if let Some((handle, _)) = &self.runtime {
                    handle.cancel(&conversation)?;
                }
            }
            Work::Approve(conversation, id, approved) => {
                if self.approvals.get(&conversation).map(|r| r.id) != Some(id) {
                    return Err("This local approval is no longer pending.".into());
                }
                self.runtime
                    .as_ref()
                    .ok_or("Robot worker is unavailable.")?
                    .0
                    .approve(&conversation, id, approved)?;
                self.approvals.remove(&conversation);
                self.refresh()?;
            }
            Work::Rate(conversation, rating) => {
                self.db()?
                    .rate(&conversation, rating as u8)
                    .map_err(|e| e.to_string())?;
                self.refresh()?;
            }
            Work::Delete(conversation) => {
                if self.jobs.values().any(|id| id == &conversation) {
                    return Err(
                        "Cancel work and wait for it to finish before deleting this conversation."
                            .into(),
                    );
                }
                self.db()?
                    .delete_conversation(&conversation)
                    .map_err(|e| e.to_string())?;
                if self.conversation == conversation {
                    self.conversation.clear();
                }
                self.refresh()?;
            }
            Work::Suspend(_) => unreachable!(),
        }
        Ok(())
    }
    fn event(&mut self, event: Event) {
        match event {
            Event::Approval {
                conversation,
                request,
            } => {
                self.approvals.insert(conversation, request);
                self.update(|g| g.set_status("Waiting for permission".into()));
            }
            Event::Finished {
                conversation,
                run,
                status,
                error,
            } => {
                self.jobs.remove(&run);
                self.approvals.remove(&conversation);
                self.update(move |g| {
                    g.set_status(format!("Command {status:?}").into());
                    if let Some(error) = error {
                        g.set_error(error.into());
                    }
                });
                crate::toast::info(
                    &self.weak,
                    "Robot command finished. Open its workbench to review the result.",
                );
            }
            Event::Rejected { run, message, .. } => {
                self.jobs.remove(&run);
                self.update(move |g| g.set_error(message.into()));
            }
            Event::Error {
                conversation,
                message,
            } => {
                if conversation.is_empty() {
                    self.jobs.clear();
                }
                self.update(move |g| g.set_error(message.into()));
            }
            Event::Queued { .. } => {
                self.update(|g| {
                    g.set_status("Command queued. Work continues while Orbit is hidden.".into())
                });
            }
            Event::Changed { .. } => {}
        }
        if let Err(error) = self.refresh() {
            self.update(move |g| g.set_error(error.into()));
        }
    }
    fn refresh(&self) -> Result<()> {
        let Some(profile) = &self.profile else {
            return Ok(());
        };
        let db = self.db()?;
        let history = db
            .history(&profile.id, self.page.saturating_mul(50))
            .map_err(|e| e.to_string())?;
        let conversation = db
            .conversation(&self.conversation)
            .map_err(|e| e.to_string())?;
        let context = conversation.as_ref().map(|c| &c.context);
        let model = context.map_or(&profile.model, |c| &c.model);
        let project = context.map_or(profile.project.as_str(), |c| c.project.as_str());
        let account = model
            .account
            .as_ref()
            .map(|a| format!(" · {}", a.name))
            .unwrap_or_default();
        let summary = format!(
            "{project}\n{} · {}{account}",
            model.provider.name(),
            model.id
        );
        let activity = conversation
            .as_ref()
            .map(|c| c.entries.iter().map(activity).collect::<Vec<_>>())
            .unwrap_or_default();
        let rating = conversation.as_ref().and_then(|c| c.rating).unwrap_or(0);
        let approval = self
            .approvals
            .get(&self.conversation)
            .map(|r| match &r.action {
                crate::robot_tools::LocalAction::Provider { name, scope } => format!(
                    "{name}\nProject: {}\n\n{}",
                    r.project.display(),
                    scope["scope"]
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| serde_json::to_string_pretty(scope).unwrap_or_default())
                ),
                _ => serde_json::to_string_pretty(r).unwrap_or_default(),
            })
            .unwrap_or_default();
        let approval_id = self
            .approvals
            .get(&self.conversation)
            .map(|r| r.id.to_string())
            .unwrap_or_default();
        let running = self.jobs.values().any(|id| id == &self.conversation);
        let id = self.conversation.clone();
        let page = self.page;
        self.update(move |g| {
            g.set_history(choices(
                history.into_iter().map(|h| (h.id, h.title)).collect(),
            ));
            g.set_conversation(id.into());
            g.set_history_page(page as i32);
            g.set_activity(ModelRc::new(VecModel::from(activity)));
            g.set_context(summary.into());
            g.set_rating(rating as i32);
            g.set_approval(approval.into());
            g.set_approval_id(approval_id.into());
            g.set_running(running);
        });
        Ok(())
    }
}
fn activity(entry: &Entry) -> RobotActivity {
    let (heading, text) = match entry {
        Entry::Command { text } | Entry::Queued { text, .. } => ("Command".into(), text.clone()),
        Entry::Reply { text } => ("Robot".into(), text.clone()),
        Entry::Progress { text } => ("Progress".into(), text.clone()),
        Entry::Run { status, .. } => ("Progress".into(), format!("{status:?}")),
        Entry::Tool {
            name,
            input,
            output,
            status,
        } => (
            format!("{name} · {status:?}"),
            format!(
                "{}\n{output}",
                serde_json::to_string_pretty(input).unwrap_or_default()
            ),
        ),
    };
    RobotActivity {
        heading: heading.into(),
        text: text.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_and_cross_conversation_clicks_keep_pending_approvals() {
        let mut worker = Worker {
            weak: slint::Weak::default(),
            models: HashMap::new(),
            path: PathBuf::new(),
            profile: None,
            conversation: "other".into(),
            page: 0,
            servers: vec![],
            runtime: None,
            jobs: HashMap::new(),
            approvals: HashMap::new(),
        };
        worker.approvals.insert(
            "shown".into(),
            crate::robot_tools::Approval {
                id: 42,
                project: std::env::temp_dir(),
                action: crate::robot_tools::LocalAction::Read {
                    path: "never-read".into(),
                },
            },
        );
        assert!(
            worker
                .apply(Work::Approve("other".into(), 42, true))
                .is_err()
        );
        assert!(
            worker
                .apply(Work::Approve("shown".into(), 41, true))
                .is_err()
        );
        assert_eq!(worker.approvals["shown"].id, 42);
    }
}

fn show_account(g: &RobotWorkbench<'_>, account: Option<&Account>) {
    g.set_account_index(
        account
            .and_then(|account| g.get_accounts().iter().position(|a| a.id == account.id))
            .map_or(0, |i| i as i32 + 1),
    );
    g.set_account_managed(account.is_some_and(|a| a.id.starts_with("t3-")));
    g.set_account_id(account.map(|a| a.id.as_str()).unwrap_or_default().into());
    g.set_account_name(account.map(|a| a.name.as_str()).unwrap_or_default().into());
    g.set_account_credential(
        account
            .and_then(|a| a.credential.as_deref())
            .unwrap_or_default()
            .into(),
    );
    g.set_account_home(
        account
            .and_then(|a| a.home.as_deref())
            .unwrap_or_default()
            .into(),
    );
}
