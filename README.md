<div align="center">

# Skrab

**Copy smarter. Paste faster. Edit anything.**

A lightweight clipboard manager, screenshot tool, and smart paste workspace — in one tray app.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

---

> **Status: pre-release.** Phase 0 (walking skeleton) is complete — the app builds and
> runs on macOS with a tray icon, a global hotkey, and an encrypted local database.
> Clipboard capture lands in Phase 1. There is no downloadable build yet.

## Why

The OS clipboard remembers one thing. Screenshots live in a second app, annotation in a
third, and filling a form from three sources means alt-tabbing until you lose your place.
Skrab does all of it in one place.

- **Clipboard history** — everything you copy, searchable, with favorites and categories.
- **Screenshots** — full screen, window, or region, with an annotation editor.
- **Smart paste** — pin several items to a floating widget and click each into a form.

## Privacy

**Your clipboard never leaves your machine.** Everything lives in an encrypted SQLite
database in your local app data directory. There is no account, no telemetry, no server,
and no network dependency. If cross-device sync is ever added it will be opt-in and
end-to-end encrypted.

Skrab honors the standard "concealed clipboard" markers that password managers set, so
copied passwords are never recorded. You can also blocklist specific apps.

## Platforms

| Platform | Status |
| --- | --- |
| macOS 11+ | Primary development target |
| Windows 11 | Primary target — supported from v1 |
| Linux | Deferred. Wayland blocks global hotkeys by design on most compositors; see [CLAUDE.md](CLAUDE.md) |

## Development

Requires [Node.js](https://nodejs.org) 22+, [pnpm](https://pnpm.io) 10+, and
[Rust](https://rustup.rs) stable.

```bash
pnpm install
pnpm dev
```

Useful commands:

```bash
pnpm check
```

```bash
cd apps/desktop/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

`cargo test` also regenerates the TypeScript IPC types in `packages/ipc-types/src/generated/`.

### Layout

```
apps/desktop      Tauri v2 app — React frontend in src/, Rust backend in src-tauri/
apps/landing      Marketing site + docs (Phase 2)
packages/ipc-types  TypeScript types generated from the Rust structs
```

Architecture, conventions, and the phase plan live in [CLAUDE.md](CLAUDE.md).

## Known environment issues

**macOS: `mac-notification-sys` fails to compile.** If a build errors with
`'LaunchServices/UTCoreTypes.h' file not found`, your Command Line Tools SDK has a
damaged header — the file gets renamed with a timestamp suffix
(`UTCoreTypes 10.11.03 AM.h`). Reinstall the Command Line Tools:

```bash
sudo rm -rf /Library/Developer/CommandLineTools && xcode-select --install
```

`tauri-plugin-notification` is commented out in `apps/desktop/src-tauri/Cargo.toml`
until this is resolved.

## License

MIT © Istiak — see [LICENSE](LICENSE).
