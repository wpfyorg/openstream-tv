---
title: Install on OpenWrt
description: Install the OpenStream TV package on supported OpenWrt routers and manage its procd service.
category: Installation
keywords: [OpenWrt, router, apk, opkg, procd, package]
---

# 📶 Install on OpenWrt

SSH to the router as `root`. The installer detects `apk` or `opkg`, chooses a supported package ABI, verifies the downloaded checksum and installs the package from the latest GitHub release.

| Supported package architecture | |
| :--- | :--- |
| `x86_64` | 64-bit Intel/AMD routers and VMs |
| `aarch64` Cortex-A53 | Many ARMv8 routers |
| `ARM` Cortex-A7 NEON | Older 32-bit ARM routers |

Other router architectures are not published. Set `OPENSTREAM_VERSION` to a release such as `1.5.0` to pin one.

```sh
wget -qO- https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

By default the installer enables and starts the service.

| Where to open it | URL |
| :--- | :--- |
| Browser interface | `https://<router-ip>:5443/` (accept the self-signed warning) |
| IPTV players | `http://<router-ip>:5001/` |

The normal package is `full`. For headless installations set `OPENSTREAM_VARIANT=slim` before running the script; the slim package has no web interface and needs terminal provider sign-in.

---

## 🧭 Guided Setup

Over SSH the installer also asks how to set the router up: build, account (JioTV, JioTV+, Fastway, PlayboxTV, VZY, Airtel Xstream or Tata Play, each beta option only when the package supports it), the address other devices will use, whether to expose it through Cloudflare, whether to turn on the programme guide, and the account sign-in.

> [!IMPORTANT]
> The installer stops the service before the terminal login and aborts if it cannot be stopped, so a running process cannot overwrite the new login.

Provider, tunnel and guide choices are written to UCI, so they survive a reboot and a package upgrade. The address shown during setup is a summary choice, not a UCI setting — configure `option host` manually if the service must bind to a particular address.

---

## ⚙️ UCI Options

| Option | Values | Purpose |
| :--- | :--- | :--- |
| `option tv_plus` | `0`, `1` | Enable the optional JioTV+ account (needs a JioFiber or AirFiber login). The earlier name `extras` is still read. |
| `option fastway` | `0`, `1` | Enable the optional Fastway Live TV provider. |
| `option playbox` | `0`, `1` | **Beta.** Enable PlayboxTV. |
| `option vzy` | `0`, `1` | **Beta.** Enable the VZY free tier; no account needed. |
| `option airtel` | `0`, `1` | **Beta.** Enable Airtel Xstream. |
| `option tataplay` | `0`, `1` | **Beta.** Enable Tata Play. |
| `option tunnel` | `0`, `1` | Start the optional Cloudflare tunnel with the service. |
| `option tunnel_token` | token | Named-tunnel token, passed as an environment variable rather than an argument. |
| `option epg` | `0`, `1` | Generate the XMLTV guide. Defaults to `0`. |

> [!NOTE]
> `option tunnel_token` is stored in `/etc/config/openstream`, readable by root only. `option tunnel 1` without a token starts a temporary `trycloudflare.com` address that changes every start; find the current one with `logread -e cloudflared`.

Piping the installer is supported: prompts read `/dev/tty`, and with no terminal the package installs exactly as before. Use `OPENSTREAM_ONBOARD=never` to skip the questions, or set `OPENSTREAM_PROVIDER`, `OPENSTREAM_PUBLIC_HOST`, `OPENSTREAM_TUNNEL`, `OPENSTREAM_TUNNEL_TOKEN` and `OPENSTREAM_EPG` to answer them up front.

---

## 🔁 Manage the Service

The service uses procd and starts at boot.

```sh
/etc/init.d/openstream status
/etc/init.d/openstream restart
logread -e openstream
```

Settings live in `/etc/config/openstream`; application state, including credentials and TLS material, is under `/etc/openstream`. After changing UCI settings run:

```sh
uci commit openstream
/etc/init.d/openstream restart
```

---

## ⚠️ Exposure Warning

> [!CAUTION]
> A Cloudflare tunnel publishes the router's server on a public hostname. The access key in the playlist path and the admin password are then the only protection, and the DRM license proxy becomes reachable from the internet too. Use a long admin password, prefer the temporary URL for short-lived access, and remove the tunnel when you are done. Without a tunnel, keep the service on your trusted LAN.

---

## 🔧 Update, Remove and Troubleshoot

| Task | Command |
| :--- | :--- |
| Update | Re-run the installer so the package manager updates the package. Do not use `ost update` on a package-managed install. |
| Remove | `apk del openstream` or `opkg remove openstream` (use `openstream-slim` if that was installed) |
| Check the service | `/etc/init.d/openstream status`, `logread -e openstream` |
| Unsupported architecture | Compare `uname -m` and `opkg print-architecture` (or `apk --print-arch`) with the package ABI. Do not force an incompatible package. |

Package removal preserves configuration and application data. See [OpenWrt maintenance](../maintenance-openwrt/index.md) for backup and removal details, and [OpenWrt problems](../troubleshooting-installation-startup/index.md) for startup failures.
