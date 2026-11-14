---
title: Connect Your JioTV Account
description: Sign in to the JioTV channel source with your mobile number and one-time password.
category: Account Setup
keywords: [JioTV, account, OTP, mobile number, sign in]
---

# 📲 Connect Your JioTV Account

The JioTV account supplies the channels available to your own subscription. It is separate from the admin password you use to open this server.

![Settings showing the JioTV mobile-number sign-in card while JioTV+ is connected separately](../images/settings-jiotv-plus.png)

---

## 🔑 Sign In

1. Sign in to the web interface with the server's admin password.
2. Open **Settings** and find the **JioTV** card.
3. Enter the 10-digit mobile number for your JioTV account and select **Send OTP**.
4. Enter the code sent to that number, then select **Log in**.
5. Check that the card says **Logged in**, then return to **Channels**.

**Expected result:** the JioTV account status is logged in and the channel list loads the channels available to that account. An individual channel can still need a subscription or be unavailable from the provider.

---

## 🔧 Troubleshooting

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| OTP fails or expires | Code aged out, or the request was repeated | Request a fresh code and enter it promptly |
| Channel list is empty | Account not signed in, or catalogue request failed | Refresh the page and check the account status in Settings |
| Wrong account connected | An older JioTV login is still stored | Follow [switch accounts](../account-switch-jiotv/index.md) |

For first-time server setup see [first launch and admin setup](../getting-started-first-launch/index.md); to add the optional source see [Connect JioTV+](../account-extras/index.md).
