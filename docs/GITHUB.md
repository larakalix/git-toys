# GitHub pull requests and OAuth setup

Git Toys reads GitHub.com pull requests through native Rust HTTP requests. Public repositories can be browsed anonymously, subject to GitHub's lower anonymous rate limits. GitLab and GitHub Enterprise are not supported in this version.

## Configure browser sign-in for a build

A publisher must register a GitHub OAuth application. This repository does not contain an application client ID, client secret, or hosted authentication service.

1. In the publishing GitHub account, open **Settings → Developer settings → OAuth Apps → New OAuth App**.
2. Use **Git Toys** as the application name and the actual project website as its homepage. Enter a callback URL accepted by GitHub's registration form; the device flow does not use that callback.
3. Enable **Device Flow** on the application.
4. Copy its **Client ID**. This is public configuration. Do not bundle a client secret.
5. Set the build environment variable before running Tauri:

macOS (zsh):

```sh
export GIT_TOYS_GITHUB_CLIENT_ID='your-public-client-id'
pnpm tauri build --debug --bundles app,dmg
```

Windows (PowerShell):

```powershell
$env:GIT_TOYS_GITHUB_CLIENT_ID = 'your-public-client-id'
pnpm tauri build
```

For GitHub Actions, set the repository variable **GIT_TOYS_GITHUB_CLIENT_ID**. Installer jobs pass that public variable to the Rust build. Restart/rebuild after changing it; it is compiled into the application.

An unconfigured build explains this requirement and disables sign-in while leaving anonymous public browsing available. Registering the OAuth app is an external setup step; it has not been completed for the local preview.

## Connect and disconnect

In **Settings → GitHub connection**, choose **Sign in with GitHub**. Git Toys displays a verification code and opens `https://github.com/login/device`. Enter the code, inspect the permissions, and authorize the application. Codes expire; cancellation stops polling. No client secret is used.

Public access requests no additional scope. **Authorize private repositories** explicitly requests GitHub OAuth's broad `repo` scope, which includes write permissions even though this integration performs read-only API operations. Organization restrictions or SSO approval can still prevent access. To change access, disconnect, revoke the existing grant on GitHub, and reconnect with the desired option: a prior broader OAuth grant may remain authorized by GitHub.

The access token is stored in macOS Keychain or Windows Credential Manager. Preferences and the activity journal contain no token, verification/device code, description, patch, or API response. Disconnect removes the local token; revoke the OAuth application in **GitHub Settings → Applications → Authorized OAuth Apps** to invalidate it remotely. Existing Git SSH/credential helpers are separate and unchanged.

## Review workflow

1. Add a GitHub.com SSH or HTTPS remote in Git Toys.
2. Open **Pull requests**. When several GitHub remotes exist, choose one explicitly.
3. Filter by state or search text, then select a request.
4. Browse checks/legacy statuses, changed files, commits and reviews, using their page controls. Review entries are individual historical reviews, not a definitive approval decision. GitHub can omit large/binary patches or impose endpoint pagination limits; use **Open on GitHub** for the full result.
5. With a clean repository, choose **Check out for review** and confirm the displayed head commit and review branch.

Checkout fetches the base repository's `refs/pull/<number>/head`, including fork contributions, into `refs/git-toys/pull/<number>`. It verifies the fetched commit against the reviewed API head and switches to `review/pr-<number>-<short-hash>`. Existing local review commits and ignored files are not overwritten. No PR comments, approvals, merges, or pushes are performed.

Descriptions and review bodies render as plain text. Embedded HTML and remote images are disabled. HTTP requests stay on GitHub's fixed API/authentication endpoints, redirects are disabled, cached responses stay in bounded memory, and errors omit response bodies and authorization secrets.

See [GitHub's device-flow documentation](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow) and [pull request API documentation](https://docs.github.com/en/rest/pulls/pulls).
