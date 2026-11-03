use crate::Launcher;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slint::ComponentHandle;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub provider: i32,
    pub address: String,
    pub local_model: String,
    pub openai_model: String,
    pub gemini_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: 0,
            address: "http://127.0.0.1:11434".into(),
            local_model: "qwen3:4b".into(),
            openai_model: String::new(),
            gemini_model: String::new(),
        }
    }
}

impl Settings {
    pub fn valid(&self) -> Result<(), String> {
        let local = self
            .address
            .strip_prefix("http://127.0.0.1:")
            .or_else(|| self.address.strip_prefix("http://localhost:"))
            .and_then(|port| port.parse::<u16>().ok())
            .is_some_and(|p| p > 0);
        if !(0..=2).contains(&self.provider)
            || (self.provider == 0 && (!local || self.local_model.trim().is_empty()))
        {
            return Err("Choose a local model and a loopback Ollama address.".into());
        }
        if [&self.openai_model, &self.gemini_model]
            .iter()
            .any(|model| {
                !model
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.:".contains(c))
            })
        {
            return Err(
                "Enter a model ID containing letters, numbers, dots, colons or hyphens.".into(),
            );
        }
        Ok(())
    }
    pub fn from_ui(ui: &Launcher) -> Self {
        Self {
            provider: ui.get_ai_provider(),
            address: ui.get_ai_address().into(),
            local_model: ui.get_ai_local_model().into(),
            openai_model: ui.get_ai_openai_model().into(),
            gemini_model: ui.get_ai_gemini_model().into(),
        }
    }
    pub fn update_ui(self, ui: &Launcher) {
        ui.set_ai_address(self.address.into());
        ui.set_ai_local_model(self.local_model.into());
        ui.set_ai_openai_model(self.openai_model.into());
        ui.set_ai_gemini_model(self.gemini_model.into());
        ui.set_ai_provider(self.provider);
    }
}

fn request_body(settings: &Settings, prompt: &str) -> Result<Value, String> {
    settings.valid()?;
    if prompt.trim().is_empty() {
        return Err("Enter a prompt.".into());
    }
    match settings.provider {
        0 => Ok(
            json!({"model":settings.local_model,"messages":[{"role":"user","content":prompt}],"stream":false,"keep_alive":"5m"}),
        ),
        1 if !settings.openai_model.is_empty() => {
            Ok(json!({"model":settings.openai_model,"input":prompt,"store":false,"stream":false}))
        }
        2 if !settings.gemini_model.is_empty() => {
            Ok(json!({"contents":[{"role":"user","parts":[{"text":prompt}]}]}))
        }
        _ => Err("Choose a cloud model in Settings → Search bar → AI.".into()),
    }
}

fn answer(provider: i32, value: Value) -> Result<String, String> {
    let text = match provider {
        0 => value["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        1 => {
            if value["status"] != "completed" {
                return Err(
                    "The model did not complete an answer. Try a shorter prompt or another model."
                        .into(),
                );
            }
            value["output"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| item["type"] == "message")
                .flat_map(|item| item["content"].as_array().into_iter().flatten())
                .filter(|part| part["type"] == "output_text")
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => {
            let candidate = &value["candidates"][0];
            if candidate["finishReason"] != "STOP" {
                return Err(
                    "Gemini returned no complete answer; the request may be blocked or too long."
                        .into(),
                );
            }
            candidate["content"]["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|part| part["thought"] != true)
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
    };
    if text.trim().is_empty() {
        Err("The model returned no text answer. Try another prompt or model.".into())
    } else {
        Ok(text)
    }
}

fn http_error(status: u16) -> String {
    match status {
        401 | 403 => "API key rejected or model access denied. Check the key in AI settings.",
        404 => "Selected model is unavailable. Check the model ID in AI settings.",
        429 => "Provider rate or billing limit reached. Check your account and retry later.",
        _ => "AI provider request failed. Check the connection and try again.",
    }
    .into()
}

pub(crate) fn client(seconds: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(seconds)))
        .max_redirects(0)
        .build()
        .into()
}

pub(crate) fn read_json(mut response: ureq::http::Response<ureq::Body>) -> Result<Value, String> {
    response
        .body_mut()
        .with_config()
        .limit(4 * 1024 * 1024)
        .read_json()
        .map_err(|_| "Invalid or oversized AI response.".into())
}

pub(crate) fn network_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(status) => http_error(status),
        ureq::Error::Timeout(_) => {
            "AI request timed out. Try a shorter prompt or another model.".into()
        }
        _ => "Couldn't reach the AI provider. Check the connection and AI settings.".into(),
    }
}

