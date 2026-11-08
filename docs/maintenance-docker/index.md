---
title: Docker Updates, Backups and Logs
description: Maintain an OpenStream TV Docker container without losing its persistent data volume.
category: Maintenance
keywords: [Docker, update, backup, restore, logs, volume, container]
---

# Docker updates, backups and logs

The container writes persistent state to `/app/.openstream-tv`. Keep the named volume mounted whenever you recreate the container; it holds login state, settings and the self-signed TLS certificate.

## Logs and status

```sh
docker ps --filter name=openstream
docker logs -f openstream
```

Stop the container with `docker stop openstream`. Start it again with `docker start openstream`.

## Update

Pull the new image, remove the old container, and recreate it using the same run options and named volume:

```sh
docker pull ghcr.io/wpfyorg/openstream-tv:latest
docker stop openstream
docker rm openstream
```

Then rerun the `docker run` command from the [Docker installation guide](../installations-docker/index.md), retaining `-v openstream-data:/app/.openstream-tv` and any ports or environment settings you had configured.

## Back up and restore

Stop the container before backing up. For a named volume called `openstream-data`, an archive can be made with a temporary helper container:

```sh
docker run --rm -v openstream-data:/data -v "$PWD:/backup" alpine \
  tar -czf /backup/openstream-data.tar.gz -C /data .
```

Keep the archive private. To restore, stop OpenStream TV, extract the archive into the same volume, then start or recreate the container. Confirm the volume name before running volume commands; removing a volume deletes its contents.

## Uninstall

Stop and remove the container with `docker stop openstream && docker rm openstream`. Remove the image with `docker image rm ghcr.io/wpfyorg/openstream-tv:latest` if wanted. Keep `openstream-data` for a later reinstall. Remove the volume only when you intend to erase all saved settings and credentials.
