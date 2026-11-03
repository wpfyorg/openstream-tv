---
title: Maintain OpenStream TV on OpenWrt
description: Update, inspect, back up and remove the OpenStream TV OpenWrt package and service.
category: Maintenance
keywords: [OpenWrt, update, apk, opkg, backup, logread, uninstall]
---

# Maintain OpenStream TV on OpenWrt

## Update

Run the [OpenWrt installer](../installations-openwrt/index.md) again. It downloads the package for the detected package manager and architecture, verifies its checksum, then updates through `apk` or `opkg`. Do not run `openstream update` on a package installation.

## Service and logs

```sh
/etc/init.d/openstream status
/etc/init.d/openstream restart
logread -e openstream
```

Boot startup is controlled by `/etc/init.d/openstream enable` and `disable`. The service configuration is `/etc/config/openstream`; after UCI edits, use `uci commit openstream` and restart the service.

## Back up and uninstall

Stop the service and privately copy `/etc/openstream` and `/etc/config/openstream`. These contain credentials, TLS key material and settings. Package removal with `apk del openstream` or `opkg remove openstream` preserves these paths. Use `openstream-slim` in the removal command if that is the installed package.

To erase OpenStream TV data after removing the package, separately delete `/etc/openstream` and `/etc/config/openstream` only after confirming that you no longer need their contents.
