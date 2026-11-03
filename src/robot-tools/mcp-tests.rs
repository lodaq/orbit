use super::*;

#[test]
fn rejects_malformed_and_oversized_protocol_messages() {
    assert!(read_message(&mut &b"not json\n"[..]).is_err());
    assert!(read_message(&mut &b"{}"[..]).is_err());
    assert!(read_message(&mut &vec![b'x'; LIMIT + 1][..]).is_err());
    assert_eq!(read_message(&mut &b"{\"id\":1}\n"[..]).unwrap()["id"], 1);
}

#[cfg(unix)]
#[test]
fn stdio_initialization_pagination_permissions_errors_and_secret_redaction() {
    let script = r#"
import json, os, sys
for line in sys.stdin:
    m = json.loads(line)
    method = m.get('method')
    if method == 'notifications/initialized': continue
    if method == 'initialize':
        result = {'protocolVersion':'2025-06-18','capabilities':{'tools':{}}}
    elif method == 'tools/list':
        result = {'tools':[{'name':'echo' if not m['params'].get('cursor') else 'fail','inputSchema':{'type':'object'}}]}
        if not m['params'].get('cursor'): result['nextCursor'] = 'next'
    elif method == 'tools/call':
        if m['params']['name'] == 'fail':
            print(json.dumps({'jsonrpc':'2.0','id':m['id'],'error':{'code':-1,'message':os.environ['TEST_SECRET']}}), flush=True)
            continue
        result = {'content':[{'type':'text','text':os.environ['TEST_SECRET'] + '\nمرحبا'}]}
    print(json.dumps({'jsonrpc':'2.0','id':m['id'],'result':result}), flush=True)
"#;
    let server = McpServer {
        id: "fixture".into(),
        command: "python3".into(),
        arguments: vec!["-u".into(), "-c".into(), script.into()],
        secret_env: BTreeMap::new(),
        enabled_tools: vec!["echo".into(), "fail".into()],
    };
    let mut session = Session::start(
        &server,
        &std::env::temp_dir(),
        BTreeMap::from([("TEST_SECRET".into(), "secret-value\"with-quote".into())]),
    )
    .unwrap();
    assert_eq!(session.discover().unwrap().len(), 2);
    assert!(session.call(&server, "disabled", json!({})).is_err());
    let mut changed = server.clone();
    changed.enabled_tools.clear();
    assert!(session.call(&changed, "echo", json!({})).is_err());
    let output = session.call(&server, "echo", json!({})).unwrap();
    assert_eq!(output["content"][0]["text"], "[redacted]\nمرحبا");
    let error = session.call(&server, "fail", json!({})).unwrap_err();
    assert!(!error.contains("secret-value"));
    assert!(error.contains("[redacted]"));
    assert!(
        session
            .call(&server, "echo", json!({}))
            .unwrap_err()
            .contains("reconnect")
    );
}
