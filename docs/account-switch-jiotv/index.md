---
title: Switch to Another JioTV Account
description: Replace the active JioTV login with another account safely using Settings or the CLI.
category: Account Setup
keywords: [switch account, JioTV, login reset, logout, OTP, change number, account picker]
---

# 🔄 Switch to Another JioTV Account

The server uses **one active JioTV provider account at a time**, and the web interface has no multi-account picker. To use a different mobile number, sign the current JioTV connection out and sign in again. Your **admin password** and any optional **JioTV+** connection are separate.

---

## 🖥️ Switch in Settings

1. Open **Settings** while signed in to the web interface.
2. In the **JioTV** card select **Log out**, then confirm. It appears for a connected account when provider logout is allowed — the default, unless an administrator sets `disable_logout = true` or `OPENSTREAM_DISABLE_LOGOUT=true`.
3. In the same card, enter the **new** account's mobile number and request an OTP.
4. Enter the OTP sent to that number to finish signing in.
5. Return to **Channels** and reload the channel list.

**Expected result:** Settings reports **Logged in** for the new provider account. Available channels and guide data may change with that account's plan, and existing playlist links may need refreshing afterwards.

The screenshot below shows the JioTV sign-in form while JioTV+ stays connected separately; it shows where the replacement account goes, not a completed switch.

![Settings with the JioTV sign-in card available while JioTV+ remains connected](../images/settings-jiotv-plus.png)

---

## ⌨️ Switch with the CLI

Use this when logout is disabled in Settings or you run a headless server. Stop OpenStream TV **before** changing credentials so the running process cannot overwrite the new state.

```sh
ost login reset
ost login otp
```

> [!IMPORTANT]
> Run both commands as the **same operating-system user, with the same configuration and data directory** as the server. The first clears the JioTV login; the second prompts for the new number and OTP.

Then restart OpenStream TV with your original command or service manager. On OpenWrt follow its [service instructions](../maintenance-openwrt/index.md); on Docker prefer the Settings workflow unless you know how to run commands against the container's persistent volume.

---

## 🔧 If Switching Does Not Work

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Old account still appears | Wrong server stopped, or a different data directory used | Stop the actual server and run the commands with its data directory |
| OTP fails | Code expired | Request a fresh one |
| Channels differ after login | The new account has different entitlements | Expected — see [login troubleshooting](../troubleshooting-accounts-and-extras/index.md) |

Switching JioTV does not automatically reconnect or disconnect [JioTV+](../account-extras/index.md).
