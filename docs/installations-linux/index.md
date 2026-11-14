---
title: Install on Linux
description: Install OpenStream TV on supported Linux computers and configure its first run.
category: Installation
keywords: [Linux, install, curl, binary, systemd, Ubuntu, Debian]
---

# 🐧 Install on Linux

The installer picks a release for x86_64, 64-bit ARM, ARMv7 or 32-bit x86 Linux and verifies it against `SHA256SUMS` before installing. ARMv6 and MIPS are not release targets.

---

## 📥 Step 1: Install the Binary

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

The default variant is `full`. To choose a variant or pin a version, download the script first and pass the settings:

```sh
curl -fsSLO https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh
OPENSTREAM_VARIANT=slim OPENSTREAM_VERSION=1.5.0 sh install.sh
```

| Where it lands | When |
| :--- | :--- |
| `/usr/local/bin` | Running as root with a writable directory |
| `$HOME/.local/bin` | Every other case |
| `OPENSTREAM_INSTALL_DIR` | Set it to choose another directory |

If `ost` is not found afterwards, add the install directory to `PATH` and open a new terminal.

---

## 🧭 Step 2: Guided Setup

In a terminal the installer walks through setup instead of only dropping a binary in place:

| Prompt | What it asks |
| :--- | :--- |
| **Build** | `full` (browser interface plus IPTV) or `slim` (IPTV only). Arrow keys and Enter. |
| **Account** | JioTV, JioTV+, Fastway, PlayboxTV (beta), VZY (beta), Airtel Xstream (beta) or Tata Play (beta). The installer describes JioTV, PlayboxTV, Airtel Xstream and Tata Play as mobile-number OTP flows, JioTV+ as needing an eligible JioFiber/AirFiber login, Fastway as needing credentials its provider accepts, and VZY as needing no account. |
| **Address** | The LAN address other devices will use, pre-filled with the detected one. Enter accepts it. |
| **Remote access** | LAN only, a temporary Cloudflare URL, or a permanent Cloudflare tunnel with a token. |
| **Programme guide** | Whether to turn the XMLTV guide on. It is off by default; Yes runs `ost epg on` and prints the guide URL in the summary. |
| **Credentials** | The sign-in for the chosen account, then an admin password for the `full` build. |

> [!NOTE]
> Piping the script into `sh` is supported: every prompt reads `/dev/tty`, never the `curl` pipe. With no terminal (Docker, CI, cron, redirected output) the flow is skipped and the installer behaves as a plain install.

> [!IMPORTANT]
> Sign-in is retried up to three times. If it still fails, the binary stays installed, no service is installed or started, and the installer exits non-zero with the command to finish sign-in later. One-time codes are never written to disk. The installer does not save a password itself, but the provider login it runs is kept in the data directory: JioTV stores tokens and Fastway stores its username and password so an expired token can be renewed. The tunnel token is stored in the service configuration when a service is installed.

---

## 🔑 Step 3: Sign In and Start

If you skipped the guided setup, sign in with the command for your provider:

```sh
# JioTV: mobile-number OTP
ost login otp

# JioTV+: eligible JioFiber or AirFiber login
ost tv+ login

# Fastway: prompts for the provider username and password
ost fastway login
```

Then protect and start the full web build:

```sh
ost admin password
ost serve --host 0.0.0.0 --tls
```

Enter the OTP when prompted. Fastway asks for its username and password. The admin password protects the web interface.

| Where to open it | URL |
| :--- | :--- |
| On the server | `https://localhost:5443/` (accept the self-signed warning) |
| From a trusted LAN device | `https://<server-ip>:5443/` |
| IPTV clients | The HTTP playlist on port `5001` |

The `slim` build has no browser interface.

For boot-time startup see [Background service](../maintenance-background-service/index.md); for updates and removal see [Update and uninstall](../maintenance-update-uninstall/index.md).

---

## ⚙️ Installer Options

| Variable | Values | Effect |
| :--- | :--- | :--- |
| `OPENSTREAM_VARIANT` | `full`, `slim` | Which build to install. |
| `OPENSTREAM_VERSION` | `latest` or a release version | Which release to install. |
| `OPENSTREAM_REPO` | `owner/repository` | Where releases come from. |
| `OPENSTREAM_INSTALL_DIR` | directory | Where the binary goes. |
| `OPENSTREAM_INSTALL_TLS` | `0` | Changes only the printed next-step instructions, never a running server's TLS setting. |
| `OPENSTREAM_ONBOARD` | `auto`, `always`, `never` | `auto` asks when a terminal exists; `never` keeps the plain install; `always` still degrades to the plain install without a terminal. |
| `OPENSTREAM_PROVIDER` | `jiotv`, `tv+`, `fastway`, `playbox`, `vzy`, `airtel`, `tataplay` | Account to configure; the beta four are only offered when the release supports them. The earlier value `extras` is still accepted for `tv+`. |
| `OPENSTREAM_PUBLIC_HOST` | address | LAN address shown in the summary instead of the detected one. |
| `OPENSTREAM_TUNNEL` | `none`, `temp`, `named` | LAN only, temporary Cloudflare URL, or named tunnel. |
| `OPENSTREAM_TUNNEL_TOKEN` | token | Token for the named tunnel. Never pass it as a command-line argument. |
| `OPENSTREAM_EPG` | `true`, `false` | Turns the guide on or off and skips the guide question. Leave unset to be asked. |

---

## 🔧 Verify and Troubleshoot

| Check | Command or action |
| :--- | :--- |
| Binary installed | `command -v ost` then `ost --help` |
| Page opens | Load the server address while the process runs |
| `ost` missing | Check `$HOME/.local/bin` or your chosen directory, then `PATH` |
| Port busy | `ost serve --port 5002`, or see [startup fixes](../troubleshooting-installation-startup/index.md) |

Update with `ost update` and restart your service. To uninstall, stop the process or service and remove the executable; keep the data directory if you may reinstall. See [safe removal](../maintenance-update-uninstall/index.md).
