use crate::text_direction::text_is_rtl;
use crate::{Launcher, TickTickProject, TickTickTask, TickTickView, ticktick};
use chrono::{Datelike, Local, NaiveDate};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::rc::Rc;

#[derive(Clone)]
enum Request {
    Refresh(String),
    Connect(String),
    Disconnect,
    Save(
        String,
        String,
        String,
        String,
        String,
        String,
        i32,
        Vec<String>,
        String,
    ),
    PrefixSave(String, String, i32, Vec<String>, bool, String),
    Delete(String, String),
    Complete(String, String, bool),
}

enum Response {
    All(Vec<ticktick::Project>, Vec<ticktick::Task>),
    List(String, Vec<ticktick::Task>),
    Created(ticktick::Task),
    Moved(String, ticktick::Task),
    Done,
    Disconnected,
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    Rc::new(VecModel::from(items)).into()
}

pub fn connect(ui: &Launcher) {
    ui.on_ticktick_views_for(|projects, tasks| model(task_views(&projects, &tasks)));
    ui.on_ticktick_current_date(|| Local::now().format("%Y-%m-%d").to_string().into());
    let weak = ui.as_weak();
    ui.on_load_ticktick(move || {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Refresh(String::new()));
        }
    });
    let weak = ui.as_weak();
    ui.on_refresh_ticktick_list(move |project| {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Refresh(project.into()));
        }
    });
    let weak = ui.as_weak();
    ui.on_save_ticktick_token(move |token| {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Connect(token.into()));
        }
    });
    let weak = ui.as_weak();
    ui.on_remove_ticktick_token(move || {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Disconnect);
        }
    });
    let weak = ui.as_weak();
    ui.on_save_ticktick_task(
        move |id, project, previous, title, content, desc, priority, tags, due_date| {
            if let Some(ui) = weak.upgrade() {
                start(
                    &ui,
                    Request::Save(
                        id.into(),
                        project.into(),
                        previous.into(),
                        title.into(),
                        content.into(),
                        desc.into(),
                        priority,
                        tags.split(',')
                            .map(str::trim)
                            .filter(|tag| !tag.is_empty())
                            .map(str::to_owned)
                            .collect(),
                        due_date.into(),
                    ),
                );
            }
        },
    );
    let weak = ui.as_weak();
    ui.on_delete_ticktick_task(move |id, project| {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Delete(id.into(), project.into()));
        }
    });
    let weak = ui.as_weak();
    ui.on_complete_ticktick_task(move |id, project| {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Complete(id.into(), project.into(), true));
        }
    });
    let weak = ui.as_weak();
    ui.on_reopen_ticktick_task(move |id, project| {
        if let Some(ui) = weak.upgrade() {
            start(&ui, Request::Complete(id.into(), project.into(), false));
        }
    });
    ui.on_ticktick_list_index(|projects, id| {
        projects
            .iter()
            .position(|p| p.id == id)
            .unwrap_or_else(|| projects.iter().position(|p| p.name == "Inbox").unwrap_or(0))
            as i32
    });
    ui.on_ticktick_list_name(|projects, id| match id.as_str() {
        id if id.starts_with("@tag:") => format!("#{}", &id[5..]).into(),
        "" => "Inbox (all tasks)".into(),
        "@today" => "Today".into(),
        "@week" => "Next 7 Days".into(),
        _ => projects
            .iter()
            .find(|p| p.id == id)
            .map_or_else(|| "List unavailable".into(), |p| p.name),
    });
    ui.on_ticktick_task_count(|tasks, id, completed| {
        tasks
            .iter()
            .filter(|t| matches_view(t, &id, completed))
            .count() as i32
    });
    ui.on_ticktick_visible_tasks(|tasks, id, completed, offset, count| {
        let mut tasks: Vec<TickTickTask> = tasks
            .iter()
            .filter(|t| matches_view(t, &id, completed))
            .collect();
        tasks.sort_by(compare_tasks);
        let offset =
            (offset.max(0) as usize).min(tasks.len().saturating_sub(count.max(0) as usize));
        model(
            tasks
                .into_iter()
                .skip(offset)
                .take(count.max(0) as usize)
                .collect(),
        )
    });
    start(ui, Request::Refresh(String::new()));
}

