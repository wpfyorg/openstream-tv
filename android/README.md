# JioTV Go for Android

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

The helper builds the root crate with `--no-default-features --features slim` and installs three Android executables into the app JNI library directories using the agreed filename. Set `ANDROID_API_LEVEL` to change the NDK linker API (default 23). The generated binary files are build inputs; they are not checked into source control. CI runs this helper before Gradle.

## Use

Open JioTV Go and start the local server. It binds only to `127.0.0.1:5001`; the app stores runtime state under its private files directory and sets `JIOTV_PATH_PREFIX` for every CLI process. Choose **JioTV** or **JioTV+**, enter the matching account's mobile number, request an OTP, then enter the OTP yourself and verify. JioTV+ is enabled for the Android server process and uses the separate JioTV+ login endpoints; if a number has multiple eligible JioFiber/AirFiber connections, the app asks which connection should receive the OTP. OTP requests use the keyed route prefix obtained from `ost key show`. The app never puts the mobile number or OTP in its notification or process arguments.

The app masks the access key in the displayed playlist URL. Copy and **Open in IPTV app** still use the full URL. The address is loopback, so it is usable by apps on the same Android device. The server is not exposed to the LAN. No LAN exposure control is included. On Android TV the app groups server, provider, and playlist controls beside login and import controls; phone actions keep the IPTV app button full width. Startup-screen permission has three options: Android's **Display over other apps** settings page, optional Shizuku, or **Setup startup permissions using ADB**. ADB setup uses a persistent key stored under the app's private no-backup directory and runs only `appops set org.jiotvgo.android SYSTEM_ALERT_WINDOW allow`; it verifies the grant with Android and disconnects. The ADB button first tries `127.0.0.1:5555`, so a TV with classic TCP/network ADB already listening there can show Android's native debugging authorization prompt without typing an address or port. If that local endpoint is unavailable or Wireless Debugging pairing is required, the app opens the manual fallback with `127.0.0.1` and `5555` prefilled; on Android 11+ enter the Wireless Debugging pairing address/code and pairing port plus its separate debugging port as needed. The app never enables debugging itself. Approve Android's authorization prompt using the device controls. If debugging is off, enable it in Developer Options yourself first. Shizuku must be installed, running, and authorized when selected ([official download and setup](https://shizuku.rikka.app/)); non-root Shizuku started through ADB needs restarting after reboot. A stopped or unavailable Shizuku service does not affect the server. Neither setup method is needed for the server or boot receiver to start.

Use **Import TOML config** to select a TOML file. Android TV uses the app's built-in D-pad file browser, which starts in shared-storage Downloads and only shows folders plus `.toml` files. Phones use Android's document picker when available and fall back to the built-in browser when it is missing. The built-in browser may request legacy shared-storage permission on older Android versions. The app stages the selected file privately, validates it with the bundled executable and an isolated private data path, then atomically replaces the active config. A failed validation leaves the current config in place. Config content and command output are never displayed or logged. Stop and start the service to apply an imported config.

Use **Export TOML config** to save the current `config.toml`. Android TV uses the same built-in browser to choose a shared-storage folder; phones keep Android's document picker with the built-in browser as a fallback. If no config exists yet, the app exports a valid TOML file containing the Rust runtime defaults. Export runs off the UI thread and does not include the separate login store or provider credentials.

Use **Import login store backup** separately to restore a `store_v4.toml` backup. The app validates the expected `[data]` store schema with the bundled executable while sending all command output to `/dev/null`. If the server is running, it waits for the service to stop before atomically replacing the store, then restarts it. A failed validation leaves the current login store unchanged. Store values are never displayed or logged. The Android provider selector offers JioTV and JioTV+; Fastway is not exposed by this slim Android build.

## Service and limitations

Boot autostart is enabled by default. The boot receiver starts the foreground server independently of whether Android allows the app window to open. On Android 10 (API 29) and later, automatic UI launch requires the user-granted **Display over other apps** permission; otherwise open JioTV Go from its notification. On earlier Android versions the receiver also attempts to open the app. Pressing **Stop local server** disables boot autostart; **Start local server** enables it again. The receiver waits for `BOOT_COMPLETED`, so startup occurs after user unlock.

The foreground service keeps the Rust server alive while Android runs the app's service. Force-stopping the app or the OS terminating its process stops the server. Login itself requires Internet connectivity and a valid JioTV account. A URL containing the access key is a credential; share it only with trusted apps/devices. The Android interface does not provide account logout or key rotation yet.

## Checks

Run `gradle testDebugUnitTest assembleDebug` after building the three Rust binaries. Automated unit tests cover playlist path validation, masked URL display, and login-store schema checks. A successful Android build still needs the SDK/NDK and all three Rust Android targets installed.
