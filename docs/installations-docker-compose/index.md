---
title: Install OpenStream TV with Docker Compose
description: Use a Compose file to run and update OpenStream TV with persistent account data.
category: Installation
keywords: [Docker Compose, compose.yaml, Docker, volume, container, update, logs]
---

# 🧾 Install with Docker Compose

Compose keeps the container ports and data volume in one reusable file, so updates and restarts are one command each. The released image supports Linux `amd64`, `arm64`, ARMv7 and 386; Docker selects the matching architecture.

> [!NOTE]
> Install Docker Engine or Docker Desktop with the **Compose** plugin first, and check that `docker compose version` works.

---

## 📝 Step 1: Create the Compose File

Make a new directory such as `openstream-compose` and save this as `compose.yaml`:

```yaml
services:
  openstream:
    image: ghcr.io/wpfyorg/openstream-tv:latest
    restart: unless-stopped
    ports:
      - "5001:5001"
      - "5443:5443"
    volumes:
      - openstream-data:/app/.openstream-tv

volumes:
  openstream-data:
```

| Setting | Meaning |
| :--- | :--- |
| `latest` image | Includes the web interface. Use the `slim` tag only for IPTV access without the browser UI. |
| `5001` | Plain HTTP for compatible IPTV clients. |
| `5443` | HTTPS for the browser interface. |
| `openstream-data` | Named volume holding credentials, configuration and local TLS files. Keep it private. |

---

## ▶️ Step 2: Start and Check the Service

From the directory containing `compose.yaml`:

```sh
docker compose config
docker compose up -d
docker compose ps
docker compose logs --tail 50 openstream
```

**Expected result:** `docker compose ps` shows a running `openstream` service. Open `https://localhost:5443/` on the host, or `https://<docker-host-ip>:5443/` from a trusted network device. A warning about the generated self-signed certificate is expected.

---

## 🔑 Step 3: Set the Admin Password and Connect a Provider

The first-run server can print a private web setup URL in its logs — do not share that link, it contains an access key. Or set the password interactively from your Compose directory:

```sh
docker compose exec openstream /usr/local/bin/ost admin password
docker compose restart openstream
```

Then open the page, sign in with the admin password and connect your provider in **Settings** using its OTP form. See [first launch](../getting-started-first-launch/index.md) and [connect JioTV](../account-connect-jiotv/index.md).

---

## 🔄 Step 4: Update

Stop playback first, then:

```sh
docker compose pull
docker compose up -d
docker compose ps
docker compose logs --tail 50 openstream
```

**Expected result:** Compose recreates the service when the image changed and keeps the same `openstream-data` volume. Reopen Settings to confirm the account is still connected. Back up the volume before major updates — see [Docker maintenance](../maintenance-docker/index.md).

---

## ⏹️ Step 5: Stop or Remove

| Command | Effect |
| :--- | :--- |
| `docker compose stop` | Temporary stop |
| `docker compose start` | Resume |
| `docker compose down` | Remove the container and network, keeping the saved volume |

> [!WARNING]
> Do **not** use `docker compose down -v` unless you deliberately want to delete the saved provider accounts, password and settings.

Port conflicts and missing data are covered in [Docker troubleshooting](../troubleshooting-server-and-maintenance/index.md).
