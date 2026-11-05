# Repository Guidelines

## Project Overview

OpenStream TV is a Rust CLI/server for live TV, catch-up, EPG, IPTV/M3U clients, and supported on-demand playback. It currently integrates JioTV and JioTV+ provider flows and is being generalized for additional providers. The HTTP server uses Axum/Tokio. The full build embeds a Svelte/Vite UI; the slim build omits browser assets.

## Architecture

- `src/main.rs` owns startup, command dispatch, data-directory resolution, server lifecycle, background mode, and migration fallbacks.
- `src/cli.rs` parses the `openstream` CLI.
- `src/config.rs` owns TOML configuration and environment loading. `OPENSTREAM_*` is primary; legacy `JIOTV_*` names remain compatibility aliases.
- `src/server.rs` composes Axum routes and serves the embedded UI in full builds.
- `src/television.rs` and `src/login.rs` contain JioTV provider API behavior.
- `src/extras.rs` and `src/extras_state.rs` own the optional JioTV+ source.
- `src/stream.rs`, `src/catchup.rs`, `src/dash.rs`, `src/secureurl.rs`, and related modules own playback proxying and protected local URLs.
- `src/store.rs` owns persistent runtime state.
- `src/autostart.rs`, `src/tunnel.rs`, `src/tls.rs`, and `src/update.rs` own service integration, Cloudflare tunnels, TLS, and self-update.
- `web/ui/` contains the Svelte 5/Vite frontend. `web/ui/dist/` is generated and embedded by the full build.
- `openwrt/openstream/` contains OpenWrt package/service definitions.
- `scripts/` contains installers, packaging helpers, docs tooling, and integration tests.

Preserve provider-specific JioTV identifiers where they refer to the real upstream API, account, routes, payload fields, or compatibility state. Use OpenStream TV/openstream for application identity, commands, services, packages, release assets, and new configuration.

## Repository Skills

- `.agents/skills/change-configuration/SKILL.md` — use for `OPENSTREAM_*`, legacy `JIOTV_*`, or TOML configuration changes.
- `.agents/skills/diagnose-playback/SKILL.md` — use for live, catch-up, HLS, DASH/DRM, browser, or IPTV playback failures.

## Development Commands

```sh
cargo build
cargo test
cargo test --no-default-features --features slim
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features --features slim -- -D warnings

cd web/ui
npm ci
npm test
npm run build
```

Installer and packaging checks:

```sh
sh scripts/tests/install-test.sh
sh scripts/tests/onboard-test.sh
sh scripts/tests/onboard-ps1-test.sh
sh scripts/tests/openstream-token-test.sh
```

Use focused `cargo test <name>` runs while iterating, then run the relevant full matrix before completion.

## Code Conventions

- Format changed Rust files with `rustfmt`; avoid unrelated formatting churn.
- Prefer existing concrete modules and shared application state over new abstraction layers without a real need.
- Keep async/network behavior bounded and propagate operational errors with context.
- Keep tests offline and deterministic where practical.
- Frontend code follows existing Svelt/Vite patterns; do not add another frontend framework.
- New public environment variables use `OPENSTREAM_*`. Keep documented legacy `JIOTV_*` fallbacks unless intentionally performing a breaking migration.
- Runtime credentials/data belong in the resolved OpenStream data directory, normally `$HOME/.openstream-tv` or `/etc/openstream` on OpenWrt. Legacy directories may be read for migration compatibility.
- Never commit `.env` files, credentials, tokens, generated runtime state, logs, private playlist URLs, or user data.

## Diagnostic Evidence and Sanitization

- Ask for the minimum evidence needed: version/build, OS/architecture, install method, provider/client, exact failure time/timezone, reproduction steps, scope, and a short log excerpt around one fresh reproduction.
- Never ask users to paste full credential-bearing state or configuration files such as `store_v4.toml`, the OpenStream data directory, `/etc/config/openstream`, `.env` files, generated service environment files, browser profiles/storage, or TLS private keys.
- Never ask for or expose OTPs, passwords, cookies, authorization headers, access/refresh/SSO/session/tunnel tokens, provider account identifiers, device credentials, private hostnames, or playlist/access-key URLs.
- When preparing an issue, report, fixture, handoff, or saved diagnostic excerpt, inspect and sanitize it before writing or presenting it. Do not claim a capture is safe unless it was actually inspected.
- Preserve diagnostic fidelity after redaction: keep timestamps, HTTP status codes, component names, non-secret request paths, stack/error text, and ordering.
- Use stable typed placeholders so repeated values stay correlatable: the same token becomes `<TOKEN_1>`, the same account ID becomes `<ACCOUNT_ID_1>`, and the same private host becomes `<PRIVATE_HOST_1>`. Strip secret query parameters instead of replacing the entire useful path when possible.
- If a value may grant access to an account, stream, device, server, or private network and its sensitivity is uncertain, redact it.
- Do not commit raw logs, HAR captures, packet captures, screenshots with secrets, or copied runtime state. Keep diagnostic artifacts outside the repository unless they are deliberately sanitized test fixtures.
- The user must enter every OTP themselves. Agents may describe where to enter it but must not request, relay, store, or reuse the OTP.
- User-facing collection and redaction instructions live in `docs/troubleshooting-reporting-logs/index.md`.

## Important Files

- `Cargo.toml` — Rust package, executable, feature definitions.
- `src/main.rs` — application entry point and lifecycle.
- `src/config.rs` — configuration schema/loading.
- `src/server.rs` — route composition.
- `src/television.rs` — JioTV provider client.
- `src/store.rs` — persistent state.
- `src/update.rs` — GitHub release self-update.
- `scripts/install.sh` / `scripts/install.ps1` ― supported installers.
- `openwrt/openstream/` — OpenWrt package files.
- `web/ui/package.json` — frontend scripts and dependencies.

## Testing and Release

CI exercises default and slim Rust tests/clippy, shell installer flows, the token-switch helper, and Windows installer parsing/smoke tests. UI source changes also require `npm test` and `npm run build`.

Release assets use the `openstream-*` prefix. The public repository is `wpfyorg/openstream-tv`; GitHub Pages lives at `https://wpfyorg.github.io/openstream-tv/`.

Use Conventional Commits. Keep one logical change per commit. Do not manually bump release versions or create tags for normal work. Do not push unless the user has asked for remote publication.