pub(crate) fn credential(provider: i32) -> Result<keyring::Entry, String> {
    #[cfg(target_os = "linux")]
    crate::ticktick::activate_secret_service()?;
    let name = match provider {
        1 => "openai",
        2 => "gemini",
        _ => return Err("Select OpenAI or Gemini first.".into()),
    };
    keyring::Entry::new("orbit-ai", name)
        .map_err(|_| "Couldn't open the system credential store.".into())
}

fn generate(settings: &Settings, prompt: &str) -> Result<String, String> {
    let body = request_body(settings, prompt)?;
    let agent = client(120);
    let response = match settings.provider {
        0 => {
            let info = read_json(client(10).post(&format!("{}/api/show", settings.address))
                .send_json(json!({"model":settings.local_model})).map_err(network_error)?)?;
            if ["remote_host", "remote_model"].iter().any(|key| info[key].as_str().is_some_and(|s| !s.is_empty())) {
                return Err("Choose a local Ollama model. Use OpenAI or Gemini for cloud requests.".into());
            }
            agent.post(&format!("{}/api/chat", settings.address)).send_json(body)
        }
        provider => {
            let key = credential(provider)?.get_password().map_err(|_| "Add an API key in Settings → Search bar → AI.".to_owned())?;
            if key.trim().is_empty() { return Err("Add an API key in AI settings.".into()); }
            if provider == 1 {
                agent.post("https://api.openai.com/v1/responses").header("Authorization", format!("Bearer {key}")).send_json(body)
            } else {
                agent.post(&format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", settings.gemini_model))
                    .header("x-goog-api-key", key).send_json(body)
            }
        }
    }.map_err(network_error)?;
    answer(settings.provider, read_json(response)?)
}

fn local_models(settings: &Settings) -> Result<Vec<String>, String> {
    let local = Settings {
        provider: 0,
        ..settings.clone()
    };
    local.valid()?;
    let value = read_json(
        client(10)
            .get(&format!("{}/api/tags", local.address))
            .call()
            .map_err(network_error)?,
    )?;
    let mut models = Vec::new();
    for model in value["models"]
        .as_array()
        .ok_or("Invalid Ollama model list.")?
    {
        let Some(name) = model["name"].as_str() else {
            continue;
        };
        let info = read_json(
            client(10)
                .post(&format!("{}/api/show", local.address))
                .send_json(json!({"model":name}))
                .map_err(network_error)?,
        )?;
        if info["capabilities"]
            .as_array()
            .is_some_and(|items| items.iter().any(|v| v == "completion"))
            && !["remote_host", "remote_model"]
                .iter()
                .any(|key| info[key].as_str().is_some_and(|s| !s.is_empty()))
        {
            models.push(name.to_owned());
        }
    }
    Ok(models)
}

enum Work {
    Generate(u64, Settings, String, String),
    Models(Settings),
    Key(i32, Option<String>),
}

fn deliver(
    ui: &Launcher,
    current: &AtomicU64,
    id: u64,
    request: (Settings, String, String),
    result: Result<String, String>,
) {
    let (settings, prompt, prefix) = request;
    // Slint change callbacks are deferred; inspect live state before accepting a reply.
    if current.load(Ordering::Relaxed) != id
        || !ui.get_launcher_visible()
        || ui.get_settings()
        || Settings::from_ui(ui) != settings
        || ui.get_search_text() != prompt
        || ui.get_activated_prefix_id() != prefix
    {
        return;
    }
    ui.set_ai_loading(false);
    match result {
        Ok(text) => {
            ui.set_ai_answer(text.into());
            ui.set_ai_status("".into());
        }
        Err(error) => ui.set_ai_status(error.into()),
    }
}

fn update_models(ui: &Launcher, address: &str, result: Result<Vec<String>, String>) {
    ui.set_ai_models_loading(false);
    if ui.get_ai_address() != address {
        return;
    }
    match result {
        Ok(models) => {
            let index = models
                .iter()
                .position(|m| m == ui.get_ai_local_model().as_str())
                .unwrap_or(0);
            ui.set_ai_local_models(
                std::rc::Rc::new(slint::VecModel::from(
                    models
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
            ui.set_ai_local_index(index as i32);
            ui.set_ai_settings_status("Local models refreshed".into());
        }
        Err(error) => ui.set_ai_settings_status(error.into()),
    }
}

fn update_key(provider: i32, value: Option<String>) -> Result<(), String> {
    let entry = credential(provider)?;
    match value {
        Some(key) => entry
            .set_password(&key)
            .map_err(|_| "Couldn't save the API key in the system credential store.".into()),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("Couldn't remove the API key from the system credential store.".into()),
        },
    }
}

fn run(work: Work, weak: &slint::Weak<Launcher>, current: &Arc<AtomicU64>) {
    match work {
        Work::Generate(id, settings, prompt, prefix) => {
            if current.load(Ordering::Relaxed) != id {
                return;
            }
            let result = generate(&settings, &prompt);
            let current = current.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                deliver(&ui, &current, id, (settings, prompt, prefix), result)
            });
        }
        Work::Models(settings) => {
            let result = local_models(&settings);
            let _ =
                weak.upgrade_in_event_loop(move |ui| update_models(&ui, &settings.address, result));
        }
        Work::Key(provider, value) => {
            let result = update_key(provider, value);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_ai_key_busy(false);
                ui.set_ai_settings_status(
                    result
                        .err()
                        .unwrap_or_else(|| "API key updated".into())
                        .into(),
                );
            });
        }
    }
}

