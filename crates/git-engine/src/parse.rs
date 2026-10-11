use crate::types::{FileChange, GitError, Result};

pub fn status(bytes: &[u8]) -> Result<Vec<FileChange>> {
    let mut fields = bytes.split(|b| *b == 0).filter(|v| !v.is_empty());
    let mut files = Vec::new();
    while let Some(field) = fields.next() {
        let value = std::str::from_utf8(field).map_err(|_| GitError::new("unsupportedPath", "This repository contains a non-UTF-8 filename. Use Git in the terminal for that path."))?;
        let kind = value.as_bytes()[0];
        if kind == b'#' || kind == b'!' {
            continue;
        }
        if kind == b'?' {
            files.push(FileChange {
                path: value[2..].into(),
                original_path: None,
                index: "?".into(),
                worktree: "?".into(),
                conflict: false,
                untracked: true,
                submodule: false,
            });
            continue;
        }
        let count = match kind {
            b'1' => 9,
            b'2' => 10,
            b'u' => 11,
            _ => continue,
        };
        let columns: Vec<_> = value.splitn(count, ' ').collect();
        if columns.len() != count {
            return Err(GitError::new("parse", "Malformed Git status record"));
        }
        let xy = columns[1].as_bytes();
        if xy.len() != 2 {
            return Err(GitError::new("parse", "Malformed Git file status"));
        }
        let original_path = if kind == b'2' {
            Some(
                std::str::from_utf8(
                    fields
                        .next()
                        .ok_or_else(|| GitError::new("parse", "Missing rename source"))?,
                )
                .map_err(|_| GitError::new("unsupportedPath", "Non-UTF-8 rename source"))?
                .into(),
            )
        } else {
            None
        };
        files.push(FileChange {
            path: columns[count - 1].into(),
            original_path,
            index: (xy[0] as char).to_string(),
            worktree: (xy[1] as char).to_string(),
            conflict: kind == b'u',
            untracked: false,
            submodule: columns[2].starts_with('S'),
        });
    }
    Ok(files)
}

pub fn hunks(patch: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut header = String::new();
    let mut current = String::new();
    for line in patch.split_inclusive('\n') {
        if line.starts_with("@@ ") {
            if !current.is_empty() {
                result.push(current);
            }
            current = header.clone();
            current.push_str(line);
        } else if current.is_empty() {
            header.push_str(line);
        } else {
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}
