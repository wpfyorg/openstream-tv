---
title: Tata Play Smart Provider Evidence
description: Recovered Tata Play Smart API contracts, live provider behaviour, runtime gaps and verification notes.
category: Provider Reference
keywords: [Tata Play Smart, provider, OTP, EPG, entitlement, DASH, Widevine]
---

# OTT provider: Tata Play Smart

This record covers two app generations: the older `com.ryzmedia.tatasky`
509.0/894 installed on the A15, and the newer `com.tataplay.livetv` 1.0.10/101
artifact whose native Retrofit layer is recoverable.

## Provider status

The newer app's verified Retrofit contracts are implemented behind
`tataplay = true` / `OPENSTREAM_TATAPLAY=true` (legacy alias
`JIOTV_TATAPLAY=true`) as a shared live TV provider. Enabling it and logging in
makes Tata Play the active product when no other OTT provider is connected, so
its channels appear in `/api/channels`, the M3U playlist, the XMLTV EPG and the
`/live` playback flow.

Routes:

- `GET /api/tataplay/status`
- `POST /api/tataplay/login/sendOTP`
- `POST /api/tataplay/login/verifyOTP`
- `POST /api/tataplay/refresh`
- `POST /api/tataplay/logout`
- `GET /api/tataplay/channels`
- `GET /api/tataplay/catalogue-seed`
- `GET /api/tataplay/epg`
- `GET /api/tataplay/entitlement`
- `GET /api/tataplay/play/:id?contentType=LIVE`

Credentials, playback URLs and license URLs stay server-side; `/live` and the
license proxy resolve them per request.

### Recovered runtime details

The provider persists a stable synthetic device identity because the server is
not running inside the Android APK. `device_details` is now the JSON the app
sends: the `DeviceDetails` model serialized by `C4099ep1.invokeSuspend` with its
observed constants (`device_type` `android`, `device_platform` `tv`,
`os_version` `13`, `app_version` `1.0.10`, `pl` `ios`, `bv` `133`) plus the
server's own `device_id` and screen size.

The header interceptor (`RD0`, case 0) maps `ADD_HEADERS` tokens to session
values and sends the app version under the header name `appVersion`.

Catalogue entries come from `homescreen/pub/api/v1/lit/hierarchy/LIT_LIVETV`:
every rail's `contentList` is flattened and deduplicated, and each channel is
exposed as `tp_<id>` where `<id>` is the channel's content id.

Playback URLs are AES-encrypted. `pas/api/v1/access/control/LIVE/<id>` returns
`playbackUrls.dashWidewinePlayUrl` and `dashWidewineLicenseUrl` as
`base64(ciphertext)#v1` or `#v2`; the key is `app.dd.v1` or `app.dd.v2` from
`lit-ls-config/pub/api/v1/config`, decoded with AES/ECB/PKCS5Padding. Both
config and `channels/all` answer without authentication, so the keys and the
document-id map are available before login.

### Known gaps

- EPG programmes are not published: the guide endpoint needs epoch-millisecond
  `epgStartTime`/`epgEndTime` bounds whose runtime encoding is unproven, so
  XMLTV contains channels only.
- The OTP step still needs the `refId` and `refIdMode` values the app obtains
  from the send-OTP JWT, so the client must pass the chosen account's `refId`.

### Playback sessions

`NetworkApi.X` opens a concurrency session with a bodyless
`POST smgs/api/v1/session`, carrying `x-device-name` and the `x-session-id`
returned by access control. The provider registers that session as soon as
playback resolves, repeats the call every `heartbeatInterval` seconds, and
sends the bodyless `DELETE smgs/api/v1/session` when the channel is stopped or
the playback cache is invalidated. When the response omits `heartbeatInterval`
the provider beats every 30 seconds.


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

**Conclusion for the original `com.ryzmedia.tatasky` base artifact:** no login,
catalogue, EPG, playback, license or subscription contract is statically
recoverable from its encrypted fallback bundle or DEX files. This historical
conclusion does not apply to the separate `com.tataplay.livetv` artifact below.
The diagnostic beta uses recovered contracts from that newer app and keeps Tata
Play out of the shared catalogue and shared playback paths.

