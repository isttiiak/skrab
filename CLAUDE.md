# CLAUDE.md — Skrab

> Cross-platform clipboard manager + screenshot tool + smart paste workspace.
> Single source of truth for Claude Code sessions on this project.

---

## Project Identity

- **Name:** Skrab (working title — a trademark/domain check is still open)
- **Tagline:** Copy smarter. Paste faster. Edit anything.
- **Repo:** `https://github.com/isttiiak/Skrab` (planned)
- **License:** MIT
- **Author:** Istiak (GitHub: `isttiiak`)
- **Status:** Phase 0 complete — walking skeleton runs on macOS

---

## What This App Does

Skrab is a lightweight, always-running system tray app that replaces the default OS
clipboard with a clipboard manager, screenshot capture tool, annotation editor, and
"smart paste" workspace in one tool.

### Core problems it solves

1. **Clipboard amnesia** — the OS clipboard holds one item. Skrab remembers everything.
2. **Scattered workflows** — clipboard history, screenshots, and annotation are three
   separate apps today. Skrab unifies them.
3. **Form-filling friction** — pin multiple copied items as floating widgets and paste
   each with one click, instead of alt-tabbing between sources.

### The non-negotiable promise

**Your clipboard never leaves your machine.** This is the product's differentiator and
the reason the storage architecture looks the way it does. Any feature that would break
it needs an explicit, opt-in, end-to-end-encrypted design — not a default.

---

## Tech Stack

All versions verified against crates.io / npm on 2026-07-26. When adding a dependency,
check its current version rather than copying one from memory — this table will drift.

### Frontend

