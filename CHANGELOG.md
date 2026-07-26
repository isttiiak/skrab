# Changelog

All notable changes to Skrab are documented here. Release notes on GitHub are
generated automatically from [Conventional Commits](https://www.conventionalcommits.org/);
this file records the human summary per version.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] — 2026-07-27

Fixes from the first real use of v0.1.0 on macOS and Windows.

### Fixed

- **Dark mode was unreadable.** `index.html` carried a leftover `bg-transparent`
  class that beat the themed `background-color`, so the window never painted its
  background. In dark mode the near-white text landed on the webview's default white
  while surfaces correctly went dark, leaving labels and clip previews almost
  invisible. Contrast on secondary text and borders was raised in both schemes too.
- **The launch-at-login toggle did nothing** outside Skrab's own database — it never
  reached the autostart plugin.
- **A failed copy blanked the history list**, because the error state replaced the
  whole list with a red message. Failures now show a toast with the real reason and
  leave the list alone.
- **A row's copy action used the selected index**, so acting on a row you had not
  selected first copied the wrong clip.
- The DMG no longer shows a click-through license agreement before mounting.
- CI's generated-types check no longer fails on Windows because of CRLF checkouts.

### Added

- **A copy button on every clip**, always visible rather than hidden behind hover.
  It leaves the panel open so you can collect several items; `Enter` still copies and
  dismisses.
- An error boundary, so a render fault shows a message instead of a blank window.

### Changed

- The footer hint now reads **"copy & close"** rather than "paste". Skrab puts the
  clip on your clipboard; it does not inject a paste into the previously focused app.
  That is Phase 5.

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

[Unreleased]: https://github.com/isttiiak/skrab/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/isttiiak/skrab/releases/tag/v0.1.1
[0.1.0]: https://github.com/isttiiak/skrab/releases/tag/v0.1.0
