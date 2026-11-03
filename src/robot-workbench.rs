use crate::robot_data::{Database, Entry, McpServer, Model, Profile, Provider, Role};
use crate::robot_runtime::{self, Event, Handle};
use crate::{Launcher, RobotActivity, RobotChoice, RobotWorkbench};
use slint::{ComponentHandle, ModelRc, VecModel};
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
    Open(String, bool),
    Save(Profile, bool),
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
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_open(move |profile, editing| {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            g.set_visible(true);
            g.set_busy(true);
            g.set_error("".into());
        }
        let _ = send.send(Work::Open(profile.into(), editing));
    });
    let send = sender.clone();
    let weak = ui.as_weak();
    g.on_save_profile(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<RobotWorkbench>();
            let provider = match g.get_provider() {
                1 => Provider::OpenAi,
                2 => Provider::Gemini,
                _ => Provider::Ollama,
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
                    ollama_address: (provider == Provider::Ollama).then(|| g.get_address().into()),
                    provider,
                    id: g.get_model().into(),
                },
                role_id: (!g.get_role_id().is_empty()).then(|| g.get_role_id().into()),
                skills: lines(g.get_skills().as_str()),
                mcp_servers: vec![],
            };
            g.set_busy(true);
            let _ = send.send(Work::Save(profile, g.get_adding()));
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
        let profiles = db.profiles().map_err(|e| e.to_string())?;
        let roles = db.roles().map_err(|e| e.to_string())?;
        self.update(move |g| {
            g.set_profiles(choices(
                profiles.into_iter().map(|p| (p.id, p.name)).collect(),
            ));
            g.set_roles(choices(roles.into_iter().map(|r| (r.id, r.name)).collect()));
        });
        Ok(())
    }
    fn show_profile(&mut self, profile: Profile, editing: bool) -> Result<()> {
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
            g.set_profile_id(profile.id.into());
            g.set_name(profile.name.into());
            g.set_project(profile.project.into());
            g.set_provider(match profile.model.provider {
                Provider::Ollama => 0,
                Provider::OpenAi => 1,
                Provider::Gemini => 2,
            });
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
            Work::Open(id, editing) => {
                self.lists()?;
                let profile = if id.is_empty() {
                    let ai = crate::config::load().0.ai;
                    Profile {
                        id: String::new(),
                        name: String::new(),
                        project: String::new(),
                        model: Model {
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
            Work::Save(mut profile, adding) => {
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
                self.show_profile(profile, false)?;
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
                let server = self.servers.iter().find(|s| s.id == id).cloned();
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
                self.update(|g| g.set_status("Waiting for local action approval".into()));
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
        let summary = format!("{project}\n{:?} · {}", model.provider, model.id);
        let activity = conversation
            .as_ref()
            .map(|c| c.entries.iter().map(activity).collect::<Vec<_>>())
            .unwrap_or_default();
        let rating = conversation.as_ref().and_then(|c| c.rating).unwrap_or(0);
        let approval = self
            .approvals
            .get(&self.conversation)
            .map(|r| serde_json::to_string_pretty(r).unwrap_or_default())
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
