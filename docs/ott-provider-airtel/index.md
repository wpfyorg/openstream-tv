---
title: Airtel Xstream Beta Provider Evidence
description: Evidence-backed Airtel Xstream login, catalogue, EPG and HLS/DASH playback contracts and what the provider implements.
category: Provider Reference
keywords: [Airtel, Xstream, provider, Wynk, Accedo, Widevine, EPG, OTP]
---

# Beta OTT provider: Airtel Xstream (`tv.accedo.airtel.wynk` 1.124.6)

Evidence record for `Airtel Xstream Play.apk`. The provider is off by default and
needs an Airtel mobile number, because the catalogue, EPG and playback calls only
return data for a signed-in account.

## 1. Artifact identity (observed fact)

- `Airtel Xstream Play.apk`, SHA-256
  `f5b414f6d7148acac66523c77d515922431e8b105b60fca012f17e287e242b2b`,
  98,244,105 bytes.
- `package="tv.accedo.airtel.wynk"`, `versionName="1.124.6"`,
  `versionCode=13164`, minSdk 24 / targetSdk 35.
- Layers: Accedo UI (`tv.accedo.airtel.wynk`), shared middleware SDK
  (`tv.accedo.wynk.android.airtel`), `com.wynk.*`, `tv.airtel.*`, ExoPlayer 1.4.1
  with `PlayerDRMCallback`.

## 2. Service identity (observed static)

Hosts from `com/shared/commonutil/environment/Environment.java` (production
block):

| Purpose | Host |
|---|---|
| Middleware (login, sessions) | `https://api.airtel.tv/` |
| Catalogue and EPG | `https://epg.airtel.tv/` |
| Playback and license | `https://play.airtel.tv/` |
| App config, content detail | `https://content.airtel.tv/` |
| Layout, packages, search | `layout.`, `package.`, `cms.`, `search.airtel.tv` |

`Constants.APP_ID = "MOBILITY"` is sent as `appId` on every request.
`DEVICE_TYPE_VALUE = "A3EN1JAJB9JB0C"`, `VALUE_DEVICE_KEY = "APADdd8231gg"`.

## 3. Request signing (observed static)

`Utility.getSignatureString` and `Signature.calculateRFC2104HMAC` build:

```
signed = METHOD + encodedPath + ("?" + encodedQuery when present) + body
x-atv-utkn = uid + ":" + base64(HMAC-SHA1(signed, key = login token))
```

`MiddlewareRequestInterceptor` adds `appId` to every request, appends the
account's content properties (`cp`, `cl`, `ln`, …) as query parameters on the
catalogue, EPG, package and content hosts, and adds `x-atv-traceid`
(`uuid-epochMillis`), `x-atv-segment`, `x-atv-ab`, `x-atv-adid`,
`x-atv-did` (`deviceId:deviceType:platform:osVersion:versionCode:versionName:deviceName:deviceModel`)
and `Content-Type: application/json`.

Before login it signs with the shipped fallback identity
`v72UE-WQrmdaed_gi53xZLV5pa02` / `FKmr3XQti5C8WyRi4tRNgzgVeMk=`, which reaches
`content.airtel.tv` but not the account data.

## 4. Login (partly verified live)

| Step | Contract |
|---|---|
| Send OTP | `POST https://api.airtel.tv/v2/user/profile/generateOtp` JSON `{msisdn:"+91XXXXXXXXXX", msgTxt}` |
| Sign in | `POST https://api.airtel.tv/v8/user/login` (falls back to `v5/user/login`) JSON `{deviceType, deviceKey:"APADdd8231gg", requireOtp:"false", msisdn, otp, wifi:"1"}` with `x-atv-did`, `x-atv-imsi`, `x-atv-network-type`, `x-atv-last-purge` |
| Refresh | `POST https://api.airtel.tv/v2/user/session/refreshAuthToken`; a rotated `token` means every signed call must be retried |

The login response carries `uid`, `token` (the signing key), `authToken` (sent
as `x-atv-stkn` for playback), `success` and `userConfig.userContentProperties`,
which becomes the `cp`/`cl`/`ln` query parameters.

Verified live without an account: the OTP endpoints answer with Airtel's own
error JSON, and the fallback identity is rejected for account data, so no login
flow was completed during analysis. What was executed is recorded in section 7.

## 5. Catalogue, EPG and playback

| Operation | Contract |
|---|---|
| Channel list | `GET https://epg.airtel.tv/app/v3/content/channels` → `channels[]{id,title,channelNo,hd,lang,cpId,imgs}` |
| EPG | `GET https://epg.airtel.tv/app/v3/content/epg?startTime=&endTime=` (epoch millis) → `programGuide{channelId:[{startTime,endTime,title,subtitle,contentId,programType}]}` |
| Playback | `GET https://play.airtel.tv/v4/user/content/playback?contentId=&psl=&usl=` with `x-atv-cp`, `x-atv-stkn` → `playback{playUrl,headers,timeShift,safeLive}` plus top-level `drm` |

`playUrl` is a DASH manifest for protected channels and an HLS playlist
otherwise. The `drm` block carries `licenceUrl`, `headers`, `body` and
`licensePayloadKey`; `PlayerDRMCallback` posts the Widevine challenge to
`licenceUrl` with those headers, wraps the challenge in the vendor JSON body when
`licensePayloadKey` is not `"default"`, and base64-decodes the response field
named by `licensePayloadKey`.

## 6. What this provider implements

- Config `airtel = true` / `OPENSTREAM_AIRTEL=true`; off by default.
- `ost airtel login|logout`, plus the installer and OpenWrt option.
- `/api/airtel/{status,login/sendOTP,login/verifyOTP,logout,channels,epg/:id,play/:id}`.
- Channel catalogue cached 6 h with a 15 s retry backoff, published to the
  playlist and XMLTV guide; EPG window 6 h forward and 1 h back.
- HLS through `/render.m3u8`, DASH through the shared MPD proxy with the
  provider's license branch, and media headers forwarded on manifests, segments
  and license requests.

## 7. Executed evidence

| Call | Result |
|---|---|
| `GET https://content.airtel.tv/app/v1/config/appConfig?os=Android&bn=13164&appId=MOBILITY` with the fallback signature | HTTP 200, 244,320 bytes of config JSON, proving the signature scheme |
| `GET https://epg.airtel.tv/app/v3/content/channels?appId=MOBILITY` | HTTP 200 `{"etag":"d41d8cd98f00b204e9800998ecf8427e"}`, i.e. an empty catalogue without a session |
| `GET https://epg.airtel.tv/app/v3/content/epg?appId=MOBILITY` | HTTP 412 `{"success":false,"errorcode":"ATV001","errortitle":"Illegal Argument"}` |
| `GET https://api.airtel.tv/app/v3/content/channels` | HTTP 404 (wrong host) |
| `GET https://play.airtel.tv/v4/user/content/playback?contentId=…` | HTTP 404 for an unknown id |

## 8. Known gaps

- Only OTP login is implemented; QR (`v7/user/login`) and SSO
  (`v9/user/login`, partner SSO) are not.
- Legacy playback (`v2/user/content/playback`) is unused; the app prefers
  `v4/user/content/playback`.
- No live end-to-end playback was confirmed, because that needs an Airtel
  account and OTP.
- Channel logos come from the `imgs` map; unusual image shapes fall back to the
  generated placeholder.
