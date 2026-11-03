# Contributing to OpenStream TV

Thanks for helping improve OpenStream TV. Bug reports, documentation fixes, provider work, and code contributions are welcome.

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). For support and development discussion, join the [OpenStream TV Discord](https://discord.gg/VWDvwqTjY).

## Development setup

You need a stable Rust toolchain, Node.js LTS with npm, and Git.

```sh
git clone https://github.com/YOUR_USERNAME/openstream-tv.git
cd openstream-tv

cargo build
cargo test
cargo test --no-default-features --features slim

cd web/ui
npm ci
npm test
npm run build
```

The primary executable is `openstream`. The default build includes the Svelte UI; the slim build omits it.

For installer-related changes, also run:

```sh
sh scripts/tests/install-test.sh
sh scripts/tests/onboard-test.sh
sh scripts/tests/onboard-ps1-test.sh
sh scripts/tests/openstream-token-test.sh
```

`onboard-ps1-test.sh` requires `pwsh`; outside CI it skips when PowerShell is unavailable.

## Reporting bugs and proposing features

Open an issue in the [OpenStream TV issue tracker](https://github.com/wpfyorg/openstream-tv/issues). Include the OpenStream TV version, platform, relevant configuration with secrets removed, expected behavior, and the smallest reproducible steps.

Keep provider names precise. **OpenStream TV** is the application. **JioTV**, **JioTV+**, Fastway, and future sources are providers or account types. Do not rename provider-specific API paths, payload fields, or compatibility identifiers merely because they contain `jiotv`.

## Pull requests

Create a focused branch, keep each commit to one logical change, and use Conventional Commits such as `feat:`, `fix:`, `docs:`, `test:`, `chore:`, or `refactor:`.

Before opening a pull request:

- Run the Rust tests relevant to both full and slim builds.
- Run `cargo clippy --all-targets -- -D warnings` and the slim equivalent when Rust code changes.
- Run `npm test` and `npm run build` in `web/ui` when the web UI changes.
- Run installer tests when packaging, installation, service, or release behavior changes.
- Update documentation and examples with any user-facing configuration or command change.
- Never commit credentials, tokens, runtime state, logs, or private playlist URLs.

Normal changes target the repository's active development branch. Release tags and version bumps are handled by the release workflow rather than by ordinary feature commits.
