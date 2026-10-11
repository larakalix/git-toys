use git_toys_engine::{types::*, Repository};
use std::{path::Path, process::Command};
use tempfile::TempDir;
fn git(path: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim_end().into()
}
fn fixture() -> (TempDir, Repository) {
    let temp = TempDir::new().unwrap();
    git(temp.path(), &["init", "-b", "main"]);
    git(temp.path(), &["config", "user.name", "Git Toys Test"]);
    git(
        temp.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    git(temp.path(), &["config", "commit.gpgsign", "false"]);
    git(temp.path(), &["config", "core.autocrlf", "false"]);
    std::fs::write(temp.path().join("hello.txt"), "hello\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "Initial commit"]);
    let mut repo = Repository::open(temp.path()).unwrap();
    repo.trusted = true;
    (temp, repo)
}
fn act(repo: &Repository, action: Action) {
    repo.execute(&action, &repo.snapshot().unwrap().state_token)
        .unwrap();
}
#[test]
fn feature_branch_stage_commit_merge_and_push() {
    let (temp, repo) = fixture();
    let remote = TempDir::new().unwrap();
    git(remote.path(), &["init", "--bare"]);
    act(
        &repo,
        Action::AddRemote {
            name: "origin".into(),
            url: remote.path().to_string_lossy().into(),
        },
    );
    act(
        &repo,
        Action::CreateBranch {
            name: "feature/greeting".into(),
            start: None,
        },
    );
    act(
        &repo,
        Action::SwitchBranch {
            name: "feature/greeting".into(),
        },
    );
    std::fs::write(temp.path().join("hello.txt"), "hello world\n").unwrap();
    act(
        &repo,
        Action::Stage {
            paths: vec!["hello.txt".into()],
        },
    );
    act(
        &repo,
        Action::Commit {
            message: "Add greeting".into(),
            amend: false,
        },
    );
    act(
        &repo,
        Action::SwitchBranch {
            name: "main".into(),
        },
    );
    act(
        &repo,
        Action::Merge {
            target: "feature/greeting".into(),
        },
    );
    act(
        &repo,
        Action::Push {
            remote: "origin".into(),
            publish: true,
            force: false,
        },
    );
    assert_eq!(
        git(temp.path(), &["rev-parse", "HEAD"]),
        git(remote.path(), &["rev-parse", "refs/heads/main"])
    );
    assert!(repo.snapshot().unwrap().files.is_empty());
}
#[test]
fn history_and_diff_use_real_commit_data() {
    let (_temp, repo) = fixture();
    let commits = repo
        .history(&HistoryQuery {
            revision: None,
            search: None,
            path: None,
            skip: 0,
            limit: 200,
        })
        .unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "Initial commit");
    let diff = repo
        .diff(&DiffRequest {
            path: "hello.txt".into(),
            staged: false,
            commit: Some(commits[0].oid.clone()),
            base: None,
        })
        .unwrap();
    assert_eq!(diff.original, "");
    assert_eq!(diff.modified, "hello\n");
    assert!(diff.patch.contains("+hello"));
}
#[test]
fn filenames_with_spaces_unicode_and_newlines_are_preserved() {
    let (temp, repo) = fixture();
    let path = "a space —\nfile.txt";
    std::fs::write(temp.path().join(path), "new\n").unwrap();
    assert!(repo
        .snapshot()
        .unwrap()
        .files
        .iter()
        .any(|f| f.path == path));
    act(
        &repo,
        Action::Stage {
            paths: vec![path.into()],
        },
    );
    assert_eq!(
        repo.snapshot()
            .unwrap()
            .files
            .iter()
            .find(|f| f.path == path)
            .unwrap()
            .index,
        "A"
    );
}
#[test]
fn stale_actions_and_untrusted_mutations_are_rejected() {
    let (temp, mut repo) = fixture();
    let token = repo.snapshot().unwrap().state_token;
    std::fs::write(temp.path().join("hello.txt"), "external edit\n").unwrap();
    let action = Action::Stage {
        paths: vec!["hello.txt".into()],
    };
    assert_eq!(
        repo.execute(&action, &token).unwrap_err().code,
        "staleState"
    );
    repo.trusted = false;
    assert_eq!(
        repo.execute(&action, &repo.snapshot().unwrap().state_token)
            .unwrap_err()
            .code,
        "untrusted"
    );
}
#[test]
fn merge_conflicts_can_be_edited_and_continued() {
    let (temp, repo) = fixture();
    act(
        &repo,
        Action::CreateBranch {
            name: "feature".into(),
            start: None,
        },
    );
    std::fs::write(temp.path().join("hello.txt"), "main\n").unwrap();
    act(
        &repo,
        Action::Stage {
            paths: vec!["hello.txt".into()],
        },
    );
    act(
        &repo,
        Action::Commit {
            message: "main edit".into(),
            amend: false,
        },
    );
    act(
        &repo,
        Action::SwitchBranch {
            name: "feature".into(),
        },
    );
    std::fs::write(temp.path().join("hello.txt"), "feature\n").unwrap();
    act(
        &repo,
        Action::Stage {
            paths: vec!["hello.txt".into()],
        },
    );
    act(
        &repo,
        Action::Commit {
            message: "feature edit".into(),
            amend: false,
        },
    );
    let action = Action::Merge {
        target: "main".into(),
    };
    assert!(repo
        .execute(&action, &repo.snapshot().unwrap().state_token)
        .is_err());
    assert_eq!(repo.snapshot().unwrap().operation.as_deref(), Some("merge"));
    let conflict = repo.conflict("hello.txt").unwrap();
    assert_eq!(conflict.base, "hello\n");
    assert_eq!(conflict.ours, "feature\n");
    act(
        &repo,
        Action::Resolve {
            path: "hello.txt".into(),
            content: "combined\n".into(),
        },
    );
    act(&repo, Action::Continue);
    assert!(repo.snapshot().unwrap().operation.is_none());
}
#[test]
fn hunks_stage_and_unstage_independently() {
    let (temp, repo) = fixture();
    let original = (1..40).map(|i| format!("line {i}\n")).collect::<String>();
    std::fs::write(temp.path().join("hello.txt"), &original).unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "many lines"]);
    let modified = original
        .replace("line 2\n", "changed 2\n")
        .replace("line 35\n", "changed 35\n");
    std::fs::write(temp.path().join("hello.txt"), modified).unwrap();
    let req = DiffRequest {
        path: "hello.txt".into(),
        staged: false,
        commit: None,
        base: None,
    };
    assert_eq!(repo.diff(&req).unwrap().hunks.len(), 2);
    act(
        &repo,
        Action::StageHunk {
            path: "hello.txt".into(),
            hunk: 0,
            reverse: false,
        },
    );
    let staged = git(temp.path(), &["diff", "--cached"]);
    assert!(staged.contains("changed 2"));
    assert!(!staged.contains("changed 35"));
    act(
        &repo,
        Action::StageHunk {
            path: "hello.txt".into(),
            hunk: 0,
            reverse: true,
        },
    );
    assert_eq!(git(temp.path(), &["diff", "--cached"]), "");
}
#[test]
fn empty_repository_and_detached_head_are_readable() {
    let temp = TempDir::new().unwrap();
    git(temp.path(), &["init", "-b", "main"]);
    let repo = Repository::open(temp.path()).unwrap();
    assert_eq!(repo.snapshot().unwrap().head, "(initial)");
    assert!(repo
        .history(&HistoryQuery {
            revision: None,
            search: None,
            path: None,
            skip: 0,
            limit: 200
        })
        .unwrap()
        .is_empty());
    let (temp, repo) = fixture();
    git(temp.path(), &["checkout", "--detach"]);
    assert_eq!(repo.snapshot().unwrap().branch, "(detached)");
}
#[test]
fn path_traversal_is_rejected() {
    let (_temp, repo) = fixture();
    for p in [
        "../outside",
        ".git/config",
        "/tmp/outside",
        "nested/../../out",
        "nested/.GiT/config",
    ] {
        assert!(repo.relative(p).is_err(), "{p}");
    }
}
#[test]
fn hook_failure_preserves_message_and_index() {
    let (temp, repo) = fixture();
    let hook = temp.path().join(".git/hooks/pre-commit");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'Intentional hook failure' >&2\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(temp.path().join("hello.txt"), "hook\n").unwrap();
    act(
        &repo,
        Action::Stage {
            paths: vec!["hello.txt".into()],
        },
    );
    let error = repo
        .execute(
            &Action::Commit {
                message: "blocked".into(),
                amend: false,
            },
            &repo.snapshot().unwrap().state_token,
        )
        .unwrap_err();
    assert!(error.detail.contains("Intentional hook failure"));
    assert_eq!(repo.snapshot().unwrap().files[0].index, "M");
}
#[test]
fn linked_worktrees_share_the_lock_identity() {
    let (temp, repo) = fixture();
    let path = temp.path().join("linked");
    git(
        temp.path(),
        &["worktree", "add", path.to_str().unwrap(), "-b", "linked"],
    );
    let linked = Repository::open(&path).unwrap();
    assert_eq!(repo.common_dir, linked.common_dir);
    assert_ne!(repo.git_dir, linked.git_dir);
}

