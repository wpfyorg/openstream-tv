---
title: Network Access, Ports and HTTPS
description: Make the JioTV web interface reachable and protect streams and account access.
category: Configuration
keywords: [host, port 5001, TLS 5443, public, LAN, authentication, security, DRM]
---

# Network Access, Ports and HTTPS

By default JioTV listens on `localhost:5001` so only the server can connect. A TV or phone on your home network needs an address the TV can reach.

## Open the web interface on your network

1. Find your server's private LAN address (for example, `192.168.1.20`).
2. Start `jiotv serve --host 0.0.0.0 --port 5001` on a trusted network.
3. On another device on the same LAN, visit `http://192.168.1.20:5001/` with **your own server address**.
4. Sign in with your admin password. Ensure any local firewall permits only the devices you intend.

The option `--public` binds to all IPv6 interfaces; its meaning is broader than “allow my TV.” Avoid router port-forwarding for this service.

## HTTPS and protected browser playback

Run `jiotv serve --tls` to add HTTPS on port **5443** while retaining the HTTP IPTV listener on **5001**. Browsers may require a secure origin for DRM/Widevine or encrypted media APIs. The generated certificate is self-signed, so plan for proper trust on each viewing device or provide `--tls-cert` and `--tls-key`. Supplying certificate flags alone, without `--tls`, has no effect.

When you open **Settings → IPTV playlist**, the UI offers plain-HTTP playlist/EPG links when accessed over JioTV's HTTPS listener because some IPTV players do not trust a self-signed certificate.

## Cloudflare HTTPS tunnel or reverse proxy

If Cloudflare or another trusted proxy accepts HTTPS and forwards HTTP to port 5001, set the public hostname explicitly:

```sh
JIOTV_PUBLIC_ORIGIN=https://tv.example.com jiotv serve --host 127.0.0.1 --port 5001
```

Only requests whose `Host` matches `tv.example.com` then receive HTTPS links in the playlist, EPG, live-stream and DRM license URLs. Requests from LAN clients (for example `192.168.1.20:5001`) continue to receive HTTP links; the direct TLS listener also retains its own HTTPS origin. The server ignores client-supplied `Forwarded` and `X-Forwarded-*` headers for URL generation. The proxy must pass the public hostname in `Host` and protect access to the local HTTP listener.

Run `jiotv tunnel setup --domain example.com --subdomain tv --port 5001` with a scoped `CLOUDFLARE_API_TOKEN` to create a managed tunnel and save its connector token and public origin privately. Grant the token **Zone Read**, **DNS Edit**, and **Cloudflare Tunnel Edit** for the intended zone/account. Then start `jiotv serve --host 127.0.0.1 --port 5001 --tunnel --tunnel-saved` (use the port you supplied during setup). Quick tunnels started with `--tunnel --tunnel-quick` discover their temporary HTTPS origin automatically. For a manually created named tunnel using `--tunnel-token` or `JIOTV_TUNNEL_TOKEN`, set `JIOTV_PUBLIC_ORIGIN` yourself. A configured origin takes precedence over the discovered tunnel hostname.

## Account and playlist security

- Keep `disable_auth = false` and `disable_url_encryption = false`.
- Store your server data directory privately. It contains sessions, login tokens and access keys.
- Never paste an `/k/<key>/...` playlist URL into public forums, screenshots or issue reports.
- **Make a new key** in Settings invalidates old IPTV links on all clients; update those clients afterwards.
- An admin password change signs other browser sessions out.
- Sign out of the web interface when using a shared browser.

For config values, see [Configuration Reference](../configuration-reference/index.md); for lost access, see [troubleshooting](../troubleshooting/index.md).
