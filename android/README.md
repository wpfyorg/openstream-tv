# Openstream TV Slim for Android

Native Android phone and Android TV front end for the Rust `slim` executable. The APK carries one executable per ABI at `lib/<abi>/libjiotv_exec.so`. Android extracts it to `nativeLibraryDir`; the app runs it directly as a child process. No Termux installation is used.

## Build

Install Android Studio/SDK, Android NDK, Gradle 8.9+, `cargo-ndk`, and Rust with these targets:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/<installed-version>"
./android/scripts/build-rust-binaries.sh
cd android
gradle assembleDebug
```

The helper builds the root crate with `--no-default-features --features slim,android-epg` and installs three Android executables into the app JNI library directories using the agreed filename. `android-epg` forces EPG on even if the imported config or saved override disables it; downloading channel data and generating EPG still requires a working account and Internet connection. Set `ANDROID_API_LEVEL` to change the NDK linker API (default 23). The generated binary files are build inputs; they are not checked into source control. CI runs this helper before Gradle.

## Use

Open Openstream TV Slim. The local server starts automatically by default. It binds only to `127.0.0.1:5001`; the app stores runtime state under its private files directory and sets `JIOTV_PATH_PREFIX` for every CLI process. Choose JioTV or JioTV+ (Beta) on the phone or Android TV; only one provider is active at a time, and the channel list follows the active provider. Enter exactly 10 digits for your mobile number and select **Send OTP**. Once the request succeeds, enter the 6-digit code and select **Login**. JioTV+ selection enables the existing `JIOTV_TVPLUS` backend mode and restarts a running local server so the selected provider is applied. OTP requests use the keyed route prefix obtained from `jiotv key show`. The app never puts the mobile number or OTP in its notification or process arguments.

The app masks the access key in the displayed playlist URL. **Copy**, **Save M3U**, and **Open in IPTV app** use the full keyed URL internally. The address is loopback, so it is usable by apps on the same Android device. The server is not exposed to the LAN. No LAN exposure control is included. On Android TV the dashboard follows the Openstream TV Slim two-column layout from `design/tv-app-ui.pen`.

Use **Import TOML config** or **Import login store backup** to open the D-pad TV file picker. On Android versions where scoped storage prevents direct public-folder enumeration, the picker offers Android's system file picker as a safe fallback. The app stages imports privately, validates them with the bundled executable and an isolated private data path, then replaces the active file only after validation. A failed validation leaves current data in place. Config/store content and command output are never displayed or logged.

Use **Export TOML config** to save the current `config.toml` through Android's document picker. If no config exists yet, the app exports a valid TOML file containing the Rust runtime defaults. Export runs off the UI thread and does not include the separate login store or provider credentials.

Use **Import login store backup** separately to restore a `store_v4.toml` backup. The app validates the expected `[data]` store schema with the bundled executable while sending all command output to `/dev/null`. If the server is running, it waits for the service to stop before atomically replacing the store, then restarts it. A failed validation leaves the current login store unchanged. Store values are never displayed or logged.

## Service and limitations

Boot autostart is enabled by default. The **Auto-start** toggle controls that preference. When enabled, the manual **Start/Stop local server**, **Setup using ADB**, and **Setup using Shizuku** buttons are hidden; turn auto-start off to access them. Starting and stopping the server manually does not change this preference. The boot receiver starts the foreground server independently of whether Android allows the app window to open. On Android 10 (API 29) and later, automatic UI launch requires the user-granted **Display over other apps** permission; otherwise open Openstream TV Slim from its notification. On earlier Android versions the receiver also attempts to open the app. The receiver waits for `BOOT_COMPLETED`, so startup occurs after user unlock.

On Android TV, **Setup using ADB** connects to `127.0.0.1:5555` only when selected, presents the device's normal ADB key authorization prompt for new keys when supported, and runs the fixed overlay-permission command after an authorized connection. TCP ADB must already be listening on that address; enabling debugging or wireless pairing is device-dependent. A previously approved key may reconnect without another prompt. The app cannot silently approve Android's authorization dialog. **Setup using Shizuku** is a separate flow: it connects to the locally installed, running Shizuku service, asks for Shizuku authorization, and grants only Openstream TV Slim's `SYSTEM_ALERT_WINDOW` app-op for its startup screen. Shizuku must be started on the TV first (on non-rooted devices, its ADB-backed service must be started again after reboot). If Shizuku is missing or stopped, the button opens Shizuku when installed and explains what to do. Permissions are requested only when the relevant setup button is pressed.

The foreground service keeps the Rust server alive while Android runs the app's service. Force-stopping the app or the OS terminating its process stops the server. Login itself requires Internet connectivity and a valid JioTV account. A URL containing the access key is a credential; share it only with trusted apps/devices. The Android interface does not provide account logout or key rotation yet.

## Checks

Run `gradle testDebugUnitTest assembleDebug` after building the three Rust binaries. Automated unit tests cover playlist path validation, masked URL display, and login-store schema checks. A successful Android build still needs the SDK/NDK and all three Rust Android targets installed.
