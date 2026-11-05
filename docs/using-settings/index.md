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

The **JioTV** card shows whether the provider account is logged in. The optional **JioTV+** card appears when JioTV+ is enabled and shows whether it is connected. When the server enables Fastway, a **Fastway** card accepts that provider's username and password. Use each card's login or disconnect controls to manage that provider account. See [JioTV account sign-in](../account-connect-jiotv/index.md), [JioTV+ channels](../account-extras/index.md), and [Fastway Live TV](../account-fastway/index.md).

You can [switch JioTV accounts](../account-switch-jiotv/index.md) by logging out and signing in with another number; an administrator can hide provider logout controls with a server configuration option.

## Playlist and guide links

Copy the **Playlist** URL into a compatible IPTV app. Copy **EPG** for the programme guide when EPG is enabled. On a protected server these URLs contain the `/k/<key>/` access-key path even if the input field is visually clipped. **Make a new key** invalidates old key-scoped playlist and stream links, so update every IPTV app afterward. When authentication is deliberately disabled, Settings uses unkeyed `/playlist.m3u` and `/epg.xml.gz` paths and key rotation is unavailable. See [IPTV playlists and XMLTV](../using-iptv-playlists/index.md).

## Change the admin password

1. Enter the current password.
2. Enter a new password of at least eight characters.
3. Select **Change password**.

**Expected result:** Settings confirms the change. Other browser sessions are signed out; the current browser stays signed in. See [Admin Sessions and Sign Out](../account-sessions/index.md).

## Server options

The web page does not expose every server setting. EPG generation, JioTV+ and Fastway availability, logout behavior, network binding and other runtime options are configured on the server. See [Configuration Reference](../configuration-reference/index.md), [CLI Commands](../configuration-cli/index.md), and [Network access and HTTPS](../configuration-network-security/index.md).
