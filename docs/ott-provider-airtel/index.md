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

> [!NOTE]
> This provider is compiled only into prerelease builds such as `v1.6.0-beta.2`. A stable release has no Airtel routes, Settings card or `ost airtel login` command. See [Install a beta build](../installations-beta/index.md).

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

## 4. Login (static evidence only)

| Step | Contract |
|---|---|
| Send OTP | `POST https://api.airtel.tv/v2/user/profile/generateOtp` JSON `{msisdn:"+91XXXXXXXXXX", msgTxt}` |
| Sign in | `POST https://api.airtel.tv/v8/user/login` (falls back to `v5/user/login`) JSON `{deviceType, deviceKey:"APADdd8231gg", requireOtp:"false", msisdn, otp, wifi:"1"}` with `x-atv-did`, `x-atv-imsi`, `x-atv-network-type`, `x-atv-last-purge` |
| Refresh | `POST https://api.airtel.tv/v2/user/session/refreshAuthToken`; a rotated `token` means every signed call must be retried |

The login response carries `uid`, `token` (the signing key), `authToken` (sent
as `x-atv-stkn` for playback), `success` and `userConfig.userContentProperties`,
which becomes the `cp`/`cl`/`ln` query parameters.

The login contract above is taken from the app's own code and request
interceptor. No login flow was completed during analysis, and no OTP request was
executed for this record; section 7 lists only the calls that were actually run.

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

## 9. Live-service findings (2026-11-18)

Probed end to end with a signed-in, DTH-linked account (`dth=true`,
`isDth=true`, `mwTvPack=45003,25002`, `ut=broadband`), from both Airtel
(AS45609) and Jio (AS55836) egress.

| Call | Result |
|---|---|
| `POST api.airtel.tv/v2/user/profile/generateOtp`, colon-joined `x-atv-did` | HTTP 412 `ATV001` |
| same, pipe-joined `x-atv-did` | HTTP 200 `{"success":true,"message":"SMS sent"}` |
| `POST api.airtel.tv/v8/user/login` with that OTP | HTTP 200, `uid`/`token`/`authToken` issued |
| `GET epg.airtel.tv/app/v3/content/channels` | HTTP 200, 1 channel for this account |
| `GET livetv.airtel.tv/v1/livechannel` | HTTP 200, 783 linear channels (`LIVETV_LIVETVCHANNEL_*`, `lcn`, `title`, `images`, `free`, `streamUrlsAvailable`) |
| `GET livetv.airtel.tv/v1/epg?channelId=…` | HTTP 200, programme titles |
| `GET epg.airtel.tv/app/v2/content/channel/epg?channelId=…` | same guide, no auth needed |
| `GET play.airtel.tv/v4/user/content/playback` | HTTP 404, empty body, every id/appId/header set, both egress networks |
| `GET play.airtel.tv/v4/user/playback`, `/v5/user/playback` | HTTP 403 `ATV013` unauthenticated; `ATV400` with the account session |

The OTT catalogue (`app/v3/content/channels`) is account-filtered and carries
the OTT live set only; DTH channels live in the linear catalogue on
`livetv.airtel.tv`, which returns the master list. `channelNo` arrives as a
JSON string there and in the OTT list.

Open: the current playback route. `v4/user/content/playback`, the 1.124.6
contract, is retired on the live service (bare 404), and the surviving
`user/playback` routes reject every parameter set the shipped APK builds
(`ATV400`). Resolving it needs a capture of a current client's play request or
a newer APK than 1.124.6.

## 10. STB client and the IPTV playback contract (2026-11-20)

Evidence from the Airtel Xstream STB firmware for `UIW4078ATL` (Amlogic,
Android TV 14, build `14.46.046-251113`, signed A/B OTA). The image carries the
operator agent `product/priv-app/ICX` (`com.airtel.tv`, OTA/provisioning,
`wss://icxdc.airtelxstreamota.com:6500/api/v2/device`,
`content://com.airtel.tv.provider/customer`) and the client itself in
`system/priv-app/TvLauncher` — the only APK that references `play.airtel.tv`,
`x-atv-stkn`, `generateOtp` and `LIVETVCHANNEL`.

Environment block embedded in `TvLauncher` (production):

