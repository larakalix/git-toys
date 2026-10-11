use crate::{types::*, Repository};
use std::{fs, path::Path};
fn utf8(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        GitError::new(
            "unsupportedText",
            "File history supports UTF-8 text and paths only.",
        )
    })
}
impl Repository {
    pub fn file_history(
        &self,
        path: &str,
        revision: Option<&str>,
        skip: u32,
        limit: u32,
    ) -> Result<Vec<FileHistoryEntry>> {
        self.relative(path)?;
        let oid = self.resolve(revision.unwrap_or("HEAD"))?;
        let bytes = self.git(&[
            "log",
            "--follow",
            "--no-ext-diff",
            "--no-textconv",
            "--format=%H%x00%P%x00%an%x00%ae%x00%aI%x00%B%x00",
            "--name-status",
            "-z",
            &format!("--max-count={}", skip.saturating_add(limit.clamp(1, 100))),
            &oid,
            "--",
            path,
        ])?;
        let fields: Vec<_> = bytes.split(|byte| *byte == 0).collect();
        let mut i = 0;
        let mut result = Vec::new();
        while i + 6 < fields.len() && !fields[i].is_empty() {
            let oid = utf8(fields[i])?;
            if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(GitError::new(
                    "historyParse",
                    "Could not read file history.",
                ));
            }
            let message = utf8(fields[i + 5])?;
            let commit = Commit {
                oid,
                parents: utf8(fields[i + 1])?
                    .split_whitespace()
                    .map(str::to_string)
                    .collect(),
                author: utf8(fields[i + 2])?,
                email: utf8(fields[i + 3])?,
                date: utf8(fields[i + 4])?,
                subject: message.lines().next().unwrap_or("").into(),
                message,
                refs: String::new(),
            };
            i += 6;
            while i < fields.len() && fields[i].is_empty() {
                i += 1;
            }
            let status = fields
                .get(i)
                .map(|field| utf8(field))
                .transpose()?
                .unwrap_or_default();
            let status = status.trim_start_matches('\n');
            if status.is_empty() {
                break;
            }
            let old = fields
                .get(i + 1)
                .ok_or_else(|| GitError::new("historyParse", "Missing historical path."))?;
            let (file_path, previous_path) = if status.starts_with('R') || status.starts_with('C') {
                let new = fields
                    .get(i + 2)
                    .ok_or_else(|| GitError::new("historyParse", "Missing renamed path."))?;
                i += 3;
                (utf8(new)?, Some(utf8(old)?))
            } else {
                i += 2;
                (utf8(old)?, None)
            };
            self.relative(&file_path)?;
            result.push(FileHistoryEntry {
                commit,
                path: file_path,
                previous_path,
                deleted: status == "D",
            });
            while i < fields.len() && fields[i].is_empty() {
                i += 1;
            }
        }
        Ok(result
            .into_iter()
            .skip(skip as usize)
            .take(limit.clamp(1, 100) as usize)
            .collect())
    }
    fn text_blob(&self, revision: &str, path: &str) -> Result<(bool, String, Option<String>)> {
        self.relative(path)?;
        let bytes = self.git(&["ls-tree", "-z", revision, "--", path])?;
        if bytes.is_empty() {
            return Ok((false, String::new(), None));
        }
        let header = bytes.split(|b| *b == b'\t').next().unwrap_or_default();
        let header = utf8(header)?;
        let mut parts = header.split_whitespace();
        let mode = parts.next().unwrap_or("");
        let kind = parts.next().unwrap_or("");
        let oid = parts.next().unwrap_or("");
        if !matches!(mode, "100644" | "100755") || kind != "blob" {
            return Ok((
                true,
                String::new(),
                Some("Symlinks, directories, and submodules have no text history viewer.".into()),
            ));
        }
        let size = self
            .git_text(&["cat-file", "-s", oid])?
            .trim()
            .parse::<usize>()
            .unwrap_or(usize::MAX);
        if size > 2_000_000 {
            return Ok((
                true,
                String::new(),
                Some("This version exceeds the 2 MB editor limit.".into()),
            ));
        }
        let bytes = self.git(&["cat-file", "blob", oid])?;
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            return Ok((
                true,
                String::new(),
                Some("Binary and non-UTF-8 versions cannot be displayed or restored here.".into()),
            ));
        }
        Ok((true, utf8(&bytes)?, None))
    }
    pub fn file_revision(
        &self,
        path: &str,
        revision: &str,
        previous_path: Option<&str>,
    ) -> Result<FileRevision> {
        let oid = self.resolve(revision)?;
        let (exists, content, unsupported) = self.text_blob(&oid, path)?;
        let original = if let Ok(parent) = self.resolve(&format!("{oid}^1")) {
            self.text_blob(&parent, previous_path.unwrap_or(path))?.1
        } else {
            String::new()
        };
        Ok(FileRevision {
            revision: oid,
            path: path.into(),
            original,
            content,
            exists,
            unsupported,
        })
    }
    pub fn blame(&self, path: &str, revision: &str) -> Result<Vec<BlameLine>> {
        let file = self.file_revision(path, revision, None)?;
        if !file.exists || file.unsupported.is_some() {
            return Err(GitError::new(
                "unsupportedText",
                file.unsupported
                    .unwrap_or("File is absent at this revision.".into()),
            ));
        }
        let bytes = self.git(&["blame", "--line-porcelain", &file.revision, "--", path])?;
        let text = utf8(&bytes)?;
        let mut lines = Vec::new();
        let mut oid = String::new();
        let mut line = 0;
        let mut original_line = 0;
        let mut author = String::new();
        let mut timestamp = String::new();
        for value in text.split('\n') {
            if let Some(content) = value.strip_prefix('\t') {
                lines.push(BlameLine {
                    oid: oid.clone(),
                    line,
                    original_line,
                    author: author.clone(),
                    timestamp: timestamp.clone(),
                    content: content.into(),
                });
            } else if let Some(value) = value.strip_prefix("author ") {
                author = value.into();
            } else if let Some(value) = value.strip_prefix("author-time ") {
                timestamp = value.into();
            } else {
                let fields: Vec<_> = value.split_whitespace().collect();
                if fields.len() >= 3
                    && matches!(fields[0].len(), 40 | 64)
                    && fields[0].bytes().all(|b| b.is_ascii_hexdigit())
                {
                    oid = fields[0].into();
                    original_line = fields[1].parse().unwrap_or(0);
                    line = fields[2].parse().unwrap_or(0);
                    author.clear();
                    timestamp.clear();
                }
            }
        }
        Ok(lines)
    }
    pub fn restore_preview(
        &self,
        target: &str,
        source: &str,
        revision: &str,
    ) -> Result<FileRevision> {
        let dest = self.relative(target)?;
        if self.git_dir.join("index.lock").exists() {
            return Err(GitError::new("indexLocked", "Git’s index is locked. Wait for the other Git operation; Git Toys will not remove its lock."));
        }
        let snapshot = self.snapshot()?;
        if snapshot.operation.is_some()
            || snapshot
                .files
                .iter()
                .any(|file| file.path == target || file.original_path.as_deref() == Some(target))
        {
            return Err(GitError::new(
                "restoreDirty",
                "Commit, stash, or discard local changes to this file before restoring a version.",
            ));
        }
        let mut file = self.file_revision(source, revision, None)?;
        if !file.exists || file.unsupported.is_some() {
            return Err(GitError::new(
                "unsupportedText",
                file.unsupported
                    .unwrap_or("File does not exist at this revision.".into()),
            ));
        }
        let mut check = self.root.clone();
        for part in Path::new(target).components() {
            check.push(part);
            if fs::symlink_metadata(&check).is_ok_and(|meta| meta.file_type().is_symlink()) {
                return Err(GitError::new(
                    "invalidPath",
                    "Refusing to restore through a symlink.",
                ));
            }
        }
        file.original = if dest.exists() {
            let bytes = fs::read(&dest)?;
            if bytes.len() > 2_000_000 || bytes.contains(&0) {
                return Err(GitError::new(
                    "unsupportedText",
                    "Current file exceeds the text viewer limit.",
                ));
            }
            utf8(&bytes)?
        } else {
            String::new()
        };
        Ok(file)
    }
    pub(crate) fn restore_file_version(
        &self,
        target: &str,
        source: &str,
        revision: &str,
    ) -> Result<String> {
        let dest = self.relative(target)?;
        let file = self.restore_preview(target, source, revision)?;
        let tracked = !self
            .git(&["ls-files", "--stage", "-z", "--", target])?
            .is_empty();
        if fs::symlink_metadata(&dest)
            .is_ok_and(|meta| !meta.is_file() || meta.file_type().is_symlink() || !tracked)
        {
            return Err(GitError::new(
                "invalidPath",
                "Refusing to overwrite a symlink, directory, or untracked file.",
            ));
        }
        let relative = Path::new(target);
        let mut parent = self.root.clone();
        if let Some(parts) = relative.parent() {
            for part in parts.components() {
                parent.push(part);
                if fs::symlink_metadata(&parent)
                    .is_ok_and(|meta| meta.file_type().is_symlink() || !meta.is_dir())
                {
                    return Err(GitError::new(
                        "invalidPath",
                        "Refusing to restore through a symlink or file.",
                    ));
                }
                if !parent.exists() {
                    fs::create_dir(&parent)?;
                }
                if !parent.canonicalize()?.starts_with(&self.root) {
                    return Err(GitError::new(
                        "invalidPath",
                        "Restore destination is outside the repository.",
                    ));
                }
            }
        }
        // Byte-for-byte restoration avoids executing configured smudge filters and never changes the index.
        if dest.exists() {
            fs::write(&dest, file.content.as_bytes())?;
        } else {
            use std::io::Write;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)?
                .write_all(file.content.as_bytes())?;
        }
        Ok("File version restored as an unstaged change. Review it before committing.".into())
    }
}
