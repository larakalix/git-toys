# README screenshots

These are captures of the current UI from `tests/e2e/workflows.spec.ts`, using disposable real Git repositories and the test-only IPC transport. They do not establish native Windows behavior or live AI connectivity. The AI draft and GitHub pull request data are mocked responses. Notification overlays are hidden only during screenshot capture; UI behavior is unchanged.

Regenerate with `pnpm test:e2e` and copy the relevant PNG files from `test-results/` into this directory. Keep these copies here: `test-results/` is disposable and ignored. `icon.png` comes from the app's bundled icon.

| Documentation asset      | Test output              |
| ------------------------ | ------------------------ |
| `history-light.png`      | `history-light.png`      |
| `ai-draft.png`           | `ai-draft.png`           |
| `file-history.png`       | `file-time-machine.png`  |
| `worktrees.png`          | `worktrees.png`          |
| `welcome.png`            | `welcome-green.png`      |
| `commit-organizer.png`   | `commit-organizer.png`   |
| `merge-preview.png`      | `merge-preview.png`      |
| `interactive-rebase.png` | `interactive-rebase.png` |
| `pull-requests.png`      | `pull-requests.png`      |
