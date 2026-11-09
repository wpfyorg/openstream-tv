---
title: Install on Linux
description: Install OpenStream TV on supported Linux computers and configure its first run.
category: Installation
keywords: [Linux, install, curl, binary, systemd, Ubuntu, Debian]
---

# Install on Linux

The installer selects a release for x86_64, 64-bit ARM, ARMv7, or 32-bit x86 Linux. It verifies the downloaded binary using `SHA256SUMS` before installing it. ARMv6 and MIPS are not release targets.

## Install

On a machine with `curl` or `wget`, run:

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

The default variant is `full`. To choose a variant or version, download the script first, then run it with the environment setting:

```sh
curl -fsSLO https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh
OPENSTREAM_VARIANT=slim OPENSTREAM_VERSION=1.5.0 sh install.sh
```

The installer uses `/usr/local/bin` when run as root and writable; otherwise it uses `$HOME/.local/bin`. Set `OPENSTREAM_INSTALL_DIR` to choose another directory. If the command is not found afterward, add the install directory to `PATH` and open a new terminal.

## Guided setup

Run the installer from a terminal and it walks through setup instead of only dropping a binary in place:

1. **Build** — `full` (browser interface plus IPTV) or `slim` (IPTV only). Use the arrow keys and press Enter.
2. **Account** — one of JioTV, JioTV+, Fastway, PlayboxTV (beta), VZY (beta), Airtel Xstream (beta) or Tata Play (beta). The installer describes JioTV, PlayboxTV, Airtel Xstream and Tata Play as mobile-number OTP flows, JioTV+ as needing an eligible JioFiber/AirFiber login, Fastway as needing credentials accepted by its provider, and VZY as needing no account. ACT eligibility, where offered, depends on the provider. Only providers supported by the downloaded release are listed.
3. **Address** — the LAN address other devices will use, pre-filled with the detected one. Press Enter to accept it.
4. **Remote access** — LAN only, a temporary Cloudflare URL, or a permanent Cloudflare tunnel with a token.
5. **Programme guide** — whether to turn on the XMLTV guide. It is off by default; answering Yes runs `ost epg on` and prints the guide URL in the summary.
6. **Credentials** — the sign-in for the account you chose, then an admin password for the `full` build.

Piping the script into `sh` still works: every prompt reads `/dev/tty`, never the `curl` pipe. When there is no terminal — Docker, CI, cron, redirected output — the flow is skipped and the installer behaves exactly as it did before.

Sign-in is retried up to three times. If it still fails, the binary stays installed, no service is installed or started, and the installer exits non-zero with the command to finish sign-in later. Passwords and one-time codes are never written to disk by the installer; the tunnel token is stored in the service configuration when a service is installed.

## Start and sign in

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

For JioTV, enter the one-time password when prompted. Fastway instead asks for its provider username and password. The admin password protects the web interface. On the same machine, open `https://localhost:5443/` and accept the self-signed certificate warning. From another trusted LAN device, use `https://<server-ip>:5443/`. IPTV applications use the HTTP playlist on port `5001`. The `slim` build has no browser interface.

To start at boot on a systemd Linux machine, see [Background service](../maintenance-background-service/index.md). To update or remove OpenStream TV, see [Update and uninstall](../maintenance-update-uninstall/index.md).

## Installer options

`OPENSTREAM_VARIANT` accepts `full` or `slim`; `OPENSTREAM_VERSION` accepts `latest` or a release version; `OPENSTREAM_REPO` selects the GitHub owner/repository; and `OPENSTREAM_INSTALL_DIR` sets the binary directory. `OPENSTREAM_INSTALL_TLS=0` changes the installer’s printed next-step instructions only. It does not change a running server's TLS setting.

The guided setup can be scripted with environment variables, which also skip the matching question:

| Setting | Values | Effect |
| --- | --- | --- |
| `OPENSTREAM_ONBOARD` | `auto`, `always`, `never` | `auto` asks when a terminal is available; `never` keeps the plain install; `always` cannot invent a terminal, so it still degrades to the plain install when there is none. |
| `OPENSTREAM_PROVIDER` | `jiotv`, `tv+`, `fastway`, `playbox`, `vzy`, `airtel`, `tataplay` | Account to configure; `playbox`, `vzy`, `airtel` and `tataplay` are beta (the previous value `extras` is still accepted for `tv+`). Ignored when the downloaded release does not support it. |
| `OPENSTREAM_PUBLIC_HOST` | address | LAN address shown in the summary instead of the detected one. |
| `OPENSTREAM_TUNNEL` | `none`, `temp`, `named` | LAN only, a temporary Cloudflare URL, or a named tunnel. |
| `OPENSTREAM_TUNNEL_TOKEN` | token | Token for the named tunnel. Never pass it as a command-line argument. |
| `OPENSTREAM_EPG` | `true`, `false` | Turns the programme guide on or off and skips the guide question. Leave it unset to be asked. |


## Verify and troubleshoot

Run `command -v ost` and `ost --help` to confirm the command is installed. Open the server address in a browser while the process is running. If the shell says the command is missing, check `$HOME/.local/bin` or your chosen install directory and `PATH`. If startup reports the port is busy, select an unused port with `ost serve --port 5002`; see [startup fixes](../troubleshooting-installation-startup/index.md).

To update, use `ost update` and restart your service. To uninstall, stop the process or service, then remove the installed executable; see [safe removal](../maintenance-update-uninstall/index.md). Preserve the data directory if you may reinstall.
