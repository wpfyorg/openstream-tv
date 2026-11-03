---
title: Connect Fastway Live TV
description: Enable and connect the optional Fastway provider, and understand how its channels replace the JioTV catalogue.
category: Account Setup
keywords: [Fastway, provider, account, DASH, Widevine, PallyCon, channels]
---

# Connect Fastway Live TV

**Fastway** is an optional third-party live-TV source. It is separate from the JioTV account and from JioTV+ (formerly called Extras), and it is disabled by default. You need credentials that Fastway accepts for live-TV service; the server never grants or bypasses provider entitlements. The bundled installer labels the choice **Fastway/ACT**, but v1.5.0 does not prove eligibility for every ACT plan.

## Enable Fastway

Fastway is available in the standard build but stays off until the server configuration enables it:

```toml
# openstream.toml
fastway = true
```

Or set `OPENSTREAM_FASTWAY=true` for the process. Restart the server after changing the setting. When enabled, a **Fastway** card appears in **Settings**; see [Configuration Reference](../configuration-reference/index.md).

## Connect an account

Use the Fastway username and password for your subscription. This provider does not use the JioTV mobile-number OTP flow.

### Web interface

1. Open **Settings** and find the **Fastway** card.
2. Enter the Fastway username and password, then select **Connect**.
3. Check that the card says **Connected**. The header also shows a Fastway indicator while the provider is enabled and connected.

### Terminal

```bash
openstream fastway login
```

The command prompts for the username and password and saves the login in the server's data directory. Restart a running server after a CLI login so it picks up the new credentials. To remove the saved login:

```bash
openstream fastway logout
```

The password is kept locally because Fastway has no refresh-token endpoint; the server signs in again with the saved credentials when the provider rejects an expired access token. Do not share the data directory or its store file.

## Security limitation in v1.5.0

The shipped Fastway client uses a plain-HTTP provider API (`fwlivetvapp.netplus.co.in:8080`) and a plain-HTTP PallyCon license endpoint. That means the Fastway username/password exchange, provider tokens and DRM-license traffic are not protected by TLS on those upstream hops. Local `openstream serve --tls` protects the browser-to-OpenStream connection only; it does not change Fastway's upstream URLs.

Avoid using Fastway over networks you do not trust, protect the local data directory, and do not expose Fastway credentials or provider tokens in logs or support requests. This is a transport limitation of the v1.5.0 integration, not a DRM bypass feature.

## What changes while Fastway is connected

- Fastway channels replace the JioTV catalogue in **Channels**, the M3U playlist and the XMLTV guide. Local custom channels remain available.
- If both Fastway and JioTV+ are connected, Fastway takes priority. Disconnecting Fastway falls back to JioTV+, then to JioTV.
- Only channels the Fastway account is subscribed to are listed. A channel can still be unavailable if the provider rejects playback or the plan does not include it.
- Use **Fastway only** in **Channels** to narrow a mixed list, and look for the **Fastway** badge on channel tiles.
- Fastway playback is DASH. Protected channels use Widevine through the provider's PallyCon license service; the server proxies the license request but does not decrypt or bypass DRM. See [Protected playback troubleshooting](../troubleshooting-protected-playback/index.md).
- IPTV clients receive Fastway channels as `/live/mpd/...` entries with `inputstream.adaptive` and `manifest_type=mpd`.

Fastway does not use the JioTV+ unlock-code gate. It is controlled by the `fastway` configuration flag and the saved login.

## Disconnect

Select **Disconnect** on the Fastway card, or run `openstream fastway logout`. Disconnecting removes the saved Fastway login and returns the active catalogue to JioTV+ or JioTV. If the server sets `disable_logout = true`, the web interface hides provider disconnect controls; the CLI command still works.

## Troubleshooting

- **The card is missing:** confirm `fastway = true` or `OPENSTREAM_FASTWAY=true` is active for the running process, then restart.
- **Connect fails:** check the username and password with Fastway. The server reports the provider's message; it does not reveal whether the account has a subscription.
- **Channels are missing after connecting:** refresh the channel list. Only subscribed channels are shown, and the provider catalogue request may have failed temporarily.
- **Playback or license errors:** see [Fastway troubleshooting](../troubleshooting-fastway/index.md) first, then [Protected playback](../troubleshooting-protected-playback/index.md) for browser/Widevine checks.
