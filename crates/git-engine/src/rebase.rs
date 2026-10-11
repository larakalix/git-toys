use crate::{runner, simulation, types::*, Repository};
use std::{collections::HashSet, path::Path, sync::atomic::AtomicBool, time::Instant};

impl Repository {
    pub fn rebase_range(&self, base: &str) -> Result<RebaseRange> {
        let snap = self.snapshot()?;
        if snap.branch.starts_with('(') || !snap.files.is_empty() || snap.operation.is_some() {
            return Err(GitError::new("rebaseState", "Choose an attached branch with a clean index and working tree, and finish any active operation."));
        }
        let head = self.resolve("HEAD")?;
        let base = self.resolve(base)?;
        self.git(&["merge-base", "--is-ancestor", &base, &head])?;
        let range = format!("{base}..{head}");
        let ids = self.git_text(&["rev-list", "--reverse", &range])?;
        let mut commits = self.history(&HistoryQuery {
            revision: Some(range),
            search: None,
            path: None,
            skip: 0,
            limit: 200,
        })?;
        if ids.lines().count() != commits.len()
            || commits.is_empty()
            || commits.iter().any(|c| c.parents.len() != 1)
        {
            return Err(GitError::new(
                "rebaseRange",
                "Select 1–200 commits in a linear range without merge commits.",
            ));
        }
        commits.sort_by_key(|c| ids.lines().position(|v| v == c.oid).unwrap_or(usize::MAX));
        let mut published = false;
        for commit in &commits {
            if !self
                .git_text(&[
                    "for-each-ref",
                    "--format=%(refname)",
                    &format!("--contains={}", commit.oid),
                    "refs/remotes/",
                ])?
                .is_empty()
            {
                published = true;
                break;
            }
        }
        let steps = commits
            .iter()
            .map(|c| RebaseStep {
                oid: c.oid.clone(),
                action: "pick".into(),
                message: c.message.clone(),
            })
            .collect();
        Ok(RebaseRange {
            plan: RebasePlan { head, base, steps },
            commits,
            published,
        })
    }
    pub(crate) fn validate_rebase(&self, plan: &RebasePlan) -> Result<()> {
        let range = self.rebase_range(&plan.base)?;
        let original: HashSet<_> = range.plan.steps.iter().map(|s| &s.oid).collect();
        let submitted: HashSet<_> = plan.steps.iter().map(|s| &s.oid).collect();
        if plan.head != range.plan.head
            || plan.base != range.plan.base
            || original != submitted
            || plan.steps.len() != original.len()
        {
            return Err(GitError::new(
                "staleRebase",
                "History changed or the plan contains invalid commits. Load the range again.",
            ));
        }
        let mut retained = false;
        for step in &plan.steps {
            match step.action.as_str() {
                "pick" | "reword" => retained = true,
                "squash" | "fixup" if retained => {}
                "drop" => {}
                _ => {
                    return Err(GitError::new(
                        "rebasePlan",
                        "Choose valid actions; squash and fixup need a preceding retained commit.",
                    ))
                }
            }
            if matches!(step.action.as_str(), "reword" | "squash")
                && (step.message.trim().is_empty()
                    || step.message.len() > 8192
                    || step.message.contains('\0'))
            {
                return Err(GitError::new(
                    "rebaseMessage",
                    "Write a nonempty commit message of at most 8192 bytes.",
                ));
            }
        }
        if !retained {
            return Err(GitError::new("rebasePlan", "Keep at least one commit."));
        }
        Ok(())
    }
    fn editor(&self) -> Result<std::path::PathBuf> {
        self.editor_executable
            .clone()
            .map(Ok)
            .unwrap_or_else(|| std::env::current_exe().map_err(Into::into))
    }
    pub(crate) fn interactive_rebase(&self, plan: &RebasePlan) -> Result<String> {
        self.validate_rebase(plan)?;
        let name = format!(
            "git-toys/recovery/{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        self.git(&["branch", &name, &plan.head])?;
        let file = self.git_dir.join("git-toys-rebase-plan.json");
        if std::fs::symlink_metadata(&file)
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err(GitError::new("rebasePlan", "Unsafe editor plan path."));
        }
        std::fs::write(
            &file,
            serde_json::to_vec(plan).map_err(|e| GitError::new("rebasePlan", e.to_string()))?,
        )?;
        let result = runner::run_output_editor(
            &self.root,
            true,
            self.auth.as_ref(),
            &[
                "-c",
                "rebase.abbreviateCommands=false",
                "-c",
                "core.abbrev=40",
                "rebase",
                "--interactive",
                "--no-autostash",
                "--no-autosquash",
                "--no-rebase-merges",
                &plan.base,
            ],
            None,
            Some((&self.editor()?, &file)),
        );
        if self.operation().is_none() {
            let _ = std::fs::remove_file(&file);
        }
        result.map(|_| format!("History updated. Original commits remain on {name}."))
    }
    pub(crate) fn continue_visual_rebase(&self, abort: bool) -> Result<String> {
        let file = self.git_dir.join("git-toys-rebase-plan.json");
        let result = runner::run_output_editor(
            &self.root,
            true,
            self.auth.as_ref(),
            &["rebase", if abort { "--abort" } else { "--continue" }],
            None,
            Some((&self.editor()?, &file)),
        );
        if self.operation().is_none() {
            let _ = std::fs::remove_file(file);
        }
        result.map(|(out, err)| {
            format!(
                "{}{}",
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&err)
            )
        })
    }
    pub(crate) fn simulate_rebase(
        &self,
        dest: &Path,
        plan: &RebasePlan,
        cancel: &AtomicBool,
        deadline: Instant,
    ) -> Result<(bool, Vec<u8>, String)> {
        let file = dest.join(".git/git-toys-rebase-plan.json");
        std::fs::write(
            &file,
            serde_json::to_vec(plan).map_err(|e| GitError::new("rebasePlan", e.to_string()))?,
        )?;
        // The helper runs the same restricted native todo in the disposable repository.
        let editor = self.editor()?;
        let editor = editor.to_string_lossy().replace('\\', "/");
        let quoted = format!("'{}'", editor.replace('\'', "'\\''"));
        let mut command = simulation::command(dest);
        command
            .env("GIT_SEQUENCE_EDITOR", &quoted)
            .env("GIT_EDITOR", &quoted)
            .env("GIT_TOYS_REBASE_PLAN", &file);
        simulation::run_command(
            command,
            &[
                "-c",
                "rebase.abbreviateCommands=false",
                "-c",
                "core.abbrev=40",
                "rebase",
                "--interactive",
                "--no-autostash",
                "--no-autosquash",
                "--no-rebase-merges",
                &plan.base,
            ],
            cancel,
            deadline,
        )
    }
}

