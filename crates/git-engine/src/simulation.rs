//! Simulations own a disposable repository; they never write source refs or index.
use crate::{types::*, Repository};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub fn command(path: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(path)
        .args([
            "--no-pager",
            "-c",
            "core.hooksPath=",
            "-c",
            "core.attributesFile=",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "commit.gpgSign=false",
            "-c",
            "tag.gpgSign=false",
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.file.allow=always",
            "-c",
            "user.name=Git Toys Preview",
            "-c",
            "user.email=preview@gittoys.invalid",
            "-c",
            "core.autocrlf=false",
        ])
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_CONFIG_COUNT", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true");
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG_PARAMETERS",
    ] {
        c.env_remove(key);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        c.process_group(0);
    }
    c
}
fn read(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len() + count > 16_000_000 {
            continue;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}
pub fn run(
    path: &Path,
    args: &[&str],
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<(bool, Vec<u8>, String)> {
    if cancel.load(Ordering::Relaxed) {
        return Err(GitError::new("previewCancelled", "Preview cancelled."));
    }
    run_command(command(path), args, cancel, deadline)
}
pub fn run_command(
    mut command: Command,
    args: &[&str],
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<(bool, Vec<u8>, String)> {
    if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
        return Err(GitError::new(
            "previewCancelled",
            "Preview cancelled or timed out.",
        ));
    }
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GitError::new("preview", "Missing output"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GitError::new("preview", "Missing errors"))?;
    let out = std::thread::spawn(move || read(stdout));
    let err = std::thread::spawn(move || read(stderr));
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Ok(status.success());
        }
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            #[cfg(unix)]
            {
                let _ = Command::new("/bin/kill")
                    .args(["-KILL", &format!("-{}", child.id())])
                    .status();
            }
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill.exe")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .output();
            }
            let _ = child.kill();
            let _ = child.wait();
            break Err(GitError::new(
                if cancel.load(Ordering::Relaxed) {
                    "previewCancelled"
                } else {
                    "previewTimeout"
                },
                "Preview cancelled or exceeded two minutes.",
            ));
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    let stdout = out
        .join()
        .map_err(|_| GitError::new("preview", "Could not read preview output"))??;
    let stderr = err
        .join()
        .map_err(|_| GitError::new("preview", "Could not read preview errors"))??;
    Ok((status?, stdout, String::from_utf8_lossy(&stderr).into()))
}
fn required(path: &Path, args: &[&str], cancel: &AtomicBool, deadline: Instant) -> Result<Vec<u8>> {
    let (ok, out, error) = run(path, args, cancel, deadline)?;
    if !ok {
        return Err(GitError::new("preview", error));
    }
    Ok(out)
}
impl Repository {
    pub fn simulate(
        &self,
        parent: &Path,
        request: &PreviewRequest,
        cancel: Arc<AtomicBool>,
        progress: impl Fn(&str),
    ) -> Result<PreviewResult> {
        let mut probe = self.clone();
        probe.trusted = false;
        probe.auth = None;
        let names = probe
            .git_text(&[
                "config",
                "--name-only",
                "--get-regexp",
                r"^filter\..*\.(clean|smudge|process|required)$",
            ])
            .unwrap_or_default();
        for name in names.lines() {
            probe.query_config.push("-c".into());
            probe.query_config.push(format!(
                "{name}={}",
                if name.ends_with(".required") {
                    "false"
                } else {
                    ""
                }
            ));
        }
        probe.simulate_inner(parent, request, cancel, progress)
    }
    fn simulate_inner(
        &self,
        parent: &Path,
        request: &PreviewRequest,
        cancel: Arc<AtomicBool>,
        progress: impl Fn(&str),
    ) -> Result<PreviewResult> {
        if cancel.load(Ordering::Relaxed) {
            return Err(GitError::new("previewCancelled", "Preview cancelled."));
        }

        if !matches!(request.kind.as_str(), "merge" | "rebase" | "interactive") {
            return Err(GitError::new("preview", "Choose merge or rebase."));
        }
        let head = self
            .resolve("HEAD")
            .map_err(|_| GitError::new("preview", "Create an initial commit first."))?;
        let target = self.resolve(&request.target)?;
        if let Some(plan) = &request.plan {
            if request.kind != "interactive" || target != plan.base {
                return Err(GitError::new(
                    "preview",
                    "Preview base does not match the rebase plan.",
                ));
            }
        }
        // No configuration from the source repository is copied. Reject attributes rather than
        // pretending simulations without the user's custom drivers/filters predict those results.
        let mut inspected = vec![head.clone(), target.clone()];
        if request.kind != "merge" {
            let replayed =
                self.git_text(&["rev-list", "--max-count=201", &format!("{target}..{head}")])?;
            if replayed.lines().count() > 200 {
                return Err(GitError::new(
                    "previewUnsupported",
                    "Preview supports at most 200 replayed commits. Choose a smaller range.",
                ));
            }
            inspected.extend(replayed.lines().map(str::to_string));
        }
        if !self
            .git_text(&["config", "--path", "--get", "core.attributesfile"])
            .unwrap_or_default()
            .is_empty()
            || std::fs::read(self.common_dir.join("info/attributes"))
                .is_ok_and(|bytes| !bytes.is_empty())
        {
            return Err(GitError::new("previewUnsupported", "Global or repository-local attributes may change merge behavior. Preview cannot simulate them accurately."));
        }
        let mut paths = std::collections::HashSet::new();
        for oid in &inspected {
            let tree = self.git_text(&["ls-tree", "-r", "-z", oid])?;
            paths.extend(
                tree.split('\0')
                    .filter_map(|line| line.split_once('\t').map(|(_, path)| path.to_string())),
            );
            if tree.split('\0').any(|line| {
                line.starts_with("160000 ")
                    || line.starts_with("120000 ")
                    || line
                        .split('\t')
                        .nth(1)
                        .is_some_and(|p| p == ".gitattributes" || p.ends_with("/.gitattributes"))
            }) {
                return Err(GitError::new("previewUnsupported", "Preview currently requires regular files without submodules, symlinks, or .gitattributes. Custom filters and merge drivers cannot be simulated accurately."));
            }
        }
        let attribute_input: Vec<u8> = paths
            .iter()
            .flat_map(|p| p.as_bytes().iter().copied().chain(std::iter::once(0)))
            .collect();
        if !crate::runner::run(
            &self.root,
            false,
            None,
            &["check-attr", "--all", "-z", "--stdin"],
            Some(&attribute_input),
        )?
        .is_empty()
        {
            return Err(GitError::new("previewUnsupported", "Effective Git attributes may change merge behavior. Preview requires files without custom attributes."));
        }
        if !self
            .git_text(&[
                "config",
                "--local",
                "--get-regexp",
                r"^(merge\..*\.driver|filter\..*\.(clean|smudge|process)|core\.attributesfile)$",
            ])
            .unwrap_or_default()
            .is_empty()
        {
            return Err(GitError::new("previewUnsupported", "Custom merge drivers or filters are configured. Use the regular Git operation and review conflicts."));
        }
        // Validate working-tree state only after rejecting unsupported trees. Status can
        // recurse into submodules whose executable configuration is outside this probe.
        if let Some(plan) = &request.plan {
            self.validate_rebase(plan)?;
        }
        std::fs::create_dir_all(parent)?;
        let temp = tempfile::Builder::new()
            .prefix("simulation-")
            .tempdir_in(parent)?;
        let deadline = Instant::now() + Duration::from_secs(120);
        progress("Preparing isolated repository…");
        let dest = temp.path().join("repo");
        let root = self
            .root
            .to_str()
            .ok_or_else(|| GitError::new("unsupportedPath", "Non-UTF-8 repository path"))?;
        let destination = dest
            .to_str()
            .ok_or_else(|| GitError::new("unsupportedPath", "Non-UTF-8 preview path"))?;
        required(
            temp.path(),
            &[
                "clone",
                "--shared",
                "--no-checkout",
                "--no-tags",
                "--",
                root,
                destination,
            ],
            &cancel,
            deadline,
        )?;
        required(&dest, &["checkout", "--detach", &head], &cancel, deadline)?;
        progress("Simulating operation…");
        let (ok, _, detail) = if request.kind == "merge" {
            run(
                &dest,
                &["merge", "--no-ff", "--no-commit", "--no-edit", &target],
                &cancel,
                deadline,
            )?
        } else if request.kind == "interactive" {
            let plan = request
                .plan
                .as_ref()
                .ok_or_else(|| GitError::new("rebase", "Missing rebase plan"))?;
            self.simulate_rebase(&dest, plan, &cancel, deadline)?
        } else {
            run(
                &dest,
                &["rebase", "--no-autostash", "--no-rebase-merges", &target],
                &cancel,
                deadline,
            )?
        };
        let conflicts = names(&required(
            &dest,
            &["diff", "--name-only", "--diff-filter=U", "-z"],
            &cancel,
            deadline,
        )?)?;
        if !ok && conflicts.is_empty() {
            return Err(GitError::new("previewFailed", detail));
        }
        let files = names(&required(
            &dest,
            &["diff", "--name-only", "-z", &head],
            &cancel,
            deadline,
        )?)?;
        let patch = String::from_utf8_lossy(&required(
            &dest,
            &[
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                &head,
            ],
            &cancel,
            deadline,
        )?)
        .into_owned();
        if patch.len() > 2_000_000 {
            return Err(GitError::new(
                "previewLarge",
                "Preview diff exceeds the 2 MB editor limit.",
            ));
        }
        let stopped_commit = if !ok && request.kind != "merge" {
            run(
                &dest,
                &["rev-parse", "--verify", "REBASE_HEAD"],
                &cancel,
                deadline,
            )
            .ok()
            .filter(|v| v.0)
            .map(|v| String::from_utf8_lossy(&v.1).trim().into())
        } else {
            None
        };
        let revision = if request.kind == "merge" {
            format!("{head}..{target}")
        } else {
            format!("{target}..{head}")
        };
        let commits = self.history(&HistoryQuery {
            revision: Some(revision),
            search: None,
            path: None,
            skip: 0,
            limit: 200,
        })?;
        progress("Preview ready");
        Ok(PreviewResult {
            target_ref: request.target.clone(),
            kind: request.kind.clone(),
            head,
            target,
            commits,
            files,
            patch,
            remaining_untested: !ok && request.kind != "merge",
            conflicts,
            stopped_commit,
        })
    }
}
fn names(bytes: &[u8]) -> Result<Vec<String>> {
    bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| {
            String::from_utf8(s.to_vec())
                .map_err(|_| GitError::new("unsupportedPath", "Non-UTF-8 preview path"))
        })
        .collect()
}