fn task_views(
    projects: &ModelRc<TickTickProject>,
    tasks: &ModelRc<TickTickTask>,
) -> Vec<TickTickView> {
    let view = |id: &str, name: &str, heading| TickTickView {
        id: id.into(),
        name: name.into(),
        heading,
        rtl: text_is_rtl(name),
    };
    let mut views = vec![
        view("@today", "Today", false),
        view("@week", "Next 7 Days", false),
        view("", "Inbox (all tasks)", false),
    ];
    // Inbox is already represented by the all-tasks view.
    let lists: Vec<_> = projects.iter().filter(|p| !p.inbox).collect();
    if !lists.is_empty() {
        views.push(view("", "Lists", true));
        views.extend(lists.iter().map(|p| view(&p.id, &p.name, false)));
    }
    let mut tags = std::collections::BTreeSet::new();
    for task in tasks.iter() {
        for tag in task.tag_names.split('\0').filter(|tag| !tag.is_empty()) {
            tags.insert(tag.to_owned());
        }
    }
    if !tags.is_empty() {
        views.push(view("", "Tags", true));
        views.extend(
            tags.iter()
                .map(|tag| view(&format!("@tag:{tag}"), &format!("#{tag}"), false)),
        );
    }
    views
}

fn compare_tasks(a: &TickTickTask, b: &TickTickTask) -> std::cmp::Ordering {
    b.priority
        .cmp(&a.priority)
        .then_with(|| (a.day == 0).cmp(&(b.day == 0)))
        .then_with(|| a.day.cmp(&b.day))
}

fn matches_view(task: &TickTickTask, id: &str, completed: bool) -> bool {
    if !completed && task.done {
        return false;
    }
    match id {
        "" => true,
        "@today" | "@week" => {
            let today = Local::now().date_naive().num_days_from_ce();
            task.day > 0 && task.day <= today + if id == "@week" { 6 } else { 0 }
        }
        id if id.starts_with("@tag:") => task.tag_names.split('\0').any(|tag| tag == &id[5..]),
        _ => task.project_id == id,
    }
}

pub fn create_from_prefix(ui: &Launcher, query: &str) {
    ui.set_busy(false);
    let result = crate::apps::parse_ticktick_query(query).and_then(|draft| {
        if !ui.get_ticktick_connected() && !ui.get_ticktick_loading() {
            return Err("Connect or refresh TickTick in Settings first".into());
        }
        if ui.get_ticktick_loading() {
            return Err("TickTick is busy. Try again when it finishes".into());
        }
        let name = draft.list.as_deref().unwrap_or("Inbox");
        let projects = ui.get_ticktick_projects();
        let index = ticktick::resolve_list_name(projects.iter().map(|project| project.name), name)?;
        let project = projects
            .row_data(index)
            .ok_or("TickTick list is unavailable")?;
        Ok(Request::PrefixSave(
            project.id.into(),
            draft.title,
            draft.priority,
            draft.tags,
            draft.today,
            query.into(),
        ))
    });
    match result {
        Ok(request) => start(ui, request),
        Err(error) => crate::toast::error(&ui.as_weak(), &error),
    }
}

