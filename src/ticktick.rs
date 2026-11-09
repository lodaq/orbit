use serde::{Deserialize, Serialize};
use std::time::Duration;

const API: &str = "https://api.ticktick.com/open/v1";
const KEYRING_SERVICE: &str = "Orbit";
const KEYRING_USER: &str = "ticktick-api-token";

pub fn resolve_list_name<S: AsRef<str>>(
    names: impl IntoIterator<Item = S>,
    query: &str,
) -> Result<usize, String> {
    let query = query.trim().to_lowercase();
    let mut candidate = None;
    let mut ambiguous = false;
    for (index, name) in names.into_iter().enumerate() {
        let name = name.as_ref().trim();
        if name.to_lowercase() == query {
            return Ok(index);
        }
        // TickTick names can include a decorative emoji before their readable name.
        let readable = name.trim_start_matches(|c: char| !c.is_alphanumeric());
        if !query.is_empty() && readable.to_lowercase() == query {
            ambiguous |= candidate.is_some();
            candidate = Some(index);
        }
    }
    if ambiguous {
        return Err(format!(
            "More than one TickTick list matches '{query}'. Use its full name"
        ));
    }
    candidate.ok_or_else(|| format!("TickTick list '{query}' was not found. Refresh TickTick in Settings or use its full name"))
}

#[derive(Clone, Deserialize)]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub inbox: bool,
}

#[derive(Clone, Deserialize)]
struct ProjectData {
    #[serde(default)]
    tasks: Vec<Task>,
    #[serde(default)]
    project: Option<Project>,
}

#[derive(Clone, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default, rename = "createdTime")]
    pub created_time: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "dueDate")]
    pub due_date: String,
    #[serde(default, rename = "startDate")]
    pub start_date: String,
    #[serde(default, rename = "isAllDay")]
    pub all_day: bool,
    #[serde(default)]
    pub time_zone: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default, rename = "repeatFlag")]
    pub repeat_flag: String,
    #[serde(default, rename = "repeatFrom")]
    pub repeat_from: String,
    #[serde(default)]
    pub reminders: Vec<String>,
    #[serde(default)]
    pub items: Vec<ChecklistItem>,
    #[serde(default, rename = "completedTime")]
    pub completed_time: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default, rename = "assigneeUsername")]
    pub assignee: String,
}

#[derive(Clone, Deserialize)]
pub struct ChecklistItem {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewTask<'a> {
    title: &'a str,
    project_id: &'a str,
    content: &'a str,
    priority: i32,
    tags: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    due_date: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_all_day: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatedTask<'a> {
    id: &'a str,
    project_id: &'a str,
    title: &'a str,
    content: &'a str,
    desc: &'a str,
    priority: i32,
    tags: &'a [String],
    due_date: Option<&'a str>,
    is_all_day: bool,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct MovedTask<'a> {
    title: &'a str,
    project_id: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<&'a str>,
    priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_all_day: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_date: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_date: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_zone: Option<&'a str>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn request_error(error: impl std::fmt::Display) -> String {
    format!("TickTick request failed: {error}")
}

