use crate::{runner::run, types::*, Repository};
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

fn token(diff: &Diff) -> String {
    let mut hash = DefaultHasher::new();
    (&diff.path, &diff.original, &diff.modified, &diff.patch).hash(&mut hash);
    format!("{:x}", hash.finish())
}
impl Repository {
    pub fn selection_diff(&self, path: &str) -> Result<SelectionDiff> {
        let absolute = self.relative(path)?;
        if !std::fs::symlink_metadata(&absolute)
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
            || !absolute.canonicalize()?.starts_with(&self.root)
        {
            return Err(GitError::new(
                "selectionUnsupported",
                "Line grouping requires a regular file inside this repository.",
            ));
        }
        let snapshot = self.snapshot()?;
        let file = snapshot
            .files
            .iter()
            .find(|f| f.path == path)
            .ok_or_else(|| GitError::new("selection", "File is no longer changed."))?;
        let mode = self.git_text(&["ls-files", "--stage", "--", path])?;
        if file.untracked
            || file.conflict
            || file.submodule
            || file.original_path.is_some()
            || file.index != "."
            || file.worktree != "M"
            || !mode.starts_with("100")
        {
            return Err(GitError::new("selectionUnsupported", "Line grouping supports unstaged, tracked text edits only. Stage this file as a whole instead."));
        }
        let diff = self.diff(&DiffRequest {
            path: path.into(),
            staged: false,
            commit: None,
            base: None,
        })?;
        if diff.binary
            || diff.truncated
            || diff.patch.contains("old mode ")
            || diff.patch.contains("new mode ")
        {
            return Err(GitError::new(
                "selectionUnsupported",
                "Binary, oversized, and mode changes require whole-file staging.",
            ));
        }
        Ok(SelectionDiff {
            token: token(&diff),
            diff,
        })
    }
    pub(crate) fn stage_group(&self, files: &[LineSelection]) -> Result<String> {
        let snapshot = self.snapshot()?;
        if snapshot.operation.is_some()
            || snapshot
                .files
                .iter()
                .any(|f| !f.untracked && f.index != ".")
        {
            return Err(GitError::new("indexNotEmpty", "Commit or unstage existing staged changes before staging a group. Finish any active operation first."));
        }
        if files.is_empty() || files.len() > 1000 {
            return Err(GitError::new("selection", "Select changes for this group."));
        }
        let mut seen = HashSet::new();
        let mut patch = String::new();
        for selection in files {
            if !seen.insert(&selection.path) {
                return Err(GitError::new(
                    "selection",
                    "A file can appear only once in a group.",
                ));
            }
            let current = self.selection_diff(&selection.path)?;
            if current.token != selection.token {
                return Err(GitError::new(
                    "staleSelection",
                    "Selected changes have changed. Review the group's selections again.",
                ));
            }
            let modified = selected_content(&current.diff, &selection.lines)?;
            let old = quote_path(&format!("a/{}", selection.path));
            let new = quote_path(&format!("b/{}", selection.path));
            patch.push_str(&format!(
                "diff --git {old} {new}\n--- {old}\n+++ {new}\n@@ -{},{} +{},{} @@\n",
                if current.diff.original.is_empty() {
                    0
                } else {
                    1
                },
                current.diff.original.lines().count(),
                if modified.is_empty() { 0 } else { 1 },
                modified.lines().count()
            ));
            for (prefix, content) in [
                ("-", current.diff.original.as_str()),
                ("+", modified.as_str()),
            ] {
                for line in content.split_inclusive('\n') {
                    patch.push_str(prefix);
                    patch.push_str(line);
                    if !line.ends_with('\n') {
                        patch.push_str("\n\\ No newline at end of file\n");
                    }
                }
            }
        }
        // One apply invocation validates every patch before updating the index; the worktree is untouched.
        run(
            &self.root,
            self.trusted,
            self.auth.as_ref(),
            &["apply", "--cached", "--whitespace=nowarn", "-"],
            Some(patch.as_bytes()),
        )?;
        Ok("Group staged. Review the staged diff before committing.".into())
    }
}
fn selected_content(diff: &Diff, lines: &[u32]) -> Result<String> {
    let selected: HashSet<_> = lines.iter().copied().collect();
    if selected.is_empty() || selected.len() != lines.len() {
        return Err(GitError::new("selection", "Select distinct changed lines."));
    }
    let old: Vec<_> = diff.original.split_inclusive('\n').collect();
    let mut cursor = 0usize;
    let mut result = String::new();
    let mut in_hunk = false;
    let mut matched = 0;
    let mut appended_add = false;
    for (index, line) in diff.patch.split_inclusive('\n').enumerate() {
        if line.starts_with("@@ ") {
            let start: usize = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.trim_start_matches('-').split(',').next())
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| GitError::new("selection", "Invalid diff header."))?;
            let start = start.saturating_sub(1);
            if start < cursor || start > old.len() {
                return Err(GitError::new("selection", "Invalid diff range."));
            }
            for value in &old[cursor..start] {
                result.push_str(value);
            }
            cursor = start;
            in_hunk = true;
            appended_add = false;
            continue;
        }
        if !in_hunk {
            continue;
        }
        let choose = selected.contains(&(index as u32));
        match line.as_bytes().first() {
            Some(b' ') | Some(b'-') => {
                let value = old
                    .get(cursor)
                    .ok_or_else(|| GitError::new("selection", "Diff no longer matches file."))?;
                if line.starts_with(' ') || !choose {
                    result.push_str(value);
                }
                if choose && line.starts_with('-') {
                    matched += 1;
                }
                cursor += 1;
                appended_add = false;
            }
            Some(b'+') => {
                appended_add = choose;
                if choose {
                    result.push_str(&line[1..]);
                    matched += 1;
                }
            }
            Some(b'\\') => {
                if appended_add && result.ends_with('\n') {
                    result.pop();
                }
                appended_add = false;
            }
            _ => return Err(GitError::new("selection", "Unsupported diff record.")),
        }
    }
    if matched != selected.len() {
        return Err(GitError::new(
            "selection",
            "Only added or deleted lines can be selected.",
        ));
    }
    for value in &old[cursor..] {
        result.push_str(value);
    }
    if result == diff.original {
        return Err(GitError::new("selection", "Selection makes no changes."));
    }
    Ok(result)
}

fn quote_path(path: &str) -> String {
    let mut value = String::from("\"");
    for ch in path.chars() {
        if ch == '\\' || ch == '\"' {
            value.push('\\');
            value.push(ch);
        } else if ch.is_ascii_control() {
            value.push_str(&format!("\\{:03o}", ch as u32));
        } else {
            value.push(ch);
        }
    }
    value.push('\"');
    value
}
