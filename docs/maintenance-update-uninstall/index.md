---
title: Uninstall OpenStream TV Safely
description: Remove OpenStream TV binaries, services or containers while preserving saved data unless you choose to erase it.
category: Maintenance
keywords: [uninstall, remove, binary, service, Docker, OpenWrt, saved data]
---

# Uninstall OpenStream TV Safely

This page covers removal. For upgrades, use [How to Update OpenStream TV](../maintenance-update/index.md).

Before uninstalling, decide whether to keep the OpenStream TV data directory. It contains saved provider sessions, the admin password hash, the access key and TLS files. Make a private [backup](../maintenance-backup-restore/index.md) if you may reinstall later.

## Linux, macOS and other standalone binaries

1. Stop the running OpenStream TV process.
2. If you installed autostart on Linux or Termux, remove it with `openstream autostart remove` using the same operating-system user that created it.
3. Delete only the `openstream` executable from the directory where you installed it, such as `/usr/local/bin/openstream` or `$HOME/.local/bin/openstream`.
4. Keep the data directory unless you deliberately want to remove saved accounts and server secrets.

If a systemd service was created for OpenStream TV, `openstream autostart remove` removes the service definition installed by OpenStream TV. Run it with the same privilege level used during setup.

## Windows

1. Stop the server window or process.
2. Delete `openstream.exe` from its install directory. The PowerShell installer defaults to `%LOCALAPPDATA%\Programs\OpenStream TV` unless you selected another location.
3. Remove that directory from your user `PATH` if it is no longer needed.
4. Keep the data directory if you may reinstall.

The project does not install a native Windows background service, so remove any third-party service wrapper separately if you configured one yourself.

## Termux

Run `openstream autostart remove` if you enabled Termux autostart, stop the server, then delete the `openstream` executable from `$PREFIX/bin`. Android may keep Termux files after the app closes; remove the OpenStream TV data directory separately only if you want to erase saved state.

## OpenWrt

Remove the installed package with the package manager used by the router:

```sh
apk del openstream
# or
opkg remove openstream
```

Use `openstream-slim` instead if that is the installed package. Package removal preserves `/etc/openstream` and `/etc/config/openstream`; back them up or delete them separately only when you intend to erase the configuration and saved account state. See [OpenWrt maintenance](../maintenance-openwrt/index.md).

## Docker or Compose

Stop and remove the container, then remove its image if desired. Keep the persistent volume if you want to preserve configuration and account state. Delete the volume only when you intend to erase that data. See [Docker maintenance](../maintenance-docker/index.md) for the matching commands.

## Verify removal

After removal, `openstream --help` should no longer resolve from the removed install location, and the previous OpenStream TV server ports should no longer be listening unless another copy is running. If the command still resolves, check `command -v openstream` on Unix-like systems or `Get-Command openstream` in PowerShell for another installation.

Removing the executable does not automatically erase OpenStream TV data. Review [data, backups and logs](../configuration-storage-logs/index.md) before deleting any saved state.
