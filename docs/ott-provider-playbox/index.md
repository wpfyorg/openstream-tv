---
title: PlayboxTV Beta Provider Evidence
description: Evidence-backed PlayboxTV API contracts, beta capabilities, runtime gaps and verification notes.
category: Provider Reference
keywords: [PlayboxTV, beta, provider, OTP, live TV, DRM]
---

# Beta OTT provider: PlayboxTV for Android TV (`playboxtv.tv.android.in` 5.20.1/128)

Evidence-backed static record. No live calls. All account-shaped values
redacted. The first inventory had no JADX; section 3a records the contracts
recovered in the follow-up decompilation. Runtime confirmation is still
required for the items listed under "still unresolved".

## 1. Artifact identity (observed fact)

- File: `reverse-engineering/apps/playboxtv.tv.android.in_5.20.1-128_1arch_1dpi_24lang_….apkm`
  (APKMirror APKM v5, 33 files).
- SHA-256 (APKM): `570ca55d1d88f68a8f4dd9a7a9fd586682f8fa1dd6738b3b6623577d77f858f8`
- base.apk SHA-256: `91b849571ce8286e78326bc3d9f63cd1c74bc1f7643c22e26710b2db359e97c9`
  (108,764,233 B); arch split
  `split_config.armeabi_v7a.apk` SHA-256:
  `335b8f5360af75a532991609fc3ad8ffe6204d92699a9a53e75d706ed484e9db`.
- `info.json`: app `PlayboxTV for Android TV`, package
  `playboxtv.tv.android.in`, version `5.20.1` / code `128`, `armeabi-v7a`
  only, 320 dpi, minSdk 24 / targetSdk 36, leanback + leanback_standalone,
  24 locales.
- App class `com.pairip.application.Application`; launcher
  `playboxtv.tv.in.SplashScreenActivity`; Compose
  `playboxtv.tv.in.composeUI.PlayboxTVMainActivity`.

## 2. API bases (observed const-strings)

- 1st-party: `content-consume.playboxtv.in`, `user-api.playboxtv.in`,
  `ugc.playboxtv.in`, `playboxtv-vod.multitvsolution.com` (VOD thumbs).
- AWS API Gateway (ap-south-1, per-host `X-API-Key`): 12 hosts covering
  sports/EPG/lang, continue-watch, profile/avatars, Fancode, content SSO,
  app/IPTV customer, ZiggTV/hotel, livetv favorites/newrelease, Zee5
  playableURL, user SSO verify, payments, v2.
- CloudFront catalogue: `da1scxjqfmdtz.cloudfront.net` (pages v1/v3/v4/v5/v6),
  `d3veuncmu3hvmc.cloudfront.net` (v1 + search v3).
- 3rd-party content: `*.hotstar.com` (api/apix/hf-apix/pp-apix/bifrost-api),
  `apps/integration3/integration4.skie.tv`, `commerce-services.oneplay.in`.

## 3. Endpoint paths (observed)

- Catalogue/pages: `/v1/page/{movie,tv,distrotv,banner,…}`,
  `/v3/page/{livetv2,ufolivetv,sports,partner/list,premiumchannels}`,
  `content/{content}/detail`, `page/number/{pageName}`, `search/{name}`,
  `/v3/search[/livetv]`, `content/newrelease`, `language_preference[/{phone}]`.
- Live/EPG/sports: `channel/weekly_epg/`, `channel/epg`,
  `live/streamingURL`, `livetv/ads/setup/{appID}`,
  `content/livetv/{favorites,view,view/{phone}}`, `sports/…`,
  `sports/fancode/playable`, `sports/playable[Url]`, `livesports[/all]`.
- Playback resolve: `POST …/consume/v1/playback/resolve`, `…/playable-new-flow`,
  `…/consume/v1/partner-token/ltv/jio/sso`; `play/v5/playback/content/`;
  `v2/channel/zee5/playableURL`.
