# 📚 OpenStream TV Documentation

Everything you need to install OpenStream TV, connect a provider, watch on any screen, and fix playback when a channel refuses to start.

Start with the **[Help Centre](help-centre/index.md)** if you are new, or jump straight to a section below.

---

## 🧭 Where To Go

| You want to… | Start here |
| :--- | :--- |
| Install it for the first time | [Getting Started in 5 Minutes](getting-started-in-5-minutes/index.md) |
| Pick the right platform | [Installation](installations/index.md) → Linux, macOS, Windows, OpenWrt, SBC, Termux, Docker |
| Sign in to a provider | [Accounts](account/index.md), [JioTV+](account-extras/index.md), [Fastway](account-fastway/index.md) |
| Watch, browse or use the guide | [Using OpenStream TV](using/index.md) |
| Add it to VLC, Kodi or TiviMate | [IPTV playlist and EPG](configuration-iptv-epg-streaming/index.md) |
| Change server options | [Configuration reference](configuration-reference/index.md), [CLI](configuration-cli/index.md) |
| Fix a channel that will not play | [Troubleshooting](troubleshooting/index.md) |
| Update, back up or read logs | [Maintenance](maintenance/index.md) |

---

## 📺 Providers At A Glance

| Provider | Status | Sign in with |
| :--- | :--- | :--- |
| **JioTV** | Stable | mobile number + OTP |
| **JioTV+** | Stable | JioFiber / AirFiber number + OTP |
| **Fastway** | Stable | username + password |
| **Airtel Xstream** | Beta | mobile number + OTP |
| **Tata Play** | Beta | mobile number + OTP |
| **PlayboxTV** | Beta | mobile number + OTP |
| **VZY** | Beta | free tier, no account |

You can sign in to several providers; the **active** one supplies the channel list. Switch it in Settings or with `ost provider use <provider>`.

---

## 🔒 Before You Share Anything

> [!IMPORTANT]
> Your playlist link (`/playlist.m3u`) is a bearer key — anyone who has it can watch. Treat it like a password, rotate it with `ost key rotate` if it leaks, and never post it, your OTP codes, session cookies or data directory in an issue.

---

## 🌟 Elsewhere

- **Source and releases:** [github.com/wpfyorg/openstream-tv](https://github.com/wpfyorg/openstream-tv)
- **Community:** [OpenStream TV Discord](https://discord.gg/Xn3bUsBCZU)
- **All articles:** [browse the full list](SUMMARY.md)