fn get<T: for<'de> Deserialize<'de>>(
    client: &ureq::Agent,
    token: &str,
    path: &str,
) -> Result<T, String> {
    let response = client
        .get(format!("{API}{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .call()
        .map_err(request_error)?;
    response.into_body().read_json().map_err(request_error)
}

fn post<B: Serialize>(
    client: &ureq::Agent,
    token: &str,
    path: &str,
    body: &B,
) -> Result<(), String> {
    match client
        .post(format!("{API}{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .send_json(body)
    {
        Ok(_) => Ok(()),
        Err(ureq::Error::StatusCode(code)) => {
            Err(format!("TickTick request failed: status {code}"))
        }
        Err(error) => Err(format!("TickTick request failed: {error}")),
    }
}

fn post_json<T: for<'de> Deserialize<'de>, B: Serialize>(
    client: &ureq::Agent,
    token: &str,
    path: &str,
    body: &B,
) -> Result<T, String> {
    let response = client
        .post(format!("{API}{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .send_json(body)
        .map_err(request_error)?;
    response.into_body().read_json().map_err(request_error)
}

pub fn validate_token(token: &str) -> Result<Vec<Project>, String> {
    projects(token)
}

pub fn projects(token: &str) -> Result<Vec<Project>, String> {
    projects_with(&agent(), token)
}

fn projects_with(client: &ureq::Agent, token: &str) -> Result<Vec<Project>, String> {
    let mut projects = Vec::new();
    let mut offset = 0;
    loop {
        let page: Vec<Project> = get(
            client,
            token,
            &format!("/project?limit=200&offset={offset}"),
        )?;
        let count = page.len();
        projects.extend(page);
        if count < 200 {
            return Ok(projects);
        }
        offset += count;
    }
}

pub fn all_tasks(token: &str) -> Result<(Vec<Project>, Vec<Task>), String> {
    let client = agent();
    let mut projects = projects_with(&client, token)?;
    let mut tasks = Vec::new();
    for project in &projects {
        let data: ProjectData = get(&client, token, &format!("/project/{}/data", project.id))?;
        tasks.extend(data.tasks);
    }
    let inbox: ProjectData = get(&client, token, "/project/inbox/data")?;
    let mut project = inbox.project.unwrap_or_else(|| Project {
        id: inbox
            .tasks
            .first()
            .map(|t| t.project_id.clone())
            .unwrap_or_else(|| "inbox".into()),
        name: "Inbox".into(),
        inbox: true,
    });
    project.inbox = true;
    if !projects.iter().any(|p| p.id == project.id) {
        projects.push(project);
    }
    for task in inbox.tasks {
        if !tasks.iter().any(|t| t.id == task.id) {
            tasks.push(task);
        }
    }
    for task in scheduled_tasks(&client, token, None)? {
        if !tasks.iter().any(|t| t.id == task.id) {
            tasks.push(task);
        }
    }
    let completed = completed_tasks_with(&client, token, &projects)?;
    for task in completed {
        if !tasks.iter().any(|t| t.id == task.id) {
            tasks.push(task);
        }
    }
    Ok((projects, tasks))
}

pub fn project_tasks(token: &str, project: &str) -> Result<Vec<Task>, String> {
    let client = agent();
    let data: ProjectData = get(&client, token, &format!("/project/{project}/data"))?;
    let mut tasks = data.tasks;
    for task in scheduled_tasks(&client, token, Some(project))? {
        if !tasks.iter().any(|t| t.id == task.id) {
            tasks.push(task);
        }
    }
    let completed = completed_tasks_with(
        &client,
        token,
        &[Project {
            id: project.into(),
            name: String::new(),
            inbox: false,
        }],
    )?;
    for task in completed {
        if !tasks.iter().any(|t| t.id == task.id) {
            tasks.push(task);
        }
    }
    Ok(tasks)
}

// Project data can omit older tasks. Query each priority separately to retain them for Today.
fn scheduled_tasks(
    client: &ureq::Agent,
    token: &str,
    project: Option<&str>,
) -> Result<Vec<Task>, String> {
    let mut tasks = Vec::new();
    for priority in [5, 3, 1, 0] {
        let mut filter = serde_json::json!({ "status": [0], "priority": [priority] });
        if let Some(project) = project {
            filter["projectIds"] = serde_json::json!([project]);
        }
        let batch: Vec<Task> = post_json(client, token, "/task/filter", &filter)?;
        for task in batch {
            if !tasks.iter().any(|found: &Task| found.id == task.id) {
                tasks.push(task);
            }
        }
    }
    Ok(tasks)
}

#[allow(dead_code)]
pub fn completed_tasks(token: &str, projects: &[Project]) -> Result<Vec<Task>, String> {
    completed_tasks_with(&agent(), token, projects)
}

fn completed_tasks_with(
    client: &ureq::Agent,
    token: &str,
    projects: &[Project],
) -> Result<Vec<Task>, String> {
    let project_ids = projects
        .iter()
        .map(|project| &project.id)
        .collect::<Vec<_>>();
    let response = client
        .post(format!("{API}/task/completed"))
        .header("Authorization", format!("Bearer {token}"))
        .send_json(serde_json::json!({ "projectIds": project_ids }))
        .map_err(request_error)?;
    response.into_body().read_json().map_err(request_error)
}

pub fn create_task(
    token: &str,
    project_id: &str,
    title: &str,
    content: &str,
) -> Result<Task, String> {
    create_task_with_metadata(token, project_id, title, content, 0, &[])
}

pub fn create_task_with_metadata(
    token: &str,
    project_id: &str,
    title: &str,
    content: &str,
    priority: i32,
    tags: &[String],
) -> Result<Task, String> {
    create_task_with_details(token, project_id, title, content, priority, tags, None)
}

pub fn create_task_with_details(
    token: &str,
    project_id: &str,
    title: &str,
    content: &str,
    priority: i32,
    tags: &[String],
    due_date: Option<&str>,
) -> Result<Task, String> {
    let client = agent();
    let body = NewTask {
        title,
        project_id,
        content,
        priority,
        tags,
        due_date,
        is_all_day: due_date.map(|_| true),
    };
    client
        .post(format!("{API}/task"))
        .header("Authorization", format!("Bearer {token}"))
        .send_json(&body)
        .map_err(request_error)?
        .into_body()
        .read_json()
        .map_err(request_error)
}

pub fn inbox_project(token: &str) -> Result<Project, String> {
    let data: ProjectData = get(&agent(), token, "/project/inbox/data")?;
    Ok(data.project.unwrap_or_else(|| Project {
        id: data
            .tasks
            .first()
            .map_or_else(|| "inbox".into(), |task| task.project_id.clone()),
        name: "Inbox".into(),
        inbox: true,
    }))
}

pub struct TaskChanges<'a> {
    pub title: &'a str,
    pub content: &'a str,
    pub desc: &'a str,
    pub priority: i32,
    pub tags: &'a [String],
    pub due_date: Option<&'a str>,
}

pub fn update_task(
    token: &str,
    id: &str,
    project_id: &str,
    changes: TaskChanges<'_>,
) -> Result<(), String> {
    let client = agent();
    let body = UpdatedTask {
        id,
        project_id,
        title: changes.title,
        content: changes.content,
        desc: changes.desc,
        priority: changes.priority,
        tags: changes.tags,
        // TickTick requires JSON null to clear an existing due date.
        due_date: changes.due_date,
        is_all_day: changes.due_date.is_some(),
    };
    post(&client, token, &format!("/task/{id}"), &body)
}

// TickTick rejects a task update that changes its list, so a move is a copy
// into the new list followed by deleting the original. Every field is carried
// over, because the server keeps the rest of a task when it is updated.
pub fn move_task(
    token: &str,
    id: &str,
    from_project: &str,
    to_project: &str,
    changes: TaskChanges<'_>,
) -> Result<Task, String> {
    let client = agent();
    let task: Task = get(
        &client,
        token,
        &format!("/project/{from_project}/task/{id}"),
    )?;
    let body = MovedTask {
        title: changes.title,
        project_id: to_project,
        content: changes.content,
        desc: Some(changes.desc),
        priority: changes.priority,
        tags: Some(changes.tags),
        is_all_day: Some(true),
        due_date: changes.due_date,
        start_date: (!task.start_date.is_empty()).then_some(task.start_date.as_str()),
        time_zone: (!task.time_zone.is_empty()).then_some(task.time_zone.as_str()),
    };
    let created: Task = post_json(&client, token, "/task", &body)?;
    // The copy is in place; only now is the original safe to remove.
    delete_task(token, from_project, id)?;
    Ok(created)
}

pub fn delete_task(token: &str, project_id: &str, id: &str) -> Result<(), String> {
    agent()
        .delete(format!("{API}/project/{project_id}/task/{id}"))
        .header("Authorization", format!("Bearer {token}"))
        .call()
        .map(|_| ())
        .map_err(request_error)
}

pub fn complete_task(token: &str, project_id: &str, id: &str) -> Result<(), String> {
    agent()
        .post(format!("{API}/project/{project_id}/task/{id}/complete"))
        .header("Authorization", format!("Bearer {token}"))
        .send_empty()
        .map(|_| ())
        .map_err(request_error)
}

pub fn reopen_task(token: &str, project_id: &str, id: &str) -> Result<(), String> {
    let client = agent();
    post(
        &client,
        token,
        &format!("/task/{id}"),
        &serde_json::json!({ "id": id, "projectId": project_id, "status": 0 }),
    )?;
    let task: Task = get(&client, token, &format!("/project/{project_id}/task/{id}"))?;
    if task.status == 0 {
        Ok(())
    } else {
        Err("TickTick did not reopen the task".into())
    }
}

pub fn save_token(token: &str) -> Result<(), String> {
    keyring_entry()?.set_password(token).map_err(|error| {
        format!("Couldn't save the TickTick token in the system credential store: {error}")
    })
}

pub fn load_token() -> Result<Option<String>, String> {
    match keyring_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!(
            "Couldn't read the TickTick token from the system credential store: {error}"
        )),
    }
}

pub fn remove_token() -> Result<(), String> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "Couldn't remove the TickTick token from the system credential store: {error}"
        )),
    }
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    #[cfg(target_os = "linux")]
    activate_secret_service()?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| format!("Couldn't open the system credential store: {error}"))
}

