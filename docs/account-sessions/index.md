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

| Window width | How to sign out |
| :--- | :--- |
| 901px and wider | **Sign out** at the bottom of the sidebar |
| 900px and narrower | **Settings** → **This browser** → **Sign out**, because the sidebar is hidden at that width |

Sign in again with the admin password when you return.

**Expected result:** the sign-in page appears. With `disable_auth = true` sign-out does not lead to the sign-in page, because access checks are off and the interface stays signed in after the reload. Provider account status stays exactly as it was until you disconnect that account in Settings.

---

## ⏳ Session Rules

| Rule | Detail |
| :--- | :--- |
| Expiry | Admin sessions expire after 30 days |
| Password change | Signs out other browsers; the browser that changed the password stays signed in |
| Provider logout disabled | With `disable_logout` set, Settings hides provider disconnect controls; the web sign-out still works |
| Provider accounts | Unaffected by web sign-out; disconnect them separately |
| Access checks disabled | With `disable_auth = true` the interface reports itself signed in for every request, so signing out does not lock the page |

---

## 🔗 Related

- Account connection: [Connect your JioTV account](../account-connect-jiotv/index.md), [Connect JioTV+](../account-extras/index.md)
- Changing the admin password: [Settings](../using-settings/index.md)
- Sessions and access keys: [network configuration](../configuration-network-security/index.md)