- Auth/SSO: `…/prod/v1/user/auth/sso/verify`,
  `…/prod/v1/content/auth/sso/verify`, `customer/email/login`,
  `customer/verify`, `api/player/login|status|verify/otp` (skie.tv),
  `play/v1/partner/login`, `cdcustomer/assign`.
- Subs: `user-api.playboxtv.in/user/subscription`, `/user/v1[/continue/watch…]`,
  `user/{phone}/ott/{ottid}/checkOTTSubscription`,
  `customerott/customer/{phoneNo}`, `customer/{phone}/hasActivePacks`,
  `list/active/ott/plan/{phone}`, `ott/track/user`.

## 3a. Verified wire contracts (JADX 1.5.6, `base.apk`)

JADX was available for this pass (it was not for the first inventory), so the
service interfaces and models below are decompiled observations, not guesses.
Classes: `playboxtv.tv.in.api.{Api,LoginAPIService,PlayableAPIService}` and the
models under `playboxtv.tv.in.model`.

| Operation | Verified contract |
|---|---|
| OTP send | `POST https://h6el578034.execute-api.ap-south-1.amazonaws.com/prod/v2/communication`, form `appCode=play`, `mobileNo`, `messageId=10`, `countryCode=in`, header `X-API-Key` (communication key) |
| OTP verify | `POST …/prod/v2/communication/otp/verify`, form `appCode=play`, `mobileNo`, `otp`; response `OTPRequest{statusCode,data.token}`; success is `statusCode==200`, token saved from `data.token` |
| Catalogue | `GET https://da1scxjqfmdtz.cloudfront.net/v3/page/livetv2?langs=hi&identifier=1&countryCode=in`, header `X-API-Key`; response `ImageSLider{page.body.rows[].items[],livetv[],data[]}`; item fields `id,name,typeId,media.thumbnail,text.title` |
| Playback (primary) | `POST https://content-consume.playboxtv.in/consume/v1/playback/resolve`, `Authorization: Bearer <data.token>`, JSON `PlayRequestNew{contentId,typeId,deviceId,platform:"androidtv",pageId:"0",appVersion,isEncrypted:false}`; response `PlayContentRespModel{code,data.url{playable,licenceUrl,token,headers}}` |
| Playback (fallback) | `PUT https://e2vqkprvpg.execute-api.ap-south-1.amazonaws.com/prod/v2/playableURL`, form `phone,contentId,type=liveTV,typeid,deviceId,identifier=1,hasSubscription`; same `url` model |
| Subscription | `GET https://bk2zz20m15.execute-api.ap-south-1.amazonaws.com/prod/v1/customer/{phone}/hasActivePacks`, header `X-API-Key`; response `ActivePacks{active,bundleActive,statusCode}` |
| Media headers | `url.headers.user_id`, `User-Agent`, `subscriber_id`; `url.token` → `Cookie` when it contains `CloudFront-`, else `Authorization: Bearer` (from `PlayerActivity.preparePlayerApp$lambda$3`) |
| DRM | Widevine `MediaItem.DrmConfiguration` with `url.licenceUrl` and multi-session; license request headers `token`, `X-AxDRM-Message`, `pallycon-customdata-v2`, `nv-authorizations`, `Authorization: Bearer`, `Cookie` from `url.token` (`buildCustomDrmSessionManager`) |
| EPG | `GET https://4ffc6rlckj.execute-api.ap-south-1.amazonaws.com/prod/v1/channel/weekly_epg/{channel_id}/details`, header `X-API-Key`; response `{day, program:[{name,description,startTime}]}` — **no stop time**, so XMLTV programmes cannot be produced |
| Device id | `Settings.Secure.ANDROID_ID` (16 hex chars) from `CheckPackage.getDeviceID` |
| Package constants | `identifier="1"` and `countryCode="in"` for `playboxtv.tv.android.in`; `appCode="play"` from `PlayEvent.TYPE`; default `langs="hi"` |

The two gateway keys above are client-embedded public constants, not account
credentials; account tokens are only ever read from a successful login and are
never logged or returned by this backend.

### Playbox: still unresolved