fn start(ui: &Launcher, request: Request) {
    // Only TickTick writes are serialized. Launcher input and its worker stay independent.
    if ui.get_ticktick_loading() {
        return;
    }
    if matches!(request, Request::Refresh(_)) {
        crate::toast::info(&ui.as_weak(), "Refreshing TickTick tasks");
    }
    let before: Vec<_> = ui.get_ticktick_tasks().iter().collect();
    let mut optimistic = before.clone();
    let form = matches!(request, Request::Save(..) | Request::Delete(..));
    let mutation = form || matches!(request, Request::Complete(..) | Request::PrefixSave(..));
    match &request {
        Request::PrefixSave(project, title, priority, tags, today, _) => {
            let date = Local::now().date_naive();
            optimistic.insert(
                0,
                TickTickTask {
                    id: "@pending".into(),
                    project_id: project.as_str().into(),
                    rtl: text_is_rtl(title),
                    title: title.as_str().into(),
                    priority: *priority,
                    tag_names: tags.join("\0").into(),
                    tags: tags.join(", ").into(),
                    due: if *today {
                        date.format("%b %e").to_string().into()
                    } else {
                        "".into()
                    },
                    day: if *today { date.num_days_from_ce() } else { 0 },
                    ..Default::default()
                },
            );
            ui.invoke_reset_ticktick_scroll();
            // Reveal the widget immediately; retain the active Prefix for a failed request.
            ui.set_search_text("".into());
            ui.invoke_search("".into());
        }
        Request::Save(id, project, _previous, title, content, desc, priority, tags, due_date) => {
            if title.trim().is_empty() {
                ui.set_ticktick_form_error("Enter a task title".into());
                return;
            }
            if id.is_empty() {
                optimistic.insert(
                    0,
                    TickTickTask {
                        id: "@pending".into(),
                        project_id: project.as_str().into(),
                        rtl: text_is_rtl(title),
                        title: title.trim().into(),
                        priority: *priority,
                        tag_names: tags.join("\0").into(),
                        tags: tags.join(", ").into(),
                        content: content.as_str().into(),
                        desc: desc.as_str().into(),
                        ..Default::default()
                    },
                );
                ui.invoke_reset_ticktick_scroll();
            } else if let Some(task) = optimistic.iter_mut().find(|t| t.id == id) {
                task.rtl = text_is_rtl(title);
                task.title = title.trim().into();
                task.content = content.as_str().into();
                task.desc = desc.as_str().into();
                task.priority = *priority;
                task.tag_names = tags.join("\0").into();
                task.tags = tags.join(", ").into();
                task.due_date = due_date.as_str().into();
                let date = NaiveDate::parse_from_str(due_date.trim(), "%Y-%m-%d").ok();
                task.due = date
                    .map(|date| date.format("%b %e").to_string())
                    .unwrap_or_default()
                    .into();
                task.day = date.map_or(0, |date| date.num_days_from_ce());
                // A moved task leaves the list it came from, so the view updates at once.
                task.project_id = project.as_str().into();
            }
        }
        Request::Delete(id, _) => optimistic.retain(|t| t.id != id),
        Request::Complete(id, _, done) => {
            if let Some(task) = optimistic.iter_mut().find(|t| t.id == id) {
                task.done = *done;
            }
            if *done {
                std::thread::spawn(crate::platform::play_completion_sound);
            }
        }
        _ => {}
    }
    ui.set_ticktick_loading(true);
    ui.set_ticktick_refreshing(!mutation);
    ui.set_ticktick_error(false);
    ui.set_ticktick_form_error("".into());
    ui.set_ticktick_status(
        if mutation {
            "Saving changes…"
        } else {
            "Loading tasks…"
        }
        .into(),
    );
    if mutation {
        ui.set_ticktick_tasks(model(optimistic));
    }
    if form {
        ui.set_ticktick_open(false);
    }
    let prefix_name = ui.get_activated_prefix_name();
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let result = execute(&request);
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.set_ticktick_loading(false);
            ui.set_ticktick_refreshing(false);
            match result {
                Ok(response) => {
                    apply_response(&ui, response);
                    if let Request::Complete(ref id, _, done) = request {
                        ui.invoke_record_widget_completions(model(vec![
                            crate::CompletedReference {
                                id: format!("ticktick:{id}").into(),
                                title: ui
                                    .get_ticktick_tasks()
                                    .iter()
                                    .find(|task| task.id == *id)
                                    .map(|task| task.title)
                                    .unwrap_or_default(),
                                date: Local::now().format("%Y-%m-%d").to_string().into(),
                                done,
                            },
                        ]));
                    }
                    let message = match request {
                        Request::Refresh(ref id) if !id.is_empty() => "List refreshed",
                        Request::Refresh(_) => "Tasks refreshed",
                        Request::Connect(_) => "TickTick connected",
                        Request::Disconnect => "TickTick disconnected",
                        Request::Save(ref id, ..) if id.is_empty() => "Task added",
                        Request::PrefixSave(..) => "Task added",
                        Request::Save(_, ref project, ref previous, ..) if project != previous => {
                            "Task moved"
                        }
                        Request::Save(..) => "Task saved",
                        Request::Delete(..) => "Task deleted",
                        Request::Complete(_, _, true) => "Task completed",
                        Request::Complete(_, _, false) => "Task reopened",
                    };
                    ui.set_ticktick_status(message.into());
                    crate::toast::success(&ui.as_weak(), message);
                }
                Err(error) => {
                    if mutation {
                        ui.set_ticktick_tasks(model(before));
                    }
                    if let Request::PrefixSave(_, _, _, _, _, query) = &request
                        && ui.get_search_text().is_empty()
                        && ui.get_activated_prefix_name() == prefix_name
                    {
                        ui.set_search_text(query.as_str().into());
                        ui.invoke_search(query.as_str().into());
                        ui.invoke_focus_search();
                    }
                    if form {
                        ui.set_ticktick_open(true);
                        ui.set_ticktick_form_error(error.as_str().into());
                    }
                    ui.set_ticktick_error(true);
                    ui.set_ticktick_status(error.as_str().into());
                    eprintln!("orbit TickTick request: {error}");
                    let message = match request {
                        Request::Refresh(_) | Request::Connect(_) => "Couldn't refresh TickTick",
                        Request::Save(ref id, ..) if id.is_empty() => "Couldn't add TickTick task",
                        Request::PrefixSave(..) => &error,
                        Request::Save(..) => "Couldn't save TickTick task",
                        Request::Delete(..) => "Couldn't delete TickTick task",
                        Request::Complete(_, _, true) => "Couldn't complete TickTick task",
                        Request::Complete(_, _, false) => "Couldn't reopen TickTick task",
                        Request::Disconnect => "Couldn't disconnect TickTick",
                    };
                    crate::toast::error(&ui.as_weak(), message);
                }
            }
        });
    });
}

