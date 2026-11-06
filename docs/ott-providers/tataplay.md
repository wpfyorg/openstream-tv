# OTT provider: Tata Play Smart (`com.ryzmedia.tatasky` 509.0/894)

Evidence-backed static record. No live calls. All secret-shaped values
redacted. Content/auth paths are **unknown statically** (encrypted RN bundle).

## 1. Artifact identity (observed fact)

- File: `reverse-engineering/apps/com.ryzmedia.tatasky_509.0-894_4arch_…​.apkm`
  (APKMirror APKM v5, 43 inventory nodes, all hash-verified).
- SHA-256 (APKM): `ab9091ab2a3bb42eeb2be2b6c6e59cc799d17b6af852618c13d433b253b90e14`
- base.apk SHA-256: `93cb824c…095ac3bd0` (71,312,419 B).
- `info.json`: app `Tata Play Smart: 500+ Channels`, package
  `com.ryzmedia.tatasky`, version `509.0` / code `894`, minSdk 27 /
  targetSdk 36, arches `arm64-v8a/armeabi-v7a/x86/x86_64`, 7 dpis, 24 langs,
  leanback capable.
- 12 DEX files (~106 MB); React Native + Hermes (`libhermes.so`,
  `assets/index.android.bundle` 12.2 MB, **encrypted**: magic `05011cc1…`,
  zero URLs, max printable run 16 chars); CodePush OTA.

## 1a. Static follow-up (JADX 1.5.6, full base.apk)

The earlier pass could not run JADX. This pass decompiled all 12 DEX files and
re-inspected the bundle:

- `com/ryzmedia/tatasky` contains only `MainApplication`, `BuildConfig`, `R`,
  UI/utility packages and SDK glue. **There is no app-level Retrofit/OkHttp API
  class, no request builder, and no endpoint map anywhere in the DEX.**
- `BuildConfig` exposes `BASE_URL=https://tm.tapi.videoready.tv/`,
  `ACTION_NAME_CONFIG=rest-api/pub/api/v1/config`, `X_API_KEY`, `X_APP_ID=ott-app`,
  `X_APP_KEY`, `IRDETO_BASE_URL`, `CODEPUSH_SERVER_URL`, `BUNDLE_ID=com.hungama.tatasky`.
  Nothing in the DEX consumes `ACTION_NAME_CONFIG`; it is read from JS.
- `MainApplication` uses `DefaultReactNativeHost.getJSBundleFile()` →
  `CodePush.getJSBundleFile()`, so the JS layer is the API layer.
- `assets/index.android.bundle` is **encrypted**, not Hermes bytecode:
  first bytes `05 01 1c c1`, no `c6 1f bc 03` magic anywhere in 12.2 MB,
  Shannon entropy 7.997 bits/byte, no readable strings, no ZIP/gzip container.
- No AES key or bundle-decryption routine is reachable from the DEX: the app
  classes are DexGuard-obfuscated and the only native crypto-capable libraries
  are stock Hermes/React Native plus packed `libc2f2.so`/`libc7a509.so`.
  A brute-force check of plausible keys derived from the BuildConfig values
  (raw/MD5/SHA-1/SHA-256, AES-CBC/ECB, several IV placements) produced no
  Hermes header or JS text.

**Conclusion:** no login, catalogue, EPG, playback, license or subscription
contract is statically recoverable. Guessed paths were therefore removed from
the backend; `/api/tataplay/status` reports the artifact identity and this exact
gap, and no Tata Play channel can enter the shared catalogue.

## 2. API bases (observed static strings)

- `BASE_URL = https://tm.tapi.videoready.tv/` (`BASE_URL_CONFIG_AKAMAI` same;
  `API_BASE_URL = ${BASE_URL}` placeholder; `API_TIMEOUT = 15000`).
- `CODEPUSH_SERVER_URL = https://cp.tapi.videoready.tv/`.
- `IRDETO_BASE_URL = https://tatasky-csm.live.ott.irdeto.com/` (Irdeto CSM).
- Hotstar/Bifrost stack embedded (analytics/player): `api/apix/hf-apix/pp-apix/
  pp-hf-apix/bifrost-api*.hotstar.com`.
- No Tata Play content-endpoint paths (`/live`, `/epg`, `/channel`,
  `/catalog`, `/search`, `/vod`, license URL) in ~1.2 M dex strings,
  859 resources, or manifest (observed absence).

## 3. Auth (facts + gaps)

- Facts: `SmsBroadcastReceiver` (OTP auto-read, Juspay SMS-handler),
  Facebook Login SDK, Google OAuth revoke URL, Firebase Auth/Messaging,
  `EncryptSharedPreferences` migration (tokens encrypted at rest), MoEngage
  `setMobileNumber` (RMN = user identity), deeplinks `tataplay://` +
  `https://a2uuh.app.goo.gl`.
- Unknown: OTP request/verify paths, token names/expiry/refresh, device
  binding (all behind the encrypted JS bundle).

## 4. Device identity (facts)

- `READ_PHONE_STATE` (not required), AppsFlyer, MoEngage, Firebase
  (`project_id=tata-sky-new-ott`), Salesforce MarketingCloud, New Relic,
  Ad ID + AdServices. No static device-ID key names recovered.

## 5. Signing / hardening (facts)

- base.apk has no v1 JAR signature (v2/v3+ block present); bundle-tool
  `stamp-cert-sha256` observed (value withheld per policy for cert digests?
  non-secret but omitted here).
- `usesCleartextTraffic=false`, `allowBackup=false`, `extractNativeLibs=false`;
  `networkSecurityConfig` declared but no `network_security_config.xml` in
  base.apk (likely in a split or stale). Pinning posture unknown.
- DexGuard-style obfuscation (`o.*`, `libc2f2.so`/`libc7a509.so`), Play
  Integrity refs, `DETECT_SCREEN_RECORDING`.

## 6–8. Subscription / catalogue / EPG / VOD

- Facts: Juspay HyperSDK (UPI/cards), `CHECK_LICENSE` (Play license).
- No pack/entitlement, channel-list, EPG, search, VOD, or catch-up endpoints
  visible statically. One lead: Hotstar JHS media SDK + live-player
  `default_config.json` → Hotstar-derived playback stack.
- Lineage string: `BUNDLE_ID = com.hungama.tatasky` (Hungama white-label).
- Manifest: 49 activities, 35 services, 28 receivers, 15 providers; PiP,
  foreground-service playback, exact-alarm + timezone/boot receivers
  (schedule-aware, not proof of EPG source).

## 9. Playback + DRM (facts + unknown)

- Facts: dual ExoPlayer (`androidx.media3` + vendored `com.jhs.media…`),
  `ExoDownloadServiceCore` (offline), IMA + SSAI, Cast.
- DRM: Media3 `DefaultDrmSession*` + `MediaDrm`; DRM error strings; Irdeto CSM
  suggests multi-DRM/license via Irdeto; PlayReady `AcquireLicense` is an
  ExoPlayer constant, not proof of use.
- Unknown: Widevine license URL, key-system flow (expected behind encrypted
  bundle + Irdeto CSM at runtime).

## 10. Runtime gaps

MITM capture (device + user OTP) against `tm.tapi.videoready.tv` and Irdeto
CSM; decrypt/unpack `index.android.bundle` or CodePush update; JADX setup;
pinning + full cert chain. Backend implements device/token persistence,
generic OTP/login/catalogue/playback/license contracts with mock tests, and
marks content paths as unresolved.
