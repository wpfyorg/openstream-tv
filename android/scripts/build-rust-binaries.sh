#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ANDROID_DIR="$ROOT/android"
NDK="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}"
if [[ -z "$NDK" ]]; then
  SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
  [[ -n "$SDK" ]] || { echo 'Set ANDROID_NDK_HOME or ANDROID_HOME.' >&2; exit 2; }
  NDK="$(find "$SDK/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -1)"
fi
[[ -d "$NDK/toolchains/llvm/prebuilt" ]] || { echo "Android NDK not found: $NDK" >&2; exit 2; }
command -v cargo-ndk >/dev/null || { echo 'Install cargo-ndk first: cargo install cargo-ndk' >&2; exit 2; }
export ANDROID_NDK_HOME="$NDK"
API="${ANDROID_API_LEVEL:-23}"

build_one() {
  local target="$1" abi="$2"
  cargo ndk --target "$target" --platform "$API" --manifest-path "$ROOT/Cargo.toml" build --release --locked --no-default-features --features slim
  mkdir -p "$ANDROID_DIR/app/src/main/jniLibs/$abi"
  install -m 755 "$ROOT/target/$target/release/ost" "$ANDROID_DIR/app/src/main/jniLibs/$abi/libjiotv_exec.so"
}

build_one aarch64-linux-android arm64-v8a
build_one armv7-linux-androideabi armeabi-v7a
build_one x86_64-linux-android x86_64
