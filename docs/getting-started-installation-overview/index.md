---
title: Choose the Right Installation Method
description: Decide between the standard installer, Docker, OpenWrt packages and manual installation.
category: Getting Started
keywords: [install, setup, Linux, Windows, macOS, OpenWrt, Termux, Docker, full, slim]
---

# Choose the Right Installation Method

OpenStream TV is a **server**. One device runs it, and a browser or IPTV application opens its interface or playlist over the network. For example, you can run the server on a Raspberry Pi and watch from a laptop on the same home network.

Choose your server device first:

| Your device | Recommended guide |
| --- | --- |
| Linux desktop or server | [Linux](../installations-linux/index.md) |
| Mac desktop or laptop | [macOS](../installations-macos/index.md) |
| Windows PC | [Windows](../installations-windows/index.md) |
| OpenWrt router | [OpenWrt](../installations-openwrt/index.md) |
| Raspberry Pi or Linux mini-computer | [Raspberry Pi and SBC](../installations-sbc/index.md) |
| Android with a terminal app | [Termux](../installations-termux/index.md) |
| Docker host | [Docker and Compose](../installations-docker/index.md) |
| Existing downloaded executable | [Manual binary](../installations-manual-binary/index.md) |
| Developer with a Rust toolchain | [Build from source](../installations-build-from-source/index.md) |

The **full** build serves the web interface. A **slim** build has no web interface and is for IPTV clients. JioTV+ is optional, stays off until it is enabled, and needs a separate account; select the full build if this is your first installation.

## What each installation method installs

| Method | Version behavior | Command you run afterward |
| --- | --- | --- |
| Linux, macOS, or Termux installer | Downloads the latest GitHub release by default. Set `OPENSTREAM_VERSION` to pin a release. | `ost` |
| Windows PowerShell installer | Downloads the latest GitHub release by default. Use `-Version` or `OPENSTREAM_VERSION` to pin a release. | `ost` (`ost.exe` on disk) |
| OpenWrt installer | Installs the matching `openstream` or `openstream-slim` package from the latest GitHub release by default. `OPENSTREAM_VERSION` can select a release. | `/usr/bin/ost` |
| Docker or Compose | `ghcr.io/wpfyorg/openstream-tv:latest` tracks the current full image; `:slim` tracks the slim image. Use a versioned tag when you need a fixed release. | `/usr/local/bin/ost` inside the container |
| Manual binary | Uses the exact GitHub release asset you download and verify. Install or rename it as `ost`. | `ost` |
| Build from source | Builds the revision currently checked out. That revision can be newer than the latest release. | `target/release/ost` before installation |

If you need reproducible deployments, pin a release or container tag instead of following `latest`.

The Linux, macOS, Windows and OpenWrt installers run a guided setup when they have a terminal: choose the build, choose the account (JioTV, JioTV+, or Fastway when the release supports it), confirm the LAN address, decide whether to use a Cloudflare tunnel, and sign in. Docker and unattended installs skip it and use the documented environment variables instead.

Which provider you can use depends on the account: JioTV uses its mobile-number OTP flow, JioTV+ needs an eligible JioFiber or AirFiber login, and Fastway needs credentials accepted by the provider. ACT eligibility, where offered, depends on the provider.

After installing, use [Getting Started in 5 Minutes](../getting-started-in-5-minutes/index.md). If your browser runs on another device, read [Access OpenStream TV on your home network](../configuration-network-security/index.md).
