use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub code: String,
    pub message: String,
    pub detail: String,
}
impl GitError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: String::new(),
        }
    }
}
impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message, self.detail)
    }
}
impl std::error::Error for GitError {}
impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        Self::new("io", e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, GitError>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub original_path: Option<String>,
    pub index: String,
    pub worktree: String,
    pub conflict: bool,
    pub untracked: bool,
    pub submodule: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct GitRef {
    pub name: String,
    pub oid: String,
    pub kind: String,
    pub upstream: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct Remote {
    pub name: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub path: String,
    pub name: String,
    pub branch: String,
    pub head: String,
    pub upstream: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
    pub refs: Vec<GitRef>,
    pub remotes: Vec<Remote>,
    pub operation: Option<String>,
    pub state_token: String,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct Commit {
    pub oid: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub date: String,
    pub subject: String,
    pub message: String,
    pub refs: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct HistoryQuery {
    pub revision: Option<String>,
    pub search: Option<String>,
    pub path: Option<String>,
    pub skip: u32,
    pub limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct DiffRequest {
    pub path: String,
    pub staged: bool,
    pub commit: Option<String>,
    pub base: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct Diff {
    pub path: String,
    pub original: String,
    pub modified: String,
    pub patch: String,
    pub binary: bool,
    pub truncated: bool,
    pub hunks: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct Conflict {
    pub path: String,
    pub base: String,
    pub ours: String,
    pub theirs: String,
    pub result: String,
    pub binary: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct ReflogEntry {
    pub oid: String,
    pub selector: String,
    pub subject: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct Diagnostic {
    pub version: String,
    pub supported: bool,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    ApplyPreview {
        kind_name: String,
        head: String,
        target: String,
        target_ref: String,
    },
    InteractiveRebase {
        plan: RebasePlan,
    },
    CheckoutPullRequest {
        remote: String,
        number: u32,
        head: String,
    },
    StageGroup {
        files: Vec<LineSelection>,
    },
    AddWorktree {
        path: String,
        branch: String,
        new_branch: bool,
        start: String,
    },
    RemoveWorktree {
        path: String,
    },
    RestoreFileVersion {
        target: String,
        source: String,
        revision: String,
    },
    Stage {
        paths: Vec<String>,
    },
    Unstage {
        paths: Vec<String>,
    },
    Discard {
        paths: Vec<String>,
    },
    StageHunk {
        path: String,
        hunk: u32,
        reverse: bool,
    },
    Commit {
        message: String,
        amend: bool,
    },
    CreateBranch {
        name: String,
        start: Option<String>,
    },
    SwitchBranch {
        name: String,
    },
    RenameBranch {
        old: String,
        name: String,
    },
    DeleteBranch {
        name: String,
        force: bool,
    },
    SetUpstream {
        branch: String,
        upstream: String,
    },
    CreateTag {
        name: String,
        target: String,
    },
    DeleteTag {
        name: String,
    },
    AddRemote {
        name: String,
        url: String,
    },
    RemoveRemote {
        name: String,
    },
    EditRemote {
        name: String,
        url: String,
    },
    Fetch {
        remote: String,
    },
    Pull {
        remote: String,
        strategy: String,
    },
    Push {
        remote: String,
        publish: bool,
        force: bool,
    },
    Merge {
        target: String,
    },
    Rebase {
        target: String,
    },
    CherryPick {
        target: String,
    },
    Revert {
        target: String,
    },
    Reset {
        target: String,
        mode: String,
    },
    Stash {
        message: String,
    },
    ApplyStash {
        index: u32,
        pop: bool,
    },
    DropStash {
        index: u32,
    },
    Continue,
    Abort,
    Resolve {
        path: String,
        content: String,
    },
    ResolveSide {
        path: String,
        side: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub ollama_url: String,
}
impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "ollama".into(),
            model: String::new(),
            ollama_url: "http://127.0.0.1:11434".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AiFile {
    pub path: String,
    pub excluded_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AiContext {
    pub index_token: String,
    pub context_token: String,
    pub files: Vec<AiFile>,
    pub selected: Vec<String>,
    pub context: String,
    pub oversized: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AiDraft {
    pub message: String,
    pub index_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    pub branch: String,
    pub head: String,
    pub main: bool,
    pub current: bool,
    pub dirty: Option<bool>,
    pub locked: Option<String>,
    pub missing: bool,
    pub prunable: bool,
    pub operation: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct FileHistoryEntry {
    pub commit: Commit,
    pub path: String,
    pub previous_path: Option<String>,
    pub deleted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct FileRevision {
    pub revision: String,
    pub path: String,
    pub original: String,
    pub content: String,
    pub exists: bool,
    pub unsupported: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    pub oid: String,
    pub line: u32,
    pub original_line: u32,
    pub author: String,
    pub timestamp: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct LineSelection {
    pub path: String,
    pub token: String,
    pub lines: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct SelectionDiff {
    pub diff: Diff,
    pub token: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
pub struct CommitGroup {
    pub id: String,
    pub repository: String,
    pub name: String,
    pub files: Vec<LineSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    pub kind: String,
    pub target: String,
    pub plan: Option<RebasePlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub target_ref: String,
    pub kind: String,
    pub head: String,
    pub target: String,
    pub commits: Vec<Commit>,
    pub files: Vec<String>,
    pub conflicts: Vec<String>,
    pub patch: String,
    pub stopped_commit: Option<String>,
    pub remaining_untested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct RebaseStep {
    pub oid: String,
    pub action: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct RebasePlan {
    pub head: String,
    pub base: String,
    pub steps: Vec<RebaseStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct RebaseRange {
    pub plan: RebasePlan,
    pub commits: Vec<Commit>,
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GithubStatus {
    pub configured: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GithubVerification {
    pub id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PullRequestQuery {
    pub state: String,
    pub search: String,
    pub page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub draft: bool,
    pub body: String,
    pub head: String,
    pub head_label: String,
    pub base: String,
    pub mergeability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PullRequestPage {
    pub items: Vec<PullRequest>,
    pub more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GithubItem {
    pub name: String,
    pub status: String,
    pub detail: String,
    pub oid: String,
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GithubSection {
    pub items: Vec<GithubItem>,
    pub more: bool,
}
