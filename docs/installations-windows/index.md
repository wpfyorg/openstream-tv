---
title: Install on Windows
description: Install OpenStream TV on Windows with PowerShell and configure first launch.
category: Installation
keywords: [Windows, PowerShell, install, Windows 11, Windows 10]
---

# 🪟 Install on Windows

The PowerShell installer supports x64, x86 and ARM64 Windows. It downloads the latest matching release, verifies it with the SHA-256 manifest, installs `ost.exe` under your local Programs directory and adds that directory to your user `PATH`.

---

## 📥 Step 1: Install

```powershell
irm https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.ps1 | iex
```

Close and reopen PowerShell if `ost` is not found. To pin a version, use `-Version <release>` or set `$env:OPENSTREAM_VERSION`.

---

## 🧭 Step 2: Guided Setup

In a PowerShell window the installer switches to a guided setup: build (`full` or `slim`), account (JioTV, JioTV+, Fastway, or Airtel Xstream beta when the release supports it), the address other devices will use, whether to expose the server through Cloudflare, whether to turn on the programme guide, and the account sign-in. Arrow keys (or `j`/`k`, or number keys) select; Enter confirms.

| Provider | What the wizard asks |
| :--- | :--- |
| **JioTV** | Mobile number and one-time password inside the wizard. |
| **Airtel Xstream** (beta) | Same mobile number and OTP flow. |
| **JioTV+** | Lists registered connections and asks which one to use. |
| **Fastway** | Username and password. |

> [!IMPORTANT]
> The OTP is masked and provider output is captured rather than printed over the screen. The installer never stores the mobile number or OTP. It saves the selected provider, any named-tunnel token and the guide choice in your Windows user environment. A failed sign-in leaves the binary installed and prints a retry command.

Skip the questions by setting the environment before running the script:

```powershell
$env:OPENSTREAM_VARIANT = 'slim'
$env:OPENSTREAM_PROVIDER = 'jiotv'
$env:OPENSTREAM_TUNNEL = 'none'
$env:OPENSTREAM_EPG = 'true'
```

`$env:OPENSTREAM_ONBOARD = 'never'` keeps the plain non-interactive install. Redirected output (CI, scheduled tasks) skips the flow automatically.

---

## ▶️ Step 3: Start the Server

The guided installer configures the account and admin password but does not keep the server running. Reuse the exact `Start it with:` command it printed — it includes the provider settings and, for a temporary or named tunnel, the required `--tunnel` option.

For LAN-only access the equivalent is:

```powershell
ost serve --host 0.0.0.0 --tls
```

If you skipped guided setup with `OPENSTREAM_ONBOARD=never`, set the provider flags in the same session before signing in and keep it open for `serve`:

```powershell
# JioTV: mobile-number OTP
$env:OPENSTREAM_TVPLUS = 'false'
$env:OPENSTREAM_FASTWAY = 'false'
ost login otp

# JioTV+: eligible JioFiber or AirFiber login
$env:OPENSTREAM_TVPLUS = 'true'
$env:OPENSTREAM_FASTWAY = 'false'
ost tv+ login

# Fastway: prompts for the provider username and password
$env:OPENSTREAM_TVPLUS = 'false'
$env:OPENSTREAM_FASTWAY = 'true'
ost fastway login
```

For the full web build, set the admin password before serving:

```powershell
ost admin password
```

| Where to open it | URL |
| :--- | :--- |
| On this PC | `https://localhost:5443/` (accept the self-signed warning) |
| Other trusted LAN devices | `https://<this-PC-ip>:5443/` (`ipconfig` shows the address) |
| IPTV players | The authenticated playlist URL printed by the installer; the guide URL prints below it when the guide is on |

The `slim` build has no browser interface.

> [!NOTE]
> The installer does not register a Windows service. Keep the terminal running while serving, or use a separately managed service wrapper only if you know how to configure and secure it.

---

## ⚙️ Installer Parameters

| Parameter | Values |
| :--- | :--- |
| `-Variant` | `full` or `slim` |
| `-Version` | `latest` or a release version |
| `-InstallDir` | Target directory |
| `-Repo` | `owner/repository` |

Installing a pinned release into a custom directory:

```powershell
irm https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.ps1 -OutFile install-openstream.ps1
.\install-openstream.ps1 -Version 1.5.0 -InstallDir "$env:LOCALAPPDATA\OpenStream TV"
```

---

## 🔧 Verify and Troubleshoot

| Check | Command or action |
| :--- | :--- |
| Binary installed | New PowerShell window: `Get-Command ost` then `ost --help` |
| Page opens | Browse the selected address while the server runs |
| Windows cannot find `ost` | Confirm the install folder is on your user `PATH`, then reopen PowerShell |
| Port cannot bind | See [startup errors](../troubleshooting-installation-startup/index.md) |

Update with `ost update` and restart the server; see [How to Update OpenStream TV](../maintenance-update/index.md). To uninstall, stop it, remove `ost.exe`, and remove an unneeded `PATH` entry — see [uninstall and saved data](../maintenance-update-uninstall/index.md).
