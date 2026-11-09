---
title: IPTV, EPG and Streaming Settings
description: Choose guide generation, media proxy and DRM settings safely.
category: Configuration
keywords: [EPG, XMLTV, M3U, proxy, HLS, DASH, DRM, secureurl]
---

# IPTV, EPG and Streaming Settings

Most viewers can leave these settings alone. This reference is for setting up an IPTV app or troubleshooting stream delivery. The values below come from the current Rust configuration schema.

## Programme guide (EPG)

By default `epg = false`, so the XMLTV guide is not generated automatically. To enable it, create or edit a TOML configuration file:

```toml
epg = true
```

You can also start OpenStream TV with the environment variable `OPENSTREAM_EPG=true`. The guided installer asks whether to turn the guide on and, when you accept, runs `ost epg on` for you. Declining clears an override that an earlier enable stored, so the guide stays off. Restart the server after changing the setting. The EPG endpoint is `/epg.xml.gz`; in **Settings**, copy the **EPG** URL shown for your installation. The command `ost epg generate` creates a guide, and `ost epg delete` removes the generated cache. The programme schedule depends on account access and upstream data.

For headless and slim installs, `ost epg on` stores a persistent EPG override in the OpenStream TV data directory; `ost epg off` reverses it. The stored override takes precedence over the config-file or environment default. On managed OpenWrt/systemd installs the command restarts OpenStream TV only when the service is already running; a stopped service stays stopped and uses the setting on its next start. Unmanaged foreground/background servers still need a manual restart. `ost epg on` prints the EPG URL after applying the change. On non-OpenWrt installs, replace `<server-ip>` with the device address and, if shown, `<port>` with the server port. When EPG is enabled, server startup also prints the URL directly below the M3U playlist URL.

## M3U playlist and access key

The full web interface shows playlist and EPG links in **Settings**. A protected installation uses `/k/<key>/playlist.m3u` and `/k/<key>/epg.xml.gz`. If authentication is deliberately disabled, the paths are `/playlist.m3u` and `/epg.xml.gz` and access-key rotation is unavailable. Do not replace the full link with a guessed path; use **Copy** for the link supplied by your server.

![Settings shows the M3U and EPG locations and warns that the playlist must stay private](../images/settings-jiotv-plus.png)

If you choose **Make a new key**, clients with old links stop working. Copy the new URL to every IPTV app. The key belongs only to your local server; it does not change your provider subscription.

## Playback options

| Setting | Default | When to change |
| --- | --- | --- |
| `drm` / `OPENSTREAM_DRM` | `true` | Keep enabled for supported protected DASH playback. It does not provide Widevine or a subscription. |
| `disable_ts_handler` / `OPENSTREAM_DISABLE_TS_HANDLER` | `false` | Advanced HLS testing when troubleshooting a transport/segment proxy problem. |
| `disable_url_encryption` / `OPENSTREAM_DISABLE_URL_ENCRYPTION` | `false` | Leave off. Enabling this weakens protection of upstream URL parameters. |
| `proxy` / `OPENSTREAM_PROXY` | empty | Specify an outbound proxy URL only if your network needs one. |

Protected browser playback may need HTTPS and a working Widevine or other supported DRM environment. See [Network and HTTPS](../configuration-network-security/index.md) and [DRM playback errors](../troubleshooting-protected-playback/index.md). For a complete list of settings, see the [configuration reference](../configuration-reference/index.md).
