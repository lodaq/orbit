use super::*;
use crate::robot_data::{McpServer, Model, Provider, Role};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

struct Fixture {
    root: PathBuf,
    handle: Option<Handle>,
    events: Receiver<Event>,
}

impl Fixture {
    fn new(address: String, skills: bool, mcp: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "orbit-runtime-{}-{}",
            std::process::id(),
            RUN_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::open(&root.join("robots.db")).unwrap();
        db.save_role(&Role {
            id: "role".into(),
            name: "باحث".into(),
            instructions: "Original Role instructions".into(),
        })
        .unwrap();
        let mut profile = Profile {
            id: "robot".into(),
            name: "Robot".into(),
            project: root.to_string_lossy().into_owned(),
            model: Model {
                provider: Provider::Ollama,
                id: "original-model".into(),
                ollama_address: Some(address),
            },
            role_id: Some("role".into()),
            skills: vec![],
            mcp_servers: vec![],
        };
        if skills {
            std::fs::write(
                root.join("SKILL.md"),
                "# Research\nWrite a sourced Markdown report in Arabic.",
            )
            .unwrap();
            profile
                .skills
                .push(root.join("SKILL.md").to_string_lossy().into_owned());
        }
        if mcp {
            let script = r#"import json,sys
for line in sys.stdin:
 r=json.loads(line)
 if 'id' not in r: continue
 method=r['method']
 if method=='initialize': result={'protocolVersion':'2025-06-18','capabilities':{'tools':{}}}
 elif method=='tools/list': result={'tools':[{'name':'research','description':'Find sources','inputSchema':{'type':'object'}}]}
 elif method=='tools/call':
  with open('mcp-calls.txt','a') as f: f.write(r['params']['name']+'\n')
  result={'content':[{'type':'text','text':'Source: https://example.com/research'}]}
 else: raise ValueError(method)
 print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#;
            profile.mcp_servers.push(McpServer {
                id: "sources".into(),
                command: "python3".into(),
                arguments: vec!["-u".into(), "-c".into(), script.into()],
                secret_env: BTreeMap::new(),
                enabled_tools: vec!["research".into()],
            });
        }
        db.save_profile(&profile).unwrap();
        let (handle, events) = start(root.join("robots.db"));
        Self {
            root,
            handle: Some(handle),
            events,
        }
    }
    fn handle(&self) -> &Handle {
        self.handle.as_ref().unwrap()
    }
    fn next(&self) -> Event {
        self.events
            .recv_timeout(Duration::from_secs(8))
            .expect("Robot event timed out")
    }
    fn approval(&self) -> (String, Approval) {
        loop {
            match self.next() {
                Event::Approval {
                    conversation,
                    request,
                } => return (conversation, request),
                Event::Finished { error, .. } => panic!("Stopped before approval: {error:?}"),
                _ => {}
            }
        }
    }
    fn finished(&self, run: &str) -> RunStatus {
        loop {
            if let Event::Finished {
                run: id,
                status,
                error,
                ..
            } = self.next()
            {
                assert_eq!(id, run);
                assert!(error.is_none(), "{error:?}");
                return status;
            }
        }
    }
    fn conversation(&self) -> crate::robot_data::Conversation {
        Database::open(&self.root.join("robots.db"))
            .unwrap()
            .conversation("conversation")
            .unwrap()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.handle.take());
        while self.events.recv_timeout(Duration::from_secs(3)).is_ok() {}
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn model(text: &str, calls: Vec<Value>) -> Value {
    json!({"done":true,"message":{"role":"assistant","content":text,"tool_calls":calls}})
}
fn call(name: &str, arguments: Value) -> Value {
    json!({"function":{"name":name,"arguments":arguments}})
}
fn capabilities() -> Value {
    json!({"capabilities":["completion","tools"]})
}

fn server(
    replies: Vec<Value>,
    gate: Option<Receiver<()>>,
) -> (String, std::thread::JoinHandle<Vec<Value>>, Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (arrived, arrivals) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut requests = Vec::new();
        let count = replies.len();
        for (index, reply) in replies.into_iter().enumerate() {
            let deadline = Instant::now() + Duration::from_secs(8);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("Missing model request {index}: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
                assert!(header.len() < 65536);
            }
            let header = String::from_utf8(header).unwrap();
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|s| s.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            requests.push(serde_json::from_slice(&body).unwrap());
            let _ = arrived.send(());
            if index + 1 == count
                && let Some(gate) = &gate
            {
                gate.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            let body = reply.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
        requests
    });
    (address, worker, arrivals)
}

#[test]
fn queue_waits_for_exact_approval_and_resumes_with_saved_context() {
    let (address, server, _) = server(
        vec![
            capabilities(),
            model(
                "",
                vec![call(
                    "write_file",
                    json!({"path":"report.md","text":"مرحبا"}),
                )],
            ),
            model("Saved", vec![]),
            capabilities(),
            model("Second complete", vec![]),
        ],
        None,
    );
    let f = Fixture::new(address, false, false);
    let first = f
        .handle()
        .submit("conversation", "robot", "Write report")
        .unwrap();
    let (_, approval) = f.approval();
    assert!(!f.root.join("report.md").exists());
    f.handle()
        .approve("wrong-conversation", approval.id, true)
        .unwrap();
    assert!(matches!(f.next(), Event::Error { .. }));
    assert!(!f.root.join("report.md").exists());
    let second = f
        .handle()
        .submit("conversation", "robot", "Second command")
        .unwrap();
    assert!(matches!(f.next(), Event::Queued { .. }));
    let db = Database::open(&f.root.join("robots.db")).unwrap();
    let mut profile = db.profile("robot").unwrap();
    profile.model.id = "changed-model".into();
    db.save_profile(&profile).unwrap();
    db.save_role(&Role {
        id: "role".into(),
        name: "Changed".into(),
        instructions: "Changed Role".into(),
    })
    .unwrap();
    f.handle()
        .approve("conversation", approval.id, true)
        .unwrap();
    assert_eq!(f.finished(&first), RunStatus::Completed);
    assert_eq!(f.finished(&second), RunStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(f.root.join("report.md")).unwrap(),
        "مرحبا"
    );
    let requests = server.join().unwrap();
    assert!(!requests[2].to_string().contains("Second command"));
    assert!(
        requests[2]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["role"] == "tool")
    );
    assert_eq!(requests[4]["model"], "original-model");
    assert!(
        requests[4]["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Original Role")
    );
    assert!(!requests[4].to_string().contains("Changed Role"));
    assert_eq!(
        f.conversation()
            .entries
            .iter()
            .filter(|e| matches!(
                e,
                Entry::Run {
                    status: RunStatus::Completed,
                    ..
                }
            ))
            .count(),
        2
    );
}

