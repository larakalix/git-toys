# Validation evidence — 0.1.0 preview

Updated during local implementation on 2026-10-11. This is a functional preview, not a qualified public v1. No remote CI run or native Windows validation is claimed.

## Confirmed locally

| Check                                                            | Result                                                                                                                                                                                            |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pnpm check`                                                     | ESLint, Vue/TypeScript production build and 29 frontend tests passed (including AI draft preservation/cancellation/staleness, file-history response ownership and repository navigation ordering) |
| `cargo fmt --all --check`                                        | Passed                                                                                                                                                                                            |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed                                                                                                                                                                                            |
| `cargo test --workspace --locked`                                | 24 real-repository workflow tests, 21 type exports and 5 native service/preference tests passed                                                                                                   |
| `pnpm test:e2e`                                                  | 8 browser acceptance tests passed against the actual Rust engine and temporary Git repositories; AI generation response mocked                                                                    |
| `pnpm check:duplicates`                                          | jscpd 4.0.5, minimum 10 lines, generated/test code excluded: 0 exact clones                                                                                                                       |
| `pnpm audit --prod`                                              | No known vulnerabilities reported by the registry at check time                                                                                                                                   |
| macOS application bundle                                         | Built and launched locally; installed Git detection, native repository opening, history, commit details and Monaco diff rendering verified                                                        |
| macOS DMG                                                        | Debug Apple Silicon preview built successfully; SHA-256 recorded in `target/debug/bundle/SHA256SUMS`; installer mounting/installation and signing qualification pending                           |
| macOS AI credential store                                        | Unique dummy credential save/read/delete round trip passed against native Keychain; existing provider keys untouched                                                                              |
| Packaged askpass helper                                          | Fabricated credential response and cancellation passed over a local socket; no real credentials used                                                                                              |

The acceptance workflow clones a temporary bare remote, creates/switches a feature branch, stages and commits through the UI, merges to main, pushes, and compares local/remote object IDs. A separate workflow creates a real merge conflict, saves a resolution and continues. The third checks command-palette keyboard controls, preference persistence and the 960×640 layout. Screenshots are written under `test-results/` for light, dark and minimum-window views.

The browser suite injects a test IPC transport; it is not proof of native IPC or hosting-provider authentication. The native smoke check separately exercised the packaged app's repository queries. Native write workflows, full keyboard-only Git workflows and screen-reader behavior still need platform acceptance testing.

Rust tests cover merge/rebase conflicts and reopening/abort, failed hooks, stale-state rejection, untrusted mutations, empty/detached repositories, Unicode/newline filenames, independent hunk staging/unstaging, index lock preservation, linked-worktree lock identity, binary/CRLF preservation, conflict path escape prevention, stash/revert/reset and search pagination. Not every command/flag/failure combination has coverage.

## Performance measurement

`python3 scripts/benchmark.py` generated 100,000 commits with Git fast-import and measured the release engine on an Apple M4 Pro MacBook Pro with 48 GB RAM, macOS 26.6.2, Git 2.49.0 and Rust 1.91.1:

- Repository open: **25.19 ms**.
- First 200 history rows: **829.05 ms**; serialized payload **58,613 bytes**.

This meets the two-second engine-query target on that machine. The fixture is synthetic, mostly linear and uses empty trees. It does not measure native first paint, cold disk caches, complex merge graphs, huge working trees, full-history search, sustained memory or scrolling performance. History requests are bounded and lists are virtualized, but those broader performance claims remain unverified.

## DRY evidence

The first pinned scan found one exact 12-line clone in reflog/stash parsing. Extracting the precise shared reader removed it; the identical scan now reports zero clones. This is exact-clone evidence only, not a claim that semantic or renamed duplication is absent.

## Remaining release gates

- Native Windows x64, Intel macOS and minimum-OS smoke tests; clean install/upgrade and installer verification.
- Apple signing/notarization and Windows signing with the owner's credentials.
- Live SSH/HTTPS/enterprise authentication, proxy behavior, existing credential helpers, signing tools and complete hook failure/output scenarios.
- VoiceOver/Narrator and end-to-end keyboard-only workflows; more graph and file-list performance stress tests.
- Rejected push, interrupted process, lock contention, case-sensitive/case-insensitive filename, symlink and linked-worktree UI scenarios on both platforms.
- Approved release destination for manual update checks. No feed was supplied or invented.
- Rust dependency advisory review in addition to the completed frontend production audit.

The Monaco editor is lazy loaded; its approximately 2.28 MB uncompressed chunk produces Vite's size warning. Text files above 2 MB are excluded from the editor. Git output is retained up to 16 MB per stream; unusually verbose hooks may exceed that limit. Windows preference replacement has a brief remove/rename window. See [architecture](ARCHITECTURE.md) and [release process](RELEASING.md).

## Feedback and graph update

Push now shows an immediate disabled button spinner, an indeterminate progress strip and the running operation in the footer. Completion and failure clear pending state. Notifications cover success, info, warnings and errors; success/info dismiss after five seconds, pausing on pointer hover or keyboard focus. Errors/warnings remain until dismissed, with expandable Git output. Repeated active errors and unchanged repository warnings are deduplicated. Notifications stay in memory and are not persisted.

The history graph has twelve theme-specific colors, wider lanes, distinct merge nodes, colored ref badges and parent-lane colors at splits. New heads have no phantom incoming edge; existing loaded row topology remains stable when appending pages. Tests cover these cases and notification timer/focus behavior. The real-Git browser acceptance suite asserts pending push feedback (with a test-only delay), disabled controls and successful completion. Light/dark/minimum-window screenshots were inspected. Graph color contrast against the base surface has a measured minimum of 4.63:1 in light mode and 6.87:1 in dark mode across all twelve colors. Native screen-reader qualification remains pending.

## Diff refresh and panel-width regression fixes

Diff watchers compare scalar inputs instead of array identity. Unchanged snapshot polls no longer request another diff. A changed snapshot refreshes matching content in place, with staging/discard/hunk actions disabled until fresh content arrives. Failed refreshes preserve the displayed content, report a notification, and expose retry. Repository/file/comparison changes clear old content and invalidate outstanding responses. Monaco view state is restored when model text changes.

Panel dragging rounds/clamps widths to integer pixels. The preferences IPC boundary independently normalizes widths to the existing 190–360 px sidebar and 340–750 px detail ranges; non-finite values become null. Rust retains its u32 fields. Existing Git configuration and files are unaffected.

Four additional component tests cover identical polling snapshots, background refresh, failure/retry and stale-response rejection. Width tests cover rounding, bounds, invalid values and the IPC save boundary. The added real-Git browser workflow verifies three actual polling cycles without another diff request, no editor remount or scroll reset during an external edit, disabled actions while refreshing, fractional pointer resizing, keyboard resizing, and persistence after page reload. These checks use the test IPC transport; native Windows layout persistence remains unverified.

## Settings color palettes

Settings now offers 25 named palettes independently of light/dark/system appearance, with native radio navigation, visible focus, immediate accent/highlight preview, and local persistence. Old preference files default to teal; unrecognized frontend palette IDs render teal safely. Warning/error/success/info colors retain their semantic roles.

All 22 frontend tests, five browser workflows, the native preference migration/round-trip test, 13 generated binding tests, and 17 Git integration tests passed. Lint, typecheck/production build, Rust Clippy and the pinned duplicate scan passed. Primary label contrast and accent text against base surfaces meet 4.5:1 for every palette in both appearances. Browser tests verify arrow-key selection, light/dark updates, persisted selection after reload, and settled light/dark screenshots. Screenshots were visually inspected. Browser persistence uses the test IPC transport; Rust serialization is tested separately. Native Windows and screen-reader acceptance remain pending.

## AI drafts, parallel workspaces and file time machine

Three native HTTP adapters were exercised against local mock servers, checking payloads (including OpenAI `store: false`) and redacted failures. Cancellation, request-slot cleanup and a simulated 120-second deadline passed. Real Git fixtures cover staged-only context, credential/binary exclusions, style/token changes and oversized context without truncation. Vue tests cover exact review, selection changes, editable draft acceptance, existing-message preservation, stale index and cancelled late responses. No live provider/model request or paid API call was made. Native Keychain storage was tested separately with a disposable account; Windows Credential Manager remains unverified.

Worktree integration tests cover new/existing branches, shared lock identity, destination validation, checked-out branches, dirty/untracked/ignored/locked/missing/actively operating/main removal rejection and branch preservation. Browser workflows create/open/reuse/remove a workspace and verify affected tabs close after success. Independent external Git clients are still outside app mutation serialization.

File-history fixtures cover rename following across pages, deletion recovery, Unicode paths, blame, historical bytes restored without staging, local-change refusal, binary rejection and symlink escape protection. Browser workflows exercise timeline keyboard selection, blame display, Monaco changes, restore preview/confirmation and real index/worktree results. Timeline/blame rows are virtualized; deep-history latency and large blame payloads have not been benchmarked. Native macOS/Windows end-to-end UI, VoiceOver/Narrator and live provider qualification remain pending.

The three new screens were visually inspected in browser fixtures. Dialog content now appears above notification cards so transient feedback cannot obscure primary dialog controls. AI key and history preview failures also appear inside their dialogs. The Apple Silicon debug app and DMG were rebuilt for this feature set, with installer SHA-256 regenerated; this remains an unsigned preview.

## Supabase-inspired desktop styling

The supplied reference guided neutral light/charcoal surfaces, finer dividers, restrained green defaults, compact controls, and clearer heading hierarchy. The welcome screen uses explicit open/clone/initialize cards and a recent-repository list. Existing saved palette selections remain intact; the legacy default teal ID now renders green. Monaco shares neutral surface, foreground and gutter tokens. Dialog scrims/content cover notifications to prevent primary-control overlap.

After this styling pass, lint, typecheck/production build, all 29 frontend tests and all eight real-Git browser workflows passed. Default green welcome, light history, dark changes, custom-palette welcome, file-history dialog and 960×640 layouts were visually inspected. Palette contrast checks now use the new neutral dark surfaces. Existing Git engine behavior was unchanged; native Windows and screen-reader qualification remain pending. The Apple Silicon preview app/DMG and SHA-256 were regenerated.

## Commit organizer, simulations, visual rebase and GitHub hub

The feature set passed 33 frontend tests, 34 real-Git integration tests, 36 generated-contract export tests, and 10 native service/preference tests (including GitHub HTTP mocks). One existing native credential-store test remains explicitly ignored in the normal suite. Frontend lint/typecheck/production builds, Rust formatting/clippy, and the pinned exact-clone check passed.

All 11 browser workflows passed after the feature additions. The new workflows exercise keyboard line selection, assigning a named group, preserving an existing composer draft, staging/committing separate edits, isolated conflict prediction, a reviewed reword with recovery-branch creation, and mocked GitHub checks/diffs with untrusted descriptions rendered as text. The screenshots in `docs/images/` use disposable repositories; GitHub data is mocked. These browser workflows do not establish native Windows UI or live GitHub behavior.

Real-Git fixtures cover CRLF/Unicode/missing-final-newline selections, stale selection rejection, source-ref/index/worktree preservation during simulations, conflict detection, unsupported attributes, cancellation before simulation, and terminating a running Git process and its children on cancellation/deadline. Rebase fixtures cover reorder/reword/squash/fixup/drop, invalid sequences, visible recovery branches, abort after reopening and continuing a conflicting reword after reopening. PR checkout uses a disposable bare remote with Git URL rewriting to verify dirty-state rejection, advertised-head mismatch rejection and review-branch reuse without making a GitHub network request.

GitHub mocks verify device authorization, public/private requested scope, pending polling, cancellation, expiration, conditional ETag requests, pagination routes, fork metadata, changed-file patches, commit/review sections and redacted revoked-token/rate-limit errors. No GitHub OAuth application has been registered or client ID configured for the local preview. Live sign-in, organization/SSO authorization and real PR checkout remain unverified. OAuth tokens are kept in the OS credential vault; Windows Credential Manager qualification remains pending.

The Apple Silicon debug app and DMG were rebuilt, and installer checksums regenerated. A disposable native smoke test invoked the **bundled macOS application executable** as both Git sequence and message editor, completed an interactive reword and verified the resulting messages and clean worktree. This establishes the bundled helper path (including spaces), not complete native UI or Windows qualification. Signing/notarization, Intel and minimum-OS smoke tests, VoiceOver/Narrator and native Windows workflows remain release gates.

Preview source queries disable configured filters and executable hooks/fsmonitor. Disposable repositories use isolated Git configuration, disable system/global attributes, signing and network transports, and reject effective source attributes or unsupported tree entries. A preview is advisory: the actual trusted operation can still fail due to hooks, signing, permissions or external edits. Old app-owned simulation directories are cleaned after ten minutes rather than deleting a possible second running instance's current preview.

## Interaction motion

Shared dialogs use paired 220 ms entrances and 120 ms exits with `cubic-bezier(0.22, 1, 0.36, 1)`, a restrained 2% scale, and opacity. Interrupted exits keep the same surface and retarget CSS transitions. Closed dialogs become inert and leave the accessibility tree immediately; heavy contents unmount after the exit. The lightweight Reka surface stays mounted after first use to avoid double accessibility cleanup across sequential operations. The command palette remains immediate. Notifications enter over 180 ms and exit over 120 ms; primary/icon controls press immediately and settle over 120 ms. Theme changes suppress transitions for one frame.

Reduced motion replaces spatial transitions with 80 ms fades, removes press scaling, and pauses loading loops while keeping textual progress. No navigation, diff-refresh, history-row, or drag-layout animations were introduced. Source inspection found no layout-property transitions, `transition: all`, or permanent `will-change` declarations.

All 33 frontend tests and 12 real-Git browser workflows passed, along with lint, typecheck and the production build. The new browser test slows real CSS transitions to 10% through the Animations API and samples opacity/timing, checks matching scrim timing, immediate focus, same-node interruption, button press feedback, immediate command-palette opening, theme-switch cleanup, and the settings task under reduced motion. Existing polling/scroll preservation and Git workflows still pass. The macOS preview was rebuilt and checksummed. Native Windows, real touch-device interactions, and VoiceOver/Narrator motion qualification remain unverified.
