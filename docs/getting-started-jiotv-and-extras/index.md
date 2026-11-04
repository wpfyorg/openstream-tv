---
title: JioTV, JioTV+ and Fastway: Which Account Do You Need?
description: Understand the three provider accounts, their prerequisites, and which OpenStream TV features use them.
category: Getting Started
keywords: [JioTV, JioTV+, Fastway, TV+, provider, subscription, channels, on demand]
---

# JioTV, JioTV+ and Fastway: Which Account Do You Need?

OpenStream TV can hold three provider accounts. Provider eligibility and subscriptions decide what each account can use:

| Account | Needs | Gives you |
| --- | --- | --- |
| **JioTV** | A JioTV account that can complete the mobile-number OTP flow | Live television and programme information |
| **JioTV+** | An eligible JioFiber or AirFiber login | JioTV+ channels plus supported on-demand catalogues |
| **Fastway** | Fastway credentials accepted by the provider | Fastway live TV only |

- **JioTV** is the default provider flow. OpenStream TV does not add a fixed-line-network gate to this login, but the provider still decides account and content availability.
- **JioTV+** is the optional second source. It is off until `tv_plus = true` (or `JIOTV_TVPLUS=true` / `OPENSTREAM_TVPLUS=true`) is set, and it needs its own sign-in with the JioFiber or AirFiber number, separate from JioTV.
- **Fastway** is the optional third source. Enable it with `fastway = true` (or `OPENSTREAM_FASTWAY=true`) and connect with credentials accepted by the provider. ACT eligibility, where offered, depends on the provider and plan. Fastway supplies live TV, not the on-demand catalogue. See [Connect Fastway Live TV](../account-fastway/index.md).

The installers list only the accounts the downloaded release supports and print this same hint above the account menu. The older name **Extras** may still appear in older guides and error messages. `ost extras login` remains a command alias; `ost tv+ login` is the current spelling.

## Check which source is active

1. Open **Settings** in the web interface.
2. Find the **JioTV** card. If it says **Not logged in**, use **Send OTP** to connect.
3. If the server has JioTV+ enabled, the **JioTV+** card shows whether that account is connected. A connected account adds content and a small active icon in the header.
4. Open **Channels**. Use **JioTV+ only** if you want to inspect the optional source. If JioTV+ is disconnected, its content might be unavailable.

![Settings showing a disconnected JioTV card and a connected JioTV+ card](../images/settings-jiotv-plus.png)

The screenshot is from a disposable v1.5.0 server with JioTV+ connected and JioTV disconnected. Fastway is disabled in this capture. The existence of a channel tile never guarantees permission to play it.

## Understand labels and restrictions

**Subscription required** means you may need a plan or additional rights. **Unavailable** can also mean your account is disconnected, the provider rejected playback, the programme is region restricted, or the browser does not support its DRM system. JioTV cannot unlock subscriptions or bypass DRM.

If you need more channels, check the subscription with the provider before changing server settings. See [Connect a JioTV account](../account-connect-jiotv/index.md), [Connect JioTV+](../account-extras/index.md) and [Why a channel will not play](../troubleshooting-channels-and-playback/index.md).