- Token lifetime/refresh: the app ships no refresh endpoint; a session is
  replaced by signing in again.
- Whether the server uses `playback/resolve` or the legacy `playableURL` for a
  given account is chosen by a remote flag; both are verified and implemented
  as primary + fallback.
- Exact catalogue server behaviour (which of `rows`/`livetv` is populated for a
  given account) — the backend walks both observed lists.
- VOD/search/show models were not promoted because their playback semantics
  were not traced end-to-end.

## 4. Auth flow (observed UI + endpoints; mapping is inference)

- Mobile-number + 6-digit OTP UI (`OTPRequest`, `LoginViewModel$getOTPfromAPI`,
  `OTPVerifyViewModel$verifyOTPAPI`), email login, SSO-verify endpoints,
  `partnerToken`/`authToken` fields, `PREF_ASSIGN_JIO_HOTSTAR_*`,
  `callUserSSO authToken`.
- Inference: OTP → session token → per-partner SSO assignment
  (`assignJioHotstar`) → partner tokens for `playback/resolve`. No
  `Bearer`/`Basic` const-strings; tokens likely in body/custom headers.

## 5. Device identity (observed)

- `AdvertisingIdClient` (Ad ID), `android_id`, Firebase install/auth-token,
  FCM `PlayboxTVFcmMessageListenerService`. No Jio-style device-auth bootstrap.

## 6. Signing / keys / encryption (values redacted)

- ~13 distinct `X-API-Key` values (21 occurrences) = per-gateway client keys.
- `assets/decrypt.js`: AES-256-CBC + PBKDF2-SHA256×1989 (WebView in-app
  browser). Amazon Appstore `assets/*.pem`. Cleartext permitted globally;
  user certs trusted. Room/ObjectBox unencrypted. No EncryptedSharedPreferences.

## 7. Subscription (observed)

- `CheckSubscriptionRespModal`, `SubscriptionHelper`, `ActivePack*`,
  payment initiate/complete, Amazon IAP, per-OTT `checkOTTSubscription`,
  per-item `Subscription` flags. Server-side entitlement; client paywalls.

## 8. Catalogue / EPG / search / VOD (observed)

- Live: `LiveTVAPIService`, `JioLiveTvPlayerActivity` (Jio LTV via
  partner-token SSO), `DistroTVActivity`, `UFOFanTVActivity`,
  `page/livetv2`, `ufolivetv`.
- EPG: `weekly_epg`, `channel/epg`, `EpgPlayRequest`, `NextEpg`; embedded
  `com/example/mytv` SDK (`EPGRepository`, `LiveRepository`).
- Search: `SearchAPIService`, CloudFront `/v3/search`, per-provider
  subscription flags.
- VOD/shows: `MoviesListAPIService`, `TVShowsAPIService`, seasons/episodes,
  continue-watch + watchlist, offline downloads DB (licence/offlineDrmId).

## 9. Playback + DRM (observed)

- Media3 ExoPlayer (incl. Hotstar `jhs.media` fork + `TokenRefreshInterceptor`);
  FFmpeg/dav1d; `PlayContentRespModel = {Url, Provider, Headers, …,
  ADS/Preroll/Midroll/Postroll/ScteMarker, Subtitle, NextEpg/NextEpisode}`.
- Widevine L1/L3 config (`forceL3widevine`, `widevineSecurityLevel`),
  MediaDrm sessions, offline licence persistence. **No license-server URL
  const-string** (likely inside `playback/resolve` responses — runtime gap).

## 10. Refresh / retry (observed)

- Hotstar SDK token refresh + heartbeat retry; app `refreshData`,
  `refreshContinueWatching`; player Retry/Go-back CTAs. No app OAuth
  refresh-token strings.

## 11. Runtime gaps (must not be claimed statically)

Retrofit base-URL↔service mapping, request/response schemas, token header
names, refresh cadence, actual HLS/DASH/license URLs, Jio SSO shape, catch-up
(no catch-up strings found). Backend implements tolerant parsers + mock-tested
contracts and marks schemas as candidates.
