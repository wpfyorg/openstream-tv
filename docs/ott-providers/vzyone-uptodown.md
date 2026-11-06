# OTT provider: `com.vzy.one` file → Uptodown App Store (`com.uptodown` 7.44)

Evidence-backed static record. No live calls. No credentials.

## 1. Artifact identity (observed fact)

- File: `reverse-engineering/apps/com.vzy.one.apk` (original tree; git-ignored).
- SHA-256: `8f8a7b2416d84d64d418dc4e194cd8ab6f4394d51651bf2e1aac47207503fd92`
- Size: 15,790,385 bytes.
- Zip: 2,230 entries; `classes.dex` (8.7 MB) + `classes2.dex` (0.55 MB) +
  `classes3.dex` (7.5 MB) + `classes4.dex` (7.7 MB).
- DEX contains `com.uptodown` (24 hits) and zero `com.vzy` / `vzy` hits.
- Native libs for `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64`
  (`libuptodown-native.so`, `libutd-services-native.so` + androidx libs).
- Manifest (apktool): `package="com.uptodown"`, `versionName="7.44"`,
  `versionCode="744"`, `minSdkVersion="23"`, `targetSdkVersion="36"`.
- App label: `Uptodown App Store`; app class `com.uptodown.UptodownApp`;
  TV entry `com.uptodown.tv.ui.activity.TvMainActivity`.
- Conclusion: the filename is wrong. This artifact is **not** a `com.vzy.one`
  TV service. It is the Uptodown third-party app store.

## 2. API surface (static strings only, not runtime-confirmed)

- Primary: `https://api.uptodown.app:443`.
- Downloads: `https://dw.uptodown.com/dwn/`, `https://www.uptodown.com:443/dwn/`.
- Others: `sdk.uptodown.app`, `t/u/www.uptodown.app`, `en.uptodown.com`.
- v1: `/v1/app/`, `/v1/app/search`, `/v1/category…`, `/v1/file…`,
  `/v1/home`, `/v1/installations`, `/v1/latest-updates`, `/v1/new-releases`,
  `/v1/top-downloads`.
- v2: `/v2/app/`, `/v2/categories…`, `/v2/my-wishlist`,
  `/v2/tracker/device…`, `/v2/user/login`, `/v2/user/avatars`.
- Only login-shaped path: `/v2/user/login` (method/body unknown).

## 3–6. Auth / device / signing / subscription

- Auth: Google sign-in (AndroidX Credentials) + `/v2/user/login`; no OTP,
  SSO-token, refresh, or logout strings attributable to Uptodown auth.
- Device: `/v2/tracker/device`, `/v2/tracker/identifier`, `/v1/installations`
  imply install registration; field names not recovered (no JADX here).
- Signing: native `NativeApiKey` (`getApiKeySeed`/`getAuthApikey`) suggests a
  native-derived API key; derivation not analyzed.
- Subscription: none found. Only `url_turbo` (Uptodown Turbo web page). No
  entitlement, in-app-billing, or paywall strings.

## 7–12. Catalogue / EPG / VOD / playback / DRM

- None. Catalogue APIs are app-store listings (home, top-downloads,
  latest-updates, search, similar/recommended). No `/epg`, `/channel`,
  `/live`, `/vod`, `/shows`, `/seasons`, `/episodes`, catch-up, `m3u8`,
  `mpd`, ExoPlayer/Media3, Widevine license URL, or playback-token strings.

## 13. Runtime gaps

- `/v2/user/login` method/body, `Authorization`/`Bearer` usage, native key
  derivation, tracker fields, and download flow all need instrumented-device
  capture. Not performed.
- Tooling limit: REA JADX tools required `REA_JADX_MCP_JAR` (unset), so no
  class/method decompilation; conclusions rest on zip inventory, DEX string
  scans, apktool manifest/resources, and REA `inspect_artifact` /
  `decode_android_resources` evidence (see subagent ledger).

## Implementation consequence

This service provides app distribution, not TV. The backend models it as a
status-only provider (`src/vzyone.rs`, IDs `ud_*` reserved but no channels):
status reports package/version/presence, login/playback/EPG are explicitly
unsupported. No feature parity is forced.
