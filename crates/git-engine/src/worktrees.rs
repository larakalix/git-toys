use crate::{types::*, Repository};
use std::path::Path;
impl Repository {
    pub fn worktrees(&self) -> Result<Vec<Worktree>> {
        let bytes = self.git(&["worktree", "list", "--porcelain", "-z"])?;
        let text = String::from_utf8(bytes)
            .map_err(|_| GitError::new("unsupportedPath", "A worktree has a non-UTF-8 path."))?;
        let mut result = Vec::new();
        for record in text.split("\0\0").filter(|record| !record.is_empty()) {
            let fields: Vec<_> = record.split('\0').collect();
            let value = |prefix: &str| fields.iter().find_map(|field| field.strip_prefix(prefix));
            let Some(path) = value("worktree ") else {
                continue;
            };
            let missing = !Path::new(path).is_dir();
            let other = if missing {
                None
            } else {
                Repository::open(Path::new(path))
                    .ok()
                    .filter(|repo| repo.common_dir == self.common_dir)
            };
            let dirty = other.as_ref().and_then(|repo| {
                repo.git(&[
                    "status",
                    "--porcelain=v2",
                    "-z",
                    "--untracked-files=all",
                    "--ignored=matching",
                ])
                .ok()
                .map(|bytes| !bytes.is_empty())
            });
            result.push(Worktree {
                path: path.into(),
                branch: value("branch refs/heads/").unwrap_or("(detached)").into(),
                head: value("HEAD ").unwrap_or("").into(),
                main: result.is_empty(),
                current: Path::new(path)
                    .canonicalize()
                    .is_ok_and(|path| path == self.root),
                dirty,
                locked: fields.iter().find_map(|field| {
                    if *field == "locked" {
                        Some(String::new())
                    } else {
                        field.strip_prefix("locked ").map(str::to_string)
                    }
                }),
                missing,
                prunable: fields
                    .iter()
                    .any(|field| *field == "prunable" || field.starts_with("prunable ")),
                operation: other.as_ref().and_then(Repository::operation),
            });
        }
        Ok(result)
    }
    pub(crate) fn add_worktree(
        &self,
        path: &str,
        branch: &str,
        new_branch: bool,
        start: &str,
    ) -> Result<String> {
        let destination = Path::new(path);
        if !destination.is_absolute() || std::fs::symlink_metadata(destination).is_ok() {
            return Err(GitError::new(
                "worktreeDestination",
                "Choose a new absolute folder path that does not exist.",
            ));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| GitError::new("invalidPath", "Choose a destination folder."))?;
        if !parent.is_dir() {
            return Err(GitError::new(
                "worktreeDestination",
                "The destination’s parent folder must already exist.",
            ));
        }
        self.git(&["check-ref-format", "--branch", branch])?;
        if branch.starts_with('-') || branch.contains(['\n', '\r']) {
            return Err(GitError::new("invalidRef", "Invalid branch name."));
        }
        if self.worktrees()?.iter().any(|tree| tree.branch == branch) {
            return Err(GitError::new(
                "worktreeBranch",
                "That branch is already checked out in a workspace.",
            ));
        }
        if new_branch {
            let oid = self.resolve(start)?;
            self.git(&["worktree", "add", "-b", branch, "--", path, &oid])?;
        } else {
            self.resolve(&format!("refs/heads/{branch}"))?;
            self.git(&["worktree", "add", "--", path, branch])?;
        }
        Ok("Workspace created. Open it from Worktrees.".into())
    }
    pub(crate) fn remove_worktree(&self, path: &str) -> Result<String> {
        let tree = self
            .worktrees()?
            .into_iter()
            .find(|tree| {
                tree.path == path
                    || Path::new(path)
                        .canonicalize()
                        .is_ok_and(|requested| Path::new(&tree.path) == requested)
            })
            .ok_or_else(|| {
                GitError::new(
                    "worktreeMissing",
                    "Workspace is no longer registered. Refresh the list.",
                )
            })?;
        if tree.main
            || tree.missing
            || tree.locked.is_some()
            || tree.prunable
            || tree.dirty != Some(false)
            || tree.operation.is_some()
        {
            return Err(GitError::new("worktreeUnsafe", "Cannot remove a main, missing, locked, dirty, unreadable, or actively operating workspace. Resolve its state first. Ignored files also count as local work."));
        }
        self.git(&["worktree", "remove", "--", &tree.path])?;
        Ok("Workspace removed. Its branch is preserved.".into())
    }
}
