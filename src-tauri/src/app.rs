use crate::auth::{self, Replies};
use git_toys_engine::{
    diagnostics,
    runner::{self, Auth},
    types::*,
    Repository,
};
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager, State};

#[derive(Clone)]
struct Session {
    repo: Repository,
    gate: Arc<Mutex<()>>,
    _watcher: Option<Arc<notify::RecommendedWatcher>>,
}
#[derive(Default)]
struct AppState {
    sessions: Mutex<HashMap<String, Session>>,
    gates: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    auth: Mutex<Option<Auth>>,
    replies: Replies,
    settings_gate: Mutex<()>,
    ai_requests: crate::ai::Requests,
    github: crate::github::Service,
    previews: Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Opened {
    id: String,
    snapshot: Snapshot,
    trusted: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationEvent {
    id: String,
    repo_id: String,
    kind: String,
    status: String,
    output: String,
    error: Option<GitError>,
    removed_path: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalEntry {
    id: String,
    repo_id: String,
    kind: String,
    status: String,
    timestamp: u64,
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Preferences {
    #[serde(default)]
    commit_groups: Vec<CommitGroup>,
    recent: Vec<String>,
    favorites: Vec<String>,
    theme: String,
    #[serde(default = "default_color_theme")]
    color_theme: String,
    #[serde(default)]
    ai: AiSettings,
    sidebar_width: Option<u32>,
    detail_width: Option<u32>,
    journal: Vec<JournalEntry>,
}
fn default_color_theme() -> String {
    "teal".into()
}
fn err() -> GitError {
    GitError::new(
        "state",
        "Application state is unavailable. Restart Git Toys.",
    )
}
fn session(state: &AppState, id: &str) -> Result<Session> {
    state
        .sessions
        .lock()
        .map_err(|_| err())?
        .get(id)
        .cloned()
        .ok_or_else(|| GitError::new("repositoryClosed", "Open this repository again"))
}
fn register(app: &tauri::AppHandle, state: &AppState, path: &Path) -> Result<Opened> {
    let mut repo = Repository::open(path)?;
    repo.auth = state.auth.lock().map_err(|_| err())?.clone();
    let mut sessions = state.sessions.lock().map_err(|_| err())?;
    if let Some((id, s)) = sessions.iter().find(|(_, s)| s.repo.root == repo.root) {
        let _guard = s.gate.lock().map_err(|_| err())?;
        return Ok(Opened {
            id: id.clone(),
            snapshot: s.repo.snapshot()?,
            trusted: s.repo.trusted,
        });
    }
    let gate = state
        .gates
        .lock()
        .map_err(|_| err())?
        .entry(repo.common_dir.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = gate.lock().map_err(|_| err())?;
    let snapshot = repo.snapshot()?;
    drop(_guard);
    let id = uuid::Uuid::new_v4().to_string();
    let app = app.clone();
    let watch_id = id.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if result.is_ok() {
            let _ = app.emit("repository-changed", &watch_id);
        }
    })
    .ok();
    if let Some(watcher) = watcher.as_mut() {
        let _ = watcher.watch(&repo.root, RecursiveMode::Recursive);
        let _ = watcher.watch(&repo.common_dir, RecursiveMode::Recursive);
    }
    sessions.insert(
        id.clone(),
        Session {
            repo,
            gate,
            _watcher: watcher.map(Arc::new),
        },
    );
    Ok(Opened {
        id,
        snapshot,
        trusted: false,
    })
}
fn pref_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| GitError::new("settings", e.to_string()))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("preferences.json"))
}
fn read_prefs(app: &tauri::AppHandle) -> Result<Preferences> {
    let path = pref_path(app)?;
    if !path.exists() {
        return Ok(Preferences {
            theme: "system".into(),
            color_theme: default_color_theme(),
            ..Default::default()
        });
    }
    serde_json::from_slice(&std::fs::read(path)?)
        .map_err(|e| GitError::new("settings", e.to_string()))
}
fn write_prefs(app: &tauri::AppHandle, prefs: &Preferences) -> Result<()> {
    let path = pref_path(app)?;
    let data =
        serde_json::to_vec_pretty(prefs).map_err(|e| GitError::new("settings", e.to_string()))?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    #[cfg(windows)]
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::rename(tmp, path)?;
    Ok(())
}
#[tauri::command]
fn git_diagnostics() -> Diagnostic {
    diagnostics()
}
#[tauri::command]
fn open_repository(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Opened> {
    register(&app, &state, Path::new(&path))
}
#[tauri::command]
fn close_repository(state: State<'_, AppState>, id: String) -> Result<()> {
    state.sessions.lock().map_err(|_| err())?.remove(&id);
    Ok(())
}
#[tauri::command]
fn trust_repository(state: State<'_, AppState>, id: String, trusted: bool) -> Result<()> {
    let mut sessions = state.sessions.lock().map_err(|_| err())?;
    let s = sessions.get_mut(&id).ok_or_else(err)?;
    s.repo.trusted = trusted;
    Ok(())
}
#[tauri::command]
async fn snapshot(state: State<'_, AppState>, id: String) -> Result<Snapshot> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = s.gate.lock().map_err(|_| err())?;
        s.repo.snapshot()
    })
    .await
    .map_err(|_| err())?
}
#[tauri::command]
async fn history(
    state: State<'_, AppState>,
    id: String,
    query: HistoryQuery,
) -> Result<Vec<Commit>> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.history(&query))
        .await
        .map_err(|_| err())?
}
fn github_remote(state: &AppState, id: &str, remote: &str) -> Result<String> {
    let s = session(state, id)?;
    let snapshot = s.repo.snapshot()?;
    snapshot
        .remotes
        .iter()
        .find(|r| r.name == remote)
        .and_then(|r| git_toys_engine::github_repository(&r.url))
        .ok_or_else(|| GitError::new("githubRemote", "Select a configured GitHub.com remote."))
}
#[tauri::command]
async fn github_status(state: State<'_, AppState>) -> Result<GithubStatus> {
    state.github.status().await
}
#[tauri::command]
async fn github_start(state: State<'_, AppState>, private: bool) -> Result<GithubVerification> {
    state.github.start(private).await
}
#[tauri::command]
async fn github_finish(state: State<'_, AppState>, request_id: String) -> Result<()> {
    state.github.finish(&request_id).await
}
#[tauri::command]
fn github_cancel(state: State<'_, AppState>) -> Result<()> {
    state.github.cancel()
}
#[tauri::command]
async fn github_disconnect(state: State<'_, AppState>) -> Result<()> {
    state.github.disconnect().await
}
#[tauri::command]
async fn github_pulls(
    state: State<'_, AppState>,
    id: String,
    remote: String,
    query: PullRequestQuery,
) -> Result<PullRequestPage> {
    let repository = github_remote(&state, &id, &remote)?;
    state.github.pulls(&repository, query).await
}
#[tauri::command]
async fn github_pull(
    state: State<'_, AppState>,
    id: String,
    remote: String,
    number: u32,
) -> Result<PullRequest> {
    if number == 0 {
        return Err(GitError::new("githubQuery", "Invalid pull request number."));
    }
    let repository = github_remote(&state, &id, &remote)?;
    state.github.pull(&repository, number).await
}
#[tauri::command]
async fn github_section(
    state: State<'_, AppState>,
    id: String,
    remote: String,
    number: u32,
    section: String,
    page: u32,
) -> Result<GithubSection> {
    let repository = github_remote(&state, &id, &remote)?;
    state
        .github
        .section(&repository, number, &section, page)
        .await
}
#[tauri::command]
fn github_open(repository: Option<String>, number: Option<u32>) -> Result<()> {
    let url = if let Some(repository) = repository {
        if git_toys_engine::github_repository(&format!("https://github.com/{repository}"))
            .as_deref()
            != Some(&repository)
        {
            return Err(GitError::new("githubUrl", "Invalid GitHub repository."));
        }
        format!(
            "https://github.com/{repository}/pull/{}",
            number
                .filter(|n| *n > 0)
                .ok_or_else(|| GitError::new("githubUrl", "Invalid pull request."))?
        )
    } else {
        "https://github.com/login/device".into()
    };
    open_url(&url)
}
fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("rundll32.exe");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    command.arg(url).spawn()?;
    Ok(())
}
#[tauri::command]
async fn rebase_range(state: State<'_, AppState>, id: String, base: String) -> Result<RebaseRange> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = s.gate.lock().map_err(|_| err())?;
        s.repo.rebase_range(&base)
    })
    .await
    .map_err(|_| err())?
}
#[tauri::command]
fn preview_cancel(state: State<'_, AppState>, request_id: String) -> Result<()> {
    if let Some(flag) = state.previews.lock().map_err(|_| err())?.get(&request_id) {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}
#[tauri::command]
async fn preview_operation(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: String,
    request: PreviewRequest,
) -> Result<PreviewResult> {
    let s = session(&state, &id)?;
    if !s.repo.trusted {
        return Err(GitError::new(
            "untrusted",
            "Trust this repository before simulating operations.",
        ));
    }
    let folder = app
        .path()
        .app_cache_dir()
        .map_err(|_| err())?
        .join("previews");
    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut previews = state.previews.lock().map_err(|_| err())?;
        if !previews.is_empty() {
            return Err(GitError::new(
                "previewBusy",
                "A preview is already running. Cancel it or wait.",
            ));
        }
        previews.insert(request_id.clone(), Arc::clone(&flag));
    }

    let event_app = app.clone();
    let event_id = request_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _guard = s.gate.lock().map_err(|_| err())?;
        s.repo.simulate(&folder, &request, flag, |message| {
            let _ = event_app.emit(
                "preview-progress",
                serde_json::json!({ "id": event_id, "message": message }),
            );
        })
    })
    .await
    .map_err(|_| err())
    .and_then(|v| v);
    state
        .previews
        .lock()
        .map_err(|_| err())?
        .remove(&request_id);
    result
}
#[tauri::command]
async fn selection_diff(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<SelectionDiff> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = s.gate.lock().map_err(|_| err())?;
        s.repo.selection_diff(&path)
    })
    .await
    .map_err(|_| err())?
}
#[tauri::command]
async fn diff(state: State<'_, AppState>, id: String, request: DiffRequest) -> Result<Diff> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.diff(&request))
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn commit_files(
    state: State<'_, AppState>,
    id: String,
    revision: String,
    base: Option<String>,
) -> Result<Vec<String>> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.commit_files(&revision, base.as_deref()))
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn conflict(state: State<'_, AppState>, id: String, path: String) -> Result<Conflict> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.conflict(&path))
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn reflog(state: State<'_, AppState>, id: String) -> Result<Vec<ReflogEntry>> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.reflog())
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn stashes(state: State<'_, AppState>, id: String) -> Result<Vec<ReflogEntry>> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.stashes())
        .await
        .map_err(|_| err())?
}
#[tauri::command]
fn mutate(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: Action,
    expected: String,
) -> Result<String> {
    let mut s = session(&state, &id)?;
    if !s.repo.trusted {
        return Err(GitError::new(
            "untrusted",
            "Trust this repository before running operations.",
        ));
    }
    let op_id = uuid::Uuid::new_v4().to_string();
    let result_id = op_id.clone();
    let kind = serde_json::to_value(&action)
        .ok()
        .and_then(|v| v.get("kind").and_then(|v| v.as_str()).map(str::to_string))
        .unwrap_or_default();
    std::thread::spawn(move || {
        let mut event = OperationEvent {
            id: op_id,
            repo_id: id,
            kind,
            status: "running".into(),
            output: String::new(),
            error: None,
            removed_path: None,
        };
        let _ = app.emit("operation", &event);
        if let Some(auth) = s.repo.auth.as_mut() {
            let progress_app = app.clone();
            let progress_event = event.clone();
            auth.progress = Some(Arc::new(move |output| {
                let mut progress = progress_event.clone();
                progress.output = output;
                let _ = progress_app.emit("operation", progress);
            }));
        }

        let removed_path = if let Action::RemoveWorktree { path } = &action {
            Path::new(path)
                .canonicalize()
                .ok()
                .and_then(|path| path.to_str().map(str::to_string))
        } else {
            None
        };
        let result = (|| {
            let _guard = s.gate.lock().map_err(|_| err())?;
            s.repo.execute(&action, &expected)
        })();
        match result {
            Ok(out) => {
                event.status = "completed".into();
                event.output = out;
                if let Some(path) = removed_path {
                    let state = app.state::<AppState>();
                    if let Ok(mut sessions) = state.sessions.lock() {
                        sessions.retain(|_, session| {
                            session.repo.root.to_string_lossy() != path.as_str()
                        });
                    }
                    event.removed_path = Some(path);
                }
            }
            Err(e) => {
                event.status = "failed".into();
                event.error = Some(e)
            }
        }
        let state = app.state::<AppState>();
        if let Ok(_guard) = state.settings_gate.lock() {
            if let Ok(mut prefs) = read_prefs(&app) {
                prefs.journal.push(JournalEntry {
                    id: event.id.clone(),
                    repo_id: s.repo.root.to_string_lossy().into(),
                    kind: event.kind.clone(),
                    status: event.status.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
                if prefs.journal.len() > 200 {
                    prefs.journal.drain(..prefs.journal.len() - 200);
                }
                let _ = write_prefs(&app, &prefs);
            }
        }
        let _ = app.emit("operation", event);
    });
    Ok(result_id)
}
#[tauri::command]
async fn create_repository(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    url: Option<String>,
    trusted: bool,
) -> Result<Opened> {
    if !trusted {
        return Err(GitError::new(
            "untrusted",
            "Confirm that this source is trusted before creating a repository.",
        ));
    }
    let auth = state.auth.lock().map_err(|_| err())?.clone();
    let destination = PathBuf::from(&path);
    let created = tauri::async_runtime::spawn_blocking(move || -> Result<PathBuf> {
        if destination.exists() && std::fs::read_dir(&destination)?.next().is_some() {
            return Err(GitError::new(
                "destinationNotEmpty",
                "Choose a new or empty directory.",
            ));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| GitError::new("invalidPath", "Choose a destination directory"))?;
        std::fs::create_dir_all(parent)?;
        let dest = destination.to_string_lossy();
        match url {
            Some(url) => {
                if url.starts_with('-') || url.starts_with("ext::") || url.is_empty() {
                    return Err(GitError::new("invalidRemote", "Enter a valid clone URL"));
                }
                runner::run(
                    parent,
                    true,
                    auth.as_ref(),
                    &["clone", "--progress", "--", &url, &dest],
                    None,
                )?;
            }
            None => {
                std::fs::create_dir_all(&destination)?;
                runner::run(
                    &destination,
                    false,
                    None,
                    &["init", "--initial-branch=main"],
                    None,
                )?;
            }
        }
        Ok(destination)
    })
    .await
    .map_err(|_| err())??;
    let opened = register(&app, &state, &created)?;
    if let Some(s) = state
        .sessions
        .lock()
        .map_err(|_| err())?
        .get_mut(&opened.id)
    {
        s.repo.trusted = true;
    }
    Ok(Opened {
        trusted: true,
        ..opened
    })
}
#[tauri::command]
fn load_preferences(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Preferences> {
    let _guard = state.settings_gate.lock().map_err(|_| err())?;
    read_prefs(&app)
}
#[tauri::command]
fn save_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mut preferences: Preferences,
) -> Result<()> {
    let _guard = state.settings_gate.lock().map_err(|_| err())?;
    preferences.journal = read_prefs(&app)?.journal;
    preferences.recent.truncate(30);
    write_prefs(&app, &preferences)
}
async fn query_repository<T: Send + 'static>(
    repo: Repository,
    query: impl FnOnce(Repository) -> Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(move || query(repo))
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn worktrees(state: State<'_, AppState>, id: String) -> Result<Vec<Worktree>> {
    query_repository(session(&state, &id)?.repo, |repo| repo.worktrees()).await
}
#[tauri::command]
async fn file_history(
    state: State<'_, AppState>,
    id: String,
    path: String,
    revision: Option<String>,
    skip: u32,
    limit: u32,
) -> Result<Vec<FileHistoryEntry>> {
    query_repository(session(&state, &id)?.repo, move |repo| {
        repo.file_history(&path, revision.as_deref(), skip, limit)
    })
    .await
}
#[tauri::command]
async fn file_revision(
    state: State<'_, AppState>,
    id: String,
    path: String,
    revision: String,
    previous_path: Option<String>,
) -> Result<FileRevision> {
    query_repository(session(&state, &id)?.repo, move |repo| {
        repo.file_revision(&path, &revision, previous_path.as_deref())
    })
    .await
}
#[tauri::command]
async fn file_restore_preview(
    state: State<'_, AppState>,
    id: String,
    target: String,
    source: String,
    revision: String,
) -> Result<FileRevision> {
    query_repository(session(&state, &id)?.repo, move |repo| {
        repo.restore_preview(&target, &source, &revision)
    })
    .await
}
#[tauri::command]
async fn file_blame(
    state: State<'_, AppState>,
    id: String,
    path: String,
    revision: String,
) -> Result<Vec<BlameLine>> {
    query_repository(session(&state, &id)?.repo, move |repo| {
        repo.blame(&path, &revision)
    })
    .await
}
#[tauri::command]
async fn ai_context(
    state: State<'_, AppState>,
    id: String,
    selected: Option<Vec<String>>,
    style: String,
) -> Result<AiContext> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = s.gate.lock().map_err(|_| err())?;
        s.repo.ai_context(selected.as_deref(), &style)
    })
    .await
    .map_err(|_| err())?
}
#[tauri::command]
async fn ai_index_token(state: State<'_, AppState>, id: String) -> Result<String> {
    let s = session(&state, &id)?;
    tauri::async_runtime::spawn_blocking(move || s.repo.index_token())
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn ai_key_status(provider: String) -> Result<bool> {
    tauri::async_runtime::spawn_blocking(move || Ok(crate::ai::credential(&provider)?.is_some()))
        .await
        .map_err(|_| err())?
}
#[tauri::command]
async fn ai_save_key(provider: String, key: Option<String>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || crate::ai::save_credential(&provider, key))
        .await
        .map_err(|_| err())?
}
#[tauri::command]
fn ai_cancel(state: State<'_, AppState>, request_id: String) -> Result<()> {
    state.ai_requests.cancel(&request_id)
}
#[tauri::command]
async fn ai_generate(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: String,
    context: AiContext,
    style: String,
) -> Result<AiDraft> {
    let s = session(&state, &id)?;
    let settings = {
        let _guard = state.settings_gate.lock().map_err(|_| err())?;
        read_prefs(&app)?.ai
    };
    crate::ai::validate(&settings)?;
    let task = async {
        let preparation = s.clone();
        let selected = context.selected.clone();
        let reviewed = tauri::async_runtime::spawn_blocking(move || {
            let _guard = preparation.gate.lock().map_err(|_| err())?;
            preparation.repo.ai_context(Some(&selected), &style)
        })
        .await
        .map_err(|_| err())??;
        if reviewed.index_token != context.index_token
            || reviewed.context_token != context.context_token
        {
            return Err(GitError::new(
                "staleIndex",
                "Staged changes or the reviewed context changed. Review again before generating.",
            ));
        }
        if reviewed.oversized || reviewed.selected.is_empty() {
            return Err(GitError::new(
                "aiContext",
                "Select staged text files with a combined context under 64 KiB.",
            ));
        }
        let message = crate::ai::generate(&settings, &reviewed.context).await?;
        let token = tauri::async_runtime::spawn_blocking(move || s.repo.index_token())
            .await
            .map_err(|_| err())??;
        if token != reviewed.index_token {
            return Err(GitError::new(
                "staleIndex",
                "Staged changes changed during generation. Regenerate the draft.",
            ));
        }
        Ok(AiDraft {
            message,
            index_token: token,
        })
    };
    state.ai_requests.run(&request_id, task).await
}
#[tauri::command]
fn auth_reply(state: State<'_, AppState>, id: String, value: Option<String>) -> Result<()> {
    if let Some(tx) = state.replies.lock().map_err(|_| err())?.remove(&id) {
        let _ = tx.send(value);
    }
    Ok(())
}
#[tauri::command]
fn desktop_open(state: State<'_, AppState>, id: String, target: String) -> Result<()> {
    let s = session(&state, &id)?;
    if !s.repo.trusted && target != "files" {
        return Err(GitError::new(
            "untrusted",
            "Trust the repository before opening external tools",
        ));
    }
    let root = &s.repo.root;
    let mut c = match target.as_str() {
        "editor" => {
            let mut c = Command::new("code");
            c.arg(root);
            c
        }
        "terminal" => {
            #[cfg(target_os = "macos")]
            {
                let mut c = Command::new("open");
                c.args(["-a", "Terminal"]).arg(root);
                c
            }
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new("wt.exe");
                c.arg("-d").arg(root);
                c
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                let mut c = Command::new("x-terminal-emulator");
                c.current_dir(root);
                c
            }
        }
        "files" => {
            #[cfg(target_os = "macos")]
            {
                let mut c = Command::new("open");
                c.arg(root);
                c
            }
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new("explorer.exe");
                c.arg(root);
                c
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                let mut c = Command::new("xdg-open");
                c.arg(root);
                c
            }
        }
        _ => return Err(GitError::new("invalidTool", "Unknown desktop tool")),
    };
    c.spawn()?;
    Ok(())
}
pub fn run() -> std::result::Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            if let Ok(cache) = app.path().app_cache_dir() {
                let previews = cache.join("previews");
                if let Ok(entries) = std::fs::read_dir(&previews) {
                    for entry in entries.flatten() {
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with("simulation-")
                            && entry
                                .file_type()
                                .is_ok_and(|t| t.is_dir() && !t.is_symlink())
                            && entry
                                .metadata()
                                .and_then(|m| m.modified())
                                .ok()
                                .and_then(|t| t.elapsed().ok())
                                .is_some_and(|age| age.as_secs() > 600)
                        {
                            let _ = std::fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
            let state = app.state::<AppState>();
            let auth = auth::start(app.handle().clone(), Arc::clone(&state.replies))?;
            *state.auth.lock().map_err(|_| err())? = Some(auth);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            github_status,
            github_start,
            github_finish,
            github_cancel,
            github_disconnect,
            github_pulls,
            github_pull,
            github_section,
            github_open,
            git_diagnostics,
            open_repository,
            close_repository,
            trust_repository,
            snapshot,
            history,
            diff,
            selection_diff,
            rebase_range,
            preview_operation,
            preview_cancel,
            commit_files,
            conflict,
            reflog,
            stashes,
            mutate,
            create_repository,
            load_preferences,
            save_preferences,
            worktrees,
            file_history,
            file_revision,
            file_blame,
            file_restore_preview,
            ai_context,
            ai_index_token,
            ai_generate,
            ai_cancel,
            ai_key_status,
            ai_save_key,
            auth_reply,
            desktop_open
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod preference_tests {
    use super::*;

    #[test]
    fn older_preferences_default_to_teal_and_round_trip_color_selection() {
        let mut preferences: Preferences = serde_json::from_str(
            r#"{"recent":[],"favorites":[],"theme":"dark","sidebarWidth":240,"detailWidth":440,"journal":[]}"#,
        ).unwrap();
        assert_eq!(preferences.color_theme, "teal");
        assert!(!preferences.ai.enabled);
        assert_eq!(preferences.ai.provider, "ollama");
        assert!(preferences.ai.model.is_empty());
        preferences.color_theme = "violet".into();
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
        assert_eq!(restored.color_theme, "violet");
        assert_eq!(restored.theme, "dark");
        assert_eq!(restored.sidebar_width, Some(240));
    }
}
