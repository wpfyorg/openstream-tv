# Discord release notifications

Add these repository Actions secrets to enable notifications:

- `DISCORD_RELEASE_WEBHOOK` receives stable releases.
- `DISCORD_BETA_WEBHOOK` receives prereleases (hyphenated version tags).

Configure the stable webhook for `releases` (`1570850604954161376`) and the beta webhook for `beta-releases` (`1570850742451839147`).

Create one webhook in each channel's **Edit Channel > Integrations > Webhooks** settings. Copy each URL directly into the matching repository secret under **GitHub Settings > Secrets and variables > Actions**. Each URL grants posting access to its channel; keep it out of source files, issues, chat, and logs. The webhooks require no Administrator role or installed bot. No OpenRouter key or paid service is needed.

Notifications use the GitHub release's `prerelease` flag to choose the channel. The existing tag publisher sets that flag for hyphenated version tags. The messages do not ping subscription roles, members, `@everyone`, or `@here`.

The release workflow calls a separate notification job after its tag-based publish job. GitHub does not emit downstream `release` Actions events for releases created with `GITHUB_TOKEN`, so this explicit call covers normal releases. The notification workflow also listens for `release: published` to cover releases published manually in GitHub, whether stable or prerelease. Both paths fetch the published release through the GitHub API and use its generated/published notes and official release URL. The bot needs no Discord application install; it posts through the channel-scoped webhooks.

Missing secrets skip notification. Drafts are skipped. Notification errors do not undo a published GitHub release, though they fail the separate notification job. Re-running that job or re-publishing a release event can post a duplicate; delivery is not idempotent. Webhook requests use the Python standard library, a 10 second timeout, and Discord's `wait=true` delivery confirmation. Discord mention parsing is disabled in the payload. Run offline validation with:

```sh
python3 scripts/discord_release_notification.py --dry-run <<'JSON'
{"tag_name":"v1.2.3","name":"v1.2.3","body":"Release notes","html_url":"https://github.com/wpfyorg/openstream-tv/releases/tag/v1.2.3","draft":false,"prerelease":false}
JSON
python3 -m unittest scripts.tests.test_discord_release_notification
```
