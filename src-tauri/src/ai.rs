//! Provider transport and OS credentials. Never log request/response content or keys.
use git_toys_engine::types::{AiSettings, GitError, Result};
use reqwest::{Client, Url};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tokio::sync::oneshot;

#[derive(Default)]
pub struct Requests(pub Mutex<HashMap<String, oneshot::Sender<()>>>);
struct RequestGuard<'a> {
    requests: &'a Requests,
    id: String,
}
impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut requests) = self.requests.0.lock() {
            requests.remove(&self.id);
        }
    }
}
impl Requests {
    pub fn cancel(&self, id: &str) -> Result<()> {
        if let Some(sender) = self
            .0
            .lock()
            .map_err(|_| GitError::new("aiState", "AI state unavailable."))?
            .remove(id)
        {
            let _ = sender.send(());
        }
        Ok(())
    }
    pub async fn run<T>(
        &self,
        id: &str,
        task: impl std::future::Future<Output = Result<T>>,
    ) -> Result<T> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut requests = self
                .0
                .lock()
                .map_err(|_| GitError::new("aiState", "AI state unavailable."))?;
            if !requests.is_empty() {
                return Err(GitError::new(
                    "aiBusy",
                    "An AI request is already running. Cancel it or wait.",
                ));
            }
            requests.insert(id.into(), sender);
        }
        let _guard = RequestGuard {
            requests: self,
            id: id.into(),
        };
        tokio::select! {
            result = task => result,
            _ = receiver => Err(GitError::new("aiCancelled", "AI generation cancelled.")),
            _ = tokio::time::sleep(Duration::from_secs(120)) => Err(GitError::new("aiTimeout", "AI generation timed out after 120 seconds.")),
        }
    }
}
fn provider(provider: &str) -> Result<()> {
    if matches!(provider, "ollama" | "openai" | "anthropic") {
        Ok(())
    } else {
        Err(GitError::new(
            "aiProvider",
            "Choose Ollama, OpenAI, or Anthropic.",
        ))
    }
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn entry(provider_name: &str) -> Result<keyring::Entry> {
    provider(provider_name)?;
    if provider_name == "ollama" {
        return Err(GitError::new("aiKey", "Ollama does not need an API key."));
    }
    keyring::Entry::new("com.gittoys.desktop.ai", provider_name)
        .map_err(|_| GitError::new("aiKey", "OS credential storage is unavailable. Check your login keychain or Credential Manager."))
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn credential(provider: &str) -> Result<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(GitError::new(
            "aiKey",
            "Could not read the API key from OS credential storage.",
        )),
    }
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn save_credential(provider: &str, key: Option<String>) -> Result<()> {
    let entry = entry(provider)?;
    match key {
        Some(key) if !key.trim().is_empty() => entry.set_password(key.trim()).map_err(|_| {
            GitError::new(
                "aiKey",
                "Could not save the API key in OS credential storage.",
            )
        }),
        _ => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(GitError::new(
                "aiKey",
                "Could not remove the stored API key.",
            )),
        },
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn credential(_: &str) -> Result<Option<String>> {
    Err(GitError::new(
        "aiKey",
        "Cloud credentials are supported on macOS and Windows.",
    ))
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn save_credential(_: &str, _: Option<String>) -> Result<()> {
    Err(GitError::new(
        "aiKey",
        "Cloud credentials are supported on macOS and Windows.",
    ))
}

pub fn validate(settings: &AiSettings) -> Result<Url> {
    provider(&settings.provider)?;
    if !settings.enabled || settings.model.trim().is_empty() || settings.model.len() > 200 {
        return Err(GitError::new(
            "aiSetup",
            "Enable AI and enter a model in Settings → AI.",
        ));
    }
    let endpoint = match settings.provider.as_str() {
        "openai" => "https://api.openai.com/v1/responses".to_string(),
        "anthropic" => "https://api.anthropic.com/v1/messages".to_string(),
        _ => {
            let url = Url::parse(&settings.ollama_url)
                .map_err(|_| GitError::new("aiEndpoint", "Enter a valid local Ollama URL."))?;
            if !matches!(url.scheme(), "http" | "https")
                || !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                return Err(GitError::new(
                    "aiEndpoint",
                    "Ollama must use a loopback URL with no credentials, path, or query.",
                ));
            }
            format!("{}/api/chat", settings.ollama_url.trim_end_matches('/'))
        }
    };
    Url::parse(&endpoint).map_err(|_| GitError::new("aiEndpoint", "Invalid AI endpoint."))
}
fn payload(settings: &AiSettings, context: &str) -> Value {
    match settings.provider.as_str() {
        "openai" => {
            json!({"model": settings.model, "store": false, "input": context, "max_output_tokens": 1024})
        }
        "anthropic" => {
            json!({"model": settings.model, "max_tokens": 1024, "messages": [{"role":"user", "content":context}]})
        }
        _ => {
            json!({"model":settings.model,"stream":false,"messages":[{"role":"user","content":context}]})
        }
    }
}
fn extract(provider: &str, response: Value) -> Result<String> {
    let message = match provider {
        "openai" => response["output"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["type"] == "message")
            .flat_map(|item| item["content"].as_array().into_iter().flatten())
            .filter(|item| item["type"] == "output_text")
            .filter_map(|item| item["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        "anthropic" => response["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["type"] == "text")
            .filter_map(|item| item["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => response["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    };
    if message.trim().is_empty() || message.len() > 8192 || message.contains('\0') {
        return Err(GitError::new(
            "aiResponse",
            "The model returned an empty or oversized draft. Try a different model.",
        ));
    }
    Ok(message.trim().into())
}
fn transport_error(error: reqwest::Error) -> GitError {
    if error.is_timeout() {
        GitError::new(
            "aiTimeout",
            "AI timed out after 120 seconds. Retry or choose a smaller local model.",
        )
    } else {
        GitError::new("aiNetwork", "Could not reach the AI provider. Check its URL, internet connection, or local Ollama service.")
    }
}
async fn send(
    settings: &AiSettings,
    context: &str,
    key: Option<&str>,
    endpoint: Url,
) -> Result<String> {
    let builder = Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none());
    let client = if settings.provider == "ollama" {
        builder.no_proxy()
    } else {
        builder
    }
    .build()
    .map_err(transport_error)?;
    let mut request = client.post(endpoint).json(&payload(settings, context));
    if settings.provider != "ollama" {
        let key = key.ok_or_else(|| {
            GitError::new("aiKey", "Add this provider’s API key in Settings → AI.")
        })?;
        request = if settings.provider == "openai" {
            request.bearer_auth(key)
        } else {
            request
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
        };
    }
    let mut response = request.send().await.map_err(transport_error)?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        let (code, message) = match status {
            401 | 403 => ("aiAuthentication", "AI authentication failed. Check your API key and model access in Settings."),
            429 => ("aiRateLimit", "The AI provider rejected the request due to rate or quota limits. Check your provider account before retrying."),
            404 => ("aiModel", "Model or endpoint not found. Check the model name in Settings."),
            _ => ("aiProvider", "AI request failed. Check provider availability and model configuration, then retry."),
        };
        // Provider bodies can echo prompts or secrets. Never return them to notifications.
        return Err(GitError::new(code, message));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if bytes.len() + chunk.len() > 262_144 {
            return Err(GitError::new(
                "aiResponse",
                "AI response exceeded the size limit.",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice(&bytes)
        .map_err(|_| GitError::new("aiResponse", "AI provider returned an invalid response."))?;
    extract(&settings.provider, value)
}
pub async fn generate(settings: &AiSettings, context: &str) -> Result<String> {
    let endpoint = validate(settings)?;
    let key = if settings.provider == "ollama" {
        None
    } else {
        credential(&settings.provider)?
    };
    send(settings, context, key.as_deref(), endpoint).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[tokio::test]
    async fn cancellation_drops_request_and_releases_the_slot() {
        let requests = std::sync::Arc::new(Requests::default());
        let worker = requests.clone();
        let (started, ready) = oneshot::channel();
        let handle = tokio::spawn(async move {
            worker
                .run::<String>("cancel-test", async {
                    let _ = started.send(());
                    std::future::pending().await
                })
                .await
        });
        ready.await.unwrap();
        assert_eq!(
            requests
                .run("other", async { Ok(()) })
                .await
                .unwrap_err()
                .code,
            "aiBusy"
        );
        requests.cancel("cancel-test").unwrap();
        assert_eq!(handle.await.unwrap().unwrap_err().code, "aiCancelled");
        assert!(requests.0.lock().unwrap().is_empty());
        assert!(requests.run("next", async { Ok(()) }).await.is_ok());
    }
    #[tokio::test(start_paused = true)]
    async fn deadline_cleans_up_without_retrying() {
        let requests = Requests::default();
        let error = requests
            .run::<()>("timeout", std::future::pending())
            .await
            .unwrap_err();
        assert_eq!(error.code, "aiTimeout");
        assert!(requests.0.lock().unwrap().is_empty());
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    #[ignore = "Explicit native OS credential-store smoke test; writes and deletes a unique test credential"]
    fn native_credential_store_round_trip() {
        let account = format!("smoke-{}", uuid::Uuid::new_v4());
        let entry = keyring::Entry::new("com.gittoys.desktop.ai", &account).unwrap();
        entry
            .set_password("git-toys-temporary-smoke-value")
            .unwrap();
        let value = entry.get_password();
        let removed = entry.delete_credential();
        assert_eq!(value.unwrap(), "git-toys-temporary-smoke-value");
        removed.unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
    }
    #[test]
    fn settings_restrict_local_endpoints_and_require_explicit_setup() {
        let mut settings = AiSettings::default();
        assert!(validate(&settings).is_err());
        settings.enabled = true;
        settings.model = "local-model".into();
        assert!(validate(&settings).is_ok());
        for url in [
            "http://example.com",
            "file:///tmp/model",
            "http://user:pass@localhost:11434",
            "http://localhost:11434/redirect",
        ] {
            settings.ollama_url = url.into();
            assert!(validate(&settings).is_err());
        }
    }
    #[tokio::test]
    async fn provider_adapters_use_expected_wire_format_and_redact_failures() {
        for provider in ["ollama", "openai", "anthropic"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint =
                Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
            let provider_name = provider.to_string();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buf = [0; 4096];
                loop {
                    let n = socket.read(&mut buf).unwrap();
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let len: usize = header
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length: "))
                            .unwrap()
                            .parse()
                            .unwrap();
                        if bytes.len() >= end + 4 + len {
                            let body: Value = serde_json::from_slice(&bytes[end + 4..]).unwrap();
                            assert_eq!(body["model"], "test-model");
                            if provider_name == "openai" {
                                assert_eq!(body["store"], false);
                                assert_eq!(body["input"], "reviewed patch");
                                assert!(header.contains("authorization: bearer test-key"));
                            } else {
                                assert_eq!(body["messages"][0]["content"], "reviewed patch");
                            }
                            if provider_name == "anthropic" {
                                assert!(header.contains("x-api-key: test-key"));
                                assert!(header.contains("anthropic-version:"));
                            }
                            break;
                        }
                    }
                    assert!(n > 0);
                }
                let body = match provider_name.as_str() {
                    "openai" => {
                        r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"Improve greeting"}]}]}"#
                    }
                    "anthropic" => r#"{"content":[{"type":"text","text":"Improve greeting"}]}"#,
                    _ => r#"{"message":{"content":"Improve greeting"}}"#,
                };
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            });
            let settings = AiSettings {
                enabled: true,
                provider: provider.into(),
                model: "test-model".into(),
                ..Default::default()
            };
            assert_eq!(
                send(&settings, "reviewed patch", Some("test-key"), endpoint)
                    .await
                    .unwrap(),
                "Improve greeting"
            );
            server.join().unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let _ = socket.read(&mut buf);
            let body = "secret-key private-patch";
            write!(
                socket,
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let settings = AiSettings {
            provider: "openai".into(),
            ..Default::default()
        };
        let error = send(&settings, "private-patch", Some("secret-key"), endpoint)
            .await
            .unwrap_err();
        assert_eq!(error.code, "aiAuthentication");
        assert!(!format!("{error}").contains("secret-key"));
        server.join().unwrap();
    }
}