fn execute(request: &Request) -> Result<Response, String> {
    if let Request::Disconnect = request {
        ticktick::remove_token()?;
        return Ok(Response::Disconnected);
    }
    if let Request::Connect(token) = request {
        let token = token.trim();
        if token.is_empty() {
            return Err("Enter a TickTick API token".into());
        }
        ticktick::validate_token(token)?;
        ticktick::save_token(token)?;
    }
    let token = ticktick::load_token()?.ok_or("Connect TickTick in Settings first")?;
    let result = match request {
        Request::Refresh(project) if !project.is_empty() && !project.starts_with('@') => {
            ticktick::project_tasks(&token, project)
                .map(|tasks| Response::List(project.clone(), tasks))
        }
        Request::Refresh(_) | Request::Connect(_) => {
            ticktick::all_tasks(&token).map(|(projects, tasks)| Response::All(projects, tasks))
        }
        Request::Save(id, project, previous, title, content, _, _, _, _) if id.is_empty() => {
            ticktick::create_task(&token, project, title.trim(), content).map(Response::Created)
        }
        Request::PrefixSave(project, title, priority, tags, today, _) => {
            let due_date =
                today.then(|| format!("{}T00:00:00+0000", chrono::Local::now().format("%Y-%m-%d")));
            ticktick::create_task_with_details(
                &token,
                project,
                title,
                "",
                *priority,
                tags,
                due_date.as_deref(),
            )
            .map(Response::Created)
        }
        // A different list means a move, which the server only accepts as a copy
        // into the new list followed by deleting the original.
        Request::Save(id, project, previous, title, content, desc, priority, tags, due_date)
            if project != previous =>
        {
            let due_date = normalize_due_date(due_date)?;
            ticktick::move_task(
                &token,
                id,
                previous,
                project,
                ticktick::TaskChanges {
                    title: title.trim(),
                    content,
                    desc,
                    priority: *priority,
                    tags,
                    due_date: due_date.as_deref(),
                },
            )
            .map(|task| Response::Moved(id.clone(), task))
        }
        Request::Save(id, project, _, title, content, desc, priority, tags, due_date) => {
            let due_date = normalize_due_date(due_date)?;
            ticktick::update_task(
                &token,
                id,
                project,
                ticktick::TaskChanges {
                    title: title.trim(),
                    content,
                    desc,
                    priority: *priority,
                    tags,
                    due_date: due_date.as_deref(),
                },
            )
            .map(|()| Response::Done)
        }
        Request::Delete(id, project) => {
            ticktick::delete_task(&token, project, id).map(|()| Response::Done)
        }
        Request::Complete(id, project, true) => {
            ticktick::complete_task(&token, project, id).map(|()| Response::Done)
        }
        Request::Complete(id, project, false) => {
            ticktick::reopen_task(&token, project, id).map(|()| Response::Done)
        }
        Request::Disconnect => unreachable!(),
    };
    result.map_err(|error| {
        format!(
            "{}: {error}",
            match request {
                Request::Save(id, ..) if id.is_empty() => "Couldn't add task",
                Request::PrefixSave(..) => "Couldn't add task",
                Request::Save(..) => "Couldn't save task",
                Request::Delete(..) => "Couldn't delete task",
                Request::Complete(_, _, true) => "Couldn't complete task",
                Request::Complete(_, _, false) => "Couldn't reopen task",
                _ => "Couldn't refresh TickTick",
            }
        )
    })
}

