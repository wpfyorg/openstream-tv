---
title: Install the Android TV and phone app
description: Run the OpenStream TV server with its web Settings UI directly on Android TV or a phone and use its local playlist in another IPTV app.
category: Installation
keywords: [Android TV, Android, APK, IPTV, local playlist, import config, provider settings]
---

# 📺 Android TV and Phone App

The app bundles the OpenStream TV server with the web Settings UI. It provides start and stop controls for the local server, a playlist URL for IPTV players on the same device, on-device provider settings, and TOML configuration import and export. It has no browser TV player: use an installed IPTV player to watch.

---

## 📦 Step 1: Build and Install

The Android project is in `android/`. The [Android TV and mobile app workflow](https://github.com/wpfyorg/openstream-tv/actions/workflows/android-app.yml) builds the full Rust server for ARM64, ARMv7 and x86_64, bundles it into one debug APK and uploads that APK as a workflow artifact. It is for sideloading and testing, not a signed store release; `android/README.md` lists build prerequisites and local build steps.

Install the APK on a phone, tablet or Android TV device that permits sideloading, then open it. Android may ask you to permit notifications when the foreground server service starts.

---

## 🔗 Step 2: Use the Local IPTV URL

Start the server in the app. It binds only to `127.0.0.1`, on port `5001` or the first free port through `5010` when another app owns it, and shows a URL shaped like:

```
http://127.0.0.1:5001/k/<unique-key>/playlist.m3u
```

Select **Copy** for the complete address; the on-screen key is masked. In an IPTV player installed **on the same device**, add a playlist from URL and paste it.

> [!IMPORTANT]
> The `127.0.0.1` host points at the Android device itself and will not work on another TV or phone. Keep the local server running while watching. The key grants access to streaming endpoints, so treat the URL as private.

---

## 🔑 Step 3: Connect a Provider

Choose a provider in the **Provider** list; selecting it opens that provider's sign-in panel directly below.

| Provider | Sign-in |
| :--- | :--- |
| **JioTV** | Mobile number and OTP |
| **JioTV+** | Its own OTP plus a connection picker; needs an eligible JioFiber or AirFiber login |
| **PlayboxTV** (Beta) | Mobile number and OTP |
| **Airtel Xstream** (Beta) | Mobile number and OTP |
| **Tata Play** (Beta) | Mobile number and OTP; asks which account when one number holds several |
| **Fastway** | Username and password |
| **VZY** (Beta) | No account needed |

A connected provider becomes active when selected, and only one is active at a time, so the playlist, EPG and streams follow that provider. The guide follows it too: PlayboxTV and Tata Play list channels without programmes because they return no usable programme times, and Airtel Xstream and VZY publish programmes only when their guide data includes them.

**Where the list sits:** on phones it is at the top of the dashboard above the playlist fields, and the sign-in panel opens separately below it, so scroll up after changing server settings. On Android TV it is in the right-hand column beside the Local server and playlist controls, with the sign-in panel under it.

---

## 🔁 Step 4: Import and Export

| Control | What it does |
| :--- | :--- |
| **Import TOML config** | Loads a saved `.toml` configuration through the Android file picker. It imports server settings only — not an authenticated provider account, tokens or the private runtime store. |
| **Export TOML config** | Saves server settings through the file picker. Login credentials stay in the separate runtime store and are not exported. |
| **Import login store backup** | Restores a `store_v4.toml` backup after validating it. It contains credentials, so keep it private. |

Restart the local server for new settings to take effect. The bundled server always enables JioTV+, Fastway, PlayboxTV, VZY, Airtel Xstream and Tata Play because the app's service sets their environment flags directly, so an imported file cannot disable those providers here.

> [!WARNING]
> Only import configuration files you trust. App data and the server's private store live in Android application storage, and the app must stay installed to keep them.

---

## ⏻ Step 5: Start at Device Boot

Auto-start is on by default. At boot the app starts only its foreground server service in the background; it does not open the dashboard, and the persistent notification opens it when needed. Starting or stopping the server manually does not change the preference. Use **Disable auto-start** and **Enable auto-start** to change it.

Boot startup needs no "display over other apps" permission, ADB or Shizuku. The app runs the server in the background until you stop it or Android force-stops it; device battery and vendor startup restrictions may still apply.

---

## 🎮 TV Remote Navigation

Use the D-pad to move between controls and OK to activate. Select a number field with OK to open the keyboard; Back dismisses it. Import and export need a file picker: Android TV on Android 12 and older opens the app's own file browser, while Android 13 and newer uses the system document picker and reports when none is installed.

---

## 🔧 Troubleshooting

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Player cannot load the playlist | Server stopped, wrong URL, or player on another device | Confirm the server says **running**, paste the complete keyed URL, and run the player on the same Android device |
| Playlist is empty | Active provider not connected | Connect the account in the Provider panel |
| Server stops after a while | Background activity and battery restrictions | Review Android's background limits for the app |
| Protected channel will not play | Player has no Widevine support | Check the player; DRM playback depends on it |

For a full terminal install on Android instead, see [Install with Termux](../installations-termux/index.md).
