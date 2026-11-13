# 🚀 OpenStream TV

A single Rust binary that turns your provider subscription into live TV, catch-up, an EPG and an IPTV playlist — with a web player, a channel guide and a settings panel served from the same process. No database, no runtime to install, no second box.

Run it on a laptop, a Raspberry Pi, a NAS or an OpenWrt router, open `http://localhost:5001`, sign in to your provider, and watch from any screen on your network or through an IPTV app you already own.

---

## 📺 What You Get

| Capability | What it means |
| :--- | :--- |
| **Live TV** | Every channel your provider account can play, proxied through your own server behind a protected local URL. |
| **Catch-up** | Programmes the provider keeps for replay, playable from the web UI. |
| **Seven-day EPG** | A programme guide in the UI and an XMLTV file for your TV box. |
| **IPTV playlist** | A ready-made `playlist.m3u` plus `epg.xml.gz` for VLC, Kodi, TiviMate, OTT Navigator and friends. |
| **Web interface** | Svelte UI served by the same binary: home rows, channel grid, guide, player, settings. |
| **Protected playback** | DASH and Widevine handling for DRM streams, with the provider's own error surfaced instead of a black screen. |
| **On demand** | Supported provider catalogues and OTT playback, where the provider exposes them. |
| **Remote access** | Optional TLS with a self-signed certificate, or a Cloudflare tunnel with one command. |
| **Multi-provider** | Sign in to several providers at once and choose which catalogue is active. |

---

## 🧩 Supported Providers

| Provider | Status | Sign in with | Turn it on with |
| :--- | :--- | :--- | :--- |
| **JioTV** | Stable | Jio mobile number + OTP | always available |
| **JioTV+** | Stable | JioFiber / AirFiber number + OTP | `tv_plus = true` |
| **Fastway** | Stable | Fastway username + password | `fastway = true` |
| **Airtel Xstream** | Beta | Airtel number + OTP | `airtel = true` |
| **Tata Play** | Beta | Tata Play number + OTP | `tataplay = true` |
| **PlayboxTV** | Beta | PlayboxTV number + OTP | `playbox = true` |
| **VZY** | Beta | anonymous free tier, no account | `vzy = true` |

> [!NOTE]
> Beta providers are complete integrations whose remaining runtime contracts are still being validated. Their channels play when the provider account is entitled; a premium channel the account does not own is reported as such instead of failing silently.

---

## 🖥️ Supported Platforms

| Platform | Install method | Notes |
| :--- | :--- | :--- |
| **Linux x86_64 / aarch64** | one-line installer | systemd user service, optional autostart |
| **macOS (Apple silicon, Intel)** | one-line installer | launchd autostart |
| **Windows 10 / 11** | PowerShell installer | Windows service support |
| **Raspberry Pi and SBCs** | one-line installer | aarch64 and armv7 builds |
| **OpenWrt** | package in `openwrt/openstream` | procd service, `/etc/openstream` data dir |
| **Docker** | `docker/` compose file | mount a data volume for credentials |
| **Android (Termux)** | one-line installer | arm64 builds |
| **Android TV / phone** | app in `android/` | talks to a server you host |

---

## ⚡ Install in One Command

The installer detects your CPU and downloads the matching release. Nothing else is needed.

**Linux, macOS, Android/Termux**

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.ps1 | iex
```

**OpenWrt**

```sh
wget -qO- https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

**Docker**

```sh
docker run -d --name openstream --restart unless-stopped \
  -p 5001:5001 -p 5443:5443 \
  -v openstream-data:/app/.openstream-tv \
  ghcr.io/wpfyorg/openstream-tv:latest
```

Upgrade later with `ost update`, or by re-running the same installer.

---

## 🔓 First Run, Step by Step

### 📍 Step 1: Start the server

```sh
ost
```

It listens on `http://localhost:5001` by default and keeps its credentials in `~/.openstream-tv` (`/etc/openstream` on OpenWrt).

