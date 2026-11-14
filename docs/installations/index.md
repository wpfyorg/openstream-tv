---
title: Install OpenStream TV
description: Choose an installation method for Linux, macOS, Windows, OpenWrt, Termux, Docker or a source build.
category: Installation
keywords: [install, Linux, macOS, Windows, OpenWrt, Raspberry Pi, SBC, Termux, Docker]
---

# 📦 Install OpenStream TV

OpenStream TV runs as a server on a computer, a router or a container. Pick the guide for the machine that will host it.

| Platform | Guide | Notes |
| :--- | :--- | :--- |
| **Linux** | [Install on Linux](../installations-linux/index.md) | x86_64, 64-bit ARM, ARMv7, 32-bit x86 |
| **Raspberry Pi and SBCs** | [Install on an SBC](../installations-sbc/index.md) | aarch64, armv7l; no ARMv6 |
| **macOS** | [Install on macOS](../installations-macos/index.md) | Apple silicon and Intel |
| **Windows** | [Install on Windows](../installations-windows/index.md) | x64, x86, ARM64 |
| **OpenWrt** | [Install on OpenWrt](../installations-openwrt/index.md) | apk/opkg package with a procd service |
| **Android TV / phone** | [Android app](../installations-android-app/index.md) | runs the full server on the device |
| **Android with Termux** | [Install with Termux](../installations-termux/index.md) | terminal install on the phone |
| **Docker** | [Install with Docker](../installations-docker/index.md) | published GHCR image |
| **Docker Compose** | [Compose setup](../installations-docker-compose/index.md) | reusable compose.yaml |
| **Manual binary** | [Manual release binary](../installations-manual-binary/index.md) | verify and install by hand |
| **Source** | [Build from source](../installations-build-from-source/index.md) | Cargo build, optional OpenWrt packages |

---

## 🧩 Full or Slim

| Variant | Contains | Use it when |
| :--- | :--- | :--- |
| **`full`** | The server, the Svelte web interface and the IPTV files | You want the browser UI. This is the default. |
| **`slim`** | The server and IPTV files, no browser UI | The host has no browser, or you only need M3U and EPG. |

> [!NOTE]
> Every supplied installer verifies the download against the release's `SHA-256` manifest before installing.

---

## ▶️ After Installing

1. Complete [first launch](../getting-started-first-launch/index.md) to set the admin password.
2. Sign in to a provider in [Accounts](../account/index.md).
3. Keep the server on a trusted network, or follow [network access and HTTPS](../configuration-network-security/index.md) before exposing it.

[Back to the help centre](../help-centre/index.md).