#[test]
fn denial_cancellation_and_stale_approval_never_write_files() {
    let write = call("write_file", json!({"path":"never.md","text":"never"}));
    let (address, server, _) = server(
        vec![
            capabilities(),
            model("", vec![write.clone()]),
            model("Action denied", vec![]),
            capabilities(),
            model("", vec![write]),
        ],
        None,
    );
    let f = Fixture::new(address, false, false);
    let first = f.handle().submit("conversation", "robot", "First").unwrap();
    let (_, approval) = f.approval();
    f.handle()
        .approve("conversation", approval.id, false)
        .unwrap();
    assert_eq!(f.finished(&first), RunStatus::Completed);
    let second = f
        .handle()
        .submit("conversation", "robot", "Second")
        .unwrap();
    let (_, approval) = f.approval();
    f.handle().cancel("conversation").unwrap();
    assert_eq!(f.finished(&second), RunStatus::Cancelled);
    f.handle()
        .approve("conversation", approval.id, true)
        .unwrap();
    assert!(matches!(f.next(), Event::Error { .. }));
    assert!(!f.root.join("never.md").exists());
    assert!(f.conversation().entries.iter().any(|e| matches!(
        e,
        Entry::Tool {
            status: ToolStatus::Denied,
            ..
        }
    )));
    server.join().unwrap();
}

