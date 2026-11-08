---
title: Install the Android TV and phone app
description: Run the slim OpenStream TV IPTV server directly on Android TV or a phone and use its local playlist in another IPTV app.
category: Installation
keywords: [Android TV, Android, APK, slim, IPTV, local playlist, import config]
---

# Android TV and phone app

The Android app bundles the **slim** OpenStream TV server. It provides controls for starting and stopping the local server, a playlist URL for IPTV players on the same Android device, and options to import or export a **TOML configuration file**. It has no browser TV player; use an installed IPTV player to watch.

## Build and install

The Android app project is in `android/`. The [Android TV and mobile app GitHub Actions workflow](https://github.com/wpfyorg/openstream-tv/actions/workflows/android-app.yml) builds the Rust slim server for ARM64, ARMv7 and x86_64, bundles it into one Android debug APK, and uploads that APK as a workflow artifact. The debug APK is for sideloading and testing, not a signed store release. See `android/README.md` for build prerequisites and local build instructions.

Install the APK on an Android phone, tablet or Android TV device that permits sideloading, then open the app. Android may ask you to permit notifications when it starts its foreground server service.

## Use the local IPTV URL

Start the server in the app. It binds only to `127.0.0.1`, using port `5001` by default or the first free port through `5010` when another app already owns it. It then shows a URL shaped like:

`http://127.0.0.1:5001/k/<unique-key>/playlist.m3u`

Use **Copy** to copy the complete address. The on-screen key is masked. In an IPTV player installed **on the same device**, add a playlist from URL and paste it. The `127.0.0.1` host points to the Android device itself; it will not work on another TV or phone. Keep the local server running while watching.

The key in the playlist URL grants access to the stream endpoints. Treat it as private. Opening the app or copying the URL does not automatically configure other IPTV apps; support for opening URLs directly differs among players.

## Import configuration and sign in

Use **Import TOML config** to select a previously saved `.toml` configuration using Android's file picker. This imports server settings; it does **not** by itself transfer an authenticated provider account, tokens or the private runtime store. Sign in with your own provider account in the app if it has not been connected. Restart the local server for new settings to take effect.

Choose **JioTV** or **JioTV+** in the app's provider row. Only one provider is active at a time, and the playlist and channel list follow the active provider. JioTV+ also needs its own eligible JioFiber or AirFiber login. The bundled server also enables Fastway, PlayboxTV and VZY (Beta); connect and activate those from the web Settings after enabling the required configuration.

Use **Export TOML config** to save server settings through Android’s file picker. Login credentials are kept in the separate runtime store and are not added to the configuration export.

Use **Import login store backup** separately to restore a `store_v4.toml` backup. The app validates it before replacing its private login store. This backup contains credentials; keep it private. Provider support depends on the bundled server and the configuration flags enabled for it.

Only import configuration files you trust. The app's local data and the server's private store are kept in Android application storage, and the application must remain installed to retain that data.

## Start at device boot

Start the server once to enable automatic startup after boot and unlock. The server runs as a foreground service with a persistent notification. Stop disables future boot startup; Start enables it again.

On Android 10+, use **Enable startup screen** and grant the system “display over other apps” permission to let the app screen open at boot. Alternatively, with an authorized ADB connection: `adb shell appops set org.opebstreamstv.android SYSTEM_ALERT_WINDOW allow`. This permission does not require displaying an overlay; it permits the background activity launch. Without it, the server can still start and its notification opens the screen. Device battery or vendor startup restrictions may also apply.

## TV remote navigation

Use the D-pad to move between controls and OK to activate them. Select a number field with OK to open the keyboard; Back dismisses the keyboard. Import and export require a compatible Android document picker. Some operator TV boxes do not include one.

## Troubleshooting

If the IPTV player cannot load the playlist, confirm that the app's server status is **running**, paste the complete keyed URL, and check that the player runs on the same Android device. An empty playlist may mean the active provider account is not yet connected. If Android stops the server after a period of inactivity, check its background activity and battery restrictions. Playback support for DRM-protected channels still depends on the IPTV player.

For an Android installation with full terminal access instead, see [Install on Android with Termux](../installations-termux/index.md).
