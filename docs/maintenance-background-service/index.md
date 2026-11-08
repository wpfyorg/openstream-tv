---
title: Run OpenStream TV as a Background Service
description: Configure supported systemd or Termux startup and inspect service status and logs.
category: Maintenance
keywords: [service, systemd, autostart, background, logs, Termux]
---

# Run OpenStream TV as a background service

`ost autostart` supports Linux systems running systemd and Termux. OpenWrt uses its package-managed procd service. Windows and macOS do not have a service installer in this project; run OpenStream TV in a terminal there.

## Linux with systemd

Run as your normal user to create a user service, or with `sudo` to create a system-wide service. Include the server arguments you need:

```sh
sudo env OPENSTREAM_PATH_PREFIX=/var/lib/openstream ost autostart --args "--host 0.0.0.0 --port 5001 --tls"
```

For a user service, use `ost autostart --args "--host 0.0.0.0 --tls"`. The command enables and starts the service. A user service may need lingering enabled to run at boot without an active login; the command prints the appropriate hint.

The guided installer calls this command after sign-in, which is how the account and tunnel choices reach the service: `ost autostart` copies the current `OPENSTREAM_*` environment into the generated environment file, including `OPENSTREAM_TUNNEL_TOKEN`.

Check status and logs:

```sh
sudo systemctl status openstream
sudo journalctl -u openstream -f
```

For a user service, add `--user` to `systemctl` and run `journalctl --user -u openstream -f`. Remove the service with `ost autostart remove` under the same account that installed it. The generated environment file may contain secrets; keep its permissions private.

## Termux

`ost autostart` adds a marked command to Termux's `bash.bashrc` to start OpenStream TV in the background when a shell opens. Remove it with `ost autostart remove`. Android power management can still stop background work.

## OpenWrt

Use `/etc/init.d/openstream start|stop|restart|enable|disable` to control the package service. Read its output with `logread -e openstream`. Configuration is in `/etc/config/openstream`; commit UCI changes before restarting.
