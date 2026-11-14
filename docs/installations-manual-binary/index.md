---
title: Install OpenStream TV Manually from a Release Binary
description: Download, verify and install a matching full or slim OpenStream TV release without running an installer.
category: Installation
keywords: [manual installation, download, binary, release, SHA256SUMS, checksum, full, slim]
---

# 🧰 Install Manually from a Release Binary

Use this when you want to inspect the download and install the executable yourself. The [platform installers](../installations/index.md) automate the same steps.

---

## 🎯 Step 1: Pick the Right Download

Open the [latest releases](https://github.com/wpfyorg/openstream-tv/releases/latest) and choose the asset matching **both** your operating system and CPU. Asset names begin with `openstream-full-` or `openstream-slim-`; use **full** for the web interface and **slim** for IPTV clients without it.

| Platform | Variants published |
| :--- | :--- |
| Linux | x86_64, aarch64, ARMv7, i686 |
| macOS | Apple silicon, Intel |
| Windows | x64, x86, ARM64 |
| Termux | aarch64, ARMv7, x86_64 |

OpenWrt users should generally install the [package](../installations-openwrt/index.md) instead. To pin a release, download the same asset and `SHA256SUMS` from `https://github.com/wpfyorg/openstream-tv/releases/download/v<version>/`.

> [!NOTE]
> The examples below use `openstream-full-x86_64-unknown-linux-musl`, the typical 64-bit Intel/AMD Linux asset. Do not use that binary on Windows, macOS, ARM or Android.

---

## ✅ Step 2: Download and Verify (Linux Example)

```sh
mkdir -p "$HOME/openstream-download"
cd "$HOME/openstream-download"
asset=openstream-full-x86_64-unknown-linux-musl
base=https://github.com/wpfyorg/openstream-tv/releases/latest/download
curl -fLO "$base/$asset"
curl -fLO "$base/SHA256SUMS"
grep -F "  $asset" SHA256SUMS | sha256sum -c -
```

**Expected result:** the checksum check prints `OK`. Stop if the file is missing or the checksum fails — the checksum only proves your download matches the published manifest, so confirm you trust the release source too.

On macOS download the matching `apple-darwin` asset and verify with `shasum -a 256` instead of `sha256sum`. On Windows the [PowerShell installer](../installations-windows/index.md) handles the executable and checksum.

---

## 📥 Step 3: Install and Check the Command

```sh
mkdir -p "$HOME/.local/bin"
install -m 0755 "$asset" "$HOME/.local/bin/ost"
"$HOME/.local/bin/ost" --help
```

**Expected result:** the help output lists `serve`, `login`, `admin` and the other commands. If typing `ost` says **command not found**, add `$HOME/.local/bin` to `PATH` or keep using the full path.

---

## ▶️ Step 4: Set Up and Start

```sh
"$HOME/.local/bin/ost" admin password
"$HOME/.local/bin/ost" login otp
"$HOME/.local/bin/ost" serve --host 127.0.0.1 --tls
```

Choose an admin password and complete the OTP prompts for your own JioTV account, then open `https://localhost:5443/` on that machine and accept the self-signed certificate warning. The server also listens on HTTP port 5001 for IPTV clients. To reach it from another device, read [network configuration](../configuration-network-security/index.md) before changing the bind address.

A **slim** binary serves no browser interface, so nothing opens on 5443. Verify it with the playlist URL it prints at startup instead — `curl -s -o /dev/null -w '%{http_code}\n' <playlist URL>` should print `200` — and give that URL to your IPTV client.

The executable runs while its terminal is open; [background services](../maintenance-background-service/index.md) covers startup options. Account sign-in can also be done in [Settings](../account-connect-jiotv/index.md).

---

## 🔧 Step 5: Update, Remove, Troubleshoot

Stop the server before updating. Run `ost update` (or the full path to your executable), restart it and check the interface; see [update guidance](../maintenance-update/index.md).

To uninstall this example, stop OpenStream TV and remove **only** `$HOME/.local/bin/ost`. Data under `$HOME/.openstream-tv` is separate — keep it if you may reinstall, and read [safe uninstallation](../maintenance-update-uninstall/index.md) before erasing saved accounts.

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Unsupported architecture | Asset does not match the machine | Compare the asset name with `uname -m` |
| Permission denied | Missing execute bit | `chmod +x` the file or reinstall with `install -m 0755` |
| Server will not start | Port, TLS or configuration problem | See [installation and startup fixes](../troubleshooting-installation-startup/index.md) |
