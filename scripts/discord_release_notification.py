#!/usr/bin/env python3
"""Post a published OpenStream TV release to its Discord webhook."""

from __future__ import annotations

import argparse
import json
import os
import sys
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
from urllib.request import Request, urlopen

TIMEOUT_SECONDS = 10
MAX_TITLE = 256
MAX_DESCRIPTION = 4096
MAX_URL = 2048


def utf16_units(value: str) -> int:
    return len(value.encode("utf-16-le")) // 2


def truncate_utf16(value: str, limit: int) -> str:
    if utf16_units(value) <= limit:
        return value
    remaining = limit - 1
    prefix: list[str] = []
    for character in value:
        units = 2 if ord(character) > 0xFFFF else 1
        if units > remaining:
            break
        prefix.append(character)
        remaining -= units
    return "".join(prefix).rstrip() + "…"


def selected_webhook(release: dict[str, Any], environ: dict[str, str]) -> tuple[str, str] | None:
    if release.get("draft") is True:
        return None
    prerelease = release.get("prerelease") is True
    key = "DISCORD_BETA_WEBHOOK" if prerelease else "DISCORD_RELEASE_WEBHOOK"
    webhook = environ.get(key, "").strip()
    if not webhook:
        return key, ""
    return key, webhook


def valid_webhook_url(value: str) -> bool:
    if not value.isascii() or any(character.isspace() for character in value):
        return False
    try:
        parsed = urlsplit(value)
        path_parts = parsed.path.split("/")
        return (
            parsed.scheme == "https"
            and parsed.hostname in {"discord.com", "discordapp.com"}
            and parsed.username is None
            and parsed.password is None
            and parsed.port in (None, 443)
            and len(path_parts) == 5
            and path_parts[:3] == ["", "api", "webhooks"]
            and path_parts[3].isdecimal()
            and bool(path_parts[4])
            and all(character.isalnum() or character in "._-" for character in path_parts[4])
            and not parsed.fragment
        )
    except ValueError:
        return False


def make_payload(release: dict[str, Any], test_notification: bool = False) -> dict[str, Any]:
    name = str(release.get("name") or release.get("tag_name") or "OpenStream TV release")
    body = str(release.get("body") or "").strip() or "Release notes are available on GitHub."
    prerelease = release.get("prerelease") is True
    release_type = "Beta release" if prerelease else "Stable release"
    footer = "OpenStream TV · Prerelease for testing" if prerelease else "OpenStream TV · Stable release"
    release_url = str(release.get("html_url") or "")
    try:
        parsed_release_url = urlsplit(release_url)
        valid_release_url = (
            parsed_release_url.scheme == "https"
            and parsed_release_url.hostname == "github.com"
            and parsed_release_url.username is None
            and "/releases/tag/" in parsed_release_url.path
        )
    except ValueError:
        valid_release_url = False
    if len(release_url) > MAX_URL or not valid_release_url:
        raise ValueError("release URL is missing or invalid")
    description = body
    read_more = f"\n\n[Read full release notes on GitHub]({release_url})"
    if utf16_units(description) > MAX_DESCRIPTION:
        description = truncate_utf16(description, MAX_DESCRIPTION - utf16_units(read_more)) + read_more
    payload = {
        "embeds": [
            {
                "title": truncate_utf16(
                    f"{'TEST · ' if test_notification else ''}OpenStream TV · {release_type} · {name}",
                    MAX_TITLE,
                ),
                "url": release_url,
                "description": description,
                "footer": {"text": footer},
            }
        ],
        "allowed_mentions": {"parse": []},
    }
    if test_notification:
        payload["content"] = "Configuration test only — no new release was published"
    return payload


def notify(
    release: dict[str, Any],
    environ: dict[str, str],
    dry_run: bool = False,
    test_notification: bool = False,
) -> str:
    route = selected_webhook(release, environ)
    if route is None:
        return "Skipped draft release."
    key, webhook = route
    payload = make_payload(release, test_notification=test_notification)
    if not webhook:
        if dry_run:
            return f"Dry run: selected {key}; webhook secret is not configured; no request sent."
        return f"Skipped: {key} secret is not configured."
    if not valid_webhook_url(webhook):
        raise ValueError(f"{key} is not a valid Discord webhook URL")
    if dry_run:
        return f"Dry run: selected {key}; payload is valid; no request sent."

    parsed_webhook = urlsplit(webhook)
    query = [(key, value) for key, value in parse_qsl(parsed_webhook.query, keep_blank_values=True) if key != "wait"]
    query.append(("wait", "true"))
    delivery_url = urlunsplit(parsed_webhook._replace(query=urlencode(query)))
    request = Request(
        delivery_url,
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        headers={"Content-Type": "application/json", "User-Agent": "OpenStream-TV-release-notifier"},
        method="POST",
    )
    try:
        with urlopen(request, timeout=TIMEOUT_SECONDS) as response:
            if not 200 <= response.status < 300:
                raise RuntimeError(f"Discord returned HTTP {response.status}")
    except HTTPError as error:
        error.close()
        raise RuntimeError(f"Discord returned HTTP {error.code}") from None
    except (URLError, TimeoutError, OSError):
        raise RuntimeError("Discord request failed or timed out") from None
    return "Discord notification sent."


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="validate and select a route without sending")
    parser.add_argument("--test-notification", action="store_true", help="label the notification as a configuration test")
    args = parser.parse_args()
    try:
        release = json.load(sys.stdin)
        if not isinstance(release, dict):
            raise ValueError("release data must be a JSON object")
        print(notify(release, os.environ, args.dry_run, args.test_notification))
        return 0
    except (json.JSONDecodeError, ValueError, RuntimeError) as error:
        print(f"Discord notification error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
