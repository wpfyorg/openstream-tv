---
title: Collect Logs and Report a Problem Safely
description: Capture useful OpenStream TV diagnostics without exposing account tokens, playlist keys or private server data.
category: Troubleshooting
keywords: [logs, bug report, diagnostics, redaction, privacy, 502, support]
---

# Collect logs and report a problem safely

Good reports show what failed, when it failed and the smallest log excerpt that explains it. Do not upload the OpenStream TV data directory, `store_v4.toml`, a full configuration dump or an unedited browser/network export.

## First five minutes

Before changing settings or reinstalling anything:

1. Note the OpenStream TV version shown in **Settings**, or the release/build you installed.
2. Record the operating system and CPU architecture, install method (binary, installer, Docker, OpenWrt or source build), provider, and browser/IPTV player.
3. Record the exact local time and timezone when the failure occurs.
4. Write the shortest reproduction steps and whether the problem affects one channel/item or everything.
5. Reproduce the failure **once**, then immediately collect the surrounding server log lines. Repeated retries can hide the first useful error or trigger provider rate limits.

Keep exact error text and HTTP status codes such as `401`, `403` or `502`. A status code is a symptom; the nearby server log normally shows which local or upstream request failed.

## Collect a fresh log

### Terminal, macOS or Windows

If you normally start OpenStream TV in a terminal, stop it and run a short diagnostic session with debug output enabled. Keep your usual `serve` command, configuration and arguments so the diagnostic run uses the same host, TLS, ports and tunnel settings as the failing server:

```sh
OPENSTREAM_DEBUG=true OPENSTREAM_LOG_TO_STDOUT=true ost serve <your-usual-serve-arguments>
```

On Windows PowerShell:

```powershell
$env:OPENSTREAM_DEBUG = "true"
$env:OPENSTREAM_LOG_TO_STDOUT = "true"
ost serve <your-usual-serve-arguments>
```

Replace the placeholder with the arguments you normally use. Do not drop options such as `--host`, `--tls`, `--port`, `--tls-port`, `--config` or `--tunnel` just to collect logs.

Reproduce the problem once and copy only the lines around that time.

### Linux systemd

System-wide service:

```sh
sudo systemctl status openstream
sudo journalctl -u openstream --since "10 minutes ago"
```

User service:

```sh
systemctl --user status openstream
journalctl --user -u openstream --since "10 minutes ago"
```

### Docker or Docker Compose

Docker:

```sh
docker ps --filter name=openstream
docker logs --since 10m openstream
```

Docker Compose:

```sh
docker compose ps openstream
docker compose logs --since 10m openstream
```

### OpenWrt

```sh
/etc/init.d/openstream status
logread -e openstream
```

Do not attach `/etc/config/openstream` or `/etc/openstream`. They can contain tunnel credentials, account state and TLS material.

## Sanitize before sharing

Read the excerpt yourself before posting it. Remove or replace sensitive values while keeping enough structure to correlate repeated values.

| Sensitive data | Replace with |
| --- | --- |
| Access, refresh, SSO, bearer, session or tunnel token | `<TOKEN_1>` |
| Cookie or authorization header value | `<AUTH_1>` |
| OTP, password or password hash | `<REDACTED_SECRET>` |
| Mobile number, subscriber/customer/user ID | `<ACCOUNT_ID_1>` |
| Device ID or device credential | `<DEVICE_ID_1>` |
| Playlist URL or access-key path | `<PLAYLIST_URL_REDACTED>` |
| Private/public tunnel hostname or private server address | `<PRIVATE_HOST_1>` |
| TLS private key or other private key material | `<PRIVATE_KEY_REDACTED>` |

Use the same placeholder for the same value within one report, for example `<TOKEN_1>` every time the same token appears. Keep timestamps, HTTP status codes, component names, non-secret request paths and the original error text.

Also sanitize screenshots. Check browser address bars, QR codes, Settings pages, terminal history and notification text before uploading them.

If you are unsure whether a value grants access to an account, stream, device or server, redact it.

## Never upload these files or values

- `store_v4.toml` or the whole OpenStream TV data directory
- `/etc/config/openstream` or `/etc/openstream` from OpenWrt
- `.env` files or generated service environment files
- cookies, `Authorization` headers, OTPs, passwords or authentication tokens
- full playlist/M3U URLs containing the access key
- browser storage, browser profiles, HAR files or network exports unless you have manually removed credentials and cookies
- TLS private keys

If support needs a specific field from one of these sources, copy only that field or the relevant redacted lines. Do not publish the original file.

## What to put in a bug report

Use this structure:

```text
OpenStream TV version/build:
OS and architecture:
Install method:
Provider:
Browser/player and version:
Exact failure time + timezone:

Steps to reproduce:
1.
2.
3.

Expected result:
Actual result:
Scope: one channel/item / multiple / all
HTTP status or visible error:

Redacted server log excerpt:
<paste only the relevant lines>
```

For playback failures, include the channel/item and whether it is live, catch-up, HLS or DASH/DRM, but do not include encrypted upstream URLs, license payloads or provider credentials.

See [Troubleshooting OpenStream TV](../troubleshooting/index.md) for symptom-specific checks and [Data Files, Backups and Logs](../configuration-storage-logs/index.md) for storage details.