| Layer | Choice | Version |
| --- | --- | --- |
| App shell | Tauri v2 | 2.11 |
| UI | React + TypeScript | 19.2 / 7.0 |
| Build | Vite (Rolldown/Oxc) | 8.1 |
| Styling | Tailwind CSS (CSS-first, no config file) | 4.3 |
| Components | shadcn/ui | latest |
| Canvas/editor | Konva.js (`react-konva`) | 10.3 / 19.2 |
| State | Zustand | 5.0 |
| Toasts | sonner (shadcn's `toast` is deprecated) | 2.0 |
| Lint + format | Biome | 2.5 |
| Tests | Vitest | 4.1 |
| Package manager | pnpm workspaces | 10.x |

### Rust backend

| Crate | Purpose |
| --- | --- |
| `rusqlite` (`bundled-sqlcipher-vendored-openssl`) | Encrypted SQLite. Rust owns all SQL. |
| `arboard` | Cross-platform clipboard read/write (Phase 1) |
| `xcap` | Screenshot capture (Phase 3) |
| `image` | Encoding, thumbnails, format conversion |
| `ts-rs` | Generates TypeScript types from Rust structs |
| `blake3` | Content hashing for dedup |
| `thiserror` | Error types |
| `enigo` | Simulated keystrokes (Phase 5, feature-gated) |

Tauri plugins in use: `global-shortcut`, `single-instance`, `autostart`, `log`, `store`,
`opener`, `os`, `process`, `window-state`. Pending: `updater` (Phase 2 — needs the
signing keypair), `notification` (blocked, see Known Issues).

### Decisions already made — do not re-litigate

1. **Tauri v2**, not v1. APIs and config differ significantly. A v3 milestone exists
   upstream with no committed roadmap; v2 is the target.
2. **SQLite, owned by Rust.** Not Supabase, not IndexedDB, not localStorage. A clipboard
   manager must record a copy in milliseconds and work offline; a network round-trip per
   copy is architecturally fatal, and clipboard contents are too sensitive to default to
   a third-party server.
3. **No `tauri-plugin-sql`.** It hands arbitrary SQL execution to the webview, and the
   webview renders untrusted copied HTML. Rust exposes typed commands instead.
4. **Konva.js** for the editor. Not Fabric.js.
5. **Biome**, not ESLint/Prettier. typescript-eslint does not support TS 7 and is blocked
   until TS 7.1; Biome has its own Rust parser and sidesteps the problem entirely.
6. **`ts-rs`, not `tauri-specta`.** tauri-specta's Tauri-v2 support exists only as
   `2.0.0-rc.25` (14 months elapsed between RCs). ts-rs 12 is stable and generates the
   types that actually drift; the invoke wrappers in `lib/tauri.ts` are hand-written,
   thin, and unit-tested.
7. **No paid dependencies.** Everything free and open source.

### Platform targets

- **macOS 11+** — `.dmg` (primary development machine: Mac mini M4)
- **Windows 11** — `.msi` / NSIS (primary target audience)
- **Linux** — deferred to Phase 6. Wayland blocks global hotkeys by design on
  Sway/Hyprland/COSMIC unless the compositor implements
  `zwp_keyboard_shortcuts_inhibit_manager_v1`, and clipboard monitoring needs the
  `wlr-data-control` protocol. Not worth carrying on an untestable platform yet.

---

## Project Structure

```
Skrab/
├── CLAUDE.md · README.md · LICENSE · biome.json
├── pnpm-workspace.yaml          # dependency catalog — bump versions HERE
├── apps/
│   ├── desktop/
│   │   ├── src/                 # React frontend
│   │   │   ├── components/{ui,clipboard,screenshot,editor,settings}/
│   │   │   └── hooks/ · stores/ · lib/ · types/ · styles/
│   │   └── src-tauri/
│   │       ├── .cargo/config.toml   # TS_RS_EXPORT_DIR
│   │       ├── capabilities/        # Tauri v2 permissions (deliberately narrow)
│   │       ├── icons/
│   │       └── src/
│   │           ├── main.rs · lib.rs · error.rs · window.rs
│   │           ├── db/          # connection, key, migrations, queries
│   │           ├── tray/ · hotkeys/ · commands/
│   │           ├── clipboard/   # monitor, history, types      (Phase 1)
│   │           ├── security/    # concealed-content, blocklist (Phase 1)
│   │           ├── screenshot/  # capture, overlay             (Phase 3)
│   │           ├── input/       # enigo, feature-gated         (Phase 5)
│   │           └── sync/        # SyncProvider trait — seam only, no cloud code
│   └── landing/                 # Astro + Starlight (Phase 2)
└── packages/
    └── ipc-types/               # generated/ is written by ts-rs — never hand-edit
```

---

## Code Conventions

### General

- **TypeScript only** — `.tsx` for components, `.ts` otherwise. Never `.js`/`.jsx`.
- **Rust for anything system-level** — clipboard, screenshots, file I/O, hotkeys, SQL.
  The frontend never touches system APIs or the database directly.
- **Tauri commands are the only bridge.** Types flow from Rust via `ts-rs` into
  `@skrab/ipc-types`; frontend wrappers live in `src/lib/tauri.ts`.
- **No `any`** — use `unknown` plus narrowing.
- **No default exports** except route/page components (enforced by Biome).
- **pnpm only** — never `npm` or `yarn`. Dependency versions go in the workspace catalog.

### Naming

| Element | Convention | Example |
| --- | --- | --- |
| React components | PascalCase | `ClipboardPanel.tsx` |
| Hooks | camelCase, `use` prefix | `useClipboard.ts` |
| Utilities | camelCase | `tauri.ts` |
| Zustand stores | camelCase + `Store` | `clipboardStore.ts` |
| Types/interfaces | PascalCase, no `I` prefix | `ClipItem` |
| Rust files/functions | snake_case | `get_clip_history` |
| Rust structs/enums | PascalCase | `ClipType` |
| Commands | snake_case in Rust → camelCase in TS | `get_clip_history` → `getClipHistory` |
| DB tables | snake_case, plural | `clip_items` |

### Frontend patterns

- Functional components only.
- Zustand for global state; `useState`/`useReducer` for local. No prop drilling past 2 levels.
- Tauri event listeners via `listen()` must be cleaned up in the effect's return.
- Every `invoke()` goes through the `call()` helper in `lib/tauri.ts`, which turns Tauri's
  bare-string rejection into a real `Error`. Surface failures via sonner toasts.
- Path alias `@/…`. **TS 7 removed `baseUrl`** — `paths` resolve relative to `tsconfig.json`.

### Rust patterns

- `thiserror` for errors; every command returns `crate::Result<T>`. **Never `unwrap()`
  outside tests.**
- Async via `tokio`; background work through `tauri::async_runtime::spawn`.
- Each subsystem is a module with a narrow public API through `mod.rs`.
- Every third-party crate touching the OS sits behind our own module boundary with our
  own types crossing it. Swapping `arboard` must be a one-file change.
- Logging via `log` + `tauri-plugin-log`: `error` for failures, `warn` for recoverable,
  `info` for significant events, `debug` for development.
- `///` doc comments on `#[derive(TS)]` types are **copied into the generated TypeScript**
  — write them for the frontend reader.

---

## Data Layer

### Storage locations

- Database: `{app_data_dir}/Skrab.db` (SQLCipher-encrypted — verified: a plain `sqlite3`
  cannot open it)
- Key: `{app_data_dir}/.dbkey`, mode 0600 — **Phase 1 moves this to the OS keychain**
- Clip images: `{app_data_dir}/clips/`
- Screenshots: `{app_data_dir}/screenshots/`

On macOS `app_data_dir` is `~/Library/Application Support/com.isttiiak.skrab/`.

### Schema rules

Migrations are an **append-only** slice in `db/migrations.rs`, versioned by
`PRAGMA user_version`, each applied in its own transaction. Never edit a shipped
migration — add a new one.

Design points that are load-bearing:

- **Timestamps are `INTEGER` unix-millis**, never ISO-8601 text. Smaller index, integer
  comparison, correct sort order.
- **`content_hash`** (blake3, unique-indexed) dedups identical copies *and* detects our
  own clipboard writes, so pasting from history doesn't record a new entry.
- **`preview`** holds the first ~200 chars so the list never loads a 10MB clip.
- **`thumb`** holds a small WebP blob; full images live on disk.
- **Soft deletes** (`deleted_at`) + monotonic **`updated_at`** are the sync seam.
- **FTS5** virtual table kept in step by triggers — never `LIKE '%…%'`.

### Sync

`sync/mod.rs` will define a `SyncProvider` trait and nothing else. There is **no cloud
code** and no cloud dependency. If sync ever ships it must be end-to-end encrypted and
opt-in.

---

## Security Rules (non-negotiable)

1. **Honor concealed-clipboard markers.** macOS: `org.nspasteboard.ConcealedType` and
   `TransientType`. Windows: `ExcludeClipboardContentFromMonitorProcessing` and
   `CanIncludeInClipboardHistory`. Password managers set these. Without this handling,
   Skrab's history panel becomes a password dump. Filtering happens **before the first
   write to disk**, not after.
2. **App blocklist** — user-editable list of source apps to never record.
3. **Narrow capabilities.** The webview gets no filesystem, shell, or SQL access.
4. **Strict CSP.** Copied HTML is untrusted input and must be sanitized before render.
5. **Never commit signing keys.** `.gitignore` covers `*.key` / `*.pem`.

---

## Feature Specifications

### F1 — Clipboard Manager

Monitoring uses the **platform change counter, never content polling**:
macOS compares `NSPasteboard.changeCount` every ~250ms (an integer read — no allocation,
no clipboard ownership churn); Windows uses `AddClipboardFormatListener` (event-driven,
zero polling). Contents are read **only** when the counter moves. Content polling is the
CPU/battery bug that plagues other clipboard managers.

Then: categorize (text/image/html/rtf/file) → concealed-content check → blake3 hash →
dedup → persist → emit event to the frontend.

Panel: virtualized list, FTS5 search, type filter, keyboard nav, favorites, pins,
categories, configurable retention (favorites exempt).

### F2 — Screenshot Capture

Full screen / monitor / window / region. Region uses a borderless transparent fullscreen
overlay window; coordinates go to Rust, which crops the capture.

### F3 — Screenshot Editor

Konva canvas, layered. Highlight, text, freehand, shapes, eraser, color picker, undo/redo
stack. Export to PNG/JPG, file or clipboard. Blur/pixelate redaction later.

### F4 — Smart Paste / Pinned Items (the differentiator)

Always-on-top, undecorated, transparent widget window showing pinned items as cards.
Click writes to clipboard; used/unused visual feedback; optional auto-advance for
sequential form fields. Auto-paste via `enigo` is **opt-in and feature-gated** — it needs
macOS Accessibility permission, which is a meaningful trust ask. Click-to-copy works
without it.

### F5 — Settings

Hotkeys, monitoring toggle, retention period, max items, autostart, theme, default
screenshot action, save directory, pin window opacity, app blocklist.

---

## Default Hotkeys

| Action | Windows | macOS |
| --- | --- | --- |
| Clipboard history | `Ctrl+Shift+V` | `Cmd+Shift+V` |
| Screenshot — full screen | `Ctrl+Shift+S` | `Cmd+Shift+S` |
| Screenshot — region | `Ctrl+Shift+A` | `Cmd+Shift+A` |
| Screenshot — window | `Ctrl+Shift+W` | `Cmd+Shift+W` |
| Toggle pinned widget | `Ctrl+Shift+P` | `Cmd+Shift+P` |
| Quick paste last N | `Ctrl+Shift+1..9` | `Cmd+Shift+1..9` |

All user-configurable. `Modifiers::SUPER` is Command on macOS but the Windows key on
Windows — the two platforms need different modifiers, not one shared constant. A failed
registration is logged and non-fatal: the accelerator may already be taken, and the tray
still works.

---

## Platform Gotchas

- **macOS Screen Recording permission** is required for `xcap`. The app must be
  **relaunched** after granting it. In dev, each rebuild changes the binary path and
  resets the grant — expect this.
- **macOS Accessibility permission** is required for simulated keystrokes (F4). The
  auto-paste setting defaults to off; `enigo` is compiled in but never invoked until
  the user opts in.
- **macOS 26 Tahoe removed the right-click→Open** Gatekeeper bypass. Unsigned builds
  need System Settings → Privacy & Security → Open Anyway. The landing page must
  document this.
- **`xcap` uses `objc2-core-graphics`, not ScreenCaptureKit.** Verify capture on current
  macOS before building UI on it (Phase 3 spike).

---

## Known Issues

**`tauri-plugin-notification` is commented out** in `apps/desktop/src-tauri/Cargo.toml`.
Its transitive `mac-notification-sys` compiles Objective-C against `Cocoa.h`, which fails
on the dev machine because the Command Line Tools SDK has a damaged header:
`UTCoreTypes.h` was renamed to `UTCoreTypes 10.11.03<U+202F>AM.h` (note the Unicode narrow
no-break space). Fix by reinstalling the CLT, then re-enable the plugin:

```bash
sudo rm -rf /Library/Developer/CommandLineTools && xcode-select --install
```

---

## Development

```bash
pnpm install
pnpm dev            # tauri dev
pnpm check          # biome + tsc --noEmit + vitest, across the workspace
```

Rust checks run from `apps/desktop/src-tauri`:
`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.

`cargo test` also regenerates `packages/ipc-types/src/generated/` via ts-rs.

### Commit conventions

Conventional Commits: `feat(clipboard): …`, `fix(screenshot): …`, `chore(deps): …`.

---

## Development Phases

- **Phase 0 — Walking skeleton. ✅ Done.** Workspace, tray + menu, hide-on-close, global
  hotkey, single-instance guard, encrypted SQLite with migrations, typed IPC via ts-rs,
  full lint/typecheck/test pipeline. *Still open: verify on Windows 11.*
- **Phase 1 — Clipboard MVP.** Change-counter monitor, concealed-content filtering,
  dedup, history panel, FTS5 search, favorites, retention, settings, keychain key storage.
- **Phase 2 — Distribution + landing page.** CI matrix, release workflow, updater
  keypair, Windows signing, Astro landing page on Cloudflare Pages, **v0.1.0 shipped**.
- **Phase 3 — Screenshot capture.** Backend done: display/window/region capture,
  multi-monitor rebasing, edge clamping. The `xcap` spike passes on macOS 26 (real
  frame with live windows, not the degraded desktop-only frame). *Still to do: the
  region-selection overlay and the capture UI.*
- **Phase 5 — Smart paste / pinned items. ✅ Done** (reordered ahead of the editor at
  the user's request — it is the differentiator, and the editor is the bigger chunk).
  Always-on-top widget, click-to-copy, used/unused feedback, auto-advance, opt-in
  auto-paste via `enigo`.
- **Phase 4 — Screenshot editor.** Konva, annotation tools, undo/redo, export.
- **Phase 6 — Linux, then OCR / redaction / AI object removal / optional E2EE sync.**

### Performance budget

This runs in the background all day. Measured on **release** builds only — a debug
build with devtools and HMR tells you nothing.

**Measured on 2026-07-27 (v0.1.1, macOS, Apple silicon):** main process **108 MB**,
plus WebKit's helpers (66 MB WebContent, 17 MB Networking).

**The <80 MB target is currently NOT met** and needs a real optimisation pass — do
not quietly restate the target as met. Likely candidates: the base64 thumbnails in
the list payload, the default page size, and whatever the webview retains.

Still holding: **idle CPU ≈ 0%** · **clip recorded < ~300ms** · **search < 100ms**
over 1000+ items.

---

## Notes for Claude Code Sessions

1. **macOS/zsh** for terminal instructions — that's the primary dev machine.
2. **Verify dependency versions** against crates.io/npm before adding them.
3. **Prefer hints over direct answers** when explaining — Istiak learns better working
   through problems with guided hints.
4. **Consider both macOS and Windows** in every clipboard/hotkey/path decision.
5. **Keep it lightweight.** Watch memory, avoid re-renders, keep SQLite queries indexed.

---

_Last updated: 2026-07-27 · CLAUDE.md version: 2.1_