#[test]
fn enabled_mcp_runs_automatically_and_skill_and_report_require_approval() {
    let report = "# بحث\n\n[Source](https://example.com/research)\n";
    let (address, server, _) = server(
        vec![
            capabilities(),
            model(
                "",
                vec![call(
                    "mcp",
                    json!({"server":"sources","tool":"research","arguments":{}}),
                )],
            ),
            model(
                "",
                vec![call(
                    "write_file",
                    json!({"path":"research.md","text":report}),
                )],
            ),
            model("Report saved", vec![]),
        ],
        None,
    );
    let f = Fixture::new(address, true, true);
    let run = f
        .handle()
        .submit(
            "conversation",
            "robot",
            "Follow the Skill and save a sourced report",
        )
        .unwrap();
    let (_, skill) = f.approval();
    assert!(matches!(skill.action, LocalAction::Skill { .. }));
    f.handle().approve("conversation", skill.id, true).unwrap();
    let (_, write) = f.approval();
    assert!(matches!(write.action, LocalAction::Write { .. }));
    assert_eq!(
        std::fs::read_to_string(f.root.join("mcp-calls.txt")).unwrap(),
        "research\n"
    );
    assert!(!f.root.join("research.md").exists());
    f.handle().approve("conversation", write.id, true).unwrap();
    assert_eq!(f.finished(&run), RunStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(f.root.join("research.md")).unwrap(),
        report
    );
    let requests = server.join().unwrap();
    assert!(
        requests[1]
            .to_string()
            .contains("Write a sourced Markdown report in Arabic")
    );
    assert!(
        requests[2]
            .to_string()
            .contains("https://example.com/research")
    );
    assert!(
        f.conversation()
            .entries
            .iter()
            .any(|e| matches!(e,Entry::Tool{name,status:ToolStatus::Completed,..} if name=="mcp"))
    );
}

#[test]
fn dropping_observer_does_not_cancel_background_completion() {
    let (gate, wait) = mpsc::channel();
    let (address, server, _) = server(
        vec![capabilities(), model("Background reply", vec![])],
        Some(wait),
    );
    let mut f = Fixture::new(address, false, false);
    let run = f
        .handle()
        .submit("conversation", "robot", "Finish in background")
        .unwrap();
    while !matches!(f.next(), Event::Changed { .. }) {}
    let (_, replacement) = mpsc::channel();
    drop(std::mem::replace(&mut f.events, replacement));
    gate.send(()).unwrap();
    server.join().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if f.conversation()
            .entries
            .iter()
            .any(|e| matches!(e,Entry::Run{id,status:RunStatus::Completed} if id==&run))
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn restart_marks_pending_work_interrupted_without_replay() {
    let (address, server, _) = server(
        vec![capabilities(), model("Only new command", vec![])],
        None,
    );
    let f = Fixture::new(address, false, false);
    let mut db = Database::open(&f.root.join("robots.db")).unwrap();
    db.enqueue("conversation", "robot", "old", "Do not replay", 1)
        .unwrap();
    let run = f
        .handle()
        .submit("conversation", "robot", "New command")
        .unwrap();
    assert_eq!(f.finished(&run), RunStatus::Completed);
    assert!(
        f.conversation()
            .entries
            .iter()
            .any(|e| matches!(e,Entry::Run{id,status:RunStatus::Interrupted} if id=="old"))
    );
    assert_eq!(server.join().unwrap().len(), 2);
    db.snapshot().unwrap().validate().unwrap();
}

#[test]
fn deleting_a_conversation_while_waiting_prevents_the_approved_side_effect() {
    let (address, server, _) = server(
        vec![
            capabilities(),
            model(
                "",
                vec![call(
                    "write_file",
                    json!({"path":"never.md","text":"never"}),
                )],
            ),
        ],
        None,
    );
    let f = Fixture::new(address, false, false);
    let run = f
        .handle()
        .submit("conversation", "robot", "Write a file")
        .unwrap();
    let (_, approval) = f.approval();
    Database::open(&f.root.join("robots.db"))
        .unwrap()
        .delete_conversation("conversation")
        .unwrap();
    f.handle()
        .approve("conversation", approval.id, true)
        .unwrap();
    loop {
        if let Event::Finished {
            run: id,
            status,
            error,
            ..
        } = f.next()
        {
            assert_eq!(id, run);
            assert_eq!(status, RunStatus::Failed);
            assert!(error.is_some());
            break;
        }
    }
    assert!(!f.root.join("never.md").exists());
    server.join().unwrap();
}

#[test]
fn a_second_robot_cannot_execute_while_the_first_waits_for_approval() {
    let (address, server, _) = server(
        vec![
            capabilities(),
            model(
                "",
                vec![call(
                    "write_file",
                    json!({"path":"first.md","text":"first"}),
                )],
            ),
            model("First done", vec![]),
            capabilities(),
            model("Second done", vec![]),
        ],
        None,
    );
    let f = Fixture::new(address, false, false);
    let db = Database::open(&f.root.join("robots.db")).unwrap();
    let mut profile = db.profile("robot").unwrap();
    profile.id = "other-robot".into();
    db.save_profile(&profile).unwrap();
    let first = f.handle().submit("conversation", "robot", "First").unwrap();
    let (_, approval) = f.approval();
    let second = f
        .handle()
        .submit("other-conversation", "other-robot", "Second")
        .unwrap();
    assert!(matches!(f.next(), Event::Queued { .. }));
    assert!(
        !db.conversation("other-conversation")
            .unwrap()
            .unwrap()
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Run { .. }))
    );
    f.handle()
        .approve("conversation", approval.id, true)
        .unwrap();
    assert_eq!(f.finished(&first), RunStatus::Completed);
    assert_eq!(f.finished(&second), RunStatus::Completed);
    server.join().unwrap();
}

