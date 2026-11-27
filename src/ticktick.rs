use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(test)]
thread_local! {
    static TEST_API: std::cell::OnceCell<String> = const { std::cell::OnceCell::new() };
}

fn api_url(path: &str) -> String {
    #[cfg(test)]
    if let Some(url) = TEST_API.with(|api| api.get().map(|base| format!("{base}{path}"))) {
        return url;
    }
    format!("https://api.ticktick.com/open/v1{path}")
}
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
        .get(api_url(path))
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
        .post(api_url(path))
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
        .post(api_url(path))
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
        .post(api_url("/task/completed"))
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
        .post(api_url("/task"))
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
        .delete(api_url(&format!("/project/{project_id}/task/{id}")))
        .header("Authorization", format!("Bearer {token}"))
        .call()
        .map(|_| ())
        .map_err(request_error)
}

pub fn complete_task(token: &str, project_id: &str, id: &str) -> Result<(), String> {
    agent()
        .post(api_url(&format!(
            "/project/{project_id}/task/{id}/complete"
        )))
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
pub(crate) mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    pub(crate) struct Exchange(pub &'static str, pub Option<Value>, pub Value);

    pub(crate) fn serve(exchanges: Vec<Exchange>) -> std::thread::JoinHandle<()> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        TEST_API.with(|api| {
            api.set(format!("http://{}", listener.local_addr().unwrap()))
                .unwrap()
        });
        std::thread::spawn(move || {
            for Exchange(request, expected, response) in exchanges {
                let deadline = std::time::Instant::now() + Duration::from_secs(3);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(std::time::Instant::now() < deadline, "missing {request}");
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accepting {request}: {error}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert_eq!(line.trim_end().strip_suffix(" HTTP/1.1"), Some(request));
                let mut content_length = 0;
                let mut authorized = false;
                loop {
                    line.clear();
                    assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                    if line == "\r\n" {
                        break;
                    }
                    let (name, value) = line.split_once(':').unwrap();
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse().unwrap();
                    }
                    if name.eq_ignore_ascii_case("authorization") {
                        authorized = value.trim() == "Bearer test-token";
                    }
                }
                assert!(authorized);
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).unwrap();
                let body =
                    (!body.is_empty()).then(|| serde_json::from_slice::<Value>(&body).unwrap());
                assert_eq!(body, expected);

                let response = response.to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{response}",
                    response.len()
                )
                .unwrap();
            }
        })
    }

    #[test]
    fn all_tasks_merges_projects_inbox_scheduled_and_completed() {
        let open = json!({"id": "open", "projectId": "work"});
        let inbox = json!({"id": "inbox-task", "projectId": "inbox"});
        let scheduled = json!({"id": "scheduled", "projectId": "work"});
        let completed = json!({"id": "completed", "projectId": "work", "status": 2});
        let mut exchanges = vec![
            Exchange(
                "GET /project?limit=200&offset=0",
                None,
                json!([{"id": "work"}]),
            ),
            Exchange("GET /project/work/data", None, json!({"tasks": [open]})),
            Exchange("GET /project/inbox/data", None, json!({"tasks": [inbox]})),
        ];
        for priority in [5, 3, 1, 0] {
            let response = if priority == 5 {
                json!([open, scheduled])
            } else {
                json!([])
            };
            exchanges.push(Exchange(
                "POST /task/filter",
                Some(json!({"status": [0], "priority": [priority]})),
                response,
            ));
        }
        exchanges.push(Exchange(
            "POST /task/completed",
            Some(json!({"projectIds": ["work", "inbox"]})),
            json!([completed]),
        ));
        let server = serve(exchanges);
        let (projects, tasks) = all_tasks("test-token").unwrap();
        assert_eq!(
            projects
                .iter()
                .map(|project| project.id.as_str())
                .collect::<Vec<_>>(),
            ["work", "inbox"]
        );
        assert!(projects[1].inbox);
        assert_eq!(
            tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            ["open", "inbox-task", "scheduled", "completed"]
        );
        server.join().unwrap();
    }

    #[test]
    fn move_task_copies_fields_before_deleting_original() {
        let due = "2026-11-30T00:00:00+0000";
        let start = "2026-11-29T00:00:00+0000";
        let server = serve(vec![
            Exchange(
                "GET /project/inbox/task/old",
                None,
                json!({"id": "old", "projectId": "inbox", "startDate": start}),
            ),
            Exchange(
                "POST /task",
                Some(json!({
                    "projectId": "work", "title": "Renamed", "content": "Notes", "desc": "Details",
                    "priority": 5, "tags": ["tag"], "isAllDay": true, "dueDate": due, "startDate": start
                })),
                json!({"id": "new", "projectId": "work"}),
            ),
            Exchange("DELETE /project/inbox/task/old", None, Value::Null),
        ]);
        let moved = move_task(
            "test-token",
            "old",
            "inbox",
            "work",
            TaskChanges {
                title: "Renamed",
                content: "Notes",
                desc: "Details",
                priority: 5,
                tags: &["tag".into()],
                due_date: Some(due),
            },
        )
        .unwrap();
        assert_eq!(moved.id, "new");
        assert_eq!(moved.project_id, "work");
        server.join().unwrap();
    }
}
