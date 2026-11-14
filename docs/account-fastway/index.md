---
title: Connect Fastway Live TV
description: Enable and connect the optional Fastway provider, and understand how its channels replace the JioTV catalogue.
category: Account Setup
keywords: [Fastway, provider, account, DASH, Widevine, PallyCon, channels]
---

# 📡 Connect Fastway Live TV

**Fastway** is an optional third-party live-TV source, separate from both the JioTV account and JioTV+ (formerly called Extras). It is disabled by default. You need credentials Fastway accepts for live-TV service — the server never grants or bypasses provider entitlements, and ACT eligibility, where offered, depends on the provider and plan.

---

## ⚙️ Step 1: Enable Fastway

Fastway ships in the standard build but stays off until the configuration enables it:

```toml
# openstream.toml
fastway = true
```

Or set `OPENSTREAM_FASTWAY=true` for the process, then restart the server. A **Fastway** card then appears in **Settings**. See [configuration reference](../configuration-reference/index.md).

---

## 🔑 Step 2: Connect an Account

Use the Fastway username and password for your subscription; this provider does not use the JioTV mobile-number OTP flow.

### Web interface

1. Open **Settings** and find the **Fastway** card.
2. Enter the Fastway username and password, then select **Connect**.
3. Check that the card says **Connected**. The header shows a Fastway indicator only while Fastway is the active provider.

### Terminal

```bash
ost fastway login
```

The command prompts for the username and password, saves the login in the server's data directory and records Fastway as the active provider for the next start, so you do not need **Use Fastway** afterwards.

> [!IMPORTANT]
> Stop a running server before a CLI login and start it again afterwards: a running server can save its in-memory session back over the new credentials. Remove the saved login with:

```bash
ost fastway logout
```

The password is kept locally because Fastway has no refresh-token endpoint; the server signs in again with the saved credentials when the provider rejects an expired access token. Do not share the data directory or its store file.

---

## ⚠️ Security Limitation in v1.5.0

> [!CAUTION]
> The shipped Fastway client uses a plain-HTTP provider API (`fwlivetvapp.netplus.co.in:8080`) and a plain-HTTP PallyCon license endpoint. The username/password exchange, provider tokens and DRM-license traffic are therefore not protected by TLS on those upstream hops.

Local `ost serve --tls` protects only the browser-to-OpenStream connection; it does not change Fastway's upstream URLs. Avoid Fastway on networks you do not trust, protect the local data directory, and keep credentials and provider tokens out of logs and support requests. This is a transport limitation of the v1.5.0 integration, not a DRM bypass feature.

---

## 🧭 What Changes While Fastway Is Active

| Area | Effect |
| :--- | :--- |
| Catalogue | Fastway channels replace the JioTV catalogue in **Channels**, the M3U playlist and the XMLTV guide. Local custom channels stay available. |
| Switching | Use **Use JioTV**, **Use JioTV+**, **Use PlayboxTV**, **Use VZY**, **Use Airtel Xstream** or **Use Tata Play** on another connected card, or run `ost provider use <id>` and restart; one provider is active at a time. |
| Disconnecting | Falls back to the next available provider: JioTV+, PlayboxTV, Airtel Xstream, Tata Play, then JioTV. The anonymous VZY free tier is used only when no JioTV account is connected. |
| Channels listed | Only channels the Fastway account subscribes to. A channel can still be unavailable if the provider rejects playback or the plan excludes it. |
| Playback | DASH. Protected channels use Widevine through the provider's PallyCon service; the server proxies the license request but does not decrypt or bypass DRM. See [protected playback](../troubleshooting-protected-playback/index.md). |
| IPTV clients | Fastway channels appear as `/live/mpd/...` entries with `inputstream.adaptive` and `manifest_type=mpd`. |

Fastway does not use the JioTV+ unlock-code gate; it is controlled by the `fastway` flag and the saved login.

---

## 🔌 Disconnect

Select **Disconnect** on the Fastway card, or stop the server, run `ost fastway logout` and start it again. The CLI removes the saved credentials, but a running server keeps its Fastway session in memory and can write it back, so stop the server first. With `disable_logout = true` the web interface hides provider disconnect controls; the CLI command still works.

---

## 🔧 Troubleshooting

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| The card is missing | `fastway = true` or `OPENSTREAM_FASTWAY=true` is not active for the running process | Enable it and restart |
| Connect fails | Credentials rejected by Fastway | Check the username and password. The server reports the provider's message and does not reveal whether the account has a subscription. |
| Channels missing after connecting | Catalogue request failed, or nothing is subscribed | Refresh the channel list; only subscribed channels are shown |
| Playback or license errors | DRM or environment problem | See [Fastway troubleshooting](../troubleshooting-fastway/index.md), then [protected playback](../troubleshooting-protected-playback/index.md) |
