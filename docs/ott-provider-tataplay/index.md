---
title: Tata Play Smart Beta Provider Evidence
description: Recovered Tata Play Smart API contracts, live provider behaviour, runtime gaps and verification notes.
category: Provider Reference
keywords: [Tata Play Smart, provider, OTP, EPG, entitlement, DASH, Widevine]
---

# Beta OTT provider: Tata Play Smart

This record covers two app generations: the older `com.ryzmedia.tatasky`
509.0/894 installed on the A15, and the newer `com.tataplay.livetv` 1.0.10/101
artifact whose native Retrofit layer is recoverable.

> [!NOTE]
> This provider is compiled only into prerelease builds such as `v1.6.0-beta.2`. A stable release has no Tata Play routes, Settings card or `ost tataplay login` command. See [Install a beta build](../installations-beta/index.md).

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

### Live-verified contracts (2026-11-21)

The OTP, guide, entitlement and playback paths were verified against a live
LIT account. The details below replace the earlier runtime gaps:

- **Send OTP** (`POST login-service/pub/api/v3/signin/generate/otp`) needs the
  `device_details` header, a locale and an OkHttp user agent in addition to
  `appVersion`/`platform`/`deviceId`/`deviceName`. Without the device profile
  the edge answers a bare `403`.
- **Verify OTP** (`POST login-service/pub/api/v3/signin/validate/otp`) sends the
  chosen `refId` in both `refId` and `sid`. The app sets `sid` from the refId
  whenever the offered account lists a LIT or DTH service, which is every ref
  this client offers, so an empty `sid` is rejected.
- **Login response** has no top-level `subscriberId`: the session `sid` under
  `userDetails.sid` is the subscriber identity used by the device, access
  control and heartbeat endpoints. The same value is sent as `subscriberId`,
  `x-subscriber-id`, `sid` and `x-authenticated-userid`.
- **Catalogue**: `homescreen/.../hierarchy/LIT_LIVETV` returns an empty `data`
  array for this app generation, and `channels/all` carries only document ids.
  The authenticated guide is the catalogue source: it pairs every channel with
  its number, logo, HD/catch-up flags and pack ids.
- **Guide**: `lit-mobile-common-bff/api/v2/channels/guide` takes epoch
  **milliseconds** in `epgStartTime`/`epgEndTime` and an IST `date`. A page of
  200 channels carries programmes; a page of 530 returns metadata only. The
  client therefore reads metadata at 600 and programmes in 200-channel pages.
- **Entitlements**: `getEntitlementDetails` returns the held pack ids in
  `data.entitlements`; it is kept for diagnostics. The authenticated guide
  with `subscribedFilter=true` is the account lineup (356 of 530 on the live
  account). A second intersection with the held pack ids dropped zero rows and
  is not applied.
- **Playback**: `pas/api/v1/access/control/LIVE/<id>` returns a session id, a
  60-second heartbeat interval and AES-encrypted URLs under
  `data.playbackUrls`. Channels the account cannot stream return a null
  `playbackUrls` with a success code. All live media is Widevine DASH.
- **Media**: the decrypted manifest URL carries an Akamai `hdnea` token whose
  ACL covers the channel directory. Akamai accepts it as the `hdnea` cookie;
  the proxy sends both `hdnea` and the older `__hdnea__` name.
- **License**: the Irdeto Widevine endpoint requires the playback JWT from
  `data.tokenInfo.token` as `Authorization: Bearer <token>` on the license
  request. Without it the same challenge is answered
  `403 {"code":130001,"message":"License denied... [__anonymous]"}`. With it,
  the license carries the key id that matches the manifest PSSH; a 2-byte dummy
  challenge returns a license with a different key, which is why the token must
  accompany the real 2.5–4 KB challenge.
- **Per-channel detail**: `content-detail/pub/api/v1/lit/ls/channel/<id>`
  returns plain (unsigned) manifest/license URLs plus current, forward and
  backward programme rows. The unsigned URLs are not playable by themselves;
  access control remains the playback source.