| Key | Value |
|---|---|
| `playbackhost` | `https://play.airtel.tv` |
| `iptvLinearPlayApiUrl` | `https://iptv-playback.airtel.tv/` |
| `linearHeartBeatApiUrl` | `http://iptvhb.airtel.com/` |
| `livetv` | `https://livetv.airtel.tv` |
| `livesmshost` | `https://sms-xstream.airtel.tv` |
| `masterIQApiUrl` / `playIQApiUrl` | `https://user.iqvideo.in` / `https://play.iqvideo.in/` |

Routes built by `data/repo/PlayerRepository`:

| Purpose | Request |
|---|---|
| VOD playback | `play.airtel.tv` + `v5/user/playback` |
| Linear (IPTV) playback | `iptv-playback.airtel.tv` + `playback/v2/info` |
| Linear heartbeat | `iptvhb.airtel.com` + `v1/network/validation` |
| Box identity | `v1/box/detail?chipId=&bsn=` with `x-box-auth-key`, `x-box-auth-value` |
| Device registration | `v2/user/profile/deviceData`, `v1/device/activate` |

`PlayerMiddlewareApi.linearStreamingCallback` (GET) sends `x-org-id`,
`x-app-id`, `x-os-id`, `x-version-id`, `x-atv-did`, `x-atv-chip-id`,
`x-atv-bsn`, `x-atv-dth-subscriber-id`, `x-group-name` with query `contentId`,
`sh`, `resolution`, `cdnId`.

Probed with a valid phone session: `GET /playback/v2/info` → `401 ATV013
"Request not authenticated"`, with and without empty box headers. The gate is
box identity, not a server secret: `BoxAuthDetails` derives it entirely on the
device.

```
scramble = matrix-shuffle(chipId + bsn + macId + time, rows = floor(sqrt(n)))
key      = AES-urlsafe("chipId|bsn|macId|time", "u9mH9s5T7S8sdfkJ3lK1lA==")
value    = "v=v1;s=" + base64url(HMAC-SHA256(scramble, key = str(time)))
```

The AES key is hardcoded in the APK, so both headers are reproducible off-box.
`getIptvChipId()` reads `readPropFromVmx("uniqueIdentifier")`, `getIptvBsnId()`
reads the matching vendor property, `macId` comes from the network interface.
What cannot be invented is which box is linked to the account, and
`v1/box/detail` resolves `chipId`/`bsn`/`macId` to the DTH `subscriberId` that
`playback/v2/info` requires as `x-atv-dth-subscriber-id`. A second gate is
likely: `linearHeartBeatApiUrl` is `iptvhb.airtel.com/v1/network/validation`,
i.e. the box's network is validated too.

Consequence: linear/DTH playback is reachable only with the identity of an STB
that is actually linked to the account — readable on that box over ADB, not
derivable from OTT credentials. Without the box, the OTT catalogue
(`app/v3/content/channels`) remains the only playable surface, and the
783-channel linear catalogue on `livetv.airtel.tv` stays display-only.




## 11. Working linear playback (2026-11-20)

The `v3` route plus three transport details make live channels play without
the STB, but only for partners the play API itself serves.

| Change | Evidence |
|---|---|
| `x-atv-did` must be pipe-joined | colon-joined → `412 ATV202`/`ATV001`; pipe-joined → `200` |
| `livetv.airtel.tv` 404s any request advertising gzip | `accept-encoding: gzip` → `404` (121 B); `identity` → `200` (797,843 B) |
| playback is `v3/user/content/playback` | `v4/user/content/playback` → bare `404` |
| `x-atv-cp` must be the channel's partner | no cp → `ATV030 Content Partner is invalid`; `MWTV` → `200` |
| Airtel's own live pack is STB-only | `LIVETV` → `ATV001` on the v3 route, `iptv: true` in the catalogue |

`PLAYABLE_LINEAR_CPS` therefore starts as `["MWTV"]`: those five channels
resolve to a direct HLS URL with `isDrm: false` and play through the normal
proxy. Everything else in the 783-channel linear catalogue answers `ATV030`
(partner not served by the play API) or `ATV001` (Airtel live pack), and the
`iptv-playback.airtel.tv` host remains box-gated.

End-to-end check: `/live/airtel_MWTV_LIVETVCHANNEL_10000000063500000.m3u8`
returns the HLS master, the variant playlist and a 3.4 MB segment.
