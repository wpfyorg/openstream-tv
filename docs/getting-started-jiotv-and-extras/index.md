---
title: JioTV, JioTV+ and Fastway: Which Account Do You Need?
description: Understand the three accounts, which network each one needs, and what content they unlock.
category: Getting Started
keywords: [JioTV, JioTV+, Fastway, TV+, provider, subscription, channels, on demand]
---

# JioTV, JioTV+ and Fastway: Which Account Do You Need?

The server can hold three accounts. Your network decides which one you have:

| Account | Needs | Gives you |
| --- | --- | --- |
| **JioTV** | Any network | Live television and programme information |
| **JioTV+** | A JioFiber or AirFiber login (Jio network) | JioTV+ channels, movies and shows |
| **Fastway** | A Fastway subscription, or an ACT connection | Live TV from Fastway |

- **JioTV** is the one to start with. It is the only account that works on any network, so it is enough for a new installation.
- **JioTV+** is the optional second source, marked **Beta**. It is off until `tv_plus = true` (or `JIOTV_TVPLUS=true`) is set, and it needs its own sign-in with the JioFiber or AirFiber number, separate from JioTV.
- **Fastway** is only offered when the installed release supports it.

The installers list only the accounts the downloaded release supports and print this same hint above the account menu. The older name **Extras** may still appear in older guides and error messages. `jiotv extras login` remains a command alias; `jiotv tv+ login` is the current spelling.

## Check which source is active

1. Open **Settings** in the web interface.
2. Each provider card shows whether it is logged in or connected and whether it is the **Active** provider.
3. Only one provider can be active at a time. Signing in to a provider makes it active; use **Use JioTV** or **Use JioTV+** on a connected card to switch. JioTV+ is marked **Beta**.
4. Open **Channels**. The list, IPTV playlist and programme guide show only the active provider's catalogue.

![Real Settings screen showing a disconnected JioTV account. JioTV+ is disabled in this example.](../images/settings-no-account.png)

The screenshot is from a clean installation with no account connected. Your screen may include more account controls when JioTV+ or Fastway is enabled. The existence of a channel tile never guarantees permission to play it.

## Understand labels and restrictions

**Subscription required** means you may need a plan or additional rights. **Unavailable** can also mean your account is disconnected, the provider rejected playback, the programme is region restricted, or the browser does not support its DRM system. JioTV cannot unlock subscriptions or bypass DRM.

If you need more channels, check the subscription with the provider before changing server settings. See [Connect a JioTV account](../account-connect-jiotv/index.md), [Connect JioTV+](../account-extras/index.md) and [Why a channel will not play](../troubleshooting-channels-and-playback/index.md).
