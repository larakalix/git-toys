use crate::{runner::run_output, types::*, Repository};
impl Repository {
    fn branch_name(&self, name: &str) -> Result<()> {
        self.git(&["check-ref-format", "--branch", name])?;
        if name.starts_with('-') || name.contains(['\n', '\r']) {
            return Err(GitError::new("invalidRef", "Invalid branch name"));
        }
        Ok(())
    }
    fn remote_name(&self, name: &str) -> Result<()> {
        if name.is_empty()
            || name.starts_with('-')
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        {
            return Err(GitError::new(
                "invalidRemote",
                "Use letters, numbers, dash, underscore or dot in remote names.",
            ));
        }
        Ok(())
    }
    pub fn execute(&self, action: &Action, expected: &str) -> Result<String> {
        if !self.trusted {
            return Err(GitError::new("untrusted","Trust this repository before running operations. Hooks, filters, and signing tools can execute local programs."));
        }
        if self.snapshot()?.state_token != expected {
            return Err(GitError::new(
                "staleState",
                "Repository changed outside Git Toys. Refresh and review the action again.",
            ));
        }
        let mut args: Vec<String> = Vec::new();
        let mut input: Option<Vec<u8>> = None;
        let mut set = |values: &[&str]| args = values.iter().map(|s| (*s).into()).collect();
        match action {
            Action::InteractiveRebase { plan } => return self.interactive_rebase(plan),
            Action::ApplyPreview {
                kind_name,
                head,
                target,
                target_ref,
            } => {
                let snap = self.snapshot()?;
                if snap.head != *head || !snap.files.is_empty() || snap.operation.is_some() {
                    return Err(GitError::new(
                        "stalePreview",
                        "Repository changed or is dirty. Generate a new preview.",
                    ));
                }
                if self.resolve(target_ref)? != *target {
                    return Err(GitError::new(
                        "stalePreview",
                        "Target branch changed. Generate a new preview.",
                    ));
                }
                let target = self.resolve(target)?;
                let next = match kind_name.as_str() {
                    "merge" => Action::Merge { target },
                    "rebase" => Action::Rebase { target },
                    _ => return Err(GitError::new("preview", "Invalid preview action")),
                };
                return self.execute(&next, expected);
            }
            Action::CheckoutPullRequest {
                remote,
                number,
                head,
            } => return self.checkout_pull_request(remote, *number, head),
            Action::StageGroup { files } => return self.stage_group(files),
            Action::AddWorktree {
                path,
                branch,
                new_branch,
                start,
            } => return self.add_worktree(path, branch, *new_branch, start),
            Action::RemoveWorktree { path } => return self.remove_worktree(path),
            Action::RestoreFileVersion {
                target,
                source,
                revision,
            } => return self.restore_file_version(target, source, revision),
            Action::Stage { paths } | Action::Unstage { paths } | Action::Discard { paths } => {
                if paths.is_empty() {
                    return Err(GitError::new("invalidPath", "Select at least one file"));
                }
                let snap = self.snapshot()?;
                for path in paths {
                    self.relative(path)?;
                    if !snap.files.iter().any(|f| &f.path == path) {
                        return Err(GitError::new(
                            "staleState",
                            "Selected file is no longer changed",
                        ));
                    }
                }
                match action {
                    Action::Stage { .. } => set(&["add", "--"]),
                    Action::Unstage { .. } => {
                        if snap.head == "(initial)" {
                            set(&["rm", "--cached", "--"])
                        } else {
                            set(&["restore", "--staged", "--"])
                        }
                    }
                    _ => {
                        if paths.iter().any(|p| {
                            snap.files
                                .iter()
                                .any(|f| &f.path == p && (f.untracked || f.conflict || f.submodule))
                        }) {
                            return Err(GitError::new("unsupported","Discard is limited to tracked, non-conflicted files. Use the terminal for untracked files or submodules."));
                        }
                        set(&["restore", "--worktree", "--"])
                    }
                }
                args.extend(paths.iter().cloned());
                if matches!(action, Action::Unstage { .. }) {
                    for file in &snap.files {
                        if paths.contains(&file.path) {
                            if let Some(original) = &file.original_path {
                                args.push(original.clone());
                            }
                        }
                    }
                }
            }
            Action::StageHunk {
                path,
                hunk,
                reverse,
            } => {
                let diff = self.diff(&DiffRequest {
                    path: path.clone(),
                    staged: *reverse,
                    commit: None,
                    base: None,
                })?;
                let patch = diff
                    .hunks
                    .get(*hunk as usize)
                    .ok_or_else(|| GitError::new("invalidHunk", "Hunk is no longer available"))?;
                set(&["apply", "--cached", "--recount", "--whitespace=nowarn"]);
                if *reverse {
                    args.push("--reverse".into())
                }
                input = Some(patch.as_bytes().to_vec());
            }
            Action::Commit { message, amend } => {
                if message.trim().is_empty() {
                    return Err(GitError::new("invalidMessage", "Write a commit message"));
                }
                set(&["commit", "--file=-"]);
                if *amend {
                    args.push("--amend".into())
                }
                input = Some(message.as_bytes().to_vec())
            }
            Action::CreateBranch { name, start } => {
                self.branch_name(name)?;
                set(&["branch", name]);
                if let Some(start) = start.as_deref().filter(|v| !v.is_empty()) {
                    args.push(self.resolve(start)?)
                }
            }
            Action::SwitchBranch { name } => {
                self.branch_name(name)?;
                set(&["switch", "--no-guess", name])
            }
            Action::RenameBranch { old, name } => {
                self.branch_name(old)?;
                self.branch_name(name)?;
                set(&["branch", "-m", old, name])
            }
            Action::DeleteBranch { name, force } => {
                self.branch_name(name)?;
                set(&["branch", if *force { "-D" } else { "-d" }, name])
            }
            Action::SetUpstream { branch, upstream } => {
                self.branch_name(branch)?;
                self.branch_name(upstream)?;
                set(&["branch", "--set-upstream-to", upstream, branch])
            }
            Action::CreateTag { name, target } => {
                self.git(&["check-ref-format", &format!("refs/tags/{name}")])?;
                if name.starts_with('-') {
                    return Err(GitError::new("invalidRef", "Invalid tag name"));
                }
                set(&["tag", name]);
                args.push(self.resolve(target)?)
            }
            Action::DeleteTag { name } => {
                self.git(&["check-ref-format", &format!("refs/tags/{name}")])?;
                set(&["tag", "-d", "--", name])
            }
            Action::AddRemote { name, url } | Action::EditRemote { name, url } => {
                self.remote_name(name)?;
                if url.is_empty() || url.starts_with('-') || url.starts_with("ext::") {
                    return Err(GitError::new("invalidRemote", "Enter a valid remote URL"));
                }
                set(&[
                    "remote",
                    if matches!(action, Action::AddRemote { .. }) {
                        "add"
                    } else {
                        "set-url"
                    },
                    name,
                    url,
                ])
            }
            Action::RemoveRemote { name } => {
                self.remote_name(name)?;
                set(&["remote", "remove", name])
            }
            Action::Fetch { remote } => {
                self.remote_name(remote)?;
                set(&["fetch", "--progress", remote])
            }
            Action::Pull { remote, strategy } => {
                self.remote_name(remote)?;
                let flag = match strategy.as_str() {
                    "ff-only" => "--ff-only",
                    "merge" => "--no-rebase",
                    "rebase" => "--rebase",
                    _ => {
                        return Err(GitError::new(
                            "invalidStrategy",
                            "Choose fast-forward, merge or rebase",
                        ))
                    }
                };
                set(&["pull", flag, "--no-autostash", remote])
            }
            Action::Push {
                remote,
                publish,
                force,
            } => {
                self.remote_name(remote)?;
                let snapshot = self.snapshot()?;
                if snapshot.branch == "(detached)" {
                    return Err(GitError::new(
                        "detachedHead",
                        "Create or switch to a branch before pushing",
                    ));
                }
                set(&["push", "--progress"]);
                if *publish {
                    args.push("--set-upstream".into())
                }
                if *force {
                    let tracking = format!("refs/remotes/{remote}/{}", snapshot.branch);
                    let expected = self.resolve(&tracking)?;
                    args.push(format!(
                        "--force-with-lease=refs/heads/{}:{expected}",
                        snapshot.branch
                    ));
                }
                args.push(remote.clone());
                args.push(format!("HEAD:refs/heads/{}", snapshot.branch))
            }
            Action::Merge { target }
            | Action::Rebase { target }
            | Action::CherryPick { target }
            | Action::Revert { target }
            | Action::Reset { target, .. } => {
                let oid = self.resolve(target)?;
                match action {
                    Action::Merge { .. } => set(&["merge", "--no-edit", "--no-autostash"]),
                    Action::Rebase { .. } => set(&["rebase", "--no-autostash"]),
                    Action::CherryPick { .. } => set(&["cherry-pick"]),
                    Action::Revert { .. } => set(&["revert", "--no-edit"]),
                    Action::Reset { mode, .. } => {
                        let flag = match mode.as_str() {
                            "soft" => "--soft",
                            "mixed" => "--mixed",
                            "hard" => "--hard",
                            _ => {
                                return Err(GitError::new(
                                    "invalidMode",
                                    "Choose soft, mixed or hard reset",
                                ))
                            }
                        };
                        set(&["reset", flag])
                    }
                    _ => unreachable!(),
                };
                args.push(oid);
            }
            Action::Stash { message } => set(&["stash", "push", "-u", "-m", message]),
            Action::ApplyStash { index, pop } => set(&[
                "stash",
                if *pop { "pop" } else { "apply" },
                &format!("stash@{{{index}}}"),
            ]),
            Action::DropStash { index } => set(&["stash", "drop", &format!("stash@{{{index}}}")]),
            Action::Continue | Action::Abort => {
                let op = self
                    .operation()
                    .ok_or_else(|| GitError::new("noOperation", "No operation is in progress"))?;
                if op == "rebase" && self.git_dir.join("git-toys-rebase-plan.json").exists() {
                    return self.continue_visual_rebase(matches!(action, Action::Abort));
                }
                if op == "merge" && matches!(action, Action::Continue) {
                    set(&["commit", "--no-edit"])
                } else {
                    set(&[
                        &op,
                        if matches!(action, Action::Continue) {
                            "--continue"
                        } else {
                            "--abort"
                        },
                    ])
                }
            }
            Action::Resolve { path, content } => {
                let conflict = self.conflict(path)?;
                if conflict.binary {
                    return Err(GitError::new(
                        "binary",
                        "Choose ours/theirs for binary files",
                    ));
                }
                let dest = self.relative(path)?;
                let parent = dest
                    .parent()
                    .ok_or_else(|| GitError::new("invalidPath", "Invalid conflict path"))?
                    .canonicalize()?;
                if !parent.starts_with(&self.root)
                    || std::fs::symlink_metadata(&dest).is_ok_and(|m| m.file_type().is_symlink())
                {
                    return Err(GitError::new(
                        "invalidPath",
                        "Refusing to write through a symlink",
                    ));
                }
                if content.lines().any(|line| {
                    line.starts_with("<<<<<<< ")
                        || line == "======="
                        || line.starts_with(">>>>>>> ")
                }) {
                    return Err(GitError::new(
                        "conflictMarkers",
                        "Remove conflict markers before marking resolved",
                    ));
                }
                std::fs::write(dest, content)?;
                set(&["add", "--", path]);
            }
            Action::ResolveSide { path, side } => {
                self.conflict(path)?;
                let flag = match side.as_str() {
                    "ours" => "--ours",
                    "theirs" => "--theirs",
                    _ => return Err(GitError::new("invalidSide", "Choose ours or theirs")),
                };
                self.git(&["checkout", flag, "--", path])?;
                set(&["add", "--", path]);
            }
        }
        let borrowed: Vec<_> = args.iter().map(String::as_str).collect();
        run_output(
            &self.root,
            true,
            self.auth.as_ref(),
            &borrowed,
            input.as_deref(),
        )
        .map(|(out, err)| {
            format!(
                "{}{}",
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&err)
            )
        })
    }
}
