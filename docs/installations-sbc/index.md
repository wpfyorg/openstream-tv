---
title: Install on Raspberry Pi and SBCs
description: Choose a supported OpenStream TV build for Raspberry Pi and other Linux single-board computers.
category: Installation
keywords: [Raspberry Pi, SBC, ARM, ARMv7, aarch64, Linux]
---

# 🍓 Install on Raspberry Pi and SBCs

Use a supported Linux distribution such as Raspberry Pi OS, Debian, Ubuntu or Armbian, then check the architecture:

```sh
uname -m
```

| `uname -m` | Supported |
| :--- | :--- |
| `aarch64` | ✅ 64-bit ARM |
| `armv7l` | ✅ 32-bit ARMv7 |
| `x86_64` | ✅ 64-bit Intel/AMD |
| `armv6l` | ❌ Original Pi and Pi Zero are not release targets |

> [!NOTE]
> A Pi Zero 2 needs a 64-bit operating system to use the `aarch64` build.

---

## 📥 Step 1: Install with the Linux Guide

Follow [Install on Linux](../installations-linux/index.md). The installer chooses the release target from the running system, and `full` gives the browser interface while `slim` is an intentionally headless server.

---

## 🔁 Step 2: Make It Permanent

For an always-on systemd setup follow [Background service](../maintenance-background-service/index.md).

For a system-wide service, choose a stable data directory and set `OPENSTREAM_PATH_PREFIX` consistently before login and service setup, because the service setup command saves the selected prefix. See [data, backups and logs](../configuration-storage-logs/index.md).

---

## 🔧 Step 3: Verify

```sh
ost --help
ost serve --host 0.0.0.0 --tls
```

Visit `https://<sbc-lan-address>:5443/` from another device on the trusted network.

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Architecture or loader error | Board CPU is not a release target | Confirm with `uname -m` and pick the matching release, or use a 64-bit OS on ARMv8 boards |
| Page unreachable from another device | Bound to localhost, or firewall | Bind `--host 0.0.0.0` and allow ports 5001 and 5443 on the LAN |

---

## 🧹 Step 4: Update and Remove

Stop the service, run `ost update`, restart it and check the page. To remove the installation follow [update and uninstall](../maintenance-update-uninstall/index.md); for startup and missing dependency errors see [installation troubleshooting](../troubleshooting-installation-startup/index.md).
