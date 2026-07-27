# Changelog

All notable changes to Skrab are documented here. Release notes on GitHub are
generated automatically from [Conventional Commits](https://www.conventionalcommits.org/);
this file records the human summary per version.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.1] — 2026-07-27

### Changed

- **The floating panel is now the main window, not a separate widget.** 0.2.0 opened a
  second window that showed only pinned clips; the point was to have your *whole*
  history available while filling in a form. The pin button now keeps the ordinary
  panel above other applications, and while it is pinned, copying leaves it open and
  Escape only clears the search.

### Added

- **Window capture**, so all three modes exist: region, window and whole screen. The
  window mode lists open windows rather than asking you to click one, because Skrab's
  own panel is in front and clicking through would leave no way to cancel.

### Fixed

- **Screenshots never appeared in the clipboard history.** Suppressing the clipboard
  monitor to stop a capture echoing back as a copy also stopped it being recorded at
  all. Captures are now inserted into the history directly.
- **The region overlay reopened with the previous selection still drawn**, so the first
  click looked like it cleared something. The overlay window is now destroyed on close
  rather than hidden, so each capture starts clean.
- **The region overlay was a blank white screen on Windows.** It loaded the frozen
  frame through Tauri's asset protocol, which depends on the protocol being enabled
  *and* the file falling inside a configured scope — and when either is wrong the
  result is an empty window with no way to tell why. The frame is now inlined as a
  data URI, and a frame that fails to arrive shows an explanation and a close button
  instead of a blank trap.


## [0.2.0] — 2026-07-27

### Added

- **Configurable shortcuts.** Settings → Shortcuts: click one, press the combination
  you want. Skrab registers it immediately and reports back — it names the problem
  when another application already owns the combination, or when two Skrab actions
  would clash, instead of failing silently the way global shortcuts normally do.
  Shortcuts can be cleared individually or reset to defaults.
- **Screenshot capture.** Region (`Cmd/Ctrl+Shift+A`) and full screen
  (`Cmd/Ctrl+Shift+S`), also on the panel toolbar. Region capture freezes the screen
  first and lets you drag on the still, so the overlay can never appear in its own
  capture. Captures land on the clipboard immediately.
- **Toolbar buttons** in the panel for the pinned widget and both capture modes — the
  features existed in 0.1.1 but nothing pointed at them.

### Fixed

- **macOS "Skrab.app is damaged and can't be opened."** The released bundle carried
  only the linker's implicit signature with no `_CodeSignature` seal, so macOS
  rejected it outright. Releases are now ad-hoc signed, which turns this into the
  ordinary unidentified-developer prompt. Existing downloads can be fixed with
  `xattr -dr com.apple.quarantine /Applications/Skrab.app`.

### Changed

- **Dark is now the default theme** rather than following the system. Light and
  system remain one click away in Settings.
- Documentation now covers the pinned-item workflow, screenshots, custom shortcuts,
  and how to upgrade cleanly on Windows.


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

[Unreleased]: https://github.com/isttiiak/skrab/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/isttiiak/skrab/releases/tag/v0.2.1
[0.2.0]: https://github.com/isttiiak/skrab/releases/tag/v0.2.0
[0.1.1]: https://github.com/isttiiak/skrab/releases/tag/v0.1.1
[0.1.0]: https://github.com/isttiiak/skrab/releases/tag/v0.1.0
