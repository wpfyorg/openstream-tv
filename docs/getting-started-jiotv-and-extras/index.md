---
title: JioTV, JioTV+ and the Optional Providers
description: Understand each provider account, its prerequisites, and which OpenStream TV features use it.
category: Getting Started
keywords: [JioTV, JioTV+, Fastway, PlayboxTV, VZY, Tata Play, TV+, provider, subscription, channels, on demand]
---

# JioTV, JioTV+ and the optional providers

OpenStream TV can hold several provider accounts, but only one provider catalogue is exposed at a time. Provider eligibility and subscriptions decide what each account can use:

| Provider | Needs | Gives you |
| --- | --- | --- |
| **JioTV** | A JioTV account that can complete the mobile-number OTP flow | Live television and programme information |
| **JioTV+** | An eligible JioFiber or AirFiber login | JioTV+ channels plus supported on-demand catalogues |
| **Fastway** | Fastway credentials accepted by the provider | Fastway live TV only |
| **PlayboxTV** (Beta) | A PlayboxTV mobile number that can complete the OTP flow | PlayboxTV live channels |
| **VZY** (Beta) | No account | VZY's free-tier live channels |
| **Tata Play** (Beta, diagnostic) | A Tata Play Smart account | Diagnostic login/EPG tools only; not added to the channel list or playback yet |

**JioTV, JioTV+ and Fastway** are stable providers. **PlayboxTV, VZY and Tata Play** are beta.

- **JioTV** is the default provider flow. OpenStream TV does not add a fixed-line-network gate to this login, but the provider still decides account and content availability.
- **JioTV+** is the optional second source. It is off until `tv_plus = true` (or `JIOTV_TVPLUS=true` / `OPENSTREAM_TVPLUS=true`) is set, and it needs its own sign-in with the JioFiber or AirFiber number, separate from JioTV.
- **Fastway** is enabled with `fastway = true` (or `OPENSTREAM_FASTWAY=true`) and connected with credentials accepted by the provider. ACT eligibility, where offered, depends on the provider and plan. Fastway supplies live TV, not the on-demand catalogue. See [Connect Fastway Live TV](../account-fastway/index.md).
- **PlayboxTV** is enabled with `playbox = true` (or `OPENSTREAM_PLAYBOX=true`) and uses its own OTP login. See [PlayboxTV](../ott-provider-playbox/index.md).
- **VZY** is enabled with `vzy = true` (or `OPENSTREAM_VZY=true`) and uses an anonymous free-tier session; premium channels are not advertised. See [VZY](../ott-provider-vzy/index.md).
- **Tata Play** is enabled with `tataplay = true` for diagnostics only. It is deliberately not part of the shared catalogue, playlist, `/live` or DRM proxy. See [Tata Play Smart](../ott-provider-tataplay/index.md).

The installers list only the accounts the downloaded release supports and print this same hint above the account menu. The older name **Extras** may still appear in older guides and error messages. `ost extras login` remains a command alias; `ost tv+ login` is the current spelling.

## Check which provider is active

1. Open **Settings** in the web interface.
2. Each enabled provider card shows whether it is logged in or connected and whether it is the **Active** provider.
3. Only one provider can be active at a time. Signing in to a provider makes it active; use **Use JioTV**, **Use JioTV+**, **Use Fastway**, **Use PlayboxTV** or **Use VZY** on a connected card to switch.
4. Open **Channels**. The list, IPTV playlist and programme guide show only the active provider's catalogue. Custom channels remain available.

![Settings showing a disconnected JioTV card and a connected JioTV+ card](../images/settings-jiotv-plus.png)

The screenshot is from a disposable v1.5.0 server with JioTV+ connected and JioTV disconnected. Fastway is disabled in this capture. The existence of a channel tile never guarantees permission to play it.

## Understand labels and restrictions

**Subscription required** means you may need a plan or additional rights. **Unavailable** can also mean your account is disconnected, the provider rejected playback, the programme is region restricted, or the browser does not support its DRM system. JioTV cannot unlock subscriptions or bypass DRM.

If you need more channels, check the subscription with the provider before changing server settings. See [Connect a JioTV account](../account-connect-jiotv/index.md), [Connect JioTV+](../account-extras/index.md) and [Why a channel will not play](../troubleshooting-channels-and-playback/index.md).