- **Packs held by the test account**: `Tata Play Smart Base May26`,
  `DD Complimentary`, `FTA Complimentary HSM`, `Flexi Smart Add On English`,
  `FTA Complimentary Regi`; concurrency `1`.

With the guide encoding proven, XMLTV now publishes both days for the active
catalogue (356 channels, 27,456 programmes in the verification run) and the web
guide serves `/epg/<id>/<offset>` from the same cached days.

The proxy also rewrites the live MPD with `suggestedPresentationDelay="PT24S"`
because the origin's SegmentTimeline trails its own clock by a couple of
segments; without it Shaka asks for unpublished segments and loops.

Widevine needs more than the init segment's PSSH. The samples carry a CENC
`seig` sample group whose key id is the one the CDM actually decrypts with, and
it can differ from the init segment's `tenc` default key. The proxy probes the
newest small audio segment for that key id, merges it with the init PSSH key
ids, and injects one `cenc:pssh` into every AdaptationSet. Players then ask for
every key in a single challenge instead of one licence per rendition.

A real Widevine challenge spends the playback token, so a `130601` answer is
retried once with a fresh session and token. A `130402 Too many concurrent
streams for device type` is resent only when this server just closed a local
session. Every deliberate tune gets a playback id; manifest and licence routes
from an older tune are refused, including an older tune of the same channel.
Irdeto can keep the closed channel's licence stream counted for minutes (5m41s
in one run, about 2 minutes in another), so an immediate switch returns HTTP
409 `Waiting for the previous stream to close`; the web player shows that
message and retries automatically every five seconds for two minutes with a
countdown, then leaves the manual retry.

The M3U licence property uses Kodi's four-field form:
`<license-url>||R{SSM}|R`, with any `?q=` inside the URL before the pipes.

The subscribed guide is the single lineup for `/api/channels`, the M3U, the
XMLTV channel and programme entries, and direct web-guide lookups. A changed
lineup clears cached guide days; a direct `/epg/:id` for a channel outside the
lineup returns no rows. Playback failures do not change membership. Selecting
a custom or non-Tata channel cancels a pending Tata Play resolve and closes its
session, so a late resolve cannot take the account's slot or publish itself.

XMLTV generation retries a failed startup preparation from the next request,
at most once a minute, and refreshes a stale matching artifact in the
background.

The concurrency session's `POST`/`DELETE` must carry an OkHttp user agent
(`okhttp/4.12.0`). Without it Akamai answers `403 Access Denied`, the heartbeat
never starts, and the CDN freezes the live manifest at its first `publishTime`.


### Known gaps

- Catch-up and on-demand surfaces are not wired: `catchupEpg` and
  `myBox/epg` answered `Content does not exists` for the sampled channels, and
  the VOD rails/search contracts have not been proven live.
- Channel language is not present in the guide row, so playlists publish
  `All Languages` until a per-channel detail fetch is worth the extra calls.
- Subscribed feeds can be refused at play time while remaining in the lineup
  (for example Star Sports HD without its HD add-on, and Star Utsav/Colors
  Rishtey in the verification account). A null `playbackUrls` is not treated
  as an unsubscribe signal. Per-channel `content-detail` has platform flags,
  but a catalogue-wide probe of 356 channels tripped an Akamai 403 block and
  was removed.

### Playback sessions

`NetworkApi.X` opens a concurrency session with a bodyless
`POST smgs/api/v1/session`, carrying `x-device-name` and the `x-session-id`
returned by access control. The provider registers that session as soon as
playback resolves and repeats the call every `heartbeatInterval` seconds.
Because the account's concurrency slot is one, the previous channel's session
is closed before the next access-control call; otherwise the upstream answers
the new channel with HTTP 403. It
sends the bodyless `DELETE smgs/api/v1/session` when playback is invalidated
(a CDN rejection), when the provider is cleared on logout or account switch,
when another channel replaces the session, and after five minutes without a
media or license request. The server has no explicit player-stop signal, so a
channel that is merely closed ends through that idle timeout. When the response
omits `heartbeatInterval` the provider beats every 30 seconds.


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
