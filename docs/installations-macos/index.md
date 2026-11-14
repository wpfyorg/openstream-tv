---
title: Install on macOS
description: Install and run OpenStream TV on Intel or Apple silicon Macs.
category: Installation
keywords: [macOS, Mac, Apple silicon, Intel, install, terminal]
---

# 🍎 Install on macOS

The installer supports Intel (`x86_64`) and Apple silicon (`arm64`) Macs. It downloads the latest matching GitHub release and checks its SHA-256 checksum. Pin a version with `OPENSTREAM_VERSION` when you need a specific release.

---

## 📥 Step 1: Install

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

Without elevated privileges the binary lands in `$HOME/.local/bin`. Add it to your shell `PATH` if needed — for zsh:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

---

## 🧭 Step 2: Guided Setup

In a terminal the installer also asks how the server should be set up: build (`full` or `slim`), account (JioTV, JioTV+, Fastway, PlayboxTV, VZY, Airtel Xstream or Tata Play, with beta options only when the release supports them), the LAN address other devices will use, whether to expose it through Cloudflare, whether to turn on the programme guide, and the account sign-in.

Arrow keys select, Enter confirms, and Ctrl-C cancels without leaving the terminal in a broken state.

Skip the questions up front:

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | OPENSTREAM_VARIANT=slim OPENSTREAM_PROVIDER=jiotv OPENSTREAM_TUNNEL=none sh
```

| Setting | Effect |
| :--- | :--- |
| `OPENSTREAM_ONBOARD=never` | Turns the guided flow off entirely. |
| `OPENSTREAM_EPG=true` | Turns the programme guide on without the guide question (default answer is off). |

> [!NOTE]
> Piping into `sh` is fine because prompts read `/dev/tty`, not the pipe. With no terminal the installer behaves exactly as it does without this option.

---

## 🔑 Step 3: First Run

The guided setup connects the account for you. If it printed a retry command that sources `installer.env`, use that exact command so the selected provider settings are loaded.

If you skipped guided setup, set the matching provider flags in the same shell and keep it open for `serve`:

```sh
# JioTV: mobile-number OTP
export OPENSTREAM_TVPLUS=false OPENSTREAM_FASTWAY=false
ost login otp

# JioTV+: eligible JioFiber or AirFiber login
export OPENSTREAM_TVPLUS=true OPENSTREAM_FASTWAY=false
ost tv+ login

# Fastway: prompts for the provider username and password
export OPENSTREAM_TVPLUS=false OPENSTREAM_FASTWAY=true
ost fastway login
```

Then protect and start the full web build:

```sh
ost admin password
ost serve --host 127.0.0.1 --tls
```

Open `https://localhost:5443/` and accept the self-signed certificate warning. For other devices on your home network, bind with `--host 0.0.0.0` and use your Mac's LAN address — and keep access to a trusted network. The `slim` build has no browser interface.

For boot-time service setup see [Background service](../maintenance-background-service/index.md); for updates [How to Update OpenStream TV](../maintenance-update/index.md); for removal [Uninstall OpenStream TV Safely](../maintenance-update-uninstall/index.md).

---

## 🔧 Verify and Troubleshoot

| Check | Command or action |
| :--- | :--- |
| Binary installed | `command -v ost` and `ost --help` in Terminal |
| Page opens | Browse the address chosen when starting `serve` |
| `ost` missing | Confirm your install directory is in `PATH`, then restart Terminal |
| Port occupied | See [startup fixes](../troubleshooting-installation-startup/index.md) |

Update with `ost update` and restart. To remove it, stop the server and delete the binary from your install directory; keep the data directory unless you want to erase saved account state. See [uninstall](../maintenance-update-uninstall/index.md).