fn apply_response(ui: &Launcher, response: Response) {
    match response {
        Response::All(projects, tasks) => {
            record_completions(ui, &tasks);
            ui.set_ticktick_project_names(model(
                projects.iter().map(|p| p.name.as_str().into()).collect(),
            ));
            ui.set_ticktick_projects(model(projects.iter().map(project_model).collect()));
            ui.set_ticktick_tasks(model(tasks.into_iter().map(task_model).collect()));
            ui.set_ticktick_connected(true);
        }
        Response::List(project, tasks) => {
            record_completions(ui, &tasks);
            let mut existing: Vec<_> = ui
                .get_ticktick_tasks()
                .iter()
                .filter(|t| t.project_id != project)
                .collect();
            existing.extend(tasks.into_iter().map(task_model));
            ui.set_ticktick_tasks(model(existing));
        }
        Response::Created(task) => {
            let mut tasks: Vec<_> = ui.get_ticktick_tasks().iter().collect();
            if let Some(pending) = tasks.iter_mut().find(|t| t.id == "@pending") {
                *pending = task_model(task);
            }
            ui.set_ticktick_tasks(model(tasks));
        }
        Response::Moved(old_id, task) => {
            // The move made a new task, so the old one is replaced by it.
            let mut tasks: Vec<_> = ui
                .get_ticktick_tasks()
                .iter()
                .filter(|t| t.id != old_id)
                .collect();
            tasks.insert(0, task_model(task));
            ui.set_ticktick_tasks(model(tasks));
        }
        Response::Disconnected => {
            ui.set_ticktick_tasks(model(Vec::new()));
            ui.set_ticktick_projects(model(Vec::new()));
            ui.set_ticktick_project_names(model(Vec::new()));
            ui.set_ticktick_connected(false);
        }
        Response::Done => {}
    }
}

fn project_model(project: &ticktick::Project) -> TickTickProject {
    TickTickProject {
        id: project.id.as_str().into(),
        name: project.name.as_str().into(),
        inbox: project.inbox,
    }
}

fn normalize_due_date(value: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "Enter a due date as YYYY-MM-DD".to_owned())?;
    Ok(Some(format!("{}T00:00:00+0000", date.format("%Y-%m-%d"))))
}

fn focus_details(task: &ticktick::Task) -> String {
    let mut sections: Vec<_> = [&task.content, &task.desc]
        .into_iter()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .collect();
    if !task.items.is_empty() {
        let mut checklist = vec!["Checklist".to_owned()];
        checklist.extend(task.items.iter().map(|item| {
            format!(
                "{} {}",
                if item.status == 1 { "☑" } else { "☐" },
                item.title
            )
        }));
        sections.push(checklist.join("\n"));
    }
    sections.join("\n\n")
}

fn add_task_detail(details: &mut Vec<String>, label: &str, value: &str) {
    if !value.trim().is_empty() {
        details.push(format!("{label}: {}", value.trim()));
    }
}