#[cfg(target_os = "linux")]
pub(crate) fn activate_secret_service() -> Result<(), String> {
    use dbus::blocking::Connection;

    let connection = Connection::new_session()
        .map_err(|error| format!("Couldn't connect to the desktop credential service: {error}"))?;
    let bus = connection.with_proxy(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        Duration::from_secs(3),
    );
    let (owned,): (bool,) = bus
        .method_call(
            "org.freedesktop.DBus",
            "NameHasOwner",
            ("org.freedesktop.secrets",),
        )
        .map_err(|error| format!("Couldn't check the desktop credential service: {error}"))?;
    if owned {
        return Ok(());
    }
    let (activatable,): (Vec<String>,) = bus
        .method_call("org.freedesktop.DBus", "ListActivatableNames", ())
        .map_err(|error| format!("Couldn't check desktop credential providers: {error}"))?;
    if activatable
        .iter()
        .any(|name| name == "org.freedesktop.secrets")
    {
        return Ok(());
    }

    // KDE publishes its Secret Service compatibility provider under this alias.
    let wallet = connection.with_proxy(
        "org.kde.secretservicecompat",
        "/org/freedesktop/secrets",
        Duration::from_secs(5),
    );
    let _: (String,) = wallet
        .method_call("org.freedesktop.DBus.Introspectable", "Introspect", ())
        .map_err(|error| {
            format!(
                "No desktop credential store is available. Unlock a Secret Service-compatible wallet and retry: {error}"
            )
        })?;
    Ok(())
}

