# Roadmap

Implemented preview: repository open/init/clone, recents/favorites/tabs, history/search/path and branch filters, graph, commit comparisons, inline/side-by-side text diffs, file/hunk staging and unstaging, commits/amend, branches/tags/upstream/remotes, fetch/pull/push/publish/force-with-lease, merge/rebase/cherry-pick/revert/reset, stash operations, conflict editor and continue/abort, reflog recovery, metadata activity, command palette, native tool launch, themes, panel resizing, file-change refresh and askpass; optional Ollama/OpenAI/Anthropic commit drafts; safe worktree create/open/remove; rename-following file history, blame and unstaged restoration.

Before public v1: complete the release gates in RELEASING.md, verify auth/signing against live services, validate native Windows/Intel/minimum-OS behavior, screen-reader review and broader large-repository stress testing, and configure an approved public release feed for manual update checks.

Implemented next-feature preview: named line-selection commit groups; isolated merge/rebase simulation; linear interactive rebase with recovery branches; GitHub.com PR browsing and device-flow integration (OAuth registration not configured).

After v1: AI grouping; merge-preserving interactive rebase; worktree move/repair/prune/lock management; dedicated submodule/LFS management; optional formal Git Flow feature/release/hotfix assistants; GitLab/Enterprise PR integrations and PR write workflows; signed automatic updates.

Git features without dedicated UI use the external terminal. There is no claim of complete porcelain/plumbing command coverage.
