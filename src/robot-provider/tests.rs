use super::*;
use crate::robot_data::{Context, Role};

fn conversation(provider: Provider) -> Conversation {
    Conversation {
        id: "conversation".into(),
        robot_id: "robot".into(),
        title: "Hello".into(),
        context: Context {
            project: "/tmp/project".into(),
            role: Some(Role {
                id: "role".into(),
                name: "Role".into(),
                instructions: "Answer in Arabic".into(),
            }),
            model: Model {
                account: None,
                ollama_address: if provider == Provider::Ollama {
                    Some("http://127.0.0.1:11434".into())
                } else {
                    None
                },
                provider,
                id: "model".into(),
            },
        },
        created_at: 1,
        updated_at: 1,
        rating: None,
        entries: vec![Entry::Command {
            text: "مرحبا".into(),
        }],
    }
}

#[test]
fn native_requests_keep_context_and_function_results() {
    for provider in [Provider::Ollama, Provider::OpenAi, Provider::Gemini] {
        let mut session =
            Session::with_key(&conversation(provider.clone()), Some("secret-key".into()));
        let response = match provider {
            Provider::Ollama => {
                json!({"done":true,"message":{"role":"assistant","content":"","thinking":"opaque", "tool_calls":[{"function":{"name":"read_file","arguments":{"path":"a.md"}}}]}})
            }
            Provider::OpenAi => {
                json!({"status":"completed","output":[{"type":"reasoning","encrypted_content":"opaque","summary":[]},{"type":"function_call","call_id":"call-1","name":"read_file","arguments":"{\"path\":\"a.md\"}"}]})
            }
            Provider::Gemini => {
                json!({"candidates":[{"finishReason":"STOP","content":{"role":"model","parts":[{"functionCall":{"id":"call-1","name":"read_file","args":{"path":"a.md"}},"thoughtSignature":"opaque"}]}}]})
            }
            _ => unreachable!("Only connected providers are tested here."),
        };
        let turn = session.accept(response).unwrap();
        assert_eq!(turn.calls.len(), 1);
        assert_eq!(turn.calls[0].arguments["path"], "a.md");
        session.tool_result(&turn.calls[0], "محتوى الملف");
        let body = session.body();
        let serialized = body.to_string();
        assert!(serialized.contains("opaque"));
        assert!(serialized.contains("محتوى الملف"));
        assert!(serialized.contains("Answer in Arabic"));
        assert!(serialized.contains("مرحبا"));
        assert!(!serialized.contains("secret-key"));
        match provider {
            Provider::OpenAi => {
                assert_eq!(body["store"], false);
                assert_eq!(body["input"][3]["call_id"], "call-1");
                assert_eq!(body["tools"][0]["type"], "function");
            }
            Provider::Gemini => {
                assert_eq!(
                    body["contents"][1]["parts"][0]["thoughtSignature"],
                    "opaque"
                );
                assert_eq!(
                    body["contents"][2]["parts"][0]["functionResponse"]["id"],
                    "call-1"
                );
                assert!(
                    body["tools"][0]["functionDeclarations"][0]["parametersJsonSchema"].is_object()
                );
            }
            Provider::Ollama => assert_eq!(body["messages"][3]["tool_name"], "read_file"),
            _ => unreachable!("Only connected providers are tested here."),
        }
    }
}

#[test]
fn invalid_or_incomplete_calls_cannot_partially_advance_the_session() {
    let mut session = Session::with_key(&conversation(Provider::OpenAi), None);
    let before = session.body();
    for response in [
        json!({"status":"incomplete","output":[]}),
        json!({"status":"completed","output":[{"type":"function_call","name":"run","arguments":"{}"}]}),
        json!({"status":"completed","output":[{"type":"function_call","call_id":"c","name":"run","arguments":"not JSON"}]}),
        json!({"status":"completed","output":[{"type":"function_call","call_id":"c","name":"run","arguments":"{}"},{"type":"function_call","call_id":"c","name":"run","arguments":"{}"}]}),
    ] {
        assert!(session.accept(response).is_err());
        assert_eq!(session.body(), before);
    }
}

#[test]
fn future_queued_commands_are_not_sent_to_the_active_run() {
    let mut conversation = conversation(Provider::Ollama);
    conversation.entries = vec![
        Entry::Queued {
            id: "first".into(),
            text: "first command".into(),
        },
        Entry::Run {
            id: "first".into(),
            status: RunStatus::Running,
        },
        Entry::Queued {
            id: "later".into(),
            text: "do not send yet".into(),
        },
        Entry::Tool {
            name: "read_file".into(),
            input: json!({"path":"a"}),
            output: "prior result".into(),
            status: ToolStatus::Completed,
        },
    ];
    let session = Session::with_key(&conversation, None);
    let body = session.body().to_string();
    assert!(body.contains("first command"));
    assert!(body.contains("prior result"));
    assert!(!body.contains("do not send yet"));
}

#[test]
fn provider_keys_and_private_reasoning_do_not_become_visible_output() {
    let mut session =
        Session::with_key(&conversation(Provider::Gemini), Some("private-key".into()));
    let turn = session.accept(json!({"candidates":[{"finishReason":"STOP","content":{"role":"model","parts":[
        {"thought":true,"text":"private reasoning"},{"text":"Answer private-key"},
        {"functionCall":{"name":"write_file","args":{"path":"report.md","text":"private-key"}},"thoughtSignature":"opaque"}
    ]}}]})).unwrap();
    assert_eq!(turn.text, "Answer [redacted]");
    let mut displayed = turn.calls[0].arguments.clone();
    session.redact(&mut displayed);
    assert_eq!(displayed["text"], "[redacted]");
    assert_eq!(turn.calls[0].arguments["text"], "private-key");
}

#[test]
fn saved_native_accounts_report_unconnected_execution_without_running_programs() {
    for provider in [
        Provider::OpenCode,
        Provider::Antigravity,
        Provider::Copilot,
        Provider::Codex,
    ] {
        let mut conversation = conversation(provider.clone());
        conversation.context.model.account = Some(crate::robot_data::Account {
            id: "personal".into(),
            name: "Personal".into(),
            provider,
            address: None,
            credential: None,
            home: Some("/tmp/personal".into()),
            needs_reconnection: false,
            binding: None,
        });
        let error = Session::new(&conversation)
            .err()
            .expect("native execution must remain disabled");
        assert!(error.contains("not connected yet"), "{error}");
    }
}

#[test]
fn imported_accounts_cannot_use_existing_credentials_before_reconnection() {
    let mut conversation = conversation(Provider::OpenAi);
    conversation.context.model.account = Some(crate::robot_data::Account {
        id: "work".into(),
        name: "Work".into(),
        provider: Provider::OpenAi,
        address: None,
        credential: Some("work-key".into()),
        home: None,
        needs_reconnection: true,
        binding: None,
    });
    let error = Session::new(&conversation).err().unwrap();
    assert!(error.contains("Reconnect"));
}