pub fn rebase_editor_entry() -> bool {
    let Some(plan_path) = std::env::var_os("GIT_TOYS_REBASE_PLAN") else {
        return false;
    };
    let result = (|| -> Result<()> {
        let plan_path = Path::new(&plan_path).canonicalize()?;
        let parent = plan_path
            .parent()
            .ok_or_else(|| GitError::new("editor", "Missing plan directory"))?;
        let input = std::env::args_os()
            .nth(1)
            .ok_or_else(|| GitError::new("editor", "Missing editor file"))?;
        let input = Path::new(&input).canonicalize()?;
        if !input.starts_with(parent) {
            return Err(GitError::new("editor", "Unexpected editor path"));
        }
        let plan: RebasePlan = serde_json::from_slice(&std::fs::read(&plan_path)?)
            .map_err(|_| GitError::new("editor", "Invalid plan"))?;
        if input.file_name().is_some_and(|n| n == "git-rebase-todo") {
            let mut todo = String::new();
            for step in plan.steps {
                if !matches!(
                    step.action.as_str(),
                    "pick" | "reword" | "squash" | "fixup" | "drop"
                ) || !step.oid.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(GitError::new("editor", "Invalid todo"));
                }
                todo.push_str(&format!("{} {}\n", step.action, step.oid));
            }
            std::fs::write(input, todo)?;
        } else if input.file_name().is_some_and(|n| n == "COMMIT_EDITMSG") {
            let done = std::fs::read_to_string(parent.join("rebase-merge/done"))?;
            let oid = done
                .lines()
                .filter(|l| !l.starts_with('#'))
                .next_back()
                .and_then(|l| l.split_whitespace().nth(1))
                .ok_or_else(|| GitError::new("editor", "Missing current step"))?;
            let step = plan
                .steps
                .iter()
                .find(|s| s.oid.starts_with(oid))
                .ok_or_else(|| GitError::new("editor", "Unknown current step"))?;
            if matches!(step.action.as_str(), "reword" | "squash") {
                std::fs::write(input, &step.message)?;
            }
        } else {
            return Err(GitError::new("editor", "Unexpected editor file"));
        }
        Ok(())
    })();
    if result.is_err() {
        eprintln!("Git Toys could not apply the reviewed rebase message.");
        std::process::exit(1);
    }
    true
}
