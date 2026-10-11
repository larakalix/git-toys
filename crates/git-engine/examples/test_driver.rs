//! Test-only JSON-lines bridge. Not linked into the desktop app or production web assets.
use git_toys_engine::{diagnostics, runner, types::*, Repository};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
fn dispatch(
    command: &str,
    args: &Value,
    sessions: &mut HashMap<String, Repository>,
    sandbox: &Path,
) -> Result<Value> {
    let value = |key: &str| args[key].as_str().unwrap_or("");
    let safe = |p: &str| -> Result<PathBuf> {
        let path = PathBuf::from(p);
        let parent = path
            .parent()
            .ok_or_else(|| GitError::new("path", "Missing parent"))?
            .canonicalize()?;
        if !parent.starts_with(sandbox) {
            return Err(GitError::new("path", "Outside test sandbox"));
        }
        Ok(path)
    };
    match command {
        "github_status" => Ok(json!({"configured":false,"connected":false})),
        "github_cancel" | "preview_cancel" => Ok(Value::Null),
        "git_diagnostics" => Ok(json!(diagnostics())),
        "open_repository" => {
            let path = safe(value("path"))?;
            let repo = Repository::open(&path)?;
            let id = repo.root.to_string_lossy().to_string();
            let snap = repo.snapshot()?;
            let trusted = sessions.get(&id).is_some_and(|r| r.trusted);
            sessions.entry(id.clone()).or_insert(repo);
            Ok(json!({"id":id,"snapshot":snap,"trusted":trusted}))
        }
        "create_repository" => {
            let path = safe(value("path"))?;
            if path.exists() && std::fs::read_dir(&path)?.next().is_some() {
                return Err(GitError::new(
                    "destinationNotEmpty",
                    "Destination is not empty",
                ));
            }
            if let Some(url) = args["url"].as_str() {
                let source = safe(url)?;
                runner::run(
                    sandbox,
                    true,
                    None,
                    &[
                        "clone",
                        "--",
                        source.to_str().unwrap_or(""),
                        path.to_str().unwrap_or(""),
                    ],
                    None,
                )?;
            } else {
                std::fs::create_dir_all(&path)?;
                runner::run(&path, false, None, &["init", "-b", "main"], None)?;
            }
            runner::run(
                &path,
                true,
                None,
                &["config", "user.name", "Git Toys UI Tests"],
                None,
            )?;
            runner::run(
                &path,
                true,
                None,
                &["config", "user.email", "ui@example.invalid"],
                None,
            )?;
            runner::run(
                &path,
                true,
                None,
                &["config", "commit.gpgsign", "false"],
                None,
            )?;
            let mut repo = Repository::open(&path)?;
            repo.trusted = true;
            let id = repo.root.to_string_lossy().to_string();
            let snap = repo.snapshot()?;
            sessions.insert(id.clone(), repo);
            Ok(json!({"id":id,"snapshot":snap,"trusted":true}))
        }
        "close_repository" => {
            sessions.remove(value("id"));
            Ok(Value::Null)
        }
        "trust_repository" => {
            sessions
                .get_mut(value("id"))
                .ok_or_else(|| GitError::new("id", "Unknown test repository"))?
                .trusted = args["trusted"].as_bool().unwrap_or(false);
            Ok(Value::Null)
        }
        "edit_fixture" => {
            let root = safe(value("path"))?;
            let path = root.join("hello.txt");
            std::fs::write(path, value("content"))?;
            Ok(Value::Null)
        }
        _ => {
            let repo = sessions
                .get(value("id"))
                .ok_or_else(|| GitError::new("id", "Unknown test repository"))?;
            match command {
                "selection_diff" => Ok(json!(repo.selection_diff(value("path"))?)),
                "rebase_range" => Ok(json!(repo.rebase_range(value("base"))?)),
                "preview_operation" => Ok(json!(repo.simulate(
                    &sandbox.join("previews"),
                    &serde_json::from_value(args["request"].clone())
                        .map_err(|e| GitError::new("json", e.to_string()))?,
                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    |_| {}
                )?)),
                "snapshot" => Ok(json!(repo.snapshot()?)),
                "history" => Ok(json!(repo.history(
                    &serde_json::from_value(args["query"].clone())
                        .map_err(|e| GitError::new("json", e.to_string()))?
                )?)),
                "diff" => Ok(json!(repo.diff(
                    &serde_json::from_value(args["request"].clone())
                        .map_err(|e| GitError::new("json", e.to_string()))?
                )?)),
                "commit_files" => Ok(json!(
                    repo.commit_files(value("revision"), args["base"].as_str())?
                )),
                "ai_context" => Ok(json!(repo.ai_context(
                    args["selected"]
                        .as_array()
                        .map(|values| values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>())
                        .as_deref(),
                    value("style")
                )?)),
                "ai_index_token" => Ok(json!(repo.index_token()?)),
                "worktrees" => Ok(json!(repo.worktrees()?)),
                "file_history" => Ok(json!(repo.file_history(
                    value("path"),
                    args["revision"].as_str(),
                    args["skip"].as_u64().unwrap_or(0) as u32,
                    args["limit"].as_u64().unwrap_or(50) as u32
                )?)),
                "file_revision" => Ok(json!(repo.file_revision(
                    value("path"),
                    value("revision"),
                    args["previousPath"].as_str()
                )?)),
                "file_restore_preview" => Ok(json!(repo.restore_preview(
                    value("target"),
                    value("source"),
                    value("revision")
                )?)),
                "file_blame" => Ok(json!(repo.blame(value("path"), value("revision"))?)),
                "conflict" => Ok(json!(repo.conflict(value("path"))?)),
                "reflog" => Ok(json!(repo.reflog()?)),
                "stashes" => Ok(json!(repo.stashes()?)),
                "mutate" => Ok(json!(repo.execute(
                    &serde_json::from_value(args["action"].clone())
                        .map_err(|e| GitError::new("json", e.to_string()))?,
                    value("expected")
                )?)),
                _ => Err(GitError::new("command", "Unsupported test command")),
            }
        }
    }
}
fn main() {
    if git_toys_engine::rebase_editor_entry() {
        return;
    }
    let Some(sandbox) = std::env::args()
        .nth(1)
        .and_then(|p| PathBuf::from(p).canonicalize().ok())
    else {
        return;
    };
    let mut sessions = HashMap::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines().map_while(std::result::Result::ok) {
        let response = serde_json::from_str::<Value>(&line)
            .map_err(|e| GitError::new("json", e.to_string()))
            .and_then(|v| {
                dispatch(
                    v["command"].as_str().unwrap_or(""),
                    &v["args"],
                    &mut sessions,
                    &sandbox,
                )
            });
        let value = match response {
            Ok(result) => json!({"result":result}),
            Err(error) => json!({"error":error}),
        };
        println!("{value}");
        let _ = std::io::stdout().flush();
    }
}
