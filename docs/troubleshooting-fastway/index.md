---
title: Fix Fastway Login and Playback Problems
description: Diagnose Fastway setup, credentials, catalogue loading, DASH playback and PallyCon license failures.
category: Troubleshooting
keywords: [Fastway, login, catalogue, DASH, Widevine, PallyCon, HTTP, playback]
---

# Fix Fastway login and playback problems

Fastway is optional and disabled by default. In v1.5.0 it becomes the active provider when it is both enabled and connected, taking priority over JioTV+ and then JioTV. Fastway supplies live TV only.

## Fastway card is missing

### Symptoms

Settings has JioTV and perhaps JioTV+, but no Fastway card.

### Possible causes

- `fastway` is still disabled.
- The environment change was made for a different service/process.
- The running binary predates Fastway support.

### Diagnose and fix

1. Confirm the running release supports `openstream fastway login` with `openstream --help`.
2. Enable `fastway = true` in the TOML file, or set `OPENSTREAM_FASTWAY=true` for the process/service.
3. Restart the running server or service so it reloads configuration.
4. Open Settings again.

**Verify:** the Fastway card appears. If it does not, collect the app version, startup command/service definition and a redacted startup-log excerpt.

## Login is rejected or immediately disconnects

### Symptoms

The Fastway card reports a login error, `openstream fastway login` fails, or the source does not stay connected.

### Possible causes

- Username or password is wrong.
- The provider does not accept the account for this service or plan.
- The provider API is unavailable.
- A saved token expired and automatic re-login with the stored password also failed.

### Diagnose and fix

1. Re-enter the Fastway username and password carefully. Fastway does not use the JioTV OTP flow.
2. Check the same account with the provider's supported service before changing OpenStream configuration.
3. Inspect only the redacted error around the failed request. Do not post the password, access token or data-store file.
4. If credentials changed, run `openstream fastway logout`, sign in again, then restart an already-running server after a CLI login.

**Verify:** Settings shows **Connected** and Channels loads the Fastway catalogue.

## Channels are empty, missing or replaced unexpectedly

### Symptoms

The channel list is empty, contains fewer channels than expected, or JioTV/JioTV+ channels disappear after Fastway connects.

### Possible causes

- Fastway catalogue loading failed.
- The account is entitled to only part of the catalogue.
- Fastway is active and therefore has provider priority by design.
- A channel is a local custom channel rather than a provider channel.

### Diagnose and fix

1. Check **Settings → Fastway** is connected.
2. Clear **Fastway only**, category, language, HD and search filters before judging the catalogue.
3. Remember that connected Fastway replaces the JioTV/JioTV+ provider catalogue; local custom channels remain.
4. Disconnect Fastway to verify that OpenStream falls back to JioTV+ and then JioTV.

**Verify:** the expected active provider's channels appear after a refresh. Do not treat a visible channel as proof that the provider will authorize playback.

## DASH stream or Widevine license fails

### Symptoms

The player stays on loading, DASH fails, the browser reports DRM/Widevine trouble, or a license request fails.

### Possible causes

- The provider rejected playback or the plan does not include the channel.
- The browser lacks working Widevine/Encrypted Media support or is using an insecure LAN origin.
- The Fastway CDN or PallyCon service is unavailable.
- The provider token expired and re-login failed.

### Diagnose and fix

1. Try another Fastway channel to separate a single-channel problem from an account/provider-wide failure.
2. For browser playback from another device, use OpenStream HTTPS on its configured HTTPS port (default `5443`) and a browser with supported Widevine. Plain `http://<LAN-IP>:<configured-http-port>` is not a secure browser origin; the default HTTP port is `5001`.
3. Check a short redacted server-log window for `fastway`, `DASH`, `license` or `PallyCon` errors.
4. Reconnect Fastway if the logs show the provider token was rejected and re-login could not recover it.
5. If only the IPTV client fails, confirm it supports DASH and the `inputstream.adaptive` metadata used by the Fastway M3U entries.

**Verify:** the selected channel resolves and plays in a supported client. One successful channel does not prove every Fastway channel works.

## Upstream HTTP security limitation

The v1.5.0 Fastway integration has a material transport limitation: its provider API is `http://fwlivetvapp.netplus.co.in:8080`, and its configured PallyCon license endpoint is also plain HTTP. Fastway credentials/tokens and license traffic therefore do not have TLS protection on those upstream hops. Running OpenStream TV with `--tls` secures your browser-to-server connection only.

Use Fastway only on a network path you trust, keep the OpenStream data directory private, and never include credentials, access tokens, playlist keys or raw store files in support material. This limitation is separate from whether Widevine itself authorizes a channel.

For general browser/DRM checks, see [HLS, DASH, DRM and browser playback](../troubleshooting-protected-playback/index.md). For provider setup and precedence, see [Connect Fastway Live TV](../account-fastway/index.md).
