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
disable_auth = false
debug = false
log_to_stdout = true
default_categories = []
default_languages = []
```

Save, start with `openstream --config ./openstream.toml serve`, then open `http://localhost:5001/`.

## Complete configuration table

| TOML key | Environment variable | Default | Purpose and accepted values |
| --- | --- | --- | --- |
| `epg` | `OPENSTREAM_EPG` | `false` | Boolean. Generate/refresh the XMLTV guide. |
| `debug` | `OPENSTREAM_DEBUG` | `false` | Boolean. Show more diagnostic output. |
| `disable_ts_handler` | `OPENSTREAM_DISABLE_TS_HANDLER` | `false` | Boolean. Avoid local TS/AAC segment proxying when supported; change only when diagnosing HLS. |
| `disable_logout` | `OPENSTREAM_DISABLE_LOGOUT` | `false` | Boolean. Hide/disable JioTV logout when true. |
| `drm` | `OPENSTREAM_DRM` | `true` | Boolean. Allow the protected DASH pathway; does not bypass DRM. |
| `tv_plus` | `OPENSTREAM_TVPLUS` | `false` | Boolean. Enable the optional JioTV+ account. Needs a JioFiber or AirFiber login. |
| `disable_auth` | `OPENSTREAM_DISABLE_AUTH` | `false` | Boolean. Disable app access checks. Dangerous on reachable networks; use only with a trusted external access control. |
| `title` | `OPENSTREAM_TITLE` | `"OpenStream TV"` | Display title; any TOML string. |
| `disable_url_encryption` | `OPENSTREAM_DISABLE_URL_ENCRYPTION` | `false` | Boolean. Disable encryption of local proxy parameters. Leave false. |
| `proxy` | `OPENSTREAM_PROXY` | `""` | Optional outgoing proxy URL. |
| `path_prefix` | `OPENSTREAM_PATH_PREFIX` | `""` | Directory for OpenStream TV state. Empty uses the runtime's resolved default. |
| `log_path` | `OPENSTREAM_LOG_PATH` | `""` | Reserved in this build; do not rely on file logging. |
| `log_to_stdout` | `OPENSTREAM_LOG_TO_STDOUT` | `true` | Boolean. Emit logs to process output. |
| `custom_channels_file` | `OPENSTREAM_CUSTOM_CHANNELS_FILE` | `""` | Optional path to custom-channel JSON file. |
| `default_categories` | `OPENSTREAM_DEFAULT_CATEGORIES` | `[]` | Integer category IDs; environment form is comma-separated. |
| `default_languages` | `OPENSTREAM_DEFAULT_LANGUAGES` | `[]` | Integer language IDs; environment form is comma-separated. |

Boolean environment values must be `true` or `false` in lowercase (as Rust's parser expects). Example: `OPENSTREAM_EPG=true openstream serve`. For integer lists, use `OPENSTREAM_DEFAULT_LANGUAGES=1,6`.

The TOML key is `tv_plus` and the environment variable is `OPENSTREAM_TVPLUS`. The previous TOML key `extras` and legacy environment variable `JIOTV_EXTRAS` are still accepted, so an older config file or unit file keeps working; `OPENSTREAM_TVPLUS` takes precedence when both are set. `OPENSTREAM_TUNNEL_TOKEN` is also accepted for the optional Cloudflare tunnel flag. Update uses `OPENSTREAM_UPDATE_TOKEN` for private repository access, when required.

For runtime listening options, see [CLI commands](../configuration-cli/index.md); for protection see [Network and HTTPS](../configuration-network-security/index.md); for storage see [Data and logs](../configuration-storage-logs/index.md).