#[cfg(test)]
mod move_tests {
    use super::*;

    #[test]
    #[ignore = "uses the configured TickTick account"]
    fn fetch_all_tasks() {
        let start = std::time::Instant::now();
        let token = load_token().unwrap().expect("TickTick must be connected");
        let (projects, tasks) = all_tasks(&token).unwrap();
        println!(
            "Fetched {} projects and {} tasks in {:?}",
            projects.len(),
            tasks.len(),
            start.elapsed()
        );
    }

    #[test]
    #[ignore = "uses the configured TickTick account"]
    fn create_delete_test_task() {
        let token = load_token().unwrap().expect("TickTick must be connected");
        let inbox = inbox_project(&token).unwrap();
        let due = chrono::Local::now()
            .format("%Y-%m-%dT00:00:00+0000")
            .to_string();
        let created = create_task_with_details(
            &token,
            &inbox.id,
            "Orbit delete test",
            "",
            0,
            &[],
            Some(&due),
        )
        .unwrap();
        println!(
            "CREATED '{}' id={} project={} due={}",
            created.title, created.id, created.project_id, created.due_date
        );
    }

    #[test]
    #[ignore = "uses the configured TickTick account"]
    fn report_task_lists() {
        let token = load_token().unwrap().expect("TickTick must be connected");
        let client = agent();
        let mut projects = projects(&token).unwrap();
        projects.push(inbox_project(&token).unwrap());
        for project in &projects {
            let Ok(data) = client
                .get(format!("{API}/project/{}/data", project.id))
                .header("Authorization", format!("Bearer {token}"))
                .call()
            else {
                continue;
            };
            let value = data.into_body().read_json::<serde_json::Value>().unwrap();
            let Some(tasks) = value.get("tasks").and_then(|t| t.as_array()) else {
                continue;
            };
            for task in tasks {
                let title = task["title"].as_str().unwrap_or("");
                if title == "tracky" {
                    println!(
                        "'tracky' is in list '{}' (id {}) priority={} due={:?} id={}",
                        project.name, project.id, task["priority"], task["dueDate"], task["id"]
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "uses the configured TickTick account"]
    fn clean_up_probe_tasks() {
        let token = load_token().unwrap().expect("TickTick must be connected");
        let client = agent();
        let mut projects = projects(&token).unwrap();
        projects.push(inbox_project(&token).unwrap());
        for project in &projects {
            let Ok(data) = client
                .get(format!("{API}/project/{}/data", project.id))
                .header("Authorization", format!("Bearer {token}"))
                .call()
            else {
                continue;
            };
            let value = data.into_body().read_json::<serde_json::Value>().unwrap();
            let Some(tasks) = value.get("tasks").and_then(|t| t.as_array()) else {
                continue;
            };
            for task in tasks {
                let title = task["title"].as_str().unwrap_or("");
                if title.starts_with("Orbit move") || title.starts_with("Orbit probe") {
                    let id = task["id"].as_str().unwrap();
                    let result = client
                        .delete(format!("{API}/project/{}/task/{}", project.id, id))
                        .header("Authorization", format!("Bearer {token}"))
                        .call();
                    println!(
                        "DELETE '{title}' from {}: {:?}",
                        project.name,
                        result.map(|r| r.status()).map_err(|e| e.to_string())
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "uses the configured TickTick account"]
    fn move_task_keeps_every_field_and_removes_the_original() {
        let token = load_token().unwrap().expect("TickTick must be connected");
        let projects = projects(&token).unwrap();
        let inbox = inbox_project(&token).unwrap();
        let other = projects
            .iter()
            .find(|p| p.id != inbox.id)
            .expect("a second list");

        let created = create_task_with_details(
            &token,
            &inbox.id,
            "Orbit move test",
            "notes here",
            5,
            &["movetag".into()],
            Some("2026-10-21T00:00:00+0000"),
        )
        .unwrap();
        let id = created.id.clone();

        let moved = move_task(
            &token,
            &id,
            &inbox.id,
            &other.id,
            TaskChanges {
                title: "Orbit move test renamed",
                content: "notes changed",
                desc: "",
                priority: created.priority,
                tags: &created.tags,
                due_date: Some(&created.due_date),
            },
        )
        .unwrap();

        assert_eq!(moved.project_id, other.id);
        assert_eq!(moved.title, "Orbit move test renamed");
        assert_eq!(moved.content, "notes changed");
        assert_eq!(moved.priority, 5, "priority must survive the move");
        assert_eq!(moved.tags, ["movetag"], "tags must survive the move");
        assert!(
            !moved.due_date.is_empty(),
            "the due date must survive the move"
        );
        assert_ne!(moved.id, id, "the move makes a new task");

        // The original must be gone from the list it was in. The per-task GET
        // endpoint does not check the list, so the list contents are the truth.
        let old_list = project_tasks(&token, &inbox.id).unwrap();
        assert!(
            !old_list.iter().any(|task| task.id == id),
            "the original task must leave the old list"
        );
        let new_list = project_tasks(&token, &other.id).unwrap();
        assert!(
            new_list.iter().any(|task| task.id == moved.id),
            "the moved task must be in the new list"
        );

        // The copy is in the new list.
        let new = get::<Task>(
            &agent(),
            &token,
            &format!("/project/{}/task/{}", other.id, moved.id),
        )
        .unwrap();
        assert_eq!(new.project_id, other.id);

        delete_task(&token, &other.id, &moved.id).unwrap();
    }
}
