#!/bin/sh
set -eu

if [ "$#" -ne 4 ]; then
  echo "usage: $0 SDK_DIR FULL_BINARY SLIM_BINARY VERSION" >&2
  exit 2
fi
sdk=$1
full=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
slim=$(cd "$(dirname "$3")" && pwd)/$(basename "$3")
version=$4
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)

[ -x "$full" ] || { echo "missing full binary: $full" >&2; exit 1; }
[ -x "$slim" ] || { echo "missing slim binary: $slim" >&2; exit 1; }
[ -f "$sdk/include/package.mk" ] || { echo "not an OpenWrt SDK directory: $sdk" >&2; exit 1; }

cp -R "$root/openwrt/openstream" "$sdk/package/openstream"
make -C "$sdk" defconfig
make -C "$sdk" package/openstream/compile V=s \
  PKG_VERSION="$version" OPENSTREAM_FULL_BIN="$full" OPENSTREAM_SLIM_BIN="$slim"

mkdir -p "$root/dist/openwrt"
find "$sdk/bin/packages" -type f \( -name 'openstream_*.ipk' -o -name 'openstream-*.apk' -o -name 'openstream-slim_*.ipk' -o -name 'openstream-slim-*.apk' \) \
  -exec sh -c '
    packages_root=$1
    output=$2
    shift 2
    for package do
      case "$package" in
        *.apk)
          relative=${package#"$packages_root"/}
          package_arch=${relative%%/*}
          [ "$package_arch" != "$relative" ] || {
            echo "cannot derive OpenWrt package architecture from: $package" >&2
            exit 1
          }
          basename=${package##*/}
          cp -v "$package" "$output/${basename%.apk}_${package_arch}.apk"
          ;;
        *.ipk)
          cp -v "$package" "$output/"
          ;;
      esac
    done
  ' sh "$sdk/bin/packages" "$root/dist/openwrt" {} +
test -n "$(find "$root/dist/openwrt" -type f \( -name 'openstream_*.ipk' -o -name 'openstream-*.apk' -o -name 'openstream-slim_*.ipk' -o -name 'openstream-slim-*.apk' \) -print -quit)"
