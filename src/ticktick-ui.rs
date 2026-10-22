use crate::{Launcher, TickTickProject, TickTickTask, ticktick};
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
        Request::PrefixSave(project, title, _, _, today, _) => {
            let date = Local::now().date_naive();
            optimistic.insert(
                0,
                TickTickTask {
                    id: "@pending".into(),
                    project_id: project.as_str().into(),
                    title: title.as_str().into(),
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
                        title: title.trim().into(),
                        content: content.as_str().into(),
                        desc: desc.as_str().into(),
                        ..Default::default()
                    },
                );
                ui.invoke_reset_ticktick_scroll();
            } else if let Some(task) = optimistic.iter_mut().find(|t| t.id == id) {
                task.title = title.trim().into();
                task.content = content.as_str().into();
                task.desc = desc.as_str().into();
                task.priority = *priority;
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
            let views = [
                TickTickProject {
                    id: "@today".into(),
                    name: "Today".into(),
                },
                TickTickProject {
                    id: "@week".into(),
                    name: "Next 7 Days".into(),
                },
                TickTickProject {
                    id: "".into(),
                    name: "Inbox (all tasks)".into(),
                },
            ];
            let mut views = views.to_vec();
            views.extend(projects.iter().filter(|p| !p.inbox).map(project_model));
            ui.set_ticktick_views(model(views));
            ui.set_ticktick_project_names(model(
                projects.iter().map(|p| p.name.as_str().into()).collect(),
            ));
            ui.set_ticktick_projects(model(projects.iter().map(project_model).collect()));
            ui.set_ticktick_tasks(model(tasks.into_iter().map(task_model).collect()));
            ui.set_ticktick_connected(true);
        }
        Response::List(project, tasks) => {
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
            ui.set_ticktick_views(model(Vec::new()));
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
    let details = details.join("\n");
    TickTickTask {
        id: task.id.into(),
        project_id: task.project_id.into(),
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
        tags: task.tags.join(", ").into(),
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
