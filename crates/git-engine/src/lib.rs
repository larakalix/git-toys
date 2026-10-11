mod actions;
mod ai_context;
mod file_history;
mod organizer;
mod pull_requests;
pub use pull_requests::github_repository;
mod rebase;
pub mod simulation;
pub use rebase::rebase_editor_entry;
mod parse;
pub mod runner;
pub mod types;
mod worktrees;
use runner::{run, text, Auth};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Component, Path, PathBuf},
};
use types::*;

#[derive(Clone)]
pub struct Repository {
    pub root: PathBuf,
    pub common_dir: PathBuf,
    pub git_dir: PathBuf,
    pub trusted: bool,
    pub auth: Option<Auth>,
    pub editor_executable: Option<PathBuf>,
    pub(crate) query_config: Vec<String>,
}
impl Repository {
    pub fn open(path: &Path) -> Result<Self> {
        let diagnostic = diagnostics();
        if !diagnostic.supported {
            return Err(GitError::new("unsupportedGit", diagnostic.message));
        }
        let root = PathBuf::from(text(run(
            path,
            false,
            None,
            &["rev-parse", "--show-toplevel"],
            None,
        )?))
        .canonicalize()?;
        let git_dir = PathBuf::from(text(run(
            &root,
            false,
            None,
            &["rev-parse", "--absolute-git-dir"],
            None,
        )?));
        let common_dir = PathBuf::from(text(run(
            &root,
            false,
            None,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            None,
        )?))
        .canonicalize()?;
        Ok(Self {
            root,
            common_dir,
            git_dir,
            trusted: false,
            auth: None,
            editor_executable: None,
            query_config: Vec::new(),
        })
    }
    pub fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
        let configured: Vec<&str> = self
            .query_config
            .iter()
            .map(String::as_str)
            .chain(args.iter().copied())
            .collect();
        run(
            &self.root,
            self.trusted,
            self.auth.as_ref(),
            &configured,
            None,
        )
    }
    pub fn git_text(&self, args: &[&str]) -> Result<String> {
        self.git(args).map(text)
    }
    pub fn resolve(&self, revision: &str) -> Result<String> {
        self.git_text(&[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ])
    }
    pub fn relative(&self, path: &str) -> Result<PathBuf> {
        let p = Path::new(path);
        if path.is_empty()
            || p.components().any(|c| !matches!(c, Component::Normal(_)))
            || path
                .split(['/', '\\'])
                .any(|v| v.eq_ignore_ascii_case(".git"))
        {
            return Err(GitError::new(
                "invalidPath",
                "Choose a file inside this repository.",
            ));
        }
        Ok(self.root.join(p))
    }
    pub fn operation(&self) -> Option<String> {
        [
            ("rebase-merge", "rebase"),
            ("rebase-apply", "rebase"),
            ("MERGE_HEAD", "merge"),
            ("CHERRY_PICK_HEAD", "cherry-pick"),
            ("REVERT_HEAD", "revert"),
        ]
        .iter()
        .find(|(p, _)| self.git_dir.join(p).exists())
        .map(|(_, op)| (*op).into())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let bytes = self.git(&[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ])?;
        let files = parse::status(&bytes)?;
        let headers = String::from_utf8_lossy(&bytes);
        let header = |key: &str| {
            headers
                .split('\0')
                .find_map(|s| s.strip_prefix(key))
                .unwrap_or("")
                .to_string()
        };
        let head = header("# branch.oid ");
        let branch = header("# branch.head ");
        let upstream = header("# branch.upstream ");
        let ab = header("# branch.ab ");
        let mut ahead_behind = ab.split_whitespace();
        let ahead = ahead_behind
            .next()
            .unwrap_or("+0")
            .trim_start_matches('+')
            .parse()
            .unwrap_or(0);
        let behind = ahead_behind
            .next()
            .unwrap_or("-0")
            .trim_start_matches('-')
            .parse()
            .unwrap_or(0);
        let raw_refs = self.git_text(&[
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00%(upstream:short)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ])?;
        let refs = raw_refs
            .lines()
            .filter_map(|line| {
                let mut p = line.split('\0');
                let full = p.next()?;
                let oid = p.next()?.into();
                let upstream = p.next().unwrap_or("").into();
                let (kind, name) = if let Some(v) = full.strip_prefix("refs/heads/") {
                    ("branch", v)
                } else if let Some(v) = full.strip_prefix("refs/remotes/") {
                    ("remote", v)
                } else {
                    ("tag", full.strip_prefix("refs/tags/")?)
                };
                Some(GitRef {
                    name: name.into(),
                    oid,
                    kind: kind.into(),
                    upstream,
                })
            })
            .collect();
        let mut remotes = Vec::new();
        for name in self.git_text(&["remote"])?.lines() {
            remotes.push(Remote {
                name: name.into(),
                url: self.git_text(&["config", "--get", &format!("remote.{name}.url")])?,
            });
        }
        let mut hash = DefaultHasher::new();
        bytes.hash(&mut hash);
        raw_refs.hash(&mut hash);
        for remote in &remotes {
            (&remote.name, &remote.url).hash(&mut hash);
        }
        for file in &files {
            if let Ok(meta) = std::fs::symlink_metadata(self.relative(&file.path)?) {
                meta.len().hash(&mut hash);
                meta.modified().ok().hash(&mut hash);
            }
        }
        if let Ok(meta) = std::fs::metadata(self.git_dir.join("index")) {
            meta.modified().ok().hash(&mut hash);
        }
        let operation = self.operation();
        operation.hash(&mut hash);
        let mut warnings = Vec::new();
        if self.root.join(".gitmodules").exists() {
            warnings.push("Submodules detected. This version shows their status; manage nested repositories separately.".into());
        }
        if String::from_utf8_lossy(&self.blob(":.gitattributes")).contains("filter=lfs")
            || self.root.join(".lfsconfig").exists()
        {
            warnings.push("Git LFS attributes detected. Install Git LFS; dedicated LFS management is planned.".into());
        }
        Ok(Snapshot {
            path: self.root.to_string_lossy().into(),
            name: self
                .root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            branch,
            head,
            upstream,
            ahead,
            behind,
            files,
            refs,
            remotes,
            operation,
            state_token: format!("{:x}", hash.finish()),
            warnings,
        })
    }
    pub fn history(&self, query: &HistoryQuery) -> Result<Vec<Commit>> {
        let limit = query.limit.clamp(1, 500) as usize;
        let revision = query
            .revision
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(|v| {
                if let Some((from, to)) = v.split_once("..") {
                    Ok(format!("{}..{}", self.resolve(from)?, self.resolve(to)?))
                } else {
                    self.resolve(v)
                }
            })
            .transpose()?;
        let search = query.search.as_deref().unwrap_or("").to_lowercase();
        let path = query.path.as_deref().filter(|v| !v.is_empty());
        if let Some(path) = path {
            self.relative(path)?;
        }
        let mut result = Vec::new();
        let mut offset = if search.is_empty() { query.skip } else { 0 };
        let mut matched = 0;
        loop {
            let count = if search.is_empty() { limit } else { 500 };
            let count_text = count.to_string();
            let skip_text = offset.to_string();
            let mut args = vec![
                "log",
                "--topo-order",
                "--format=%H%x00%P%x00%an%x00%ae%x00%aI%x00%B%x00%D%x00",
                "-z",
                "--max-count",
                &count_text,
                "--skip",
                &skip_text,
            ];
            if let Some(revision) = revision.as_deref() {
                args.push(revision);
            } else {
                args.push("--all");
                if self.git(&["rev-parse", "--verify", "HEAD"]).is_ok() {
                    args.push("HEAD");
                }
            }
            args.push("--");
            if let Some(path) = path {
                args.push(path);
            }
            let bytes = self.git(&args)?;
            let fields: Vec<_> = bytes.split(|b| *b == 0).collect();
            let mut scanned = 0;
            for chunk in fields.chunks(8) {
                if chunk.len() < 7 || chunk[0].is_empty() {
                    continue;
                }
                scanned += 1;
                let value = |i: usize| {
                    String::from_utf8_lossy(chunk[i])
                        .trim_matches('\n')
                        .to_string()
                };
                let oid = value(0);
                let author = value(2);
                let message = value(5);
                let subject = message.lines().next().unwrap_or("").to_string();
                let matches = search.is_empty()
                    || message.to_lowercase().contains(&search)
                    || author.to_lowercase().contains(&search)
                    || oid.starts_with(&search);
                if !matches {
                    continue;
                }
                if !search.is_empty() {
                    matched += 1;
                    if matched <= query.skip {
                        continue;
                    }
                }
                result.push(Commit {
                    oid,
                    parents: value(1).split_whitespace().map(str::to_string).collect(),
                    author,
                    email: value(3),
                    date: value(4),
                    subject,
                    message,
                    refs: value(6),
                });
                if result.len() >= limit {
                    return Ok(result);
                }
            }
            if search.is_empty() || scanned < count {
                return Ok(result);
            }
            offset += scanned as u32;
        }
    }
    pub fn commit_files(&self, revision: &str, base: Option<&str>) -> Result<Vec<String>> {
        let oid = self.resolve(revision)?;
        let from = base
            .map(|base| self.resolve(base))
            .transpose()?
            .or_else(|| self.resolve(&format!("{oid}^1")).ok());
        let bytes = if let Some(from) = from {
            self.git(&["diff", "--name-only", "-z", &from, &oid, "--"])?
        } else {
            self.git(&[
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-only",
                "-r",
                "-z",
                &oid,
                "--",
            ])?
        };
        bytes
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| {
                String::from_utf8(p.to_vec())
                    .map_err(|_| GitError::new("unsupportedPath", "Non-UTF-8 filename"))
            })
            .collect()
    }
    fn blob(&self, spec: &str) -> Vec<u8> {
        self.git(&["show", "--no-ext-diff", "--no-textconv", spec])
            .unwrap_or_default()
    }
    pub fn diff(&self, req: &DiffRequest) -> Result<Diff> {
        let path = self.relative(&req.path)?;
        let mut args = vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
            "--binary",
        ];
        let mut base_oid = None;
        let mut oid = None;
        let (original, modified) = if let Some(revision) = &req.commit {
            let to = self.resolve(revision)?;
            let from = if let Some(base) = &req.base {
                Some(self.resolve(base)?)
            } else {
                self.resolve(&format!("{to}^1")).ok()
            };
            let original = from
                .as_ref()
                .map(|v| self.blob(&format!("{v}:{}", req.path)))
                .unwrap_or_default();
            let modified = self.blob(&format!("{to}:{}", req.path));
            base_oid =
                Some(from.unwrap_or(self.git_text(&["hash-object", "-t", "tree", "--stdin"])?));
            oid = Some(to);
            (original, modified)
        } else if req.staged {
            args.push("--cached");
            (
                self.blob(&format!("HEAD:{}", req.path)),
                self.blob(&format!(":{}", req.path)),
            )
        } else {
            let meta = std::fs::symlink_metadata(&path).ok();
            let content = if meta.as_ref().is_some_and(|m| m.file_type().is_symlink()) {
                std::fs::read_link(&path)?
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec()
            } else if meta.as_ref().is_some_and(|m| m.is_file()) {
                std::fs::read(&path)?
            } else {
                Vec::new()
            };
            (self.blob(&format!(":{}", req.path)), content)
        };
        if let Some(v) = base_oid.as_deref() {
            args.push(v)
        }
        if let Some(v) = oid.as_deref() {
            args.push(v)
        }
        args.extend(["--", &req.path]);
        let patch = String::from_utf8_lossy(&self.git(&args)?).to_string();
        let binary = original.contains(&0)
            || modified.contains(&0)
            || std::str::from_utf8(&original).is_err()
            || std::str::from_utf8(&modified).is_err();
        let truncated =
            original.len() > 2_000_000 || modified.len() > 2_000_000 || patch.len() > 2_000_000;
        let safe = |v: Vec<u8>| {
            if binary || truncated {
                String::new()
            } else {
                String::from_utf8_lossy(&v).into()
            }
        };
        let hunks = if binary || truncated {
            Vec::new()
        } else {
            parse::hunks(&patch)
        };
        Ok(Diff {
            path: req.path.clone(),
            original: safe(original),
            modified: safe(modified),
            patch: if truncated { String::new() } else { patch },
            binary,
            truncated,
            hunks,
        })
    }
    pub fn conflict(&self, path: &str) -> Result<Conflict> {
        self.relative(path)?;
        if !self
            .snapshot()?
            .files
            .iter()
            .any(|f| f.path == path && f.conflict)
        {
            return Err(GitError::new(
                "notConflict",
                "This file is no longer conflicted.",
            ));
        }
        let base = self.blob(&format!(":1:{path}"));
        let ours = self.blob(&format!(":2:{path}"));
        let theirs = self.blob(&format!(":3:{path}"));
        let result = std::fs::read(self.relative(path)?).unwrap_or_default();
        let binary = [&base, &ours, &theirs, &result]
            .iter()
            .any(|v| v.contains(&0) || std::str::from_utf8(v).is_err());
        let content = |v: Vec<u8>| {
            if binary {
                String::new()
            } else {
                String::from_utf8_lossy(&v).into()
            }
        };
        Ok(Conflict {
            path: path.into(),
            base: content(base),
            ours: content(ours),
            theirs: content(theirs),
            result: content(result),
            binary,
        })
    }
    fn read_reflog(&self, args: &[&str]) -> Result<Vec<ReflogEntry>> {
        Ok(self
            .git_text(args)?
            .lines()
            .filter_map(|line| {
                let columns: Vec<_> = line.split('\0').collect();
                Some(ReflogEntry {
                    oid: (*columns.first()?).into(),
                    selector: (*columns.get(1)?).into(),
                    subject: (*columns.get(2)?).into(),
                })
            })
            .collect())
    }
    pub fn reflog(&self) -> Result<Vec<ReflogEntry>> {
        self.read_reflog(&["reflog", "--max-count=200", "--format=%H%x00%gD%x00%gs"])
    }
    pub fn stashes(&self) -> Result<Vec<ReflogEntry>> {
        self.read_reflog(&["stash", "list", "--format=%H%x00%gd%x00%gs"])
    }
}

pub fn diagnostics() -> Diagnostic {
    match run(Path::new("."), false, None, &["--version"], None) {
        Ok(bytes) => {
            let version = text(bytes);
            let parts: Vec<u32> = version
                .split_whitespace()
                .nth(2)
                .unwrap_or("")
                .split('.')
                .take(2)
                .map(|p| p.parse().unwrap_or(0))
                .collect();
            let supported = parts.as_slice() >= [2, 43].as_slice();
            Diagnostic {
                version,
                supported,
                message: if supported {
                    "Git is ready. Your existing credentials and signing configuration are used."
                } else {
                    "Install Git 2.43 or newer, then restart Git Toys."
                }
                .into(),
            }
        }
        Err(e) => Diagnostic {
            version: "Not found".into(),
            supported: false,
            message: e.message,
        },
    }
}