#[test]
fn cancellation_during_a_model_request_discards_its_tool_calls() {
    let (release, gate) = mpsc::channel();
    let (address, server, arrived) = server(
        vec![
            capabilities(),
            model(
                "",
                vec![call(
                    "write_file",
                    json!({"path":"never.md","text":"never"}),
                )],
            ),
        ],
        Some(gate),
    );
    let f = Fixture::new(address, false, false);
    let run = f
        .handle()
        .submit("conversation", "robot", "Write a file")
        .unwrap();
    for _ in 0..2 {
        arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    f.handle().cancel("conversation").unwrap();
    release.send(()).unwrap();
    loop {
        match f.next() {
            Event::Approval { .. } => panic!("Cancelled request asked for approval"),
            Event::Finished {
                run: id, status, ..
            } => {
                assert_eq!(id, run);
                assert_eq!(status, RunStatus::Cancelled);
                break;
            }
            _ => {}
        }
    }
    assert!(!f.root.join("never.md").exists());
    assert!(
        !f.conversation()
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Tool { .. }))
    );
    server.join().unwrap();
}

#[test]
fn revoking_mcp_tools_while_waiting_prevents_automatic_execution() {
    let (address, server, _) = server(
        vec![
            capabilities(),
            model(
                "",
                vec![call(
                    "mcp",
                    json!({"server":"sources","tool":"research","arguments":{}}),
                )],
            ),
        ],
        None,
    );
    let f = Fixture::new(address, true, true);
    let run = f
        .handle()
        .submit("conversation", "robot", "Research")
        .unwrap();
    let (_, approval) = f.approval();
    let db = Database::open(&f.root.join("robots.db")).unwrap();
    let mut profile = db.profile("robot").unwrap();
    profile.mcp_servers[0].enabled_tools.clear();
    db.save_profile(&profile).unwrap();
    f.handle()
        .approve("conversation", approval.id, true)
        .unwrap();
    loop {
        if let Event::Finished {
            run: id,
            status,
            error,
            ..
        } = f.next()
        {
            assert_eq!(id, run);
            assert_eq!(status, RunStatus::Failed);
            assert!(error.unwrap().contains("settings changed"));
            break;
        }
    }
    assert!(!f.root.join("mcp-calls.txt").exists());
    server.join().unwrap();
}

#[test]
fn rejected_submission_identifies_only_its_run_and_shutdown_closes_storage() {
    let (address, server, _) = server(vec![capabilities(), model("Ready", vec![])], None);
    let mut f = Fixture::new(address, false, false);
    let rejected = f.handle().submit("rejected", "missing", "Hello").unwrap();
    match f.next() {
        Event::Rejected {
            run, conversation, ..
        } => {
            assert_eq!(run, rejected);
            assert_eq!(conversation, "rejected");
        }
        other => panic!("Expected a rejected command, got {other:?}"),
    }
    let accepted = f.handle().submit("conversation", "robot", "Hello").unwrap();
    assert_eq!(f.finished(&accepted), RunStatus::Completed);
    f.handle.take().unwrap().shutdown();
    assert!(
        !Database::open(&f.root.join("robots.db"))
            .unwrap()
            .history("robot", 0)
            .unwrap()
            .is_empty()
    );
    server.join().unwrap();
}