| Flag | Default | What it does |
| :--- | :--- | :--- |
| `--host` | `localhost` | Bind address. |
| `--port` / `-p` | `5001` | HTTP port. |
| `--public` | off | Listen on every interface so other devices on your network can reach it. |
| `--tls` | off | Also serve HTTPS on `--tls-port` (5443) with a generated certificate. |
| `--tunnel` | off | Start a Cloudflare tunnel so you can watch from outside your network. |

### 🔑 Step 2: Set the admin password

Open the keyed **Web setup** link the server prints on first launch, and set the password there. Opening `http://localhost:5001/` alone cannot set the first password. Prefer the CLI? Run `ost admin password`.

### 📲 Step 3: Sign in to a provider

The Settings page lists every provider with its own sign-in card — OTP for the mobile-number providers, username and password for Fastway, nothing at all for VZY. The same logins work from a terminal:

```sh
ost login            # JioTV, OTP
ost tv+ login        # JioTV+
ost fastway login    # Fastway
ost airtel login     # Airtel Xstream
ost tataplay login   # Tata Play
ost playbox login    # PlayboxTV
```

### 📺 Step 4: Watch

The home screen leads with tonight's programme, an On now strip and category rows (Entertainment, Infotainment, Kids, Lifestyle, Movies, Music, News, Sports), ordered by what you actually watch. Channels has search, category and language filters; the Guide has the full schedule; Watch has the player, quality picker and related channels.

### 🧾 Step 5: Add it to your TV

Settings shows the two URLs to paste into any IPTV app:

| File | Where to copy it from |
| :--- | :--- |
| Playlist | Settings → **Playlist**, shaped `http://<server>:5001/k/<key>/playlist.m3u` |
| Guide | Settings → **EPG**, shaped `http://<server>:5001/k/<key>/epg.xml.gz` |

Copy the complete URL with its `/k/<key>/` segment. The bare paths return HTTP 401 when authentication is on, because the key is what authorises the playlist.

> [!NOTE]
> Anyone holding the playlist URL can watch. Treat it like a password, and use `ost key rotate` if it leaks.

---

## 🧰 CLI Cheat Sheet

| Command | Purpose |
| :--- | :--- |
| `ost` | Start the server with default settings. |
| `ost serve --public --tls` | Reachable on the LAN over HTTP and HTTPS. |
| `ost provider use fastway` | Make a signed-in provider the active catalogue. |
| `ost provider use jiotv_plus` | Same for JioTV+. |
| `ost login reset` | Forget the JioTV session. |
| `ost fastway logout` | Disconnect Fastway. |
| `ost epg generate` / `epg on` / `epg off` | Build, enable or disable the XMLTV file. |
| `ost key show` / `key rotate` | Read or replace the access key behind the playlist URLs. |
| `ost admin password` | Change the web UI password. |
| `ost background start` / `background stop` | Run detached, with logs in the data directory. |
| `ost autostart` / `autostart remove` | Register or remove the system service. |
| `ost tunnel setup --domain example.com --subdomain tv` | Create a scoped Cloudflare tunnel. |
| `ost update` | Upgrade to the latest release. |

---

## ⚙️ Configuration

Configuration is a TOML file that the server looks for in the current working directory by default (`openstream.toml`, `config.toml` and a few legacy names); point it elsewhere with `--config PATH`. Every option also has an `OPENSTREAM_*` environment variable of the same name. Legacy `JIOTV_*` names are still read for migration; the OpenStream name wins when both are set.

| Variable | Purpose |
| :--- | :--- |
| `OPENSTREAM_TVPLUS` | Enable the JioTV+ source. |
| `OPENSTREAM_FASTWAY` | Enable Fastway. |
| `OPENSTREAM_AIRTEL` | Enable Airtel Xstream. |
| `OPENSTREAM_TATAPLAY` | Enable Tata Play. |
| `OPENSTREAM_PLAYBOX` | Enable PlayboxTV. |
| `OPENSTREAM_VZY` | Enable VZY. |
| `OPENSTREAM_EPG` | Serve the generated XMLTV guide. |
| `OPENSTREAM_DRM` | Enable DRM playback paths. |
| `OPENSTREAM_PROXY` | Route provider traffic through a proxy. |
| `OPENSTREAM_PUBLIC_ORIGIN` | Force the origin used in playlist links. |
| `OPENSTREAM_PATH_PREFIX` | Path of the data directory (credentials, EPG, logs). Empty uses `~/.openstream-tv` (`/etc/openstream` on OpenWrt). |
| `OPENSTREAM_DISABLE_AUTH` | Bypass the access gate: unkeyed playlists and protected API routes become reachable without a session. Only on a trusted network. |
| `OPENSTREAM_DEFAULT_CATEGORIES` / `OPENSTREAM_DEFAULT_LANGUAGES` | Seed the channel filters. |
| `OPENSTREAM_TITLE` | Rename the instance. |
| `OPENSTREAM_DEBUG` / `OPENSTREAM_LOG_TO_STDOUT` | Verbose logging. |
| `OPENSTREAM_LOG_PATH` | Log file location. |
| `OPENSTREAM_CUSTOM_CHANNELS_FILE` | Add your own channels on top of the provider catalogue. |

