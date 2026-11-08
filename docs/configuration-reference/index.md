---
title: Configuration Reference
description: Verified TOML keys, OpenStream environment variables, defaults and accepted values.
category: Configuration
keywords: [config, TOML, OPENSTREAM, environment variables, defaults, debug, tv_plus]
---

# Configuration Reference

OpenStream TV reads an optional **TOML** file and environment variables. Environment variables override the file. The Rust build only parses TOML, even though its automatic filename search also sees YAML/JSON names. A YAML or JSON file at a matching name will cause a parse error; convert it to TOML or use environment variables.

Explicit path: `ost --config /path/to/openstream.toml serve`. The parser also accepts `ost serve --config /path/to/openstream.toml`.

If `--config` is omitted, it searches the current working directory, in this order: `openstream.toml`, `openstream.yml`, `openstream.yaml`, `openstream.json`, legacy `jiotv_go.yml`, `jiotv_go.yaml`, `jiotv_go.toml`, `jiotv_go.json`, then `config.json`, `config.yml`, `config.toml`, `config.yaml`. Only choose a `.toml` filename.

## Example

```toml
# openstream.toml (sample values; no credentials)
title = "Home TV"
epg = true
drm = true
tv_plus = false
fastway = false
# Beta: PlayboxTV account (OTP login). Off unless you sign in.
playbox = false
# Beta: Tata Play Smart diagnostics. Off by default.
tataplay = false
# Beta: VZY free tier (anonymous). Off unless you want its catalogue.
vzy = false
disable_auth = false
debug = false
log_to_stdout = true
# Set only if an HTTPS reverse proxy or named tunnel publishes this hostname.
# public_origin = "https://tv.example.com"
default_categories = []
default_languages = []
```

Save, start with `ost --config ./openstream.toml serve`, then open `http://localhost:5001/`.

## Complete configuration table

| TOML key | Environment variable(s) | Default | Purpose and accepted values |
| --- | --- | --- | --- |
| `epg` | `JIOTV_EPG`, `OPENSTREAM_EPG` | `false` | Boolean. Generate/refresh the XMLTV guide. A persisted `ost epg on`/`ost epg off` override takes precedence until changed again. |
| `debug` | `JIOTV_DEBUG`, `OPENSTREAM_DEBUG` | `false` | Boolean. Show more diagnostic output. |
| `disable_ts_handler` | `JIOTV_DISABLE_TS_HANDLER`, `OPENSTREAM_DISABLE_TS_HANDLER` | `false` | Boolean. Avoid local TS/AAC segment proxying when supported; change only when diagnosing HLS. |
| `disable_logout` | `JIOTV_DISABLE_LOGOUT`, `OPENSTREAM_DISABLE_LOGOUT` | `false` | Boolean. Hide/disable JioTV logout when true. |
| `drm` | `JIOTV_DRM`, `OPENSTREAM_DRM` | `true` | Boolean. Allow the protected DASH pathway; does not bypass DRM. |
| `tv_plus` | `JIOTV_TVPLUS`, `OPENSTREAM_TVPLUS` | `false` | Boolean. Enable the optional JioTV+ account. Needs a JioFiber or AirFiber login. |
| `fastway` | `JIOTV_FASTWAY`, `OPENSTREAM_FASTWAY` | `false` | Boolean. Enable the optional Fastway Live TV provider. Connect an account with `ost fastway login` or Settings before channels appear. |
| `playbox` | `OPENSTREAM_PLAYBOX`, `JIOTV_PLAYBOX` | `false` | **Beta.** Enable the PlayboxTV account. Sign in with `POST /api/playbox/login/sendOTP` then `POST /api/playbox/login/verifyOTP`; the channel catalogue and live playback then use the normal `/live/...` routes. |
| `tataplay` | `OPENSTREAM_TATAPLAY`, `JIOTV_TATAPLAY` | `false` | **Beta.** Enable the Tata Play Smart diagnostic API. It can test OTP/session, catalogue document IDs, EPG, entitlement and official access-control. It does not join the shared catalogue or `/live`, and never returns upstream play/license URLs or session tokens. |
| `vzy` | `OPENSTREAM_VZY`, `JIOTV_VZY` | `false` | **Beta.** Enable the VZY (Dish/Watcho white-label) **free tier**. It uses an anonymous Kaltura session; no account or OTP is needed. Only `EntitlementType=Free` channels are exposed. Premium/login-required channels are not advertised. |
| `disable_auth` | `JIOTV_DISABLE_AUTH`, `OPENSTREAM_DISABLE_AUTH` | `false` | Boolean. Disable app access checks. Dangerous on reachable networks; use only with a trusted external access control. |
| `title` | `JIOTV_TITLE`, `OPENSTREAM_TITLE` | `"OpenStream TV"` | Display title; any TOML string. |
| `disable_url_encryption` | `JIOTV_DISABLE_URL_ENCRYPTION`, `OPENSTREAM_DISABLE_URL_ENCRYPTION` | `false` | Boolean. Disable encryption of local proxy parameters. Leave false. |
| `proxy` | `JIOTV_PROXY`, `OPENSTREAM_PROXY` | `""` | Optional outgoing proxy URL. |
| `public_origin` | `OPENSTREAM_PUBLIC_ORIGIN`, `JIOTV_PUBLIC_ORIGIN` | `""` | Trusted external HTTPS origin (for example `https://tv.example.com`). Must have no path, credentials, query or fragment. Used when the request's Host matches. LAN HTTP and direct TLS remain unchanged. |
| `path_prefix` | `JIOTV_PATH_PREFIX`, `OPENSTREAM_PATH_PREFIX` | `""` | Directory for OpenStream TV state. Empty uses the runtime's resolved default. |
| `log_path` | `JIOTV_LOG_PATH`, `OPENSTREAM_LOG_PATH` | `""` | Reserved in this build; do not rely on file logging. |
| `log_to_stdout` | `JIOTV_LOG_TO_STDOUT`, `OPENSTREAM_LOG_TO_STDOUT` | `true` | Boolean. Emit logs to process output. |
| `custom_channels_file` | `JIOTV_CUSTOM_CHANNELS_FILE`, `OPENSTREAM_CUSTOM_CHANNELS_FILE` | `""` | Optional path to custom-channel JSON file. |
| `default_categories` | `JIOTV_DEFAULT_CATEGORIES`, `OPENSTREAM_DEFAULT_CATEGORIES` | `[]` | Integer category IDs; environment form is comma-separated. |
| `default_languages` | `JIOTV_DEFAULT_LANGUAGES`, `OPENSTREAM_DEFAULT_LANGUAGES` | `[]` | Integer language IDs; environment form is comma-separated. |