#[test]
fn search_pagination_counts_matches_instead_of_raw_commits() {
    let (temp, repo) = fixture();
    for subject in [
        "needle first",
        "other",
        "needle second",
        "other again",
        "needle third",
    ] {
        git(temp.path(), &["commit", "--allow-empty", "-m", subject]);
    }
    let query = HistoryQuery {
        revision: None,
        search: Some("needle".into()),
        path: None,
        skip: 1,
        limit: 1,
    };
    let result = repo.history(&query).unwrap();
    assert_eq!(result[0].subject, "needle second");
}
#[test]
fn rebase_conflict_survives_reopen_and_can_be_aborted() {
    let (temp, repo) = fixture();
    act(
        &repo,
        Action::CreateBranch {
            name: "topic".into(),
            start: None,
        },
    );
    std::fs::write(temp.path().join("hello.txt"), "base changed\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "base edit"]);
    act(
        &repo,
        Action::SwitchBranch {
            name: "topic".into(),
        },
    );
    std::fs::write(temp.path().join("hello.txt"), "topic changed\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "topic edit"]);
    let head = repo.snapshot().unwrap().head;
    assert!(repo
        .execute(
            &Action::Rebase {
                target: "main".into()
            },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
    let mut reopened = Repository::open(temp.path()).unwrap();
    reopened.trusted = true;
    assert_eq!(reopened.operation().as_deref(), Some("rebase"));
    act(&reopened, Action::Abort);
    assert_eq!(reopened.snapshot().unwrap().head, head);
}
#[test]
fn index_lock_is_respected_and_never_deleted() {
    let (temp, repo) = fixture();
    std::fs::write(temp.path().join("hello.txt"), "locked\n").unwrap();
    let lock = temp.path().join(".git/index.lock");
    std::fs::write(&lock, "owned externally").unwrap();
    assert!(repo
        .execute(
            &Action::Stage {
                paths: vec!["hello.txt".into()]
            },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
    assert_eq!(std::fs::read_to_string(lock).unwrap(), "owned externally");
}
#[test]
fn stash_revert_and_reset_preserve_expected_history() {
    let (temp, repo) = fixture();
    std::fs::write(temp.path().join("hello.txt"), "stashed\n").unwrap();
    act(
        &repo,
        Action::Stash {
            message: "work later".into(),
        },
    );
    assert!(repo.snapshot().unwrap().files.is_empty());
    act(
        &repo,
        Action::ApplyStash {
            index: 0,
            pop: true,
        },
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("hello.txt")).unwrap(),
        "stashed\n"
    );
    act(
        &repo,
        Action::Stage {
            paths: vec!["hello.txt".into()],
        },
    );
    act(
        &repo,
        Action::Commit {
            message: "update".into(),
            amend: false,
        },
    );
    act(
        &repo,
        Action::Revert {
            target: "HEAD".into(),
        },
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("hello.txt")).unwrap(),
        "hello\n"
    );
    act(
        &repo,
        Action::Reset {
            target: "HEAD~1".into(),
            mode: "soft".into(),
        },
    );
    assert_eq!(repo.snapshot().unwrap().files[0].index, "M");
    assert!(!repo.reflog().unwrap().is_empty());
}
#[test]
fn binary_and_crlf_content_are_not_corrupted() {
    let (temp, repo) = fixture();
    std::fs::write(temp.path().join("hello.txt"), b"a\r\nb\r\n").unwrap();
    let req = DiffRequest {
        path: "hello.txt".into(),
        staged: false,
        commit: None,
        base: None,
    };
    assert_eq!(repo.diff(&req).unwrap().modified, "a\r\nb\r\n");
    std::fs::write(temp.path().join("hello.txt"), [0, 1, 2, 3]).unwrap();
    assert!(repo.diff(&req).unwrap().binary);
}
#[test]
fn unstage_initial_commit_keeps_working_files() {
    let temp = TempDir::new().unwrap();
    git(temp.path(), &["init", "-b", "main"]);
    let mut repo = Repository::open(temp.path()).unwrap();
    repo.trusted = true;
    std::fs::write(temp.path().join("new.txt"), "new\n").unwrap();
    act(
        &repo,
        Action::Stage {
            paths: vec!["new.txt".into()],
        },
    );
    act(
        &repo,
        Action::Unstage {
            paths: vec!["new.txt".into()],
        },
    );
    assert!(temp.path().join("new.txt").exists());
    assert!(repo.snapshot().unwrap().files[0].untracked);
}
#[cfg(unix)]
#[test]
fn conflict_write_refuses_symlink_escape() {
    use std::os::unix::fs::symlink;
    let (temp, repo) = fixture();
    let outside = TempDir::new().unwrap();
    let dest = outside.path().join("outside.txt");
    std::fs::write(&dest, "keep").unwrap();
    symlink(&dest, temp.path().join("link")).unwrap();
    assert!(repo.relative("link").is_ok());
    // Non-conflicted paths are never accepted by the conflict writer.
    assert!(repo
        .execute(
            &Action::Resolve {
                path: "link".into(),
                content: "overwrite".into()
            },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
    assert_eq!(std::fs::read_to_string(dest).unwrap(), "keep");
}

#[test]
fn ai_context_contains_only_reviewed_staged_text_and_excludes_credentials() {
    let (temp, repo) = fixture();
    std::fs::write(temp.path().join("hello.txt"), "staged greeting\n").unwrap();
    std::fs::write(temp.path().join(".env"), "SECRET=never-send\n").unwrap();
    std::fs::write(temp.path().join("image.bin"), [0, 1, 2, 3]).unwrap();
    git(temp.path(), &["add", "."]);
    std::fs::write(temp.path().join("hello.txt"), "unstaged-private-edit\n").unwrap();
    let context = repo.ai_context(None, "plain").unwrap();
    assert_eq!(context.selected, vec!["hello.txt"]);
    assert!(context.context.contains("staged greeting"));
    assert!(!context.context.contains("unstaged-private-edit"));
    assert!(!context.context.contains("never-send"));
    assert_eq!(
        context
            .files
            .iter()
            .filter(|file| file.excluded_reason.is_some())
            .count(),
        2
    );
    assert!(repo.ai_context(Some(&[".env".into()]), "plain").is_err());
    let empty = repo.ai_context(Some(&[]), "plain").unwrap();
    assert!(empty.selected.is_empty());
    assert!(!empty.context.contains("staged greeting"));
    assert_ne!(
        context.context_token,
        repo.ai_context(None, "conventional").unwrap().context_token
    );
    let token = repo.index_token().unwrap();
    std::fs::write(temp.path().join("hello.txt"), "another unstaged edit\n").unwrap();
    assert_eq!(repo.index_token().unwrap(), token);
    git(temp.path(), &["add", "hello.txt"]);
    assert_ne!(repo.index_token().unwrap(), token);
}
#[test]
fn oversized_ai_context_is_flagged_without_silent_truncation() {
    let (temp, repo) = fixture();
    let content = "new line with useful detail\n".repeat(3000);
    std::fs::write(temp.path().join("big.txt"), &content).unwrap();
    git(temp.path(), &["add", "big.txt"]);
    let context = repo.ai_context(None, "plain").unwrap();
    assert!(context.oversized);
    assert!(context.context.len() > 65_536);
    assert!(context.context.contains("END STAGED PATCH"));
    assert!(!repo.ai_context(Some(&[]), "plain").unwrap().oversized);
}
#[test]
fn worktree_lifecycle_preserves_branch_and_rejects_unsafe_removal() {
    let (temp, repo) = fixture();
    let parent = TempDir::new().unwrap();
    let path = parent.path().join("task ü workspace");
    let dest = path.to_str().unwrap();
    act(
        &repo,
        Action::AddWorktree {
            path: dest.into(),
            branch: "feature/parallel".into(),
            new_branch: true,
            start: "HEAD".into(),
        },
    );
    let trees = repo.worktrees().unwrap();
    assert_eq!(trees.len(), 2);
    assert!(trees[0].main && trees[0].current);
    assert_eq!(trees[1].branch, "feature/parallel");
    let linked = Repository::open(&path).unwrap();
    assert_eq!(linked.common_dir, repo.common_dir);
    let remove = Action::RemoveWorktree { path: dest.into() };
    std::fs::write(path.join("untracked.txt"), "save me").unwrap();
    assert_eq!(
        repo.execute(&remove, &repo.snapshot().unwrap().state_token)
            .unwrap_err()
            .code,
        "worktreeUnsafe"
    );
    std::fs::remove_file(path.join("untracked.txt")).unwrap();
    git(
        temp.path(),
        &["worktree", "lock", "--reason", "portable drive", dest],
    );
    assert_eq!(
        repo.worktrees().unwrap()[1].locked.as_deref(),
        Some("portable drive")
    );
    assert!(repo
        .execute(&remove, &repo.snapshot().unwrap().state_token)
        .is_err());
    git(temp.path(), &["worktree", "unlock", dest]);
    let exclude = repo.common_dir.join("info/exclude");
    std::fs::write(exclude, "ignored.dat\n").unwrap();
    std::fs::write(path.join("ignored.dat"), "ignored local work").unwrap();
    assert!(repo
        .execute(&remove, &repo.snapshot().unwrap().state_token)
        .is_err());
    std::fs::remove_file(path.join("ignored.dat")).unwrap();
    std::fs::write(
        linked.git_dir.join("MERGE_HEAD"),
        git(temp.path(), &["rev-parse", "HEAD"]),
    )
    .unwrap();
    assert!(repo
        .execute(&remove, &repo.snapshot().unwrap().state_token)
        .is_err());
    std::fs::remove_file(linked.git_dir.join("MERGE_HEAD")).unwrap();
    act(&repo, remove);
    assert!(!path.exists());
    assert!(repo.resolve("refs/heads/feature/parallel").is_ok());
    assert!(repo
        .execute(
            &Action::RemoveWorktree {
                path: temp.path().to_str().unwrap().into()
            },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
}
#[test]
fn worktrees_reject_existing_destinations_checked_out_branches_and_missing_folders() {
    let (temp, repo) = fixture();
    let parent = TempDir::new().unwrap();
    let action = Action::AddWorktree {
        path: parent.path().to_str().unwrap().into(),
        branch: "other".into(),
        new_branch: true,
        start: "HEAD".into(),
    };
    assert!(repo
        .execute(&action, &repo.snapshot().unwrap().state_token)
        .is_err());
    let path = parent.path().join("existing-branch");
    assert_eq!(
        repo.execute(
            &Action::AddWorktree {
                path: path.to_str().unwrap().into(),
                branch: "main".into(),
                new_branch: false,
                start: "HEAD".into()
            },
            &repo.snapshot().unwrap().state_token
        )
        .unwrap_err()
        .code,
        "worktreeBranch"
    );
    git(temp.path(), &["branch", "available"]);
    act(
        &repo,
        Action::AddWorktree {
            path: path.to_str().unwrap().into(),
            branch: "available".into(),
            new_branch: false,
            start: "HEAD".into(),
        },
    );
    std::fs::remove_dir_all(&path).unwrap();
    assert!(repo.worktrees().unwrap()[1].missing);
    assert!(repo
        .execute(
            &Action::RemoveWorktree {
                path: path.to_str().unwrap().into()
            },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
}
#[test]
fn file_time_machine_follows_renames_paginates_blames_and_restores_without_staging() {
    let (temp, repo) = fixture();
    let initial = repo.resolve("HEAD").unwrap();
    git(temp.path(), &["mv", "hello.txt", "new ü name.txt"]);
    git(temp.path(), &["commit", "-m", "Rename greeting"]);
    std::fs::write(temp.path().join("new ü name.txt"), "hello\nnew detail\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "Add detail"]);
    let entries = repo.file_history("new ü name.txt", None, 0, 50).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[2].path, "hello.txt");
    assert_eq!(entries[1].previous_path.as_deref(), Some("hello.txt"));
    let page = repo.file_history("new ü name.txt", None, 2, 1).unwrap();
    assert_eq!(page[0].commit.oid, initial);
    assert_eq!(page[0].path, "hello.txt");
    let renamed = repo
        .file_revision(
            &entries[1].path,
            &entries[1].commit.oid,
            entries[1].previous_path.as_deref(),
        )
        .unwrap();
    assert_eq!(renamed.original, renamed.content);
    let blame = repo.blame("new ü name.txt", "HEAD").unwrap();
    assert_eq!(blame.len(), 2);
    assert_eq!(blame[0].oid, initial);
    assert_eq!(blame[1].content, "new detail");
    let before_index = repo.index_token().unwrap();
    let preview = repo
        .restore_preview("new ü name.txt", "hello.txt", &initial)
        .unwrap();
    assert_eq!(preview.original, "hello\nnew detail\n");
    assert_eq!(preview.content, "hello\n");
    act(
        &repo,
        Action::RestoreFileVersion {
            target: "new ü name.txt".into(),
            source: "hello.txt".into(),
            revision: initial.clone(),
        },
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("new ü name.txt")).unwrap(),
        "hello\n"
    );
    assert_eq!(repo.index_token().unwrap(), before_index);
    assert_eq!(
        repo.restore_preview("new ü name.txt", "hello.txt", &initial)
            .unwrap_err()
            .code,
        "restoreDirty"
    );
}
#[test]
fn deleted_file_can_be_recovered_and_binary_versions_are_explicitly_unsupported() {
    let (temp, repo) = fixture();
    let initial = repo.resolve("HEAD").unwrap();
    git(temp.path(), &["rm", "hello.txt"]);
    git(temp.path(), &["commit", "-m", "Delete greeting"]);
    let entries = repo.file_history("hello.txt", None, 0, 50).unwrap();
    assert!(entries[0].deleted);
    assert!(
        !repo
            .file_revision("hello.txt", "HEAD", None)
            .unwrap()
            .exists
    );
    act(
        &repo,
        Action::RestoreFileVersion {
            target: "hello.txt".into(),
            source: "hello.txt".into(),
            revision: initial,
        },
    );
    assert!(repo
        .snapshot()
        .unwrap()
        .files
        .iter()
        .any(|file| file.path == "hello.txt" && file.untracked));
    std::fs::write(temp.path().join("data.bin"), [0, 1, 2]).unwrap();
    git(temp.path(), &["add", "data.bin"]);
    git(temp.path(), &["commit", "-m", "Add binary"]);
    assert!(repo
        .file_revision("data.bin", "HEAD", None)
        .unwrap()
        .unsupported
        .is_some());
    assert!(repo.blame("data.bin", "HEAD").is_err());
}
#[cfg(unix)]
#[test]
fn history_restore_never_follows_symlinks_or_restores_symlink_blobs() {
    use std::os::unix::fs::symlink;
    let (temp, repo) = fixture();
    let outside = TempDir::new().unwrap();
    symlink(outside.path(), temp.path().join("escape")).unwrap();
    assert!(repo
        .restore_preview("escape/recovered.txt", "hello.txt", "HEAD")
        .is_err());
    assert!(!outside.path().join("recovered.txt").exists());
    symlink("hello.txt", temp.path().join("link.txt")).unwrap();
    git(temp.path(), &["add", "link.txt"]);
    git(temp.path(), &["commit", "-m", "Add link"]);
    assert!(repo
        .file_revision("link.txt", "HEAD", None)
        .unwrap()
        .unsupported
        .is_some());
}

#[test]
fn line_groups_split_one_file_without_changing_worktree_and_reject_stale_selections() {
    let (temp, repo) = fixture();
    std::fs::write(temp.path().join("hello.txt"), "hello\nfirst\nsecond\n").unwrap();
    let diff = repo.selection_diff("hello.txt").unwrap();
    let additions: Vec<_> = diff
        .diff
        .patch
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with('+') && !l.starts_with("+++"))
        .map(|(n, _)| n as u32)
        .collect();
    act(
        &repo,
        Action::StageGroup {
            files: vec![LineSelection {
                path: "hello.txt".into(),
                token: diff.token.clone(),
                lines: vec![additions[0]],
            }],
        },
    );
    assert_eq!(git(temp.path(), &["show", ":hello.txt"]), "hello\nfirst");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("hello.txt")).unwrap(),
        "hello\nfirst\nsecond\n"
    );
    act(
        &repo,
        Action::Commit {
            message: "First".into(),
            amend: false,
        },
    );
    let before = repo.snapshot().unwrap().state_token;
    let error = repo
        .execute(
            &Action::StageGroup {
                files: vec![LineSelection {
                    path: "hello.txt".into(),
                    token: diff.token,
                    lines: vec![additions[1]],
                }],
            },
            &before,
        )
        .unwrap_err();
    assert_eq!(error.code, "staleSelection");
    let next = repo.selection_diff("hello.txt").unwrap();
    let lines = next
        .diff
        .patch
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with('+') && !l.starts_with("+++"))
        .map(|(n, _)| n as u32)
        .collect();
    act(
        &repo,
        Action::StageGroup {
            files: vec![LineSelection {
                path: "hello.txt".into(),
                token: next.token,
                lines,
            }],
        },
    );
    act(
        &repo,
        Action::Commit {
            message: "Second".into(),
            amend: false,
        },
    );
    assert!(repo.snapshot().unwrap().files.is_empty());
}

#[test]
fn line_groups_preserve_unicode_crlf_and_missing_final_newline() {
    let (temp, repo) = fixture();
    for (name, original, modified) in [
        ("café file.txt", "a\r\nb\r\n", "a\r\nchanged\r\n"),
        ("no-newline.txt", "before", "after"),
    ] {
        std::fs::write(temp.path().join(name), original).unwrap();
        git(temp.path(), &["add", "--", name]);
        git(temp.path(), &["commit", "-m", "Fixture"]);
        std::fs::write(temp.path().join(name), modified).unwrap();
        let diff = repo.selection_diff(name).unwrap();
        let mut hunk = false;
        let lines = diff
            .diff
            .patch
            .lines()
            .enumerate()
            .filter_map(|(n, l)| {
                if l.starts_with("@@ ") {
                    hunk = true;
                }
                (hunk && (l.starts_with('+') || l.starts_with('-'))).then_some(n as u32)
            })
            .collect();
        act(
            &repo,
            Action::StageGroup {
                files: vec![LineSelection {
                    path: name.into(),
                    token: diff.token,
                    lines,
                }],
            },
        );
        let output = Command::new("git")
            .current_dir(temp.path())
            .args(["show", &format!(":{name}")])
            .output()
            .unwrap();
        assert_eq!(output.stdout, modified.as_bytes());
        act(
            &repo,
            Action::Commit {
                message: "Selected".into(),
                amend: false,
            },
        );
    }
}

fn editor_repo(repo: &mut Repository) {
    repo.editor_executable = Some(env!("CARGO_BIN_EXE_git-toys-editor").into());
}
fn committed_file(temp: &TempDir, name: &str, content: &str, message: &str) {
    std::fs::write(temp.path().join(name), content).unwrap();
    git(temp.path(), &["add", "--", name]);
    git(temp.path(), &["commit", "-m", message]);
}
#[test]
fn merge_and_rebase_previews_are_isolated_and_cancelled_previews_leave_no_directory() {
    use std::sync::{atomic::AtomicBool, Arc};
    let (temp, repo) = fixture();
    let head = repo.resolve("HEAD").unwrap();
    git(temp.path(), &["switch", "-c", "feature"]);
    committed_file(&temp, "feature.txt", "feature\n", "Feature");
    git(temp.path(), &["switch", "main"]);
    committed_file(&temp, "main.txt", "main\n", "Main");
    let before = repo.snapshot().unwrap().state_token;
    let refs = git(temp.path(), &["show-ref"]);
    let worktrees = git(temp.path(), &["worktree", "list", "--porcelain"]);
    let scratch = TempDir::new().unwrap();
    for kind in ["merge", "rebase"] {
        let preview = repo
            .simulate(
                scratch.path(),
                &PreviewRequest {
                    kind: kind.into(),
                    target: "feature".into(),
                    plan: None,
                },
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
            .unwrap();
        assert!(preview.conflicts.is_empty());
        assert!(preview.files.contains(&"feature.txt".into()));
        assert_eq!(repo.snapshot().unwrap().state_token, before);
        assert_eq!(git(temp.path(), &["show-ref"]), refs);
        assert_eq!(
            git(temp.path(), &["worktree", "list", "--porcelain"]),
            worktrees
        );
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
    }
    let error = repo
        .simulate(
            scratch.path(),
            &PreviewRequest {
                kind: "merge".into(),
                target: head,
                plan: None,
            },
            Arc::new(AtomicBool::new(true)),
            |_| {},
        )
        .unwrap_err();
    assert_eq!(error.code, "previewCancelled");
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}
#[test]
fn previews_report_first_rebase_conflict_and_reject_attributes() {
    use std::sync::{atomic::AtomicBool, Arc};
    let (temp, repo) = fixture();
    git(temp.path(), &["switch", "-c", "feature"]);
    committed_file(&temp, "hello.txt", "feature\n", "Feature greeting");
    git(temp.path(), &["switch", "main"]);
    committed_file(&temp, "hello.txt", "main\n", "Main greeting");
    let scratch = TempDir::new().unwrap();
    for kind in ["merge", "rebase"] {
        let result = repo
            .simulate(
                scratch.path(),
                &PreviewRequest {
                    kind: kind.into(),
                    target: "feature".into(),
                    plan: None,
                },
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
            .unwrap();
        assert_eq!(result.conflicts, vec!["hello.txt"]);
        assert_eq!(result.remaining_untested, kind == "rebase");
    }
    committed_file(
        &temp,
        ".gitattributes",
        "*.txt merge=custom\n",
        "Custom attributes",
    );
    assert_eq!(
        repo.simulate(
            scratch.path(),
            &PreviewRequest {
                kind: "merge".into(),
                target: "feature".into(),
                plan: None
            },
            Arc::new(AtomicBool::new(false)),
            |_| {}
        )
        .unwrap_err()
        .code,
        "previewUnsupported"
    );
}
#[test]
fn visual_rebase_reorders_rewords_squashes_drops_and_preserves_recovery() {
    use std::sync::{atomic::AtomicBool, Arc};
    let (temp, mut repo) = fixture();
    editor_repo(&mut repo);
    let base = repo.resolve("HEAD").unwrap();
    committed_file(&temp, "one.txt", "one\n", "One");
    committed_file(&temp, "two.txt", "two\n", "Two");
    committed_file(&temp, "three.txt", "three\n", "Three");
    committed_file(&temp, "four.txt", "four\n", "Four");
    let mut plan = repo.rebase_range(&base).unwrap().plan;
    let original = plan.head.clone();
    plan.steps.swap(0, 1);
    plan.steps[0].action = "reword".into();
    plan.steps[0].message = "Combined two and one".into();
    plan.steps[1].action = "squash".into();
    plan.steps[1].message = "Combined two and one".into();
    plan.steps[3].action = "drop".into();
    let scratch = TempDir::new().unwrap();
    let result = repo
        .simulate(
            scratch.path(),
            &PreviewRequest {
                kind: "interactive".into(),
                target: base.clone(),
                plan: Some(plan.clone()),
            },
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();
    assert!(result.conflicts.is_empty());
    assert_eq!(repo.resolve("HEAD").unwrap(), original);
    act(&repo, Action::InteractiveRebase { plan });
    assert_eq!(
        git(
            temp.path(),
            &["log", "--format=%s", &format!("{base}..HEAD")]
        ),
        "Three\nCombined two and one"
    );
    assert!(!temp.path().join("four.txt").exists());
    assert_eq!(
        git(
            temp.path(),
            &[
                "for-each-ref",
                "--format=%(objectname)",
                "refs/heads/git-toys/recovery/"
            ]
        ),
        original
    );
    assert!(!repo.git_dir.join("git-toys-rebase-plan.json").exists());
}
#[test]
fn visual_rebase_fixup_and_invalid_plans_and_restart_abort() {
    let (temp, mut repo) = fixture();
    editor_repo(&mut repo);
    let base = repo.resolve("HEAD").unwrap();
    committed_file(&temp, "hello.txt", "one\n", "One");
    committed_file(&temp, "hello.txt", "two\n", "Two");
    let mut plan = repo.rebase_range(&base).unwrap().plan;
    plan.steps[0].action = "squash".into();
    assert_eq!(
        repo.execute(
            &Action::InteractiveRebase { plan: plan.clone() },
            &repo.snapshot().unwrap().state_token
        )
        .unwrap_err()
        .code,
        "rebasePlan"
    );
    plan.steps[0].action = "pick".into();
    plan.steps[1].action = "fixup".into();
    act(&repo, Action::InteractiveRebase { plan });
    assert_eq!(
        git(
            temp.path(),
            &["log", "--format=%s", &format!("{base}..HEAD")]
        ),
        "One"
    );
    committed_file(&temp, "hello.txt", "three\n", "Three");
    let mut plan = repo.rebase_range(&base).unwrap().plan;
    plan.steps.swap(0, 1);
    let original = plan.head.clone();
    assert!(repo
        .execute(
            &Action::InteractiveRebase { plan },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
    let mut reopened = Repository::open(temp.path()).unwrap();
    reopened.trusted = true;
    editor_repo(&mut reopened);
    assert_eq!(reopened.operation().as_deref(), Some("rebase"));
    act(&reopened, Action::Abort);
    assert_eq!(reopened.resolve("HEAD").unwrap(), original);
}
#[test]
fn github_remote_detection_rejects_other_hosts_and_option_injection() {
    for url in [
        "https://github.com/owner/repo.git",
        "git@github.com:owner/repo.git",
        "ssh://git@github.com/owner/repo",
    ] {
        assert_eq!(
            git_toys_engine::github_repository(url).as_deref(),
            Some("owner/repo")
        );
    }
    for url in [
        "https://github.com.evil.invalid/o/r",
        "https://github.com/../r",
        "https://github.com/o/r?x",
        "-upload-pack=evil",
    ] {
        assert!(git_toys_engine::github_repository(url).is_none());
    }
}

#[test]
fn pr_checkout_verifies_fetched_head_preserves_dirty_work_and_reuses_review_branch() {
    let (temp, repo) = fixture();
    let remote = TempDir::new().unwrap();
    git(remote.path(), &["init", "--bare"]);
    let base = repo.resolve("HEAD").unwrap();
    git(temp.path(), &["switch", "-c", "contribution"]);
    committed_file(&temp, "contribution.txt", "from a fork\n", "Contribution");
    let head = repo.resolve("HEAD").unwrap();
    git(
        temp.path(),
        &[
            "push",
            remote.path().to_str().unwrap(),
            "HEAD:refs/pull/42/head",
        ],
    );
    git(temp.path(), &["switch", "main"]);
    git(
        temp.path(),
        &[
            "remote",
            "add",
            "github",
            "https://github.com/test/repository.git",
        ],
    );
    git(
        temp.path(),
        &[
            "config",
            &format!("url.{}.insteadOf", remote.path().to_str().unwrap()),
            "https://github.com/test/repository.git",
        ],
    );
    std::fs::write(temp.path().join("hello.txt"), "local edits\n").unwrap();
    assert_eq!(
        repo.execute(
            &Action::CheckoutPullRequest {
                remote: "github".into(),
                number: 42,
                head: head.clone()
            },
            &repo.snapshot().unwrap().state_token
        )
        .unwrap_err()
        .code,
        "pullRequestDirty"
    );
    git(temp.path(), &["restore", "hello.txt"]);
    assert_eq!(
        repo.execute(
            &Action::CheckoutPullRequest {
                remote: "github".into(),
                number: 42,
                head: base
            },
            &repo.snapshot().unwrap().state_token
        )
        .unwrap_err()
        .code,
        "pullRequestChanged"
    );
    assert_eq!(repo.snapshot().unwrap().branch, "main");
    act(
        &repo,
        Action::CheckoutPullRequest {
            remote: "github".into(),
            number: 42,
            head: head.clone(),
        },
    );
    assert_eq!(repo.resolve("HEAD").unwrap(), head);
    git(temp.path(), &["switch", "main"]);
    act(
        &repo,
        Action::CheckoutPullRequest {
            remote: "github".into(),
            number: 42,
            head,
        },
    );
    assert_eq!(
        repo.snapshot().unwrap().branch,
        format!("review/pr-42-{}", &repo.resolve("HEAD").unwrap()[..12])
    );
}

#[test]
fn visual_rebase_continues_after_restart_with_reviewed_reword_message() {
    let (temp, mut repo) = fixture();
    editor_repo(&mut repo);
    let base = repo.resolve("HEAD").unwrap();
    committed_file(&temp, "hello.txt", "one\n", "One");
    committed_file(&temp, "hello.txt", "two\n", "Two");
    let mut plan = repo.rebase_range(&base).unwrap().plan;
    plan.steps.swap(0, 1);
    plan.steps[0].action = "reword".into();
    plan.steps[0].message = "Two reviewed".into();
    assert!(repo
        .execute(
            &Action::InteractiveRebase { plan },
            &repo.snapshot().unwrap().state_token
        )
        .is_err());
    let mut reopened = Repository::open(temp.path()).unwrap();
    reopened.trusted = true;
    editor_repo(&mut reopened);
    act(
        &reopened,
        Action::Resolve {
            path: "hello.txt".into(),
            content: "two\n".into(),
        },
    );
    let _ = reopened.execute(&Action::Continue, &reopened.snapshot().unwrap().state_token);
    if reopened.operation().is_some() {
        act(
            &reopened,
            Action::Resolve {
                path: "hello.txt".into(),
                content: "one\n".into(),
            },
        );
        act(&reopened, Action::Continue);
    }
    assert!(reopened.operation().is_none());
    assert!(git(
        temp.path(),
        &["log", "--format=%s", &format!("{base}..HEAD")]
    )
    .contains("Two reviewed"));
}

#[test]
fn preview_processes_stop_promptly_on_cancellation_and_deadline() {
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };
    let temp = TempDir::new().unwrap();
    for cancel_request in [false, true] {
        let flag = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&flag);
        let worker = std::thread::spawn(move || {
            if cancel_request {
                std::thread::sleep(Duration::from_millis(100));
                signal.store(true, Ordering::Relaxed);
            }
        });
        let start = Instant::now();
        let result = git_toys_engine::simulation::run(
            temp.path(),
            &["-c", "alias.previewwait=!sleep 10", "previewwait"],
            &flag,
            start + Duration::from_millis(if cancel_request { 2000 } else { 100 }),
        );
        let error = result.unwrap_err();
        assert_eq!(
            error.code,
            if cancel_request {
                "previewCancelled"
            } else {
                "previewTimeout"
            }
        );
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "Preview process children outlived cancellation"
        );
        worker.join().unwrap();
    }
}
