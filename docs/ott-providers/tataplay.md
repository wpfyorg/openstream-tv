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
