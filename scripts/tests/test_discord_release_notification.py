import json
import unittest
from urllib.error import HTTPError
from urllib.parse import parse_qs, urlsplit
from unittest.mock import MagicMock, patch

from scripts.discord_release_notification import (
    MAX_DESCRIPTION,
    MAX_TITLE,
    make_payload,
    notify,
    selected_webhook,
    utf16_units,
    valid_webhook_url,
)


BASE = {
    "tag_name": "v1.2.3",
    "name": "OpenStream TV v1.2.3",
    "body": "Release notes",
    "html_url": "https://github.com/wpfyorg/openstream-tv/releases/tag/v1.2.3",
    "draft": False,
    "prerelease": False,
}
SECRETS = {
    "DISCORD_RELEASE_WEBHOOK": "https://discord.com/api/webhooks/123/secret",
    "DISCORD_BETA_WEBHOOK": "https://discord.com/api/webhooks/456/secret",
}


class DiscordReleaseNotificationTests(unittest.TestCase):
    def test_route_selection(self):
        self.assertEqual(selected_webhook(BASE, SECRETS)[0], "DISCORD_RELEASE_WEBHOOK")
        beta = {**BASE, "prerelease": True}
        self.assertEqual(selected_webhook(beta, SECRETS)[0], "DISCORD_BETA_WEBHOOK")

    def test_truncation_and_safe_mentions(self):
        payload = make_payload({**BASE, "name": "🙂" * 300, "body": "@everyone " + "🙂" * 3000})
        embed = payload["embeds"][0]
        self.assertLessEqual(utf16_units(embed["title"]), MAX_TITLE)
        self.assertLessEqual(utf16_units(embed["description"]), MAX_DESCRIPTION)
        self.assertIn("Read full release notes on GitHub", embed["description"])
        self.assertEqual(payload["allowed_mentions"], {"parse": []})
        self.assertIn("Stable release", embed["title"])
        self.assertIn("OpenStream TV", embed["footer"]["text"])
        self.assertIn("@everyone", json.dumps(payload))

    def test_whitespace_body_uses_fallback(self):
        payload = make_payload({**BASE, "body": " \n\t "})
        self.assertEqual(payload["embeds"][0]["description"], "Release notes are available on GitHub.")

    def test_beta_is_labeled_for_testing(self):
        embed = make_payload({**BASE, "prerelease": True})["embeds"][0]
        self.assertIn("Beta release", embed["title"])
        self.assertEqual(embed["footer"]["text"], "OpenStream TV · Prerelease for testing")

    def test_draft_is_skipped(self):
        self.assertEqual(notify({**BASE, "draft": True}, SECRETS), "Skipped draft release.")

    def test_missing_secret_is_skipped(self):
        self.assertIn("DISCORD_RELEASE_WEBHOOK", notify(BASE, {}))

    def test_malformed_webhook_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "valid Discord webhook URL"):
            notify(BASE, {"DISCORD_RELEASE_WEBHOOK": "https://attacker.invalid/hook"}, dry_run=True)

    def test_webhook_whitespace_and_non_ascii_are_rejected(self):
        for webhook in (
            "https://discord.com/api/webhooks/123/sec\nret",
            "https://discord.com/api/webhooks/١٢٣/secret",
            "https://discord.com/api/webhooks/123/sécret",
        ):
            with self.subTest(webhook=webhook):
                self.assertFalse(valid_webhook_url(webhook))

    def test_dry_run_never_posts(self):
        with patch("scripts.discord_release_notification.urlopen") as urlopen:
            result = notify(BASE, SECRETS, dry_run=True)
        self.assertIn("no request sent", result)
        urlopen.assert_not_called()

    def test_dry_run_works_without_secrets(self):
        self.assertIn("selected DISCORD_RELEASE_WEBHOOK", notify(BASE, {}, dry_run=True))

    def test_request_is_bounded_and_errors_hide_webhook(self):
        secret = SECRETS["DISCORD_RELEASE_WEBHOOK"]
        with patch(
            "scripts.discord_release_notification.urlopen",
            side_effect=HTTPError(secret, 500, "failed", {}, None),
        ) as urlopen:
            with self.assertRaisesRegex(RuntimeError, "HTTP 500") as raised:
                notify(BASE, SECRETS)
        self.assertNotIn(secret, str(raised.exception))
        self.assertEqual(urlopen.call_args.kwargs["timeout"], 10)

    def test_success_request_uses_selected_route_and_bounded_safe_payload(self):
        response = MagicMock()
        response.__enter__.return_value.status = 204
        release = {**BASE, "name": "🙂" * 300, "body": "Release notes 🙂 " * 500}
        selected_secrets = {
            **SECRETS,
            "DISCORD_RELEASE_WEBHOOK": SECRETS["DISCORD_RELEASE_WEBHOOK"] + "?thread_id=99&wait=false",
        }
        with patch("scripts.discord_release_notification.urlopen", return_value=response) as urlopen:
            self.assertEqual(notify(release, selected_secrets), "Discord notification sent.")

        request = urlopen.call_args.args[0]
        self.assertEqual(urlsplit(request.full_url).scheme, "https")
        self.assertEqual(urlsplit(request.full_url).hostname, "discord.com")
        self.assertEqual(urlsplit(request.full_url).path, "/api/webhooks/123/secret")
        self.assertEqual(parse_qs(urlsplit(request.full_url).query), {"thread_id": ["99"], "wait": ["true"]})
        payload = json.loads(request.data)
        self.assertEqual(payload["allowed_mentions"], {"parse": []})
        serialized = json.dumps(payload, ensure_ascii=False)
        self.assertNotIn(SECRETS["DISCORD_BETA_WEBHOOK"].rsplit("/", 1)[-1], serialized)
        self.assertIn("Stable release", payload["embeds"][0]["title"])
        self.assertLessEqual(utf16_units(payload["embeds"][0]["title"]), MAX_TITLE)
        self.assertLessEqual(utf16_units(payload["embeds"][0]["description"]), MAX_DESCRIPTION)
        self.assertEqual(urlopen.call_args.kwargs["timeout"], 10)


if __name__ == "__main__":
    unittest.main()
