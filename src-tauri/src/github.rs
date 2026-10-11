//! GitHub.com transport. No secrets, API payloads, or device codes enter preferences/logs.
use git_toys_engine::types::*;
use reqwest::{Client, Url};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

fn client_id() -> &'static str {
    option_env!("GIT_TOYS_GITHUB_CLIENT_ID").unwrap_or("")
}
struct Login {
    id: String,
    device: String,
    interval: u64,
    expires: Instant,
    cancelled: Arc<AtomicBool>,
    claimed: bool,
}
#[derive(Clone)]
struct Cached {
    etag: String,
    value: Value,
    size: usize,
}
#[derive(Default)]
pub struct Service {
    login_version: AtomicU64,
    #[cfg(test)]
    test_endpoint: Mutex<Option<String>>,
    login: Mutex<Option<Login>>,
    cache: Mutex<HashMap<String, Cached>>,
}
fn error(code: &str, message: &str) -> GitError {
    GitError::new(code, message)
}
fn client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Git-Toys/0.1")
        .build()
        .map_err(|_| error("githubNetwork", "Could not initialize GitHub networking."))
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn credential(key: Option<Option<String>>) -> Result<Option<String>> {
    let entry = keyring::Entry::new("com.gittoys.desktop.github", "github.com")
        .map_err(|_| error("githubCredential", "OS credential storage is unavailable."))?;
    match key {
        Some(Some(key)) => {
            entry.set_password(&key).map_err(|_| {
                error(
                    "githubCredential",
                    "Could not save GitHub authorization in OS credential storage.",
                )
            })?;
            Ok(None)
        }
        Some(None) => {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(_) => {
                    return Err(error(
                        "githubCredential",
                        "Could not remove GitHub authorization from OS credential storage.",
                    ))
                }
            };
            Ok(None)
        }
        None => match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(error(
                "githubCredential",
                "Could not read GitHub authorization. Check your Keychain or Credential Manager.",
            )),
        },
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn credential(_: Option<Option<String>>) -> Result<Option<String>> {
    Err(error(
        "githubCredential",
        "GitHub credential storage supports macOS and Windows.",
    ))
}
async fn key(value: Option<Option<String>>) -> Result<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || credential(value))
        .await
        .map_err(|_| error("githubCredential", "Credential storage task failed."))?
}
async fn response(mut response: reqwest::Response) -> Result<Value> {
    if response.content_length().is_some_and(|n| n > 4_000_000) {
        return Err(error(
            "githubLarge",
            "GitHub response exceeds 4 MB. Narrow the query.",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        error(
            "githubNetwork",
            "GitHub connection interrupted. Retry when connected.",
        )
    })? {
        if bytes.len() + chunk.len() > 4_000_000 {
            return Err(error("githubLarge", "GitHub response exceeds 4 MB."));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| error("githubResponse", "GitHub returned an unexpected response."))
}
fn http_error(status: reqwest::StatusCode, headers: &reqwest::header::HeaderMap) -> GitError {
    if status.as_u16() == 401 {
        return error(
            "githubExpired",
            "GitHub authorization expired or was revoked. Disconnect and sign in again.",
        );
    }
    if status.as_u16() == 429
        || headers
            .get("x-ratelimit-remaining")
            .is_some_and(|v| v == "0")
        || headers.contains_key("retry-after")
    {
        let reset = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        return error("githubRateLimit", &format!("GitHub rate limit reached. Try again later; reset Unix time: {reset}. No automatic retry was made."));
    }
    match status.as_u16() {
        403 => error("githubPermission", "GitHub denied access. Check OAuth permissions and your organization's SSO or application approval."),
        404 => error("githubMissing", "Repository or pull request is unavailable. Check the remote and private-repository authorization."),
        _ => error("githubNetwork", "GitHub request failed. Check your connection and retry."),
    }
}
impl Service {
    pub async fn status(&self) -> Result<GithubStatus> {
        Ok(GithubStatus {
            configured: !client_id().is_empty(),
            connected: key(None).await?.is_some(),
        })
    }
    pub fn cancel(&self) -> Result<()> {
        self.login_version.fetch_add(1, Ordering::Relaxed);
        if let Some(login) = self
            .login
            .lock()
            .map_err(|_| error("githubState", "GitHub state unavailable."))?
            .take()
        {
            login.cancelled.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    fn cancel_id(&self, id: &str) -> Result<()> {
        let mut login = self
            .login
            .lock()
            .map_err(|_| error("githubState", "GitHub state unavailable."))?;
        if login.as_ref().is_some_and(|l| l.id == id) {
            if let Some(login) = login.take() {
                login.cancelled.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }
    pub async fn disconnect(&self) -> Result<()> {
        self.cancel()?;
        key(Some(None)).await?;
        self.clear_cache();
        Ok(())
    }
    fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }
    pub async fn start(&self, private: bool) -> Result<GithubVerification> {
        if client_id().is_empty() {
            return Err(error("githubSetup", "GitHub sign-in needs a registered OAuth application. Build with GIT_TOYS_GITHUB_CLIENT_ID and enable Device Flow; see docs/GITHUB.md."));
        }
        self.start_at(
            &client()?,
            "https://github.com/login/device/code",
            client_id(),
            private,
        )
        .await
    }
    async fn start_at(
        &self,
        client: &Client,
        endpoint: &str,
        client_id: &str,
        private: bool,
    ) -> Result<GithubVerification> {
        self.cancel()?;
        let version = self.login_version.load(Ordering::Relaxed);
        let reply = client
            .post(endpoint)
            .header("Accept", "application/json")
            .json(&json!({ "client_id": client_id, "scope": if private { "repo" } else { "" } }))
            .send()
            .await
            .map_err(|_| {
                error(
                    "githubNetwork",
                    "Cannot reach GitHub sign-in. Check your connection.",
                )
            })?;
        if !reply.status().is_success() {
            return Err(http_error(reply.status(), reply.headers()));
        }
        let value = response(reply).await?;
        let device = value["device_code"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                error(
                    "githubSetup",
                    "GitHub Device Flow is not enabled or the client ID is invalid.",
                )
            })?;
        let code = value["user_code"]
            .as_str()
            .ok_or_else(|| error("githubResponse", "Missing verification code."))?;
        if value["verification_uri"].as_str() != Some("https://github.com/login/device") {
            return Err(error(
                "githubResponse",
                "Unexpected GitHub verification URL.",
            ));
        }
        let expires = value["expires_in"].as_u64().unwrap_or(900).clamp(1, 900) as u32;
        let id = uuid::Uuid::new_v4().to_string();
        let mut login = self
            .login
            .lock()
            .map_err(|_| error("githubState", "GitHub state unavailable."))?;
        if self.login_version.load(Ordering::Relaxed) != version {
            return Err(error("githubCancelled", "Sign-in cancelled."));
        }
        *login = Some(Login {
            id: id.clone(),
            device: device.into(),
            expires: Instant::now() + Duration::from_secs(expires.into()),
            interval: value["interval"].as_u64().unwrap_or(5).clamp(1, 60),
            cancelled: Arc::new(AtomicBool::new(false)),
            claimed: false,
        });
        Ok(GithubVerification {
            id,
            user_code: code.into(),
            verification_uri: "https://github.com/login/device".into(),
            expires_in: expires,
        })
    }
    async fn poll_at(
        &self,
        client: &Client,
        endpoint: &str,
        client_id: &str,
        id: &str,
    ) -> Result<String> {
        let (device, mut interval, expires, cancelled) = {
            let mut guard = self
                .login
                .lock()
                .map_err(|_| error("githubState", "GitHub state unavailable."))?;
            let login = guard
                .as_mut()
                .filter(|l| l.id == id && !l.claimed)
                .ok_or_else(|| error("githubCancelled", "Sign-in cancelled or already running."))?;
            login.claimed = true;
            (
                login.device.clone(),
                login.interval,
                login.expires,
                Arc::clone(&login.cancelled),
            )
        };
        loop {
            for _ in 0..interval {
                if cancelled.load(Ordering::Relaxed) {
                    return Err(error("githubCancelled", "GitHub sign-in cancelled."));
                }
                if Instant::now() >= expires {
                    return Err(error(
                        "githubExpired",
                        "Verification code expired. Start sign-in again.",
                    ));
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            let request = client.post(endpoint).header("Accept", "application/json").json(&json!({ "client_id": client_id, "device_code": device, "grant_type": "urn:ietf:params:oauth:grant-type:device_code" })).send();
            let reply = request.await.map_err(|_| {
                error(
                    "githubNetwork",
                    "GitHub sign-in connection failed. Start sign-in again.",
                )
            })?;
            if cancelled.load(Ordering::Relaxed) {
                return Err(error("githubCancelled", "GitHub sign-in cancelled."));
            }
            if !reply.status().is_success() {
                return Err(http_error(reply.status(), reply.headers()));
            }
            let value = response(reply).await?;
            if let Some(token) = value["access_token"].as_str().filter(|s| !s.is_empty()) {
                return Ok(token.into());
            }
            match value["error"].as_str() {
                Some("authorization_pending") => {},
                Some("slow_down") => interval = interval.saturating_add(5).min(120),
                Some("access_denied") => return Err(error("githubDenied", "GitHub authorization was declined.")),
                Some("expired_token" | "token_expired") => return Err(error("githubExpired", "Verification code expired. Start sign-in again.")),
                _ => return Err(error("githubSetup", "GitHub authorization failed. Check the OAuth application's device-flow configuration.")),
            }
        }
    }
    pub async fn finish(&self, id: &str) -> Result<()> {
        let result = self
            .poll_at(
                &client()?,
                "https://github.com/login/oauth/access_token",
                client_id(),
                id,
            )
            .await;
        let token = match result {
            Ok(token) => token,
            Err(e) => {
                self.cancel_id(id)?;
                return Err(e);
            }
        };
        let cancelled = {
            let login = self
                .login
                .lock()
                .map_err(|_| error("githubState", "GitHub state unavailable."))?;
            Arc::clone(
                &login
                    .as_ref()
                    .filter(|l| l.id == id)
                    .ok_or_else(|| error("githubCancelled", "Sign-in cancelled."))?
                    .cancelled,
            )
        };
        if cancelled.load(Ordering::Relaxed) {
            return Err(error("githubCancelled", "Sign-in cancelled."));
        }
        key(Some(Some(token))).await?;
        if cancelled.load(Ordering::Relaxed) {
            key(Some(None)).await?;
            return Err(error("githubCancelled", "Sign-in cancelled."));
        }
        self.cancel_id(id)?;
        self.clear_cache();
        Ok(())
    }
    async fn get_at(&self, client: &Client, url: Url, token: Option<&str>) -> Result<Value> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        token.hash(&mut hash);
        let cache_key = format!("{}:{}", hash.finish(), url);
        let cached = self
            .cache
            .lock()
            .map_err(|_| error("githubState", "GitHub cache unavailable."))?
            .get(&cache_key)
            .cloned();
        let mut request = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        if let Some(cached) = &cached {
            if !cached.etag.is_empty() {
                request = request.header("If-None-Match", &cached.etag);
            }
        }
        let reply = request.send().await.map_err(|_| {
            error(
                "githubNetwork",
                "Cannot reach GitHub. Check your connection and retry.",
            )
        })?;
        if reply.status().as_u16() == 304 {
            return cached
                .map(|c| c.value)
                .ok_or_else(|| error("githubResponse", "GitHub cache expired. Refresh again."));
        }
        if !reply.status().is_success() {
            return Err(http_error(reply.status(), reply.headers()));
        }
        let etag = reply
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let value = response(reply).await?;
        let size = value.to_string().len();
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| error("githubState", "GitHub cache unavailable."))?;
        if cache.values().map(|v| v.size).sum::<usize>() + size > 8_000_000 || cache.len() >= 40 {
            cache.clear();
        }
        cache.insert(
            cache_key,
            Cached {
                etag,
                value: value.clone(),
                size,
            },
        );
        Ok(value)
    }
    async fn get(&self, path: &str, parameters: &[(&str, String)]) -> Result<Value> {
        #[cfg(test)]
        if let Some(endpoint) = self.test_endpoint.lock().ok().and_then(|v| v.clone()) {
            let mut url = Url::parse(&format!("{endpoint}{path}"))
                .map_err(|_| error("githubQuery", "Invalid mock endpoint"))?;
            url.query_pairs_mut()
                .extend_pairs(parameters.iter().map(|(k, v)| (*k, v.as_str())));
            return self.get_at(&client()?, url, Some("mock-token")).await;
        }
        let mut url = Url::parse(&format!("https://api.github.com{path}"))
            .map_err(|_| error("githubQuery", "Invalid GitHub query."))?;
        url.query_pairs_mut()
            .extend_pairs(parameters.iter().map(|(k, v)| (*k, v.as_str())));
        let token = key(None).await?;
        self.get_at(&client()?, url, token.as_deref()).await
    }
    pub async fn pulls(
        &self,
        repository: &str,
        query: PullRequestQuery,
    ) -> Result<PullRequestPage> {
        if !matches!(query.state.as_str(), "open" | "closed" | "all")
            || query.page == 0
            || query.page > 1000
            || query.search.len() > 200
        {
            return Err(error("githubQuery", "Invalid pull request filter or page."));
        }
        let page = query.page.to_string();
        let (value, search) = if query.search.trim().is_empty() {
            (
                self.get(
                    &format!("/repos/{repository}/pulls"),
                    &[
                        ("state", query.state),
                        ("page", page),
                        ("per_page", "30".into()),
                        ("sort", "updated".into()),
                    ],
                )
                .await?,
                false,
            )
        } else {
            // Literal text only; don't let search syntax change the selected repository/type.
            let text = query.search.replace(['"', '\\', '\n', '\r'], " ");
            let state = if query.state == "all" {
                String::new()
            } else {
                format!(" is:{}", query.state)
            };
            (
                self.get(
                    "/search/issues",
                    &[
                        ("q", format!("repo:{repository} is:pr{state} \"{text}\"")),
                        ("page", page),
                        ("per_page", "30".into()),
                        ("sort", "updated".into()),
                    ],
                )
                .await?,
                true,
            )
        };
        let list = if search {
            value["items"].as_array()
        } else {
            value.as_array()
        }
        .ok_or_else(|| error("githubResponse", "Unexpected pull request list."))?;
        let items = list.iter().map(pull).collect::<Result<Vec<_>>>()?;
        Ok(PullRequestPage {
            more: items.len() == 30,
            items,
        })
    }
    pub async fn pull(&self, repository: &str, number: u32) -> Result<PullRequest> {
        pull(
            &self
                .get(&format!("/repos/{repository}/pulls/{number}"), &[])
                .await?,
        )
    }
    pub async fn section(
        &self,
        repository: &str,
        number: u32,
        section: &str,
        page: u32,
    ) -> Result<GithubSection> {
        if page == 0 || page > 1000 || number == 0 {
            return Err(error("githubQuery", "Invalid pull request page."));
        }
        let prefix = format!("/repos/{repository}/pulls/{number}");
        let parameters = [("page", page.to_string()), ("per_page", "30".into())];
        let value = match section {
            "files" | "commits" | "reviews" => {
                self.get(&format!("{prefix}/{section}"), &parameters)
                    .await?
            }
            "checks" => {
                let request = self.pull(repository, number).await?;
                let runs = self
                    .get(
                        &format!("/repos/{repository}/commits/{}/check-runs", request.head),
                        &parameters,
                    )
                    .await?;
                let mut items = runs["check_runs"]
                    .as_array()
                    .cloned()
                    .ok_or_else(|| error("githubResponse", "Unexpected checks response."))?;
                let more = (page as u64) * 30 < runs["total_count"].as_u64().unwrap_or(0);
                if page == 1 {
                    let statuses = self
                        .get(
                            &format!("/repos/{repository}/commits/{}/status", request.head),
                            &[("per_page", "100".into())],
                        )
                        .await?;
                    for status in statuses["statuses"].as_array().into_iter().flatten() {
                        items.push(json!({"name": status["context"], "status": status["state"], "conclusion": status["description"]}));
                    }
                    if statuses["total_count"].as_u64().unwrap_or(0) > 100 {
                        items.push(json!({"name":"Additional legacy statuses", "status":"unknown", "conclusion":"More than 100 statuses; view the complete result on GitHub."}));
                    }
                }
                return Ok(GithubSection {
                    more,
                    items: items
                        .iter()
                        .map(|v| GithubItem {
                            name: string(v, "name"),
                            status: string(v, "status"),
                            detail: string(v, "conclusion"),
                            oid: String::new(),
                            patch: None,
                        })
                        .collect(),
                });
            }
            _ => {
                return Err(error(
                    "githubQuery",
                    "Choose files, commits, reviews, or checks.",
                ))
            }
        };
        let list = value
            .as_array()
            .ok_or_else(|| error("githubResponse", "Unexpected GitHub section."))?;
        let items = list
            .iter()
            .map(|v| match section {
                "files" => GithubItem {
                    name: string(v, "filename"),
                    status: string(v, "status"),
                    detail: format!(
                        "+{} −{}",
                        v["additions"].as_u64().unwrap_or(0),
                        v["deletions"].as_u64().unwrap_or(0)
                    ),
                    oid: String::new(),
                    patch: v["patch"].as_str().map(str::to_string),
                },
                "commits" => GithubItem {
                    name: v["commit"]["message"]
                        .as_str()
                        .unwrap_or("")
                        .lines()
                        .next()
                        .unwrap_or("")
                        .into(),
                    status: String::new(),
                    detail: v["commit"]["author"]["name"].as_str().unwrap_or("").into(),
                    oid: string(v, "sha"),
                    patch: None,
                },
                _ => GithubItem {
                    name: v["user"]["login"].as_str().unwrap_or("").into(),
                    status: string(v, "state"),
                    detail: string(v, "body"),
                    oid: string(v, "commit_id"),
                    patch: None,
                },
            })
            .collect();
        Ok(GithubSection {
            items,
            more: list.len() == 30,
        })
    }
}
fn string(value: &Value, name: &str) -> String {
    value[name].as_str().unwrap_or("").into()
}
fn pull(v: &Value) -> Result<PullRequest> {
    let number = v["number"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX.into())
        .ok_or_else(|| error("githubResponse", "Invalid pull request number."))?
        as u32;
    Ok(PullRequest {
        number,
        title: string(v, "title"),
        author: v["user"]["login"].as_str().unwrap_or("").into(),
        state: string(v, "state"),
        draft: v["draft"].as_bool().unwrap_or(false),
        body: string(v, "body"),
        head: v["head"]["sha"].as_str().unwrap_or("").into(),
        head_label: v["head"]["label"].as_str().unwrap_or("").into(),
        base: v["base"]["ref"].as_str().unwrap_or("").into(),
        mergeability: if v["mergeable"].is_null() {
            "Calculating".into()
        } else if v["mergeable"] == true {
            "Mergeable".into()
        } else {
            "Conflicts or merge restrictions".into()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    fn server(
        replies: Vec<(u16, &'static str, &'static str)>,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, headers, body) in replies {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let n = socket.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let len: usize = header
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .unwrap_or("0")
                            .parse()
                            .unwrap();
                        if bytes.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                write!(socket, "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (url, handle)
    }
    #[tokio::test]
    async fn oauth_device_flow_uses_scopes_and_pending_polling_without_secret() {
        let (url, handle) = server(vec![
            (
                200,
                "",
                r#"{"device_code":"private-device","user_code":"USER-CODE","verification_uri":"https://github.com/login/device","interval":1,"expires_in":30}"#,
            ),
            (200, "", r#"{"error":"authorization_pending"}"#),
            (200, "", r#"{"access_token":"private-token"}"#),
        ]);
        let service = Service::default();
        let client = client().unwrap();
        let verification = service
            .start_at(&client, &url, "public-client", true)
            .await
            .unwrap();
        assert_eq!(verification.user_code, "USER-CODE");
        assert_eq!(
            service
                .poll_at(&client, &url, "public-client", &verification.id)
                .await
                .unwrap(),
            "private-token"
        );
        let requests = handle.join().unwrap();
        assert!(requests[0].contains("\"scope\":\"repo\""));
        assert!(!requests.iter().any(|r| r.contains("client_secret")));
        assert!(requests[1].contains("urn:ietf:params:oauth:grant-type:device_code"));
    }
    #[tokio::test]
    async fn oauth_cancellation_expiration_and_public_scope_are_explicit() {
        let (url, handle) = server(vec![(
            200,
            "",
            r#"{"device_code":"private-device","user_code":"USER-CODE","verification_uri":"https://github.com/login/device","interval":1,"expires_in":30}"#,
        )]);
        let service = Service::default();
        let client = client().unwrap();
        let verification = service
            .start_at(&client, &url, "public-client", false)
            .await
            .unwrap();
        assert!(handle.join().unwrap()[0].contains("\"scope\":\"\""));
        service.cancel().unwrap();
        assert_eq!(
            service
                .poll_at(&client, &url, "public-client", &verification.id)
                .await
                .unwrap_err()
                .code,
            "githubCancelled"
        );
        *service.login.lock().unwrap() = Some(Login {
            id: "expired".into(),
            device: "secret".into(),
            interval: 1,
            expires: Instant::now(),
            cancelled: Arc::new(AtomicBool::new(false)),
            claimed: false,
        });
        assert_eq!(
            service
                .poll_at(&client, &url, "public-client", "expired")
                .await
                .unwrap_err()
                .code,
            "githubExpired"
        );
    }
    #[tokio::test]
    async fn conditional_cache_and_auth_rate_limit_errors_are_redacted() {
        let (url, handle) = server(vec![
            (200, "ETag: \"version-one\"\r\n", r#"{"items":[]}"#),
            (304, "", ""),
            (401, "", r#"{"message":"secret-token"}"#),
            (429, "Retry-After: 10\r\n", r#"{"message":"secret-token"}"#),
        ]);
        let service = Service::default();
        let client = client().unwrap();
        let url = Url::parse(&url).unwrap();
        assert_eq!(
            service
                .get_at(&client, url.clone(), Some("test-token"))
                .await
                .unwrap(),
            json!({"items":[]})
        );
        assert_eq!(
            service
                .get_at(&client, url.clone(), Some("test-token"))
                .await
                .unwrap(),
            json!({"items":[]})
        );
        let revoked = service
            .get_at(&client, url.clone(), Some("test-token"))
            .await
            .unwrap_err();
        assert_eq!(revoked.code, "githubExpired");
        assert!(!revoked.to_string().contains("secret-token"));
        assert_eq!(
            service
                .get_at(&client, url, Some("test-token"))
                .await
                .unwrap_err()
                .code,
            "githubRateLimit"
        );
        let requests = handle.join().unwrap();
        assert!(requests[1]
            .to_lowercase()
            .contains("if-none-match: \"version-one\""));
    }
    #[test]
    fn pull_metadata_keeps_fork_heads_and_untrusted_descriptions_as_data() {
        let value = json!({"number":42,"title":"Fork contribution","user":{"login":"contributor"},"head":{"sha":"abc","label":"contributor:feature"},"base":{"ref":"main"},"body":"<script>alert(1)</script> ![](https://tracking.invalid/image)","mergeable":null});
        let request = pull(&value).unwrap();
        assert_eq!(request.head_label, "contributor:feature");
        assert_eq!(request.mergeability, "Calculating");
        assert_eq!(request.body, value["body"]);
    }
    #[tokio::test]
    async fn paginated_queries_and_fork_sections_use_fixed_github_routes() {
        let (url, handle) = server(vec![
            (
                200,
                "",
                r#"[{"number":42,"title":"Contribution","state":"open","user":{"login":"forker"}}]"#,
            ),
            (
                200,
                "",
                r#"{"number":42,"head":{"sha":"abcdef","label":"forker:branch"},"base":{"ref":"main"},"mergeable":true}"#,
            ),
            (
                200,
                "",
                r#"[{"filename":"space file.txt","status":"modified","additions":1,"deletions":0,"patch":"@@ +1 @@\n+line"}]"#,
            ),
            (
                200,
                "",
                r#"[{"sha":"abcdef","commit":{"message":"Title\n\nBody","author":{"name":"Author"}}}]"#,
            ),
            (
                200,
                "",
                r#"[{"user":{"login":"reviewer"},"state":"APPROVED","body":"Reviewed","commit_id":"abcdef"}]"#,
            ),
        ]);
        let service = Service::default();
        *service.test_endpoint.lock().unwrap() = Some(url);
        let list = service
            .pulls(
                "owner/repo",
                PullRequestQuery {
                    state: "open".into(),
                    search: String::new(),
                    page: 2,
                },
            )
            .await
            .unwrap();
        assert_eq!(list.items[0].number, 42);
        assert!(!list.more);
        assert_eq!(
            service.pull("owner/repo", 42).await.unwrap().head_label,
            "forker:branch"
        );
        assert!(service
            .section("owner/repo", 42, "files", 2)
            .await
            .unwrap()
            .items[0]
            .patch
            .is_some());
        assert_eq!(
            service
                .section("owner/repo", 42, "commits", 1)
                .await
                .unwrap()
                .items[0]
                .oid,
            "abcdef"
        );
        assert_eq!(
            service
                .section("owner/repo", 42, "reviews", 1)
                .await
                .unwrap()
                .items[0]
                .status,
            "APPROVED"
        );
        let requests = handle.join().unwrap();
        assert!(requests[0].contains("page=2"));
        assert!(requests[2].contains("/repos/owner/repo/pulls/42/files?"));
    }
}
