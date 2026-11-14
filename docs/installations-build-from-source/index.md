---
title: Build OpenStream TV from Source
description: Compile OpenStream TV locally and optionally build OpenWrt packages.
category: Installation
keywords: [build from source, Rust, Cargo, OpenWrt SDK, compile]
---

# 🛠️ Build from Source

Use this when you need a local build or your system has no matching release binary. You need Git and a current stable Rust toolchain with Cargo. The full build is the default; the slim build excludes the embedded browser UI.

---

## 📥 Step 1: Clone and Compile

```sh
git clone https://github.com/wpfyorg/openstream-tv.git
cd openstream-tv
cargo build --release
```

The binary is `target/release/ost`. For the slim build:

```sh
cargo build --release --no-default-features --features slim
```

---

## 📦 Step 2: Run or Install It

Run it in place to try it, or put it on `PATH`:

```sh
mkdir -p "$HOME/.local/bin"
install -m 0755 target/release/ost "$HOME/.local/bin/ost"
```

---

## 🌍 Step 3: Cross-Compiling and OpenWrt Packages

> [!IMPORTANT]
> Cross-compiling needs a Rust target toolchain and linker suited to the destination. The repository's release workflows define the supported target triples, and a successful local build does not guarantee compatibility with a different libc or CPU.

For OpenWrt, use the matching OpenWrt SDK and the supplied `scripts/build-openwrt-packages.sh`, which needs the full and slim binaries plus the SDK path and version. Follow that script's usage line for its exact arguments.

---

## 🔧 Step 4: Verify, Update, Remove

```sh
./target/release/ost --help
```

If the full build is on your `PATH`, run `ost serve` and browse `http://localhost:5001/`. A slim build serves no browser UI.

| Task | How |
| :--- | :--- |
| Update | Fetch the intended revision, rebuild, then replace the installed executable while the server is stopped |
| Uninstall | Remove the executable and any service registration you created; keep the private data directory unless you mean to erase saved logins |
| Startup problems | See [startup troubleshooting](../troubleshooting-installation-startup/index.md) |

See [first launch](../getting-started-first-launch/index.md), [CLI commands](../configuration-cli/index.md) and [update and uninstall](../maintenance-update-uninstall/index.md) for operating the compiled binary.