pub fn connect(ui: &Launcher) {
    let generation = Arc::new(AtomicU64::new(0));
    let (send, receive) = mpsc::channel();
    let weak = ui.as_weak();
    let current = generation.clone();
    std::thread::spawn(move || {
        for work in receive {
            run(work, &weak, &current);
        }
    });
    request_callbacks(ui, send.clone(), generation);
    settings_callbacks(ui, send);
}

fn request_callbacks(ui: &Launcher, send: mpsc::Sender<Work>, generation: Arc<AtomicU64>) {
    let weak = ui.as_weak();
    let current = generation.clone();
    let sender = send.clone();
    ui.on_ai_submit(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let settings = Settings::from_ui(&ui);
        let prompt = ui.get_search_text().to_string();
        ui.set_ai_answer("".into());
        if let Err(error) = request_body(&settings, &prompt) {
            ui.set_ai_status(error.into());
            return;
        }
        let id = current.fetch_add(1, Ordering::Relaxed) + 1;
        ui.set_ai_status("".into());
        ui.set_ai_loading(true);
        let _ = sender.send(Work::Generate(
            id,
            settings,
            prompt,
            ui.get_activated_prefix_id().to_string(),
        ));
    });
    let weak = ui.as_weak();
    ui.on_ai_cancel(move || {
        generation.fetch_add(1, Ordering::Relaxed);
        if let Some(ui) = weak.upgrade() {
            ui.set_ai_loading(false);
        }
    });
}

