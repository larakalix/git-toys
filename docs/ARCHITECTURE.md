# Architecture

## Boundaries

- `crates/git-engine`: Git command construction, porcelain parsing, snapshots, paged DAG history, diffs, mutation validation, conflict state, reflog/stash/worktree readers, staged AI context and file history/blame/restoration. No Tauri dependency.
- `src-tauri`: repository handle registry, per-common-Git-directory mutation locks, operation lifecycle/progress events, filesystem notifications, session trust, app-data preferences/journal, native tools, askpass authentication and separate native HTTP AI adapters.
- `src`: small Vue composition components, a Pinia repository store, generated engine contracts, shared action definitions, virtualized history/files, SVG graph lanes, lazily loaded Monaco diff surface, and accessible Reka dialogs.

Git is the source of truth. Queries use argument arrays and machine-readable output. Status is porcelain v2 with NUL-delimited paths; unusual UTF-8 paths are preserved. Non-UTF-8 paths receive a specific unsupported-path error rather than lossy mutation.

## Operations and concurrency

The frontend submits a repository handle, discriminated action and snapshot token. The native adapter allocates an operation ID and returns immediately. A worker acquires the common-directory lock (shared by linked worktrees), validates current snapshot state, and runs the operation. Git's lock files are respected and never removed. Start, stderr progress, completion and structured failure are emitted as events. A redacted metadata-only journal retains 200 operations; current-session output is not persisted.

Snapshot tokens cover HEAD/status/refs, operation state, changed-file metadata and index timestamp. They detect ordinary external edits but do not provide transactions against independent Git clients; Git's own safety checks/locks remain authoritative. Filesystem events debounce refresh by 300 ms, with focus refresh and a three-second visibility-aware polling fallback. Query generations reject stale results after repository/filter changes.

## Authentication and trust

Before trust, queries disable hooks/fsmonitor/credential helpers; diffs disable external diff/text conversion. Mutations require explicit per-session trust. Trusted operations inherit Git/SSH credential helpers and signing configuration. Git terminal prompting is disabled; a random-token loopback askpass bridge forwards prompts to the app, waits up to 120 seconds, and returns responses through stdin/stdout to the requesting Git helper. Secrets are absent from settings and journals. The askpass path is the app executable; its environment switches it into helper mode before launching any UI.

This trust model assumes the user trusts their installed Git binary, OS account, and global Git configuration. Trust is not a sandbox for malicious repository tooling. The embedded WebView has a restrictive CSP, no generic filesystem/shell plugin, and renders repository text without HTML injection.

## Persistence and compatibility

Only recent/favorite paths, appearance, panel widths, AI provider/model/endpoint choices and metadata journal are stored under Tauri's OS app-data directory. Repository trust and credentials are not stored there. Preference writes are serialized and replace a temporary file. Windows replacement has a brief non-atomic remove/rename window; unexpected corruption is reported.

Worktree move/repair/prune/lock and dedicated submodule/LFS management, binary merge editing, interactive rebase and Git Flow assistants are deferred. Existing worktrees can be opened as repositories; linked worktrees share mutation locks. Binary diffs show a clear unsupported text-diff state. Files over 2 MB are excluded from Monaco. Git subprocess output retention is bounded to 16 MB per stream; very large repositories/content and expensive search still require further stress testing.

## Optional AI transport

The Git engine builds staged-only review context and index/context tokens; it has no HTTP dependency. The native adapter rebuilds and validates the reviewed context under the common-directory gate before sending it, then checks the index again after generation. Cloud URLs are fixed; configurable Ollama endpoints must be loopback and bypass proxies. OpenAI requests set `store: false`. All adapters have bounded responses, no tool privileges, redacted errors, explicit cancellation and a 120-second total deadline. Credentials are accessed off the UI thread through native OS storage; no keys, patches or generated drafts enter preferences or the activity journal. Local process memory necessarily holds request data during generation.

Worktree mutations and file restoration use the existing trust, snapshot validation, common-directory gate and operation events. Successful removal closes registered sessions/tabs only afterward. File-history pages rescan the bounded prefix from the anchor before skipping parsed rows to preserve rename following; later-page query cost grows with depth. Blame and timeline rows are virtualized. Restoration validates target cleanliness and path ancestors, writes historical bytes to the current path and leaves the index untouched. External Git/filesystem clients remain outside the app's transaction boundary.

## Reviewed change and history tools

`organizer` derives selected patches from verified text diffs and applies a complete group in one cached Git apply, leaving the worktree unchanged. Preferences store group names, repository paths, line IDs and content fingerprints, never patches. A nonempty index or stale fingerprint prevents staging.

`simulation` borrows local objects into a disposable, detached repository. Scoped source-query overrides disable executable filters; isolated simulation configuration disables hooks, external attributes, signing and network access. The Tauri preview service tracks cancellation separately from mutations and serializes against the common Git-directory gate. Preview results carry pinned source/target commits and the original target ref; applying them revalidates both Git state and target resolution.

`rebase` validates linear ranges and restricted actions, creates a recovery branch, and executes native interactive rebase. The desktop executable handles sequence/message-editor mode before starting Tauri. A plan in the repository's Git directory survives conflicts/restart for Continue; it is removed after completion/abort. The engine's standalone editor binary is an integration-test helper and is not included in the desktop bundle.

The GitHub service owns fixed-host Rust HTTP, device authorization, OS credentials and a bounded in-memory conditional cache. Generated contracts cross IPC; no provider response or token is written to preferences/journal. Repository descriptions render as text. API authorization and native Git credentials remain separate. PR checkout verifies Git's fetched pull-request ref against the advertised API commit before switching to a review branch. Remote snapshots retain configured URL values while Git continues to honor URL rewrites during execution.
