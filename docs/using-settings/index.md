---
title: Settings and Account Controls
description: Check account status, manage passwords and playlist access, and find server configuration guidance.
category: Using OpenStream TV
keywords: [settings, admin password, playlist, EPG, access key, accounts]
---

# Settings and Account Controls

Open **Settings** from the top navigation. The page shows IPTV URLs, account connections and the admin password form.

![Settings showing local playlist and EPG fields, a disconnected JioTV card and connected JioTV+](../images/settings-jiotv-plus.png)

This disposable v1.5.0 example has JioTV+ connected and JioTV disconnected. The displayed URLs point only to the local test server; your account status and private URLs will differ.

## Accounts

The enabled provider cards show whether each account is logged in or connected and whether it is the **Active** provider. JioTV+, Fastway, PlayboxTV and VZY are marked **Beta**. Only one provider can be active at a time: signing in makes that provider active, and connected cards offer a **Use …** control to switch. See [JioTV account sign-in](../account-connect-jiotv/index.md), [JioTV+ channels](../account-extras/index.md), [Fastway Live TV](../account-fastway/index.md), [PlayboxTV](../ott-provider-playbox/index.md) and [VZY](../ott-provider-vzy/index.md).

You can [switch JioTV accounts](../account-switch-jiotv/index.md) by logging out and signing in with another number; an administrator can hide provider logout controls with a server configuration option.

## Playlist and guide links

Copy the **Playlist** URL into a compatible IPTV app. Copy **EPG** for the programme guide when EPG is enabled. On a protected server these URLs contain the `/k/<key>/` access-key path even if the input field is visually clipped. **Make a new key** invalidates old key-scoped playlist and stream links, so update every IPTV app afterward. When authentication is deliberately disabled, Settings uses unkeyed `/playlist.m3u` and `/epg.xml.gz` paths and key rotation is unavailable. See [IPTV playlists and XMLTV](../using-iptv-playlists/index.md).

## Change the admin password

1. Enter the current password.
2. Enter a new password of at least eight characters.
3. Select **Change password**.

**Expected result:** Settings confirms the change. Other browser sessions are signed out; the current browser stays signed in. See [Admin Sessions and Sign Out](../account-sessions/index.md).

## Server options

The web page does not expose every server setting. EPG generation, JioTV+, Fastway, PlayboxTV, VZY and Tata Play availability, logout behavior, network binding and other runtime options are configured on the server. See [Configuration Reference](../configuration-reference/index.md), [CLI Commands](../configuration-cli/index.md), and [Network access and HTTPS](../configuration-network-security/index.md).
