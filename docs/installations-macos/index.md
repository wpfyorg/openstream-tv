---
title: Install on macOS
description: Install and run OpenStream TV on Intel or Apple silicon Macs.
category: Installation
keywords: [macOS, Mac, Apple silicon, Intel, install, terminal]
---

# Install on macOS

The installer supports Intel (`x86_64`) and Apple silicon (`arm64`) Macs. It downloads the latest matching GitHub release by default and checks its SHA-256 checksum. Set `OPENSTREAM_VERSION` to a release such as `1.5.0` when you need to pin the installed version.

## Install

Open Terminal and run:

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

Without elevated privileges, the binary goes in `$HOME/.local/bin`. Add that directory to your shell's `PATH` if needed. For example, with zsh:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## Guided setup

In a terminal, the installer also asks how the server should be set up: build (`full` or `slim`), account (JioTV, JioTV+, or Fastway when that release supports it), the LAN address other devices will use, whether to expose it through Cloudflare, and the account sign-in. Arrow keys select, Enter confirms, and Ctrl-C cancels without leaving the terminal in a broken state.

The same settings can be supplied up front to skip the questions:

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | OPENSTREAM_VARIANT=slim OPENSTREAM_PROVIDER=jiotv OPENSTREAM_TUNNEL=none sh
```

`OPENSTREAM_ONBOARD=never` turns the flow off. Piping the script into `sh` is fine because prompts read `/dev/tty`, not the pipe; with no terminal the installer behaves exactly as it does without this option.

## First run

The guided setup connects the account for you. If you skipped it or the account sign-in did not complete, use the command for your provider:

```sh
# JioTV: mobile-number OTP
openstream login otp

# JioTV+: eligible JioFiber or AirFiber login
openstream tv+ login

# Fastway: prompts for the provider username and password
openstream fastway login
```

Then protect and start the full web build:

```sh
openstream admin password
openstream serve --host 127.0.0.1 --tls
```

Open `https://localhost:5443/` and accept the self-signed certificate warning. To access the server from other devices on your home network, bind to all interfaces with `--host 0.0.0.0` and use your Mac's LAN address. Keep access limited to a trusted network. The `slim` build has no browser interface.

For boot-time service setup, see [Background service](../maintenance-background-service/index.md). For updates, see [How to Update OpenStream TV](../maintenance-update/index.md); for removal, see [Uninstall OpenStream TV Safely](../maintenance-update-uninstall/index.md).

## Verify and troubleshoot

Run `command -v openstream` and `openstream --help` in Terminal. Confirm the browser opens the address selected when starting `serve`. If `openstream` is missing, confirm your install location is in `PATH`; restart Terminal after updating the shell profile. If a port is occupied, follow [startup fixes](../troubleshooting-installation-startup/index.md).

Update with `openstream update`, restart OpenStream TV, and check the interface. For removal, stop OpenStream TV and delete the binary from your install directory. Keep your data directory unless you want to erase your saved account state; see [uninstall](../maintenance-update-uninstall/index.md).
