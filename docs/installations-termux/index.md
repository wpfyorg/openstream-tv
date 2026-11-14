---
title: Install on Android with Termux
description: Run OpenStream TV as a server in Termux on supported Android devices.
category: Installation
keywords: [Android, Termux, phone, tablet, aarch64, ARMv7]
---

# 📱 Install on Android with Termux

Install Termux from a trusted distribution source and open its terminal. The installer detects Termux and picks a supported Android target (`aarch64`, ARMv7 or `x86_64`); it downloads the latest matching GitHub release unless `OPENSTREAM_VERSION` pins one, such as `1.5.0`.

```sh
pkg update
pkg install curl
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

The binary installs to Termux's `$PREFIX/bin`.

---

## 🧭 Step 1: Guided Setup

The installer runs the guided setup in the Termux terminal: build (`full` or `slim`), account (JioTV, JioTV+, Fastway, or Airtel Xstream beta when the release supports it), the address other devices will use, an optional Cloudflare tunnel, whether to turn on the programme guide, and the account sign-in. Arrow keys select and Enter confirms.

> [!IMPORTANT]
> Guided setup writes the provider settings and guide choice to `${XDG_CONFIG_HOME:-$HOME/.config}/openstream/installer.env`, but they are **not** loaded back into the Termux shell that ran the installer. Before a manual `ost serve` or before rerunning `ost autostart`, source that file — or reuse the installer's printed `Start it with:` command, which already sources it. When autostart succeeded the installer may print no start command at all, so source the file yourself for any manual step. Configuring autostart only affects future startup and never loads those settings into the current shell.

If you skipped the questions with `OPENSTREAM_ONBOARD=never`, set the matching flags in the same shell and keep it open for `serve`:

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

---

## ▶️ Step 2: Start the Server

```sh
ost admin password
ost serve --host 0.0.0.0 --tls
```

Keep Termux open while the foreground server runs.

| Where to open it | URL |
| :--- | :--- |
| On the phone | `https://localhost:5443/` |
| Another trusted device | `https://<phone-lan-address>:5443/` |

> [!NOTE]
> Android may restrict background processes or network access when the app is idle.

---

## 🔁 Step 3: Optional Autostart

`ost autostart` saves the current `OPENSTREAM_*` settings, including a named-tunnel token, in a private environment file, then adds a command to `bash.bashrc` that starts OpenStream TV in the background when a shell opens.

```sh
ost autostart
```

If you use a named tunnel, include its serve flags: `ost autostart --args "--tls --tunnel"`. Running it again with different flags refreshes the saved startup command and saves the current `OPENSTREAM_*` values privately.

Remove the startup entry with `ost autostart remove`.

---

## 🔧 Verify and Troubleshoot

| Check | Command or action |
| :--- | :--- |
| Binary installed | `command -v ost` and `ost --help` in Termux |
| Page opens | Browse the address while the server process runs |
| Install fails | Update Termux packages and confirm the device CPU is a supported target |
| Server stops when the screen turns off | Android background power restrictions; `autostart` alone cannot override them |

Update with `ost update`. To remove, run `ost autostart remove` if enabled, stop the server and delete its executable, keeping or privately backing up the data directory. See [How to Update OpenStream TV](../maintenance-update/index.md), [Uninstall OpenStream TV Safely](../maintenance-update-uninstall/index.md), [data, backups and logs](../configuration-storage-logs/index.md) and [startup errors](../troubleshooting-installation-startup/index.md).
