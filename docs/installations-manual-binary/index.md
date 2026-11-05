---
title: Install OpenStream TV Manually from a Release Binary
description: Download, verify and install a matching full or slim OpenStream TV release without running an installer.
category: Installation
keywords: [manual installation, download, binary, release, SHA256SUMS, checksum, full, slim]
---

# Install OpenStream TV Manually from a Release Binary

Use this method if you want to inspect the download and install the executable yourself. The normal [platform installers](../installations/index.md) automate these steps.

## 1. Pick the correct download

Visit the [OpenStream TV GitHub releases](https://github.com/wpfyorg/openstream-tv/releases/latest) and choose the asset matching **both** your operating system and CPU. Asset names begin with `openstream-full-` or `openstream-slim-`. Use **full** for the web interface; **slim** runs without it for IPTV clients. The release workflow builds Linux (x86_64, aarch64, ARMv7, i686), macOS (Apple silicon and Intel), Windows (x64, x86, ARM64), and Termux (aarch64, ARMv7, x86_64) variants. OpenWrt users should generally install the appropriate [package](../installations-openwrt/index.md).

The example below follows the latest release. To pin a release, download the same asset and `SHA256SUMS` from `https://github.com/wpfyorg/openstream-tv/releases/download/v<version>/`, replacing `<version>` with the release you selected.

For example, a typical **64-bit Intel/AMD Linux** machine uses `openstream-full-x86_64-unknown-linux-musl`. The commands below are **for this Linux target only**. Do not use that binary on Windows, macOS, ARM or Android.

## 2. Download and verify the Linux example

Open a terminal with `curl` and `sha256sum` installed, and choose a fresh directory:

```sh
mkdir -p "$HOME/openstream-download"
cd "$HOME/openstream-download"
asset=openstream-full-x86_64-unknown-linux-musl
base=https://github.com/wpfyorg/openstream-tv/releases/latest/download
curl -fLO "$base/$asset"
curl -fLO "$base/SHA256SUMS"
grep -F "  $asset" SHA256SUMS | sha256sum -c -
```

**Expected result:** the checksum check prints `OK`. Stop if the file is missing or the checksum fails. The checksum checks that your download matches the release's published manifest; for higher assurance, independently confirm you trust the release source.

On macOS, download the matching `apple-darwin` asset and verify it using `shasum -a 256` instead of `sha256sum`. On Windows, the [PowerShell installer](../installations-windows/index.md) handles the executable and checksum for you.

## 3. Install and check the command

Still in the Linux download directory:

```sh
mkdir -p "$HOME/.local/bin"
install -m 0755 "$asset" "$HOME/.local/bin/openstream"
"$HOME/.local/bin/openstream" --help
```

**Expected result:** the command shows `serve`, `login`, `admin`, and other available commands. Add `$HOME/.local/bin` to your shell's `PATH` if typing `openstream` alone says **command not found**, or keep using the full executable path.

## 4. Set up and start OpenStream TV

```sh
"$HOME/.local/bin/openstream" admin password
"$HOME/.local/bin/openstream" login otp
"$HOME/.local/bin/openstream" serve --host 127.0.0.1 --tls
```

Choose an admin password and complete the OTP prompts for your own JioTV account. Open **https://localhost:5443/** on that machine. Your browser may warn about the locally generated self-signed certificate. The server also listens on HTTP port 5001 for IPTV clients. To reach the interface from another device, see [network configuration](../configuration-network-security/index.md) before changing the bind address.

The executable runs while its terminal is open. See [background services](../maintenance-background-service/index.md) for startup options. Account sign-in can also be completed in [Settings](../account-connect-jiotv/index.md).

## 5. Update, remove or troubleshoot

Stop the running server before updating. Run `openstream update` (or the full path to your executable), restart the server, and check that the interface opens. See [update guidance](../maintenance-update/index.md).

To uninstall this example, stop OpenStream TV and remove **only** `$HOME/.local/bin/openstream`. Your data under `$HOME/.openstream-tv` is separate; keep it if you plan to reinstall. To erase saved accounts too, first read [safe uninstallation](../maintenance-update-uninstall/index.md).

If the downloaded binary reports an unsupported architecture, recheck the asset name against `uname -m`. If execution is denied, check its execute permission. If the server cannot start, see [installation and startup fixes](../troubleshooting-installation-startup/index.md).
