---
title: Install with Docker
description: Run OpenStream TV in Docker with persistent settings, credentials and TLS certificates.
category: Installation
keywords: [Docker, container, GHCR, linux, arm64, persistent volume]
---

# 🐳 Install with Docker

The published image supports Linux `amd64`, `arm64`, ARMv7 and 386; Docker selects the matching architecture.

| Tag | Contains |
| :--- | :--- |
| `latest` | Current full image with the web interface |
| `slim` | Current headless image for IPTV clients |
| `1.5.0` / `1.5.0-slim` | A pinned release in either variant |

---

## 📥 Step 1: Run the Container

```sh
docker run -d --name openstream --restart unless-stopped \
  -p 5001:5001 -p 5443:5443 \
  -v openstream-data:/app/.openstream-tv \
  ghcr.io/wpfyorg/openstream-tv:latest
```

The container starts the server on all interfaces with HTTPS enabled.

| Where to open it | URL |
| :--- | :--- |
| On the Docker host | `https://localhost:5443/` (accept the self-signed warning) |
| From a trusted LAN device | `https://<docker-host-ip>:5443/` |
| IPTV players | The HTTP playlist on port `5001` |

> [!IMPORTANT]
> Keep the container and the volume private: the volume contains credentials and the generated TLS key.

---

## 🔑 Step 2: Set the Admin Password and Sign In

```sh
docker exec -it openstream /usr/local/bin/ost admin password
docker restart openstream
```

The container keeps the store it read at startup, so restart it before signing in; only then does the server accept the new password.

Then open the web interface, sign in and connect a provider in **Settings**:

| Provider | Sign-in |
| :--- | :--- |
| JioTV | Mobile number and OTP — [JioTV guide](../account-connect-jiotv/index.md) |
| JioTV+ | Its own OTP and a connection picker — [JioTV+ guide](../account-extras/index.md) |
| Fastway | Username and password — [Fastway guide](../account-fastway/index.md) |

From a terminal, `ost login otp` is the JioTV mobile-number login only; it does not sign in whichever provider the container was set up for. Stop the container first so the running server cannot write its in-memory state over the new login, then start it again so it loads the saved credentials and active provider:

```sh
docker stop openstream
docker run --rm -it -v openstream-data:/app/.openstream-tv \
  ghcr.io/wpfyorg/openstream-tv:latest login otp
docker start openstream
```

View output with `docker logs -f openstream`. For updates and backup instructions see [Docker maintenance](../maintenance-docker/index.md).

---

## ⚙️ Optional Settings

- **Headless:** use `ghcr.io/wpfyorg/openstream-tv:slim`.
- **Configuration:** add supported options before the image name, for example `-e OPENSTREAM_TITLE="Living Room TV"`.
- **Custom command:** container arguments replace the image's default command, so keep `--skip-update-check serve --host 0.0.0.0 --port 5001 --tls --tls-port 5443` when overriding it. See [CLI options](../configuration-cli/index.md).

---

## 🔧 Verify and Troubleshoot

| Check | Command |
| :--- | :--- |
| Container running | `docker ps --filter name=openstream` |
| Startup messages | `docker logs --tail 50 openstream` |
| Port occupied | Choose a different host-side port mapping |
| Saved accounts gone after recreation | Confirm the `openstream-data` volume was retained |

Use [Docker maintenance](../maintenance-docker/index.md) to pull updates, back up the volume and uninstall the container. For network errors see [Docker and web troubleshooting](../troubleshooting-server-and-maintenance/index.md).
