---
title: OpenStream TV Help Centre
description: Install, connect and use OpenStream TV, then troubleshoot common problems.
category: Getting Started
keywords: [JioTV, help, manual, installation, live TV]
---

# 📺 OpenStream TV Help Centre

OpenStream TV is one Rust binary that serves live TV, catch-up, a seven-day guide and an IPTV playlist from your own machine, with a web player and settings panel included. Sign in to your provider once, then watch from a browser, a TV box or any IPTV app on your network.

**New here?** [Getting Started in 5 Minutes](../getting-started-in-5-minutes/index.md) sets the admin password, connects a provider and gets you to the channel list.

---

## 🚀 Pick Your Path

| I need to… | Go to |
| :--- | :--- |
| Get it running today | [Getting Started](../getting-started/index.md) |
| Install on Linux, macOS, Windows, OpenWrt, an SBC, Termux or Docker | [Installation](../installations/index.md) |
| Understand what a provider account is | [Accounts](../account/index.md) |
| Sign in to JioTV+ | [JioTV+ account](../account-extras/index.md) |
| Sign in to Fastway | [Fastway account](../account-fastway/index.md) |
| Watch, browse the guide or use the playlist | [Using OpenStream TV](../using/index.md) |
| Change ports, storage, logging or security | [Configuration](../configuration/index.md) |
| Fix a spinner, a 403 or an empty guide | [Troubleshooting](../troubleshooting/index.md) |
| Update, back up or read the logs | [Maintenance](../maintenance/index.md) |

---

## 🧩 What The Server Does

| Capability | Detail |
| :--- | :--- |
| **Live TV** | Provider channels proxied through your server behind a protected local URL. |
| **Catch-up** | Programmes the provider keeps for replay. |
| **Guide** | Seven-day EPG in the UI plus `epg.xml.gz` for IPTV clients. |
| **Playlist** | `playlist.m3u` for VLC, Kodi, TiviMate and OTT Navigator. |
| **Web UI** | Home rows, channel grid, guide, player, settings. |
| **Providers** | JioTV, JioTV+, Fastway stable; Airtel Xstream, Tata Play, PlayboxTV, VZY beta. |
| **Remote access** | Optional TLS or a Cloudflare tunnel. |

---

## 🔒 Safety First

> [!IMPORTANT]
> The playlist URL carries access to your server. Treat it like a password, keep it off screenshots and public issues, and rotate it with `ost key rotate` if it has been shared.

> [!NOTE]
> Channel availability, playback and DRM depend on your account, your browser and the provider. When a channel is not entitled, the server reports the provider's own refusal instead of hiding it.

---

For support and development discussion, join the [OpenStream TV Discord](https://discord.gg/Xn3bUsBCZU).

[Browse all articles](../SUMMARY.md) · [Documentation home](../README.md)