fn settings_callbacks(ui: &Launcher, send: mpsc::Sender<Work>) {
    let weak = ui.as_weak();
    let sender = send.clone();
    ui.on_ai_refresh_models(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_ai_models_loading() {
                return;
            }
            ui.set_ai_models_loading(true);
            ui.set_ai_settings_status("Refreshing local models…".into());
            let _ = sender.send(Work::Models(Settings::from_ui(&ui)));
        }
    });
    let weak = ui.as_weak();
    ui.on_ai_store_key(move |remove| {
        if let Some(ui) = weak.upgrade() {
            let key = ui.get_ai_key_draft().to_string();
            ui.set_ai_key_draft("".into());
            if ui.get_ai_provider() == 0 || (!remove && key.trim().is_empty()) {
                return;
            }
            ui.set_ai_key_busy(true);
            let _ = send.send(Work::Key(
                ui.get_ai_provider(),
                if remove { None } else { Some(key) },
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mock_server(replies: Vec<(u16, Value)>) -> (String, std::thread::JoinHandle<Vec<Value>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, reply) in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    header.push(byte[0]);
                }
                let header = String::from_utf8(header).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|s| s.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; length];
                stream.read_exact(&mut body).unwrap();
                requests.push(if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(&body).unwrap()
                });
                let body = reply.to_string();
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (address, thread)
    }

    #[test]
    fn local_http_requests_reject_cloud_models_and_preserve_unicode_prompts() {
        let (address, server) = mock_server(vec![
            (200, json!({"capabilities":["completion"]})),
            (200, json!({"message":{"content":"Hello"}})),
        ]);
        let settings = Settings {
            address,
            ..Default::default()
        };
        assert_eq!(generate(&settings, "مرحبا 🌍").unwrap(), "Hello");
        let requests = server.join().unwrap();
        assert_eq!(requests[1]["messages"][0]["content"], "مرحبا 🌍");
        let (address, server) = mock_server(vec![(200, json!({"remote_model":"cloud-model"}))]);
        assert!(
            generate(
                &Settings {
                    address,
                    ..settings
                },
                "private"
            )
            .unwrap_err()
            .contains("local")
        );
        assert_eq!(server.join().unwrap().len(), 1);
    }

    #[test]
    fn unresponsive_provider_times_out_without_exposing_request_data() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut byte = [0];
            let _ = stream.read(&mut byte);
            std::thread::sleep(Duration::from_millis(1100));
        });
        let error = client(1).get(&address).call().unwrap_err();
        assert!(network_error(error).contains("timed out"));
        server.join().unwrap();
    }

    #[test]
    fn http_failures_are_sanitized() {
        for status in [401, 403, 404, 429, 500] {
            let (address, server) =
                mock_server(vec![(status, json!({"error":"sensitive provider body"}))]);
            let error = client(2).get(&address).call().unwrap_err();
            assert_eq!(network_error(error), http_error(status));
            assert!(!http_error(status).contains("sensitive"));
            server.join().unwrap();
        }
    }

    #[test]
    fn cancelled_hidden_and_changed_requests_cannot_deliver_stale_replies() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Launcher::new().unwrap();
        ui.show().unwrap();
        ui.set_launcher_visible(true);
        let generation = Arc::new(AtomicU64::new(0));
        let (send, receive) = mpsc::channel();
        request_callbacks(&ui, send, generation.clone());
        ui.set_search_text("hello".into());
        ui.invoke_ai_submit();
        let Work::Generate(id, settings, prompt, prefix) = receive.recv().unwrap() else {
            panic!("expected request")
        };
        assert!(ui.get_ai_loading());
        ui.invoke_ai_cancel();
        deliver(
            &ui,
            &generation,
            id,
            (settings, prompt, prefix),
            Ok("stale".into()),
        );
        assert!(ui.get_ai_answer().is_empty());
        assert!(!ui.get_ai_loading());
        ui.invoke_ai_submit();
        let Work::Generate(id, settings, prompt, prefix) = receive.recv().unwrap() else {
            panic!("expected request")
        };
        ui.set_search_text("changed".into());
        deliver(
            &ui,
            &generation,
            id,
            (settings, prompt, prefix),
            Ok("stale edit".into()),
        );
        assert!(ui.get_ai_answer().is_empty());
        ui.invoke_ai_submit();
        let Work::Generate(id, settings, prompt, prefix) = receive.recv().unwrap() else {
            panic!("expected request")
        };
        ui.set_launcher_visible(false);
        deliver(
            &ui,
            &generation,
            id,
            (settings, prompt, prefix),
            Ok("stale hidden".into()),
        );
        assert!(ui.get_ai_answer().is_empty());
        ui.set_launcher_visible(true);
        ui.invoke_ai_submit();
        let Work::Generate(id, settings, prompt, prefix) = receive.recv().unwrap() else {
            panic!("expected request")
        };
        deliver(
            &ui,
            &generation,
            id,
            (settings, prompt, prefix),
            Ok("current".into()),
        );
        assert_eq!(ui.get_ai_answer(), "current");
    }

    #[test]
    fn requests_have_only_one_prompt_and_no_tools() {
        let settings = Settings::default();
        let body = request_body(&settings, "hello").unwrap();
        assert_eq!(body["messages"], json!([{"role":"user","content":"hello"}]));
        let settings = Settings {
            provider: 1,
            openai_model: "my-model".into(),
            ..settings
        };
        let body = request_body(&settings, "hello").unwrap();
        assert_eq!(body["store"], false);
        assert_eq!(body["input"], "hello");
        assert!(body.get("tools").is_none());
        let settings = Settings {
            provider: 2,
            gemini_model: "gemini-test".into(),
            ..settings
        };
        assert_eq!(
            request_body(&settings, "hello").unwrap()["contents"][0]["parts"][0]["text"],
            "hello"
        );
        assert!(request_body(&settings, "  ").is_err());
    }

    #[test]
    fn replies_exclude_reasoning_and_reject_empty_or_blocked_answers() {
        assert_eq!(
            answer(0, json!({"message":{"content":"Hi", "thinking":"private"}})).unwrap(),
            "Hi"
        );
        assert_eq!(answer(1, json!({"status":"completed","output":[{"type":"reasoning","summary":[]},{"type":"message","content":[{"type":"output_text","text":"Hi"}]}]})).unwrap(), "Hi");
        assert_eq!(answer(2, json!({"candidates":[{"content":{"parts":[{"text":"private","thought":true},{"text":"Hi"}]},"finishReason":"STOP"}]})).unwrap(), "Hi");
        assert!(answer(0, json!({"message":{"content":""}})).is_err());
        assert!(answer(1, json!({"status":"incomplete","output":[]})).is_err());
        assert!(answer(2, json!({"promptFeedback":{"blockReason":"SAFETY"}})).is_err());
    }

    #[test]
    fn validation_does_not_allow_remote_ollama_or_empty_cloud_models() {
        let mut settings = Settings::default();
        settings.address = "https://example.com".into();
        assert!(settings.valid().is_err());
        settings.provider = 1;
        assert!(request_body(&settings, "hello").is_err());
        settings.openai_model = "model".into();
        assert!(request_body(&settings, "hello").is_ok());
        settings.provider = 2;
        settings.gemini_model = "../model?key=secret".into();
        assert!(settings.valid().is_err());
    }

    #[test]
    fn errors_are_actionable_and_do_not_echo_provider_bodies() {
        assert!(http_error(401).contains("API key"));
        assert!(http_error(429).contains("limit"));
        assert!(http_error(404).contains("model"));
    }
}