fn task_model(task: ticktick::Task) -> TickTickTask {
    let parse_date = |value: &str| {
        if task.all_day {
            value
                .get(..10)
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        } else {
            chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f%z")
                .ok()
                .map(|d| d.with_timezone(&Local).date_naive())
                .or_else(|| {
                    value
                        .get(..10)
                        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                })
        }
    };
    let due_date = parse_date(&task.due_date);
    let display_date = due_date.or_else(|| parse_date(&task.start_date));
    let mut details = Vec::new();
    add_task_detail(
        &mut details,
        "Start date",
        task.start_date.get(..10).unwrap_or(&task.start_date),
    );
    add_task_detail(
        &mut details,
        "Due date",
        task.due_date.get(..10).unwrap_or(&task.due_date),
    );
    add_task_detail(
        &mut details,
        "Priority",
        match task.priority {
            5 => "High",
            3 => "Medium",
            1 => "Low",
            _ => "None",
        },
    );
    add_task_detail(&mut details, "Tags", &task.tags.join(", "));
    add_task_detail(&mut details, "Notes", &task.content);
    add_task_detail(&mut details, "Description", &task.desc);
    if !task.items.is_empty() {
        details.push("Checklist:".into());
        details.extend(task.items.iter().map(|item| {
            format!(
                "{} {}",
                if item.status == 1 { "☑" } else { "☐" },
                item.title
            )
        }));
    }
    add_task_detail(&mut details, "Repeats", &task.repeat_flag);
    if !task.repeat_flag.is_empty() {
        add_task_detail(
            &mut details,
            "Repeat timing",
            match task.repeat_from.as_str() {
                "0" => "Original schedule",
                "1" => "After completion",
                _ => "Calendar schedule",
            },
        );
    }
    add_task_detail(&mut details, "Reminders", &task.reminders.join(", "));
    add_task_detail(&mut details, "Time zone", &task.time_zone);
    if task.all_day {
        add_task_detail(&mut details, "Schedule", "All day");
    }
    add_task_detail(
        &mut details,
        "Status",
        match task.status {
            2 => "Completed",
            -1 => "Abandoned",
            _ => "Open",
        },
    );
    add_task_detail(&mut details, "Created", &task.created_time);
    add_task_detail(&mut details, "Completed", &task.completed_time);
    add_task_detail(&mut details, "Type", &task.kind);
    add_task_detail(&mut details, "Assigned to", &task.assignee);
    let focus_notes = focus_details(&task);
    let details = details.join("\n");
    TickTickTask {
        id: task.id.into(),
        project_id: task.project_id.into(),
        rtl: text_is_rtl(&task.title),
        title: task.title.into(),
        content: task.content.into(),
        desc: task.desc.into(),
        created_time: task.created_time.into(),
        due: display_date
            .map(|d| d.format("%b %e").to_string())
            .unwrap_or_default()
            .into(),
        due_date: due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
            .into(),
        priority: task.priority,
        tag_names: task.tags.join("\0").into(),
        tags: task.tags.join(", ").into(),
        focus_notes_rtl: text_is_rtl(&focus_notes),
        focus_notes: focus_notes.into(),
        details: details.into(),
        day: display_date.map_or(0, |d| d.num_days_from_ce()),
        done: task.status == 2,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str, priority: i32, start: &str, due: &str) -> TickTickTask {
        task_model(
            serde_json::from_value(serde_json::json!({
                "id": title, "projectId": "inbox", "title": title,
                "priority": priority, "startDate": start, "dueDate": due,
                "isAllDay": true
            }))
            .unwrap(),
        )
    }

    #[test]
    fn tag_views_match_whole_tags_and_respect_completed() {
        let mut task = task("Tagged task", 3, "", "");
        task.tag_names = "work\0تعلم\0one, two".into();
        assert!(matches_view(&task, "@tag:work", false));
        assert!(matches_view(&task, "@tag:تعلم", false));
        assert!(matches_view(&task, "@tag:one, two", false));
        assert!(!matches_view(&task, "@tag:wor", false));
        task.done = true;
        assert!(!matches_view(&task, "@tag:work", false));
        assert!(matches_view(&task, "@tag:work", true));
    }

    #[test]
    fn arabic_titles_use_the_first_strong_character() {
        assert!(task("📚 123 اقرأ الكتاب (Rust)", 0, "", "").rtl);
        assert!(task("  الحمد لله", 0, "", "").rtl);
        assert!(!task("Read كتاب", 0, "", "").rtl);
        assert!(!task("123 📚", 0, "", "").rtl);
    }

    #[test]
    fn picker_keeps_all_lists_and_unique_exact_tags() {
        let projects = model(vec![
            TickTickProject {
                id: "inbox".into(),
                name: "Inbox".into(),
                inbox: true,
            },
            TickTickProject {
                id: "custom".into(),
                name: "Inbox".into(),
                inbox: false,
            },
        ]);
        let mut first = task("First", 0, "", "");
        first.tag_names = "work\0تعلم".into();
        let mut second = task("Second", 0, "", "");
        second.tag_names = "work\0one, two".into();
        let views = task_views(&projects, &model(vec![first, second]));
        assert!(views.iter().any(|v| v.id == "custom"));
        assert!(!views.iter().any(|v| v.id == "inbox"));
        assert_eq!(views.iter().filter(|v| v.id == "@tag:work").count(), 1);
        assert!(views.iter().any(|v| v.id == "@tag:one, two"));
        assert!(views.iter().any(|v| v.id == "@tag:تعلم" && v.rtl));
        assert_eq!(views.iter().filter(|v| v.heading).count(), 2);
    }

    #[test]
    fn focus_body_keeps_notes_and_checklist_without_technical_metadata() {
        let task = task_model(serde_json::from_value(serde_json::json!({
            "id": "focus", "projectId": "inbox", "title": "Write the plan",
            "content": "First draft", "desc": "Review the outline",
            "priority": 5, "status": 0, "kind": "TEXT", "createdTime": "2026-10-28T10:00:00+0000",
            "items": [{"id":"step", "title":"Gather notes", "status":0}]
        })).unwrap());
        assert_eq!(
            task.focus_notes.as_str(),
            "First draft\n\nReview the outline\n\nChecklist\n☐ Gather notes"
        );
        assert!(self::task("Empty", 0, "", "").focus_notes.is_empty());
    }

    #[test]
    fn priority_then_oldest_displayed_date() {
        let mut tasks = [
            task("personal projects", 5, "", "2026-10-04"),
            task("no date", 5, "", ""),
            task("low priority", 1, "2026-01-01", ""),
            task("make an app", 5, "2026-03-13", ""),
        ];
        tasks.sort_by(compare_tasks);
        assert_eq!(
            tasks.map(|t| t.title.to_string()),
            [
                "make an app",
                "personal projects",
                "no date",
                "low priority"
            ]
        );
    }

    #[test]
    fn today_includes_overdue_and_excludes_future_undated_and_completed() {
        let today = Local::now().date_naive();
        for (date, done, expected) in [
            (Some(today - chrono::Duration::days(1)), false, true),
            (Some(today), false, true),
            (Some(today + chrono::Duration::days(1)), false, false),
            (None, false, false),
            (Some(today), true, false),
        ] {
            let mut task = task(
                "task",
                5,
                &date.map(|d| d.to_string()).unwrap_or_default(),
                "",
            );
            task.done = done;
            assert_eq!(matches_view(&task, "@today", false), expected);
        }
    }
}

fn record_completions(ui: &Launcher, tasks: &[ticktick::Task]) {
    let items = tasks
        .iter()
        .filter(|task| task.status == 2)
        .filter_map(|task| {
            let date = chrono::DateTime::parse_from_rfc3339(&task.completed_time)
                .or_else(|_| {
                    chrono::DateTime::parse_from_str(&task.completed_time, "%Y-%m-%dT%H:%M:%S%.f%z")
                })
                .ok()?
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string();
            Some(crate::CompletedReference {
                id: format!("ticktick:{}", task.id).into(),
                title: task.title.as_str().into(),
                date: date.into(),
                done: true,
            })
        })
        .collect::<Vec<_>>();
    if !items.is_empty() {
        ui.invoke_record_widget_completions(model(items));
    }
}
