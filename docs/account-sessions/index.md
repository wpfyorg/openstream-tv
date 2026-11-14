---
title: Admin Sessions and Sign Out
description: Understand browser sessions, sign out, and what happens when the admin password changes.
category: Account Setup
keywords: [admin session, sign out, browser, password, session expiry]
---

# 🔐 Admin Sessions and Sign Out

The web interface keeps an admin session in your browser. Signing out of the web interface does **not** disconnect the JioTV, JioTV+ or Fastway provider account.

---

## 🚪 Sign Out

| Where you are | How to sign out |
| :--- | :--- |
| Desktop or tablet | **Sign out** at the bottom of the sidebar |
| Phone | **Settings** → **This browser** → **Sign out** (the sidebar is hidden on small screens) |

Sign in again with the admin password when you return.

**Expected result:** the sign-in page appears. Provider account status stays exactly as it was until you disconnect that account in Settings.

---

## ⏳ Session Rules

| Rule | Detail |
| :--- | :--- |
| Expiry | Admin sessions expire after 30 days |
| Password change | Signs out other browsers; the browser that changed the password stays signed in |
| Provider logout disabled | With `disable_logout` set, Settings hides provider disconnect controls; the web sign-out still works |
| Provider accounts | Unaffected by web sign-out; disconnect them separately |

---

## 🔗 Related

- Account connection: [Connect your JioTV account](../account-connect-jiotv/index.md), [Connect JioTV+](../account-extras/index.md)
- Changing the admin password: [Settings](../using-settings/index.md)
- Sessions and access keys: [network configuration](../configuration-network-security/index.md)
