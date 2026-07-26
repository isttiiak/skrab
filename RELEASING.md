# Releasing Skrab

## One-time setup

### 1. Updater signing keypair

The Tauri updater refuses unsigned updates — this cannot be disabled, and it is the
reason auto-update is not enabled yet.

```bash
pnpm --filter @skrab/desktop tauri signer generate -w ~/.tauri/skrab.key
```

Then:

- Add the **private key** as the `TAURI_SIGNING_PRIVATE_KEY` repository secret and its
  password as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Put the **public key** in `apps/desktop/src-tauri/tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "pubkey": "<public key>",
    "endpoints": [
      "https://github.com/isttiiak/skrab/releases/latest/download/latest.json"
    ]
  }
}
```

- Register the plugin in `apps/desktop/src-tauri/src/lib.rs` (there is a comment
  marking the exact spot).

> **Never commit the private key.** `.gitignore` covers `*.key` and `*.pem`, but losing
> it means existing installs can no longer verify updates and every user has to
> reinstall by hand. Back it up somewhere durable.

### 2. Code signing (optional, improves install UX)

| Platform | Option | Cost |
| --- | --- | --- |
| Windows | [SignPath](https://signpath.io/) free tier for OSS, or Azure Trusted Signing | Free |
| macOS | Apple Developer Program — required for notarization | $99/year |

Until macOS signing exists, keep the "Open Anyway" instructions prominent on the
download page.

### 3. Cloudflare Pages deploy hook (optional)

Create a deploy hook for the landing project and store the URL as the
`CLOUDFLARE_DEPLOY_HOOK` secret. The release workflow pings it so the download page
picks up the new assets immediately. Without it the page refreshes on the next push.

## Cutting a release

1. Make sure `main` is green: `pnpm check`, plus `cargo clippy --all-targets -- -D warnings`
   and `cargo test` in `apps/desktop/src-tauri`.
2. Bump the version in **both** `apps/desktop/package.json` and
   `apps/desktop/src-tauri/tauri.conf.json` (and `Cargo.toml`), then update
   `CHANGELOG.md`.
3. Commit with `chore(release): v0.1.0`.
4. Tag and push:

```bash
git tag -a v0.1.0 -m "Skrab v0.1.0" && git push origin main --tags
```

The `Release` workflow then builds macOS (Apple silicon + Intel) and Windows x64,
generates release notes from the commits since the previous tag, publishes the GitHub
Release with all installers attached, and refreshes the landing page.

Pre-releases: any tag containing a hyphen (`v0.2.0-beta.1`) is published as a
pre-release automatically.

## Commit conventions

Release notes are grouped from [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(clipboard): add search filtering by content type
fix(monitor): stop re-capturing our own clipboard writes
perf(db): index clip_items on created_at
docs: document the Gatekeeper workaround
chore(deps): bump tauri to 2.11.5
```

`feat` → Features · `fix` → Fixes · `perf` → Performance · `docs` → Documentation ·
`chore`/`refactor`/`build`/`ci`/`test` → Maintenance.
