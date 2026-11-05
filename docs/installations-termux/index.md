---
title: Install on Android with Termux
description: Run OpenStream TV as a server in Termux on supported Android devices.
category: Installation
keywords: [Android, Termux, phone, tablet, aarch64, ARMv7]
---

# Install on Android with Termux

Install Termux from a trusted distribution source and open its terminal. The OpenStream TV installer detects Termux and chooses a supported Android target. It downloads the latest matching GitHub release by default; set `OPENSTREAM_VERSION` to a release such as `1.5.0` to pin it. Supported targets are aarch64, ARMv7, and x86_64.

```sh
pkg update
pkg install curl
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

The binary is installed in Termux's `$PREFIX/bin`. The installer also runs the guided setup in the Termux terminal: build (`full` or `slim`), account (JioTV, JioTV+, or Fastway when the release supports it), the address other devices will use, an optional Cloudflare tunnel, and the account sign-in. Arrow keys select and Enter confirms.

Guided setup writes the selected provider settings to `installer.env`, but those values are not automatically loaded back into the Termux shell that launched the installer. Before a later manual sign-in or `serve`, source the generated environment file (or reuse the exact `Start it with:` command printed by the installer, which already sources it). Configuring autostart only affects future startup and does not load those settings into the current shell. If you skipped the questions with `OPENSTREAM_ONBOARD=never`, set the matching provider flags in the same Termux shell before signing in and keep that shell open for `serve`:

```sh
# JioTV: mobile-number OTP
export OPENSTREAM_TVPLUS=false OPENSTREAM_FASTWAY=false
openstream login otp

# JioTV+: eligible JioFiber or AirFiber login
export OPENSTREAM_TVPLUS=true OPENSTREAM_FASTWAY=false
openstream tv+ login

# Fastway: prompts for the provider username and password
export OPENSTREAM_TVPLUS=false OPENSTREAM_FASTWAY=true
openstream fastway login
```

Then protect and start the full web build. If guided setup ran, source `installer.env` first or reuse the installer's printed start command so the selected provider settings are present:

```sh
openstream admin password
openstream serve --host 0.0.0.0 --tls
```

Keep Termux open while the foreground server runs. On the phone, use `https://localhost:5443/`; other devices on the same trusted network can use the phone's LAN address. Android may restrict background processes or network access when the app is idle. Termux autostart saves the current `OPENSTREAM_*` settings, including a named-tunnel token, in a private environment file, then adds a command to `bash.bashrc` to start OpenStream TV in the background when a shell opens:

```sh
openstream autostart
```

If you configure a named tunnel when setting up autostart manually, include its serve flags: `openstream autostart --args "--tls --tunnel"`. Autostart saves the current `OPENSTREAM_*` values privately and refreshes its startup command when you run it again with different flags.

Remove that startup entry with `openstream autostart remove`. Update with `openstream update`. See [How to Update OpenStream TV](../maintenance-update/index.md), [Uninstall OpenStream TV Safely](../maintenance-update-uninstall/index.md), and [data, backups and logs](../configuration-storage-logs/index.md).

## Verify and troubleshoot

Run `command -v openstream` and `openstream --help` in Termux, then open the browser page while the server process is running. If installation fails, check that Termux is up to date and that your device has a supported CPU architecture. If playback or the server stops when the screen turns off, investigate Android's background power restrictions; `autostart` alone cannot override them.

Update with `openstream update`; for removal, run `openstream autostart remove` if you enabled it, stop the server and delete its executable. Keep or privately back up the data directory. See [uninstall](../maintenance-update-uninstall/index.md) and [startup errors](../troubleshooting-installation-startup/index.md).
