---
title: Connect JioTV+
description: Enable and connect the optional JioTV+ account source when the server build supports it.
category: Account Setup
keywords: [JioTV+, account, OTP, JioTV+ channels, TV+]
---

# ➕ Connect JioTV+

**JioTV+** is an optional second source. It needs its own registered mobile number and an eligible JioFiber or AirFiber plan, and the **JioTV+** card only appears in Settings when the server enables `tv_plus`.

![Settings showing JioTV+ connected on OpenStream TV v1.5.0](../images/settings-jiotv-plus.png)

---

## 🔑 Connect

1. Open **Settings** and find **JioTV+**.
2. Enter the registered 10-digit mobile number and select **Continue**.
3. If asked, choose the matching connection from the list, then select **Send OTP**.
4. Enter the OTP and select **Connect**.
5. Check that the card says **Connected**. Connecting JioTV+ makes it the active provider, and the header indicator appears while it is active.
6. **Channels** now shows the JioTV+ catalogue. To switch back, use **Use JioTV** on another connected card — only one provider can be active at a time.

**Expected result:** channels supplied by the connected source appear when available. A channel can still require a plan or be unavailable for playback.

---

## 🔓 Unlock Codes

When JioTV+ is locked, the card is hidden until the unlock code is entered. Type the code into the search field on **Channels**: a code-shaped entry is checked automatically, and on success the channel list reloads and Settings shows the JioTV+ card again without a page refresh.

> [!IMPORTANT]
> The unlock code is not a provider password and does not sign out the web interface. Setting `tv_plus` in the configuration does not clear a lock by itself.

---

## 🔌 Disconnect

Select **Disconnect** on the card and confirm. This removes the saved JioTV+ login; the source then stays hidden until `tv_plus` is enabled again and the account reconnected. The server may hide provider disconnect controls when `disable_logout` is set.

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Card missing in Settings | `tv_plus` disabled for the running server | Enable `tv_plus` in [server configuration](../configuration-reference/index.md) and restart |
| Card missing after enabling `tv_plus` | JioTV+ is locked | Enter the unlock code in the **Channels** search field |
| Sign-in refused | No eligible JioFiber/AirFiber plan, or wrong number | Use the mobile number registered with that connection |
