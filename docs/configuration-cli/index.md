---
title: CLI Commands and Options
description: Reference for the OpenStream TV command and its server flags.
category: Configuration
keywords: [CLI, serve, login, update, epg, key, autostart, TLS, port, fastway]
---

# CLI Commands and Options

Run `ost --help` to see the installed binary's command list. These commands are defined by the current Rust CLI.

| Command | What it does |
| --- | --- |
| `ost serve` | Start the HTTP server. Also accepted: `ost run`, `ost start` or no command. |
| `ost login otp` | Sign in to the JioTV provider account by OTP from the server terminal. |
| `ost login reset` | Clear JioTV login state. |
| `ost tv+ login` / `ost tv+ logout` | Connect/disconnect the optional JioTV+ account when `tv_plus` is enabled. `ost extras login` is still accepted. |
| `ost fastway login` / `ost fastway logout` | Connect/disconnect the optional Fastway provider when `fastway = true` / `OPENSTREAM_FASTWAY=true`. |
| `ost playbox login` / `ost playbox logout` | Connect/disconnect the beta PlayboxTV provider when `playbox = true` / `OPENSTREAM_PLAYBOX=true`. The login asks for a mobile number and then the OTP. |
| `ost airtel login` / `ost airtel logout` | Connect/disconnect the beta Airtel Xstream provider when `airtel = true` / `OPENSTREAM_AIRTEL=true`. Run both commands with `OPENSTREAM_PATH_PREFIX` set to the server's data directory and with permission to write there, so they use the server's store. Stop the server before either command and start it again afterward; a running server may refresh its in-memory session and save it back, overwriting credentials from a concurrent login. |
| `ost tataplay login` / `ost tataplay logout` | Connect/disconnect the beta Tata Play provider when `tataplay = true` / `OPENSTREAM_TATAPLAY=true`. The login asks for a mobile number, offers the accounts on that number when it holds more than one, and then asks for the OTP. |
| `ost admin password` | Set an admin password from the CLI. |
| `ost key show` / `ost key rotate` | Show or replace the IPTV access key. Treat its output as confidential. |
| `ost epg on` / `ost epg off` | Permanently enable or disable automatic EPG generation for this data directory. Running managed OpenWrt/systemd services restart automatically; stopped services stay stopped and use the setting on their next start. `epg on` also prints the EPG URL. Restart manually only for an unmanaged foreground/background server. |
| `ost epg generate` / `ost epg delete` | Generate the current guide or remove its cache. |
| `ost background start` / `ost background stop` | Start or stop a detached server process. |
| `ost update` | Check/install a release. `--version vX.Y.Z` requests a specific release. |
| `ost autostart` / `ost autostart remove` | Install/remove supported startup integration (systemd; Termux shell startup). |
| `ost tunnel setup --domain example.com --subdomain tv [--port 5001]` | Guided Cloudflare API tunnel creation with a scoped `CLOUDFLARE_API_TOKEN`. Saves the connector token and hostname privately. |

All commands accept `--config PATH`. `--skip-update-check` suppresses the startup version check.

## Server options

| Flag | Default | Use |
| --- | --- | --- |
| `--host` / `-H` | `localhost` | IP or interface to bind. |
| `--port` / `-p` | `5001` | HTTP port. |
| `--public` / `-P` | off | Listen on `[::]` (all interfaces). Protect access before enabling. |
| `--tls` | off | Add HTTPS for browser playback. |
| `--tls-port` | `5443` | HTTPS listener port; different from HTTP. |
| `--tls-cert` / `--tls-key` | generated cert | Supply both PEM files with `--tls`. |
| `--tunnel` | off | Start a Cloudflare tunnel where supported. |
| `--tunnel-quick` | off | Start a temporary Cloudflare tunnel and use its detected public HTTPS origin. |
| `--tunnel-saved` | off | Use the token and hostname saved by `ost tunnel setup`. |
| `--tunnel-token` | unset | Named tunnel token; keep private. |

`--tunnel-token` is visible in the process list. Prefer `OPENSTREAM_TUNNEL_TOKEN`, which the guided installer and OpenWrt service use. `ost autostart` records current `OPENSTREAM_*` variables in a private environment file for systemd and Termux autostart. On other systems, persist settings in a user-owned environment file or set them in the shell that starts the server.

For manually managed proxy hostnames or named tunnels, set `OPENSTREAM_PUBLIC_ORIGIN=https://tv.example.com` (legacy `JIOTV_PUBLIC_ORIGIN` also works) or `public_origin = "https://tv.example.com"` in TOML. This keeps M3U stream, EPG and DRM license links HTTPS when the public hostname requests them. Saved and quick tunnel modes set the public origin automatically.

Example: `ost serve --host 127.0.0.1 --port 5001 --tls`. This serves HTTP on 5001 and HTTPS on 5443. When no certificate files are supplied, the server generates a self-signed certificate; browsers may show a warning until you trust it.

**Avoid exposing the service publicly** without strong controls. The access key and login are not a substitute for a network security plan. See [Network and HTTPS](../configuration-network-security/index.md).
