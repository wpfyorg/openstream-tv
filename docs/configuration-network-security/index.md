---
title: Network Access, Ports and HTTPS
description: Make the OpenStream TV web interface reachable and protect streams and account access.
category: Configuration
keywords: [host, port 5001, TLS 5443, public, LAN, authentication, security, DRM]
---

# Network Access, Ports and HTTPS

By default OpenStream TV listens on `localhost:5001` so only the server can connect. A TV or phone on your home network needs an address the TV can reach.

## Open the web interface on your network

1. Find your server's private LAN address (for example, `192.168.1.20`).
2. Start `ost serve --host 0.0.0.0 --port 5001` on a trusted network.
3. On another device on the same LAN, visit `http://192.168.1.20:5001/` with **your own server address**.
4. Sign in with your admin password. Ensure any local firewall permits only the devices you intend.

When the listener is bound to an IPv4 wildcard such as `0.0.0.0`, v1.5.0 also prints a **LAN playlist** URL for each detected private IPv4 address. The option `--public` sets the bind host to `[::]`, which is broader than “allow my TV” and can behave differently across operating systems. Avoid router port-forwarding for this service.

## HTTPS and protected browser playback

Run `ost serve --tls` to add HTTPS on port **5443** while retaining the HTTP IPTV listener on **5001**. Browsers may require a secure origin for DRM/Widevine or encrypted media APIs. The generated certificate is self-signed, so plan for proper trust on each viewing device or provide `--tls-cert` and `--tls-key`. Supplying certificate flags alone, without `--tls`, has no effect.

When you open **Settings → IPTV playlist**, the UI offers plain-HTTP playlist/EPG links when accessed over OpenStream TV's HTTPS listener because some IPTV players do not trust a self-signed certificate.

## Account and playlist security

- Keep `disable_auth = false` and `disable_url_encryption = false`.
- Store your server data directory privately. It contains sessions, login tokens and access keys.
- Never paste an `/k/<key>/...` playlist URL into public forums, screenshots or issue reports.
- **Make a new key** in Settings invalidates old IPTV links on all clients; update those clients afterwards.
- An admin password change signs other browser sessions out.
- Sign out of the web interface when using a shared browser.

The access key protects the key-scoped playlist and stream routes such as `/k/<key>/...`; it is not a blanket password for every HTTP path. Browser administration uses its own session/password checks, and the server also has intentionally public bootstrap/static/auth endpoints. Do not describe the access key as protecting the entire server.

Fastway has an additional upstream transport limitation in v1.5.0: its provider API and configured PallyCon license endpoint are plain `http://` URLs. Enabling HTTPS on your local OpenStream TV listener does not encrypt those upstream Fastway hops. See [Fastway account setup](../account-fastway/index.md) and [Fastway troubleshooting](../troubleshooting-fastway/index.md).

For config values, see [Configuration Reference](../configuration-reference/index.md); for lost access, see [troubleshooting](../troubleshooting/index.md).
