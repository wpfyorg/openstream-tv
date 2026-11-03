---
title: Configuration Reference
description: Verified TOML keys, OpenStream environment variables, defaults and accepted values.
category: Configuration
keywords: [config, TOML, OPENSTREAM, environment variables, defaults, debug, tv_plus]
---

# Configuration Reference

OpenStream TV reads an optional **TOML** file and environment variables. Environment variables override the file. The Rust build only parses TOML, even though its automatic filename search also sees YAML/JSON names. A YAML or JSON file at a matching name will cause a parse error; convert it to TOML or use environment variables.

Explicit path: `openstream --config /path/to/openstream.toml serve`. The parser also accepts `openstream serve --config /path/to/openstream.toml`.

If `--config` is omitted, it searches the current working directory, in this order: `openstream.toml`, `openstream.yml`, `openstream.yaml`, `openstream.json`, legacy `jiotv_go.yml`, `jiotv_go.yaml`, `jiotv_go.toml`, `jiotv_go.json`, then `config.json`, `config.yml`, `config.toml`, `config.yaml`. Only choose a `.toml` filename.

## Example

```toml
# openstream.toml (sample values; no credentials)
title = "Home TV"
epg = true
drm = true
tv_plus = false
fastway = false
disable_auth = false
debug = false
log_to_stdout = true
default_categories = []
default_languages = []
```

Save, start with `openstream --config ./openstream.toml serve`, then open `http://localhost:5001/`.

## Complete configuration table

| TOML key | Environment variable(s) | Default | Purpose and accepted values |
| --- | --- | --- | --- |
| `epg` | `JIOTV_EPG`, `OPENSTREAM_EPG` | `false` | Boolean. Generate/refresh the XMLTV guide. |
| `debug` | `JIOTV_DEBUG`, `OPENSTREAM_DEBUG` | `false` | Boolean. Show more diagnostic output. |
| `disable_ts_handler` | `JIOTV_DISABLE_TS_HANDLER`, `OPENSTREAM_DISABLE_TS_HANDLER` | `false` | Boolean. Avoid local TS/AAC segment proxying when supported; change only when diagnosing HLS. |
| `disable_logout` | `JIOTV_DISABLE_LOGOUT`, `OPENSTREAM_DISABLE_LOGOUT` | `false` | Boolean. Hide/disable JioTV logout when true. |
| `drm` | `JIOTV_DRM`, `OPENSTREAM_DRM` | `true` | Boolean. Allow the protected DASH pathway; does not bypass DRM. |
| `tv_plus` | `JIOTV_TVPLUS`, `OPENSTREAM_TVPLUS` | `false` | Boolean. Enable the optional JioTV+ account. Needs a JioFiber or AirFiber login. |
| `fastway` | `JIOTV_FASTWAY` | `false` | Boolean. Enable the optional Fastway Live TV provider. Connect an account with `openstream fastway login` or Settings before channels appear. |
| `disable_auth` | `JIOTV_DISABLE_AUTH`, `OPENSTREAM_DISABLE_AUTH` | `false` | Boolean. Disable app access checks. Dangerous on reachable networks; use only with a trusted external access control. |
| `title` | `JIOTV_TITLE`, `OPENSTREAM_TITLE` | `"OpenStream TV"` | Display title; any TOML string. |
| `disable_url_encryption` | `JIOTV_DISABLE_URL_ENCRYPTION`, `OPENSTREAM_DISABLE_URL_ENCRYPTION` | `false` | Boolean. Disable encryption of local proxy parameters. Leave false. |
| `proxy` | `JIOTV_PROXY`, `OPENSTREAM_PROXY` | `""` | Optional outgoing proxy URL. |
| `path_prefix` | `JIOTV_PATH_PREFIX`, `OPENSTREAM_PATH_PREFIX` | `""` | Directory for OpenStream TV state. Empty uses the runtime's resolved default. |
| `log_path` | `JIOTV_LOG_PATH`, `OPENSTREAM_LOG_PATH` | `""` | Reserved in this build; do not rely on file logging. |
| `log_to_stdout` | `JIOTV_LOG_TO_STDOUT`, `OPENSTREAM_LOG_TO_STDOUT` | `true` | Boolean. Emit logs to process output. |
| `custom_channels_file` | `JIOTV_CUSTOM_CHANNELS_FILE`, `OPENSTREAM_CUSTOM_CHANNELS_FILE` | `""` | Optional path to custom-channel JSON file. |
| `default_categories` | `JIOTV_DEFAULT_CATEGORIES` | `[]` | Integer category IDs; environment form is comma-separated. |
| `default_languages` | `JIOTV_DEFAULT_LANGUAGES` | `[]` | Integer language IDs; environment form is comma-separated. |

Boolean environment values must be `true` or `false` in lowercase (as Rust's parser expects). Use either the `JIOTV_*` environment variable or its documented `OPENSTREAM_*` alias where listed; the `OPENSTREAM_*` value takes precedence when both are set. Example: `OPENSTREAM_EPG=true openstream serve`. Integer-list settings use `JIOTV_DEFAULT_LANGUAGES=1,6`.

The TOML key is `tv_plus`; `JIOTV_TVPLUS` and `OPENSTREAM_TVPLUS` are accepted, with `OPENSTREAM_TVPLUS` taking precedence. The previous TOML key `extras` and legacy `JIOTV_EXTRAS` variable are also accepted. Both `JIOTV_*` and `OPENSTREAM_*` aliases are accepted for tunnel and update tokens.

When more than one alternate source is connected, Fastway takes priority over JioTV+, and JioTV+ takes priority over the JioTV catalogue. See [Connect Fastway Live TV](../account-fastway/index.md).

For runtime listening options, see [CLI commands](../configuration-cli/index.md); for protection see [Network and HTTPS](../configuration-network-security/index.md); for storage see [Data and logs](../configuration-storage-logs/index.md).