## 1b. New artifact and runtime evidence (follow-up pass)

Two different Tata Play packages are now in scope:

- **Installed, logged in on the A15**: `com.ryzmedia.tatasky` 509.0/894
  (this record's original artifact). Its encrypted fallback bundle was
  bypassed legally by fetching the app's own CodePush update with the
  deployment key embedded in the APK. The CodePush payload is a plain ZIP
  containing plain Hermes bytecode (`index.android.bundle`, 11.8 MB), so the
  real JS is recoverable. It exposes the full old-app API surface
  (`login-service`, `rest-api`, `lit-mobile-common-bff`,
  `lit-subscription-service`, `pas/api/v1/access/control`,
  `content-detail`, etc.) under `https://tm.tapi.videoready.tv/`.
- **New artifact in `TataPlay.zip`**: package `com.tataplay.livetv`
  (`com.tataplaylit.tv`, "Tata Play Smart"), with a native Retrofit API layer
  and a small RN subscription SDK. This is a different app generation; it is
  not the installed package.

### Runtime boundary on the logged-in A15 (no subscription)

Observed through the device UI, without capturing tokens:

- Session restoration and home screen: **work**. The account is logged in.
- Channel catalogue / home rails: **work** (channels and "Subscribe" prompts).
- Live TV guide / EPG: **work** (programme rows, channel numbers, categories).
- Search: **work** (e.g. DD National HD, DD Sports HD, DD India HD).
- Premium live playback: gated by the app's own "Subscribe" UI; the account has
  no active pack. No subscription was purchased or activated.
- Free-to-air playback was not conclusively triggered through ADB touch input
  (the leanback cards did not accept injected taps); therefore no claim is made
  that free channels play.
- No API request/response was captured: the release package is not debuggable,
  the device has no root, and the app does not trust user CAs, so TLS
  interception would require patching the app (not done). No OTP was entered by
  the agent.

### New app static contracts (recovered, beta-enabled)

The new `com.tataplay.livetv` app has a native `NetworkApi` with explicit
Retrofit annotations on `https://tm.tapi.videoready.tv/`:

- `POST login-service/pub/api/v3/signin/generate/otp` (`GenerateBody{rmn,sid}`)
- `POST login-service/pub/api/v3/signin/validate/otp` (`ValidateOtpPayLoad{…}`)
- `POST rest-api/pub/api/v3/refreshToken` (`RefreshTokenRequest{refreshToken,subscriberId}`)
- `GET lit-subscription-service/api/v1/lit/getEntitlementDetails`
- `GET portal-search/pub/api/v1/channels/all?limit&offset`
- `GET lit-mobile-common-bff/api/v2/channels/guide?…`
- `GET pas/api/v1/access/control/{contentType}/{channelId}` → `PlayUrlData`
- `GET content-detail/pub/api/v3/catchupEpg/{channelId}?isLitEnabled=true`
- `POST device-management/v1/app/lit/subscriber/{subscriberId}/device`

Header injection is explicit (`RD0` interceptor): `Authorization: bearer <KS>`,
`appVersion: 1.0.10`, `platform`, `deviceId`, `device_details`, `deviceName`,
`x-device-id`, `x-subscriber-id`, `x-authenticated-userid`, `subscriberId`,
`sid`, `countryRegion`, `profileId`. Token expiry is `expiresIn` (epoch millis
or duration) and refresh uses the `refreshToken`/`subscriberId` body.

These contracts are enabled only inside the isolated diagnostic beta. They
were recovered statically and still need runtime request/response proof. The
OTP implementation follows the app's recovered payload construction, including
`loginOption=OTP`, the pending JWT, selectable LIT/DTH `refId` values, and
the observed `refIdMode` behavior. Analytics IDs are sent as empty strings
because no backend equivalent exists and the app does not prove they must be
non-empty.

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