Boolean environment values must be `true` or `false` in lowercase (as Rust's parser expects). Use either the `JIOTV_*` environment variable or its documented `OPENSTREAM_*` alias where listed; the `OPENSTREAM_*` value takes precedence when both are set. Example: `OPENSTREAM_EPG=true ost serve`. If `ost epg off` has stored an override, that environment variable alone does not re-enable EPG; run `ost epg on` to replace the persisted override. Integer-list settings use `JIOTV_DEFAULT_LANGUAGES=1,6`.

The TOML key is `tv_plus`; `JIOTV_TVPLUS` and `OPENSTREAM_TVPLUS` are accepted, with `OPENSTREAM_TVPLUS` taking precedence. The previous TOML key `extras` and legacy `JIOTV_EXTRAS` variable are also accepted. Both `JIOTV_*` and `OPENSTREAM_*` aliases are accepted for tunnel and update tokens.

JioTV, JioTV+ and Fastway are stable providers; PlayboxTV, VZY and Tata Play are beta. The provider selected in **Settings** is the only catalogue exposed. When no selection is stored, or the selected provider is disconnected, the server falls back in the order **Fastway > JioTV+ > PlayboxTV > VZY > JioTV**. See [Connect Fastway Live TV](../account-fastway/index.md).

`playbox` / `OPENSTREAM_PLAYBOX` (legacy alias `JIOTV_PLAYBOX`) enables the **beta** PlayboxTV integration. When a PlayboxTV session is signed in it can be selected as the active provider in Settings; only one provider catalogue is exposed at a time. Its channel IDs start with `pb_`. EPG for PlayboxTV is not published because the client's guide response has no programme end time.

`vzy` / `OPENSTREAM_VZY` (legacy alias `JIOTV_VZY`) enables the **beta** VZY free tier. It signs in anonymously to the Dish/Watcho Kaltura tenant, exposes only free channels (`vzy_*` IDs), and proxies their clear HLS/DASH. Select it as the active provider in Settings to use the free-tier catalogue. It does not implement account login, premium entitlement, or DRM license forwarding for paid channels.

`tataplay` / `OPENSTREAM_TATAPLAY` (legacy alias `JIOTV_TATAPLAY`) enables a deliberately isolated **beta** built from the recovered `com.tataplay.livetv` Retrofit contracts. Its routes live under `/api/tataplay/*`: OTP send/verify, refresh, catalogue seed, EPG, entitlement and sanitized access-control. The beta preserves Tata Play's own entitlement decision. Shared catalogue selection, `/live`, DRM/license proxying and session heartbeat remain disabled until runtime testers prove those flows.

For runtime listening options, see [CLI commands](../configuration-cli/index.md); for protection see [Network and HTTPS](../configuration-network-security/index.md); for storage see [Data and logs](../configuration-storage-logs/index.md).
