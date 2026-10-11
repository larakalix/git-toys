use crate::{types::*, Repository};
/// Only GitHub.com SSH and HTTPS remotes are eligible; no API URL comes from a PR body.
pub fn github_repository(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let rest = rest
        .trim_end_matches('/')
        .strip_suffix(".git")
        .unwrap_or(rest.trim_end_matches('/'));
    let parts: Vec<_> = rest.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return None;
    }
    Some(rest.into())
}
impl Repository {
    pub(crate) fn checkout_pull_request(
        &self,
        remote: &str,
        number: u32,
        head: &str,
    ) -> Result<String> {
        let snapshot = self.snapshot()?;
        let selected = snapshot
            .remotes
            .iter()
            .find(|r| r.name == remote)
            .ok_or_else(|| GitError::new("pullRequest", "Select a configured GitHub remote."))?;
        if github_repository(&selected.url).is_none()
            || number == 0
            || !matches!(head.len(), 40 | 64)
            || !head.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(GitError::new(
                "pullRequest",
                "Invalid GitHub pull request or commit.",
            ));
        }
        if !snapshot.files.is_empty() || snapshot.operation.is_some() {
            return Err(GitError::new("pullRequestDirty", "Commit or stash local changes and finish active operations before checking out a pull request."));
        }
        let reference = format!("refs/git-toys/pull/{number}");
        self.git(&[
            "fetch",
            "--no-tags",
            "--",
            remote,
            &format!("+refs/pull/{number}/head:{reference}"),
        ])?;
        if self.resolve(&reference)? != head {
            return Err(GitError::new(
                "pullRequestChanged",
                "Pull request changed during fetch. Refresh its details and try again.",
            ));
        }
        let branch = format!("review/pr-{number}-{}", &head[..12]);
        if let Ok(existing) = self.resolve(&format!("refs/heads/{branch}")) {
            if existing != head {
                return Err(GitError::new("pullRequestBranch", "The review branch has local commits. Rename it before checking out this revision."));
            }
        } else {
            self.git(&["branch", &branch, head])?;
        }
        self.git(&["switch", "--no-overwrite-ignore", &branch])?;
        Ok(format!(
            "Checked out pull request #{number} at {} on {branch}.",
            &head[..12]
        ))
    }
}
