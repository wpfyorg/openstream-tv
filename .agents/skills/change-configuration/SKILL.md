---
name: change-configuration
description: Adds or changes OpenStream TV configuration options consistently across the Rust schema, OPENSTREAM_* environment variables, compatibility aliases, documentation, consumers, and tests. Use when adding, renaming, removing, defaulting, or changing a runtime configuration option.
---

# Change Configuration

Treat each runtime option as one contract exposed through the Rust `Config` type, an optional TOML file, and environment variables.

## Trace before editing

1. Find the field in `src/config.rs::Config` and every consumer of that field.
2. Establish precedence and semantics for omitted, zero, empty, and explicitly false values.
3. Check whether the option affects startup, persistent paths, handlers, generated playlists, installers, OpenWrt, Docker, or browser behavior.

Do not add a setting when existing behavior or an existing option already satisfies the deployment need.

## Update the contract

- Add or update the field in `src/config.rs::Config`, its `Default` value, and any Serde rename/alias needed for TOML compatibility.
- Primary environment names use `OPENSTREAM_*`. Keep a `JIOTV_*` fallback only when preserving an established legacy setting; the OpenStream name must win when both are present.
- File keys use snake case and are parsed as TOML. Automatic discovery still recognizes some legacy YAML/JSON filenames for migration, but the Rust parser only accepts TOML content.
- Keep field comments, documented defaults, environment parsing, and runtime defaults aligned.
- Update the consumer at the narrowest existing boundary. Do not read environment variables outside configuration loading when `Config` already owns the value.
- For a rename or removal, update consumers and documented names in the same change. Preserve old names only when migration compatibility is intentional.

## Synchronize user-facing surfaces

Update every applicable artifact:

- `docs/configuration-reference/index.md`
- `docs/configuration-cli/index.md` when the option changes startup or CLI behavior
- installer/deployment examples in `scripts/`, `openwrt/`, `docker/`, and documentation when users configure the option there
- `.agents/skills/diagnose-playback/SKILL.md` when the option changes playback diagnostics

Use `openstream.toml` in new examples. Do not add new YAML/JSON configuration examples for the Rust build.

## Prove behavior

Add focused Rust tests in `src/config.rs` for the changed contract:

- TOML parsing when the file key changes or has aliases.
- Primary `OPENSTREAM_*` environment parsing.
- Legacy `JIOTV_*` fallback when compatibility is retained.
- Precedence when both primary and legacy names are set.
- Omitted/default behavior and explicit zero, empty, or false behavior where meaningful.
- Consuming behavior separately when parsing success does not prove the feature.

Environment variables are process-global. Serialize tests that mutate them and restore values before the test exits.

Format the Rust files changed by the configuration work with `rustfmt`, then run focused tests, `cargo test`, and the slim build/test path when the setting is available there. Recheck the configuration reference before finishing.
