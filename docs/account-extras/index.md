---
title: Connect JioTV+
description: Enable and connect the optional JioTV+ account source when the server build supports it.
category: Account Setup
keywords: [JioTV+, account, OTP, JioTV+ channels, TV+]
---

# Connect JioTV+

**JioTV+** is an optional second source. It appears in Settings when `tv_plus` is enabled and the account has not been locked. A **Lock** selection is saved and overrides the server setting across restarts; unlock JioTV+ from **Channels** with the account's unlock code. It requires a separate registered mobile number and an eligible plan.

![Settings showing JioTV+ connected on OpenStream TV v1.5.0](../images/settings-jiotv-plus.png)

1. Open **Settings** and find **JioTV+**.
2. Enter the registered 10-digit mobile number and select **Continue**.
3. If asked, choose the matching connection from the list, then select **Send OTP**.
4. Enter the OTP and select **Connect**.
5. Check that the card says **Connected**. Connecting JioTV+ makes it the active provider, and the header indicator appears while it is active.
6. **Channels** now shows the JioTV+ catalogue. To switch back, use **Use JioTV** on another connected card; only one provider can be active at a time.

**Expected result:** channels supplied by the connected source appear when available. A channel can still require a plan or be unavailable for playback.

To disconnect, select **Disconnect** on the card and confirm. **Lock** turns off the JioTV+ source without disconnecting its provider account; it does not sign out the web interface. The server may hide provider logout controls when logout is disabled in its configuration.

If the source is unavailable in Settings, check the `tv_plus` setting in [server configuration](../configuration-reference/index.md) and whether JioTV+ has been locked. A saved lock must be cleared from **Channels** with the unlock code; changing `tv_plus` alone does not clear it.
