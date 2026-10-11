# Release process

CI builds preview artifacts; it does not publish releases. `.github/workflows/ci.yml` checks macOS and Windows and builds aarch64/x86_64 macOS plus Windows x64 installers, with SHA-256 checksums. Those workflows have been authored locally; no remote CI execution is claimed.

## Public-release prerequisites

- Configure an actual repository/release destination. The workspace began without a Git repository or Git remote.
- Supply the product owner's Apple Developer signing identity/certificate and notarization credentials, plus a Windows signing provider. Keep credentials in protected CI secrets, never in tracked config.
- Verify Windows, Intel macOS, configured minimum OS versions, clean installs/upgrades, SSH/HTTPS and enterprise/proxy authentication, credential cancellation, signed commits and manual VoiceOver/Narrator workflows.
- Resolve any security/dependency findings and rerun the acceptance suite on final build inputs.
- Publish only after approval of the exact artifacts and release notes.

## macOS

Use Tauri's [macOS signing guide](https://v2.tauri.app/distribute/sign/macos/) to import a Developer ID identity and configure `APPLE_SIGNING_IDENTITY`. For CI import, Tauri supports `APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD`. Configure notarization via an App Store Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or the documented Apple ID flow. Build app and DMG with `pnpm tauri build --bundles app,dmg`, then verify code signatures, notarization and the installed app before distribution.

No signing identity or notarization credentials were supplied for this implementation. Local preview bundles are not a notarized public release.

## Windows

Use Tauri's [Windows signing guide](https://v2.tauri.app/distribute/sign/windows/) and the chosen provider's current requirements. Modern certificates commonly require hardware/cloud signing; do not assume an exportable PFX is available. Configure `bundle.windows.signCommand` for that provider, or the documented certificate thumbprint/digest/timestamp settings where applicable. Build on Windows and verify signatures on both the executable and installer. Signing alone does not guarantee immediate SmartScreen reputation.

## Updates

No release URL was provided. The settings screen states that update checks are not configured. Once a release destination is established, add an explicit manual version check with a user-triggered HTTPS request and download link. Signed automatic updates remain a later feature. Never point the app at a made-up release feed or silently download executables.