---

## 🔧 Troubleshooting

### 1. A premium channel shows "Subscription required"
- **Symptoms:** the player stops with a message from the provider, usually HTTP 403.
- **Cause:** the account does not include that channel, or the provider refused the session.
- **Fix:** confirm the channel plays on the provider's own app with the same account. If it does not, the server is reporting the truth; use `ost login` again to refresh a stale session.

### 2. The web page does not open
- **Symptoms:** `http://localhost:5001` refuses the connection.
- **Cause:** the server is bound to `localhost` and you opened it from another device, or the port is taken.
- **Fix:** start with `ost serve --public` to listen on the LAN, and check the port is free with `--port 8080`.

### 3. Playback stalls or the browser blocks it
- **Symptoms:** a spinner, a decoder error, or a silent black frame.
- **Cause:** insecure context (plain HTTP on a LAN IP), an unsupported codec, or a DRM key the browser cannot use.
- **Fix:** open the UI over HTTPS with `--tls`, choose a lower quality, and check the message the player reports — it names the failing stage.

### 4. The guide is empty or out of date
- **Symptoms:** the Guide page says no data, or IPTV clients show no programme names.
- **Cause:** the provider has no EPG for those channels, or the XMLTV file is disabled.
- **Fix:** run `ost epg generate`, then `ost epg on`, and check `OPENSTREAM_EPG`.

### 5. OTP never arrives
- **Symptoms:** the sign-in card waits, or the provider rejects the code.
- **Cause:** rate limiting, a number the provider does not recognise, or a stale session.
- **Fix:** wait a minute before retrying, confirm the number is the one on the subscription, then `ost login reset` and sign in again.

### 6. Channels from another provider are missing
- **Symptoms:** only one catalogue is listed after signing in to two providers.
- **Cause:** one provider is active, and only its catalogue is served.
- **Fix:** switch with `ost provider use <provider>`, or the Use button on the provider's card in Settings.

---

## 🔒 Golden Rules

1. **Keep the playlist URL private.** It is a bearer key: anyone with it can watch.
2. **Put it behind a tunnel or another protected exposure before opening it to the internet.** `ost serve --tls` adds an HTTPS listener but leaves the plain-HTTP one running, so TLS alone does not remove plaintext access. Use `ost serve --tunnel` (or a reverse proxy that only exposes the protected origin).
3. **Your provider credentials live in the data directory.** Back that up; never commit it.
4. **One active provider at a time.** Sign in to as many as you like — only the active catalogue is listed.
5. **Do not share accounts.** Every stream is yours, and providers notice concurrent use.
6. **Use beta providers knowingly.** They can change upstream without warning.

---

## 🌟 Credits and Community

- **Provider integrations:** JioTV, JioTV+, Fastway, Airtel Xstream, Tata Play, PlayboxTV and VZY, built against their public app flows.
- **Stack:** Rust, Axum and Tokio on the server; Svelte 5 and Vite in the UI; Shaka Player for playback.
- **Support:** join the [OpenStream TV Discord](https://discord.gg/Xn3bUsBCZU) for setup help and development chat.
- **Documentation:** the [help centre](https://wpfyorg.github.io/openstream-tv/) covers installation, configuration and every known playback failure.

Licensed under [Creative Commons Attribution 4.0](LICENSE).
