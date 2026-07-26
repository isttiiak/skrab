# Changelog

All notable changes to Skrab are documented here. Release notes on GitHub are
generated automatically from [Conventional Commits](https://www.conventionalcommits.org/);
this file records the human summary per version.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] — 2026-07-27

First public release. Clipboard manager only — screenshots, the annotation editor,
and smart paste are on the roadmap.

### Added

- **Clipboard history.** Text, images, HTML and RTF are captured automatically and
  stored in an encrypted local database.
- **Change-counter monitoring.** The watcher samples the OS clipboard change counter
  (`NSPasteboard.changeCount` on macOS, `GetClipboardSequenceNumber` on Windows) and
  only reads the payload when it moves, so it neither polls contents nor takes
  clipboard ownership.
- **Password protection.** Content marked concealed by a password manager is never
  recorded — `org.nspasteboard.ConcealedType` / `TransientType` on macOS, and the
  `ExcludeClipboardContentFromMonitorProcessing` family on Windows. Filtering happens
  before anything reaches disk.
- **Secret-shape detection.** API keys, JWTs and private keys are skipped even without
  an OS marker. Can be turned off in Settings.
- **App blocklist.** Name any app that should never be recorded from.
- **Full-text search** over clip contents via SQLite FTS5, with type filters,
  favourites and pinning.
- **Deduplication** by BLAKE3 content hash — re-copying something moves it back to the
  top instead of creating a duplicate.
- **Encryption at rest.** SQLCipher, with the key held in the macOS Keychain or Windows
  Credential Manager.
- **Global hotkey** (`Cmd/Ctrl+Shift+V`), system tray icon, launch at login, and a
  settings panel covering retention, item cap, theme and privacy controls.
- **Landing page and docs** at `apps/landing`, including the macOS Gatekeeper and
  Windows SmartScreen walkthroughs.

### Known limitations

- Builds are **unsigned**. macOS requires an "Open Anyway" step on first launch; see
  the docs. In-app auto-update is therefore not enabled yet.
- Linux is not supported yet — Wayland blocks global shortcuts on most compositors.
- Screenshot capture, the annotation editor, and smart paste are not implemented.

[Unreleased]: https://github.com/isttiiak/skrab/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/isttiiak/skrab/releases/tag/v0.1.0
