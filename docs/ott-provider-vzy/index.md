---
title: VZY Beta Provider Evidence
description: Evidence-backed VZY free-tier catalogue, EPG and playback behavior plus premium and DRM gaps.
category: Provider Reference
keywords: [VZY, beta, provider, Kaltura, free tier, EPG]
---

# Beta OTT provider: VZY (`com.vzy.one.atv` 1.0.253)

Evidence record for `VZY.zip`, a Dish/Watcho white-label OTT app.

## 1. Artifact identity (observed fact)

- `VZY/base.apk`, SHA-256 `7cd4724cd030bc0566d8732eba4961d1ba7d0170503b615ac5b4c0d1ce80b275`.
- `package="com.vzy.one.atv"`, app label `Vzy`, `versionName="1.0.253"`,
  `versionCode=4101253`, minSdk 24 / targetSdk 36, leanback.
- Splits: `armeabi_v7a`, `en`, `xhdpi`.
- Kotlin app (`com.horizon.sott.*`) with Kaltura PlayKit, Evergent
  (`com.enveu.*`), Kaltura OTT/Phoenix, HZ CMS, Nagra/Irdeto DRM and
  Broadpeak/Conviva; no React Native bundle.

## 2. Service identity (observed static + live)

`BuildConfig` identifies a Dish/Watcho white-label:

- Kaltura partner **5093**, `KALTURA_BASE_URL=https://5093.dsp1.ott.kaltura.com/api_v3/service/`
- `EPG_BASE_URL=https://cache.dsp1.ott.kaltura.com/api_v3/`
- `SSO_BASE_URL=https://vzy-sso.dish.watcho.com/v1/`
- `HZ_BASE_URL=https://hz-cms.dish.watcho.com/`,
  `PERSONALIZER_BASE_URL=https://personalizer-api.dish.watcho.com/`
- `DRM_BASE_URL=https://vzy-sso.dish.watcho.com/v1/`,
  `NAGARA_DRM_BASE_URL=https://drm-prod.dish.watcho.com/v1/`
- `KALTURA_APPLICATION_NAME=com.Kaltura.dishtv.androidTV`, platform `STB`,
  API version `11.0.2.1`.

## 3. Verified live contracts (no account, anonymous tier)

All calls below were executed anonymously and returned real data. No account
credentials, OTPs or tokens were used.

| Operation | Verified contract |
|---|---|
| DMS bootstrap | `POST …/api_v3/service/configurations/action/serveByDevice` JSON `{apiVersion:"11.0.2.1",applicationName:"com.Kaltura.dishtv.androidTV",clientVersion:"1.0.0",partnerId:5093,platform:"STB",tag:"",udid:"ATV_…"}`; returns `params.miniEPGCollectionId` (366800) |
| Anonymous session | `POST …/api_v3/service/ottuser/action/anonymousLogin` JSON `{partnerId:5093,udid,language:"en"}`; returns `result.ks` + `result.expiry` |
| Catalogue | `POST …/asset/action/list` with `KalturaChannelFilter{idEqual:366800,dynamicOrderBy:{name:"ChannelNumber",orderBy:"META_ASC"}}`, pager 200; **161 live channels** |
| Asset detail | `POST …/asset/action/get` `{id,assetReferenceType:"MEDIA"}`; returns `mediaFiles[].url`, `metas.DRMType`, `metas.EntitlementType` |
| EPG | `GET …/service/epg/action/get/date/{midnight-UTC}/slots/all?channels=…&pageIndex=1&pageSize=100&offsetInMin=…`, `Authorization: Bearer <KS>`; returns `epgChunk` with `startDate`/`endDate` |
| Free playback | free-tier channels return clear HLS (`…/hls_fta/….m3u8`) and/or DASH (`…/dash_fta/….mpd`); HLS manifest fetched with HTTP 200 without credentials |

### Free/entitlement boundary (observed)

Of 161 channels: 72 `JHSLicense/Premium`, 32 `Nagra/Premium`,
17 `Nagra/Free`, 14 `DynamicUrl/Premium`, 12 `Free/Free`,
12 `License/Premium`, 1 `DynamicUrl/LoginRequired`, 1 `Free/LoginRequired`.

The **29 `EntitlementType == "Free"`** channels are the implemented tier. Both
`Free/Free` and `Nagra/Free` verified as clear HLS; no license request is
needed for their `hls_fta` renditions. Premium/login-required channels are
never advertised or proxied.

## 4. Implemented

- Anonymous DMS bootstrap + Kaltura session, persisted `udid`/KS, expiry-aware
  re-login.
- Free-tier catalogue only, `vzy_<assetId>` IDs, logo mapping, channel lookup,
  playlist inclusion.
- Live playback resolution to the shared `LiveUrlOutput` (HLS preferred, DASH
  fallback), proxied through the existing encrypted `/live`/`/render` flow.
- EPG for free channels with real start/end times, integrated into XMLTV.
- Status, `/api/vzy/logout`, generation-guarded caches, context invalidation.

## 5. Disabled / unknown

- Account login (Evergent OTP, activation code/QR) is not enabled: no VZY
  account was supplied and no OTP was entered.
- Premium playback and Widevine/Nagra license forwarding are disabled: the
  anonymous session is not entitled, and the license/`DRM` chain was not
  runtime-proven. No entitlement bypass is attempted.
- Catch-up/start-over and VOD/login-only rails are not implemented.
