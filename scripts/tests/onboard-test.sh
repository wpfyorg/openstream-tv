#!/bin/sh
# Drives scripts/install.sh through a pseudo-terminal so the onboarding flow is
# exercised the way a user sees it. Every prompt reads /dev/tty, so a pty is
# the only way to test it without a human at the keyboard. Keystrokes are only
# sent once the matching prompt has been rendered, which keeps arrow-key
# selection deterministic.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir -p "$tmp/bin"

cat >"$tmp/bin/uname" <<'EOF'
#!/bin/sh
case "$1" in
  -s) printf '%s\n' "Darwin" ;;
  -m) printf '%s\n' "arm64" ;;
esac
EOF
chmod +x "$tmp/bin/uname"

# The installer downloads a release asset; here it is a script that behaves
# like `ost` so the sign-in dialogs can be driven end to end.
cat >"$tmp/stub" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --help)
    echo "USAGE:"
    echo "  ost login otp"
    echo "  ost tv+ login | tv+ logout"
    echo "  ost fastway login"
    echo "  ost playbox login"
    echo "  ost vzy has no login; the free tier starts with the server"
    echo "  ost airtel login"
    echo "  ost tataplay login"
    echo "  ost admin password"
    ;;
  login)
    printf 'Enter your mobile number: +91 '
    read number
    printf 'Sending OTP to your mobile number\n'
    printf 'OTP sent to your mobile number\n'
    printf 'Enter OTP: '
    read otp
    [ -n "$otp" ] && echo "Login successful"
    ;;
  extras|tv+)
    printf 'Mobile number registered to the fibre account: +91 '
    read number
    if [ "${OPENSTREAM_STUB_NO_CONNECTIONS:-0}" = 1 ]; then
      printf 'OTP: '
    else
      printf 'Connections on this number:\n'
      printf '  1. Home, JioFiber, line ending 1234\n'
      printf '  2. Office, JioFiber, line ending 5678\n'
      printf 'Choose a connection: '
      read pick
      printf 'OTP: '
    fi
    read otp
    echo "JioTV+ login saved. Restart the server to use it."
    ;;
  airtel)
    printf 'Airtel mobile number (+91): '
    read number
    printf 'OTP sent to your mobile number\n'
    printf 'Airtel OTP: '
    read otp
    echo "Airtel login saved for +91$number. Restart the server to use it."
    ;;
  playbox)
    printf 'PlayboxTV mobile number: '
    read number
    printf 'OTP sent to +91%s\n' "$number"
    printf 'PlayboxTV OTP: '
    read otp
    echo "PlayboxTV login saved. Restart the server to use it."
    ;;
  tataplay)
    printf 'Tata Play mobile number: '
    read number
    printf 'Accounts on this number:\n'
    printf '  1) Home DTH, DTH\n'
    printf '  2) Weekend LIT, LIT\n'
    printf 'Sign in to which account [1-2]: '
    read pick
    printf 'OTP sent to +91%s\n' "$number"
    printf 'Tata Play OTP: '
    read otp
    echo "Tata Play login saved. Restart the server to use it."
    ;;
  admin)
    printf 'New admin password: '
    read pass
    [ -n "$pass" ]
    ;;
  epg)
    echo "epg $2" >>"$OPENSTREAM_TEST_STUB_LOG"
    if [ "$2" = on ]; then echo "EPG enabled permanently."; fi
    ;;
  autostart)
    echo "service installation unavailable" >&2
    exit 1
    ;;
  key)
    echo "/k/secret-key/playlist.m3u"
    ;;
esac
EOF
chmod +x "$tmp/stub"

if command -v sha256sum >/dev/null 2>&1; then hash_cmd=sha256sum; else hash_cmd='shasum -a 256'; fi

cat >"$tmp/bin/curl" <<EOF
#!/bin/sh
out=
url=
while [ "\$#" -gt 0 ]; do
  case "\$1" in
    -o) out=\$2; shift 2 ;;
    -*) shift ;;
    *) url=\$1; shift ;;
  esac
done
hash() { $hash_cmd "\$1" | awk '{print \$1}'; }
case "\$url" in
  */SHA256SUMS)
    stub_hash="\$(hash "$tmp/stub")"
    {
      printf '%s  %s\n' "\$stub_hash" 'openstream-full-aarch64-apple-darwin'
      printf '%s  %s\n' "\$stub_hash" 'openstream-slim-aarch64-apple-darwin'
    } >"\$out"
    ;;
  *)
    cp "$tmp/stub" "\$out"
    ;;
esac
EOF
chmod +x "$tmp/bin/curl"

# Waits until the prompt has been drawn before typing at it, so arrow keys are
# read in raw mode instead of piling up in the terminal's line buffer.
wait_for() {
  n=0
  while [ "$n" -lt 120 ]; do
    grep -qF "$1" "$2" 2>/dev/null && return 0
    sleep 0.2
    n=$((n + 1))
  done
  echo "timed out waiting for: $1" >&2
  return 1
}

start_flow() {
  flow_dir=$1
  flow_out=$2
  rm -f "$tmp/in"
  mkfifo "$tmp/in"
  # The installer needs a terminal, so it runs under `script`. macOS ships BSD
  # script, which rejects a FIFO on stdin but accepts a pipe, so `cat` turns the
  # FIFO into one. util-linux script takes the command with -c instead.
  # The guide question only appears when no EPG setting is inherited, so every
  # flow starts from an unset OPENSTREAM_EPG and JIOTV_EPG.
  if script --version 2>/dev/null | grep -qi 'util-linux'; then
    OPENSTREAM_INSTALL_DIR="$flow_dir" HOME="$flow_dir/home" XDG_CONFIG_HOME="$flow_dir/home/.config" OPENSTREAM_EPG='' JIOTV_EPG='' OPENSTREAM_TUNNEL="${OPENSTREAM_TEST_TUNNEL:-}" OPENSTREAM_TUNNEL_TOKEN="${OPENSTREAM_TEST_TUNNEL_TOKEN:-}" PATH="$tmp/bin:$PATH" \
      sh -c 'cat "$1" | script -q -c "sh $2" /dev/null >"$3" 2>&1' _ "$tmp/in" "$root/scripts/install.sh" "$flow_out" &
  else
    OPENSTREAM_INSTALL_DIR="$flow_dir" HOME="$flow_dir/home" XDG_CONFIG_HOME="$flow_dir/home/.config" OPENSTREAM_EPG='' JIOTV_EPG='' OPENSTREAM_TUNNEL="${OPENSTREAM_TEST_TUNNEL:-}" OPENSTREAM_TUNNEL_TOKEN="${OPENSTREAM_TEST_TUNNEL_TOKEN:-}" PATH="$tmp/bin:$PATH" \
      sh -c 'cat "$1" | script -q /dev/null sh "$2" >"$3" 2>&1' _ "$tmp/in" "$root/scripts/install.sh" "$flow_out" &
  fi
  flow_pid=$!
  exec 6>"$tmp/in"
}
send() { printf '%s' "$1" >&6; }
enter() { printf '\r' >&6; }
finish_flow() { exec 6>&-; wait "$flow_pid"; }

# ---- default path: full build, JioTV account ------------------------------
OPENSTREAM_TEST_STUB_LOG="$tmp/stub-calls.log"
export OPENSTREAM_TEST_STUB_LOG
: >"$OPENSTREAM_TEST_STUB_LOG"
start_flow "$tmp/install" "$tmp/out"
wait_for "Which build?" "$tmp/out"; enter
wait_for "Which account are you setting up?" "$tmp/out"; enter
wait_for "This machine's address" "$tmp/out"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out"; enter
wait_for "Mobile number registered with JioTV" "$tmp/out"; send '9999999999'; enter
wait_for "Enter the OTP" "$tmp/out"; send '123456'; enter
wait_for "Choose an admin password" "$tmp/out"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out"
finish_flow

grep -F "Signed in to JioTV" "$tmp/out" >/dev/null
grep -F "http://192.168.1.9:5001/k/secret-key/playlist.m3u" "$tmp/out" >/dev/null
grep -F "guide     off" "$tmp/out" >/dev/null
if grep -F "epg" "$OPENSTREAM_TEST_STUB_LOG" >/dev/null; then
  echo "declining the programme guide on a fresh install changed the EPG setting" >&2
  exit 1
fi
grep -F "export OPENSTREAM_EPG='false'" "$tmp/install/home/.config/openstream/installer.env" >/dev/null
test -x "$tmp/install/ost"
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out"; fi
echo "onboarding default flow OK"

# ---- declining clears an override an earlier enable stored ---------------
mkdir -p "$tmp/install-epg/home/.openstream-tv"
printf '[data]\nepg_enabled = "true"\n' >"$tmp/install-epg/home/.openstream-tv/store_v4.toml"
: >"$OPENSTREAM_TEST_STUB_LOG"
start_flow "$tmp/install-epg" "$tmp/out-epg"
wait_for "Which build?" "$tmp/out-epg"; enter
wait_for "Which account are you setting up?" "$tmp/out-epg"; enter
wait_for "This machine's address" "$tmp/out-epg"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out-epg"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out-epg"; enter
wait_for "Mobile number registered with JioTV" "$tmp/out-epg"; send '9999999999'; enter
wait_for "Enter the OTP" "$tmp/out-epg"; send '123456'; enter
wait_for "Choose an admin password" "$tmp/out-epg"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out-epg"
finish_flow

grep -F "Programme guide disabled" "$tmp/out-epg" >/dev/null
grep -F "epg off" "$OPENSTREAM_TEST_STUB_LOG" >/dev/null
grep -F "guide     off" "$tmp/out-epg" >/dev/null
grep -F "export OPENSTREAM_EPG='false'" "$tmp/install-epg/home/.config/openstream/installer.env" >/dev/null
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out-epg"; fi
echo "onboarding clears an earlier stored programme guide override OK"

# ---- Airtel: OTP sign-in writes the provider flag ------------------------
# The menu is jiotv, JioTV+, Fastway, PlayboxTV, VZY, Airtel, Tata Play, so the
# account question needs as many Down presses as the entry's position.
pick_provider() {
  out=$1
  steps=$2
  seq=0
  keys=
  while [ "$seq" -lt "$steps" ]; do keys="$keys$(printf '\033[B')"; seq=$((seq + 1)); done
  wait_for "Which account are you setting up?" "$out"; send "$keys"; enter
}

start_flow "$tmp/install-airtel" "$tmp/out-airtel"
wait_for "Which build?" "$tmp/out-airtel"; enter
pick_provider "$tmp/out-airtel" 5
wait_for "This machine's address" "$tmp/out-airtel"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out-airtel"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out-airtel"; enter
wait_for "Airtel mobile number" "$tmp/out-airtel"; send '+91 98765 43210'; enter
wait_for "Enter the OTP" "$tmp/out-airtel"; send '135790'; enter
wait_for "Choose an admin password" "$tmp/out-airtel"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out-airtel"
finish_flow
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out-airtel"; fi

grep -F "Signed in to Airtel Xstream" "$tmp/out-airtel" >/dev/null
# The formatted number typed above is normalised before it reaches the CLI.
grep -F "OTP sent to +919876543210" "$tmp/out-airtel" >/dev/null
grep -F "account   Airtel Xstream" "$tmp/out-airtel" >/dev/null
airtel_env="$tmp/install-airtel/home/.config/openstream/installer.env"
grep -F "export OPENSTREAM_AIRTEL='true'" "$airtel_env" >/dev/null
grep -F "export OPENSTREAM_FASTWAY='false'" "$airtel_env" >/dev/null
env -i PATH="$PATH" sh -c '. "$1"; test "$OPENSTREAM_AIRTEL" = true' _ "$airtel_env"
echo "onboarding Airtel flow OK"

# ---- PlayboxTV: OTP sign-in writes the provider flag ---------------------
start_flow "$tmp/install-playbox" "$tmp/out-playbox"
wait_for "Which build?" "$tmp/out-playbox"; enter
pick_provider "$tmp/out-playbox" 3
wait_for "This machine's address" "$tmp/out-playbox"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out-playbox"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out-playbox"; enter
wait_for "PlayboxTV mobile number" "$tmp/out-playbox"; send '9876543210'; enter
wait_for "Enter the OTP" "$tmp/out-playbox"; send '246810'; enter
wait_for "Choose an admin password" "$tmp/out-playbox"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out-playbox"
finish_flow
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out-playbox"; fi

grep -F "Signed in to PlayboxTV" "$tmp/out-playbox" >/dev/null
grep -F "account   PlayboxTV (beta)" "$tmp/out-playbox" >/dev/null
playbox_env="$tmp/install-playbox/home/.config/openstream/installer.env"
grep -F "export OPENSTREAM_PLAYBOX='true'" "$playbox_env" >/dev/null
grep -F "export OPENSTREAM_AIRTEL='false'" "$playbox_env" >/dev/null
env -i PATH="$PATH" sh -c '. "$1"; test "$OPENSTREAM_PLAYBOX" = true' _ "$playbox_env"
echo "onboarding PlayboxTV flow OK"

# ---- VZY: choosing it needs no sign-in ----------------------------------
start_flow "$tmp/install-vzy" "$tmp/out-vzy"
wait_for "Which build?" "$tmp/out-vzy"; enter
pick_provider "$tmp/out-vzy" 4
wait_for "This machine's address" "$tmp/out-vzy"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out-vzy"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out-vzy"; enter
wait_for "Choose an admin password" "$tmp/out-vzy"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out-vzy"
finish_flow
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out-vzy"; fi

grep -F "VZY needs no account" "$tmp/out-vzy" >/dev/null
grep -F "account   VZY free tier (beta)" "$tmp/out-vzy" >/dev/null
vzy_env="$tmp/install-vzy/home/.config/openstream/installer.env"
grep -F "export OPENSTREAM_VZY='true'" "$vzy_env" >/dev/null
env -i PATH="$PATH" sh -c '. "$1"; test "$OPENSTREAM_VZY" = true' _ "$vzy_env"
echo "onboarding VZY flow OK"

# ---- Tata Play: the account is picked before the OTP ---------------------
start_flow "$tmp/install-tataplay" "$tmp/out-tataplay"
wait_for "Which build?" "$tmp/out-tataplay"; enter
pick_provider "$tmp/out-tataplay" 6
wait_for "This machine's address" "$tmp/out-tataplay"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out-tataplay"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out-tataplay"; enter
wait_for "Tata Play mobile number" "$tmp/out-tataplay"; send '9876543210'; enter
wait_for "Account number" "$tmp/out-tataplay"; send '2'; enter
wait_for "Enter the OTP" "$tmp/out-tataplay"; send '112233'; enter
wait_for "Choose an admin password" "$tmp/out-tataplay"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out-tataplay"
finish_flow
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out-tataplay"; fi

grep -F "Weekend LIT, LIT" "$tmp/out-tataplay" >/dev/null
grep -F "Signed in to Tata Play" "$tmp/out-tataplay" >/dev/null
grep -F "account   Tata Play (beta)" "$tmp/out-tataplay" >/dev/null
tataplay_env="$tmp/install-tataplay/home/.config/openstream/installer.env"
grep -F "export OPENSTREAM_TATAPLAY='true'" "$tataplay_env" >/dev/null
env -i PATH="$PATH" sh -c '. "$1"; test "$OPENSTREAM_TATAPLAY" = true' _ "$tataplay_env"
echo "onboarding Tata Play flow OK"

# ---- arrow keys: slim build, JioTV+ with a connection picker --------------
start_flow "$tmp/install-slim" "$tmp/out2"
wait_for "Which build?" "$tmp/out2"; send "$(printf '\033[B')"; enter
wait_for "Which account are you setting up?" "$tmp/out2"; send "$(printf '\033[B')"; enter
wait_for "This machine's address" "$tmp/out2"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out2"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out2"; send '1'; enter
wait_for "Mobile number registered to the fibre account" "$tmp/out2"; send '9999999999'; enter
wait_for "Choose the connection" "$tmp/out2"; send '2'; enter
wait_for "OTP" "$tmp/out2"; send '123456'; enter
wait_for "Installation complete" "$tmp/out2"
finish_flow

grep -F "Office, JioFiber" "$tmp/out2" >/dev/null
grep -F "Signed in to JioTV+" "$tmp/out2" >/dev/null
grep -F "Programme guide enabled" "$tmp/out2" >/dev/null
grep -F "guide     http://192.168.1.9:5001/k/secret-key/epg.xml.gz" "$tmp/out2" >/dev/null
grep -F "epg on" "$OPENSTREAM_TEST_STUB_LOG" >/dev/null
grep -F "export OPENSTREAM_EPG='true'" "$tmp/install-slim/home/.config/openstream/installer.env" >/dev/null
grep -F "http://192.168.1.9:5001/k/secret-key/playlist.m3u" "$tmp/out2" >/dev/null
grep -F "The slim build has no browser interface" "$tmp/out2" >/dev/null
if grep -F "Choose an admin password" "$tmp/out2" >/dev/null; then
  echo "a slim install asked for an admin password" >&2
  exit 1
fi
echo "onboarding arrow keys and JioTV+ flow OK"

# ---- JioTV+ with nothing to pick: must not wait for a picker --------------
OPENSTREAM_STUB_NO_CONNECTIONS=1
OPENSTREAM_TEST_TUNNEL_TOKEN="eyJdummy'quoted"
export OPENSTREAM_STUB_NO_CONNECTIONS OPENSTREAM_TEST_TUNNEL_TOKEN
start_flow "$tmp/install-noconn" "$tmp/out3"
wait_for "Which build?" "$tmp/out3"; enter
wait_for "Which account are you setting up?" "$tmp/out3"; send "$(printf '\033[B')"; enter
wait_for "This machine's address" "$tmp/out3"; send '192.168.1.9'; enter
wait_for "Open OpenStream TV beyond your LAN?" "$tmp/out3"; send "$(printf '\033[B\033[B')"; enter
wait_for "Turn on the programme guide (EPG)?" "$tmp/out3"; enter
wait_for "Mobile number registered to the fibre account" "$tmp/out3"; send '9999999999'; enter
wait_for "Enter the OTP" "$tmp/out3"; send '123456'; enter
wait_for "Choose an admin password" "$tmp/out3"; send 'secret'; enter
wait_for "Installation complete" "$tmp/out3"
finish_flow
unset OPENSTREAM_STUB_NO_CONNECTIONS
unset OPENSTREAM_TEST_TUNNEL_TOKEN
if [ "${OPENSTREAM_TEST_SHOW:-0}" = 1 ]; then sed 's/\r$//' "$tmp/out3"; fi

grep -F "Signed in to JioTV+" "$tmp/out3" >/dev/null
env_file="$tmp/install-noconn/home/.config/openstream/installer.env"
grep -F "export OPENSTREAM_TVPLUS='true'" "$env_file" >/dev/null
grep -F "export OPENSTREAM_TUNNEL_TOKEN='eyJdummy'\\''quoted'" "$env_file" >/dev/null
env -i PATH="$PATH" sh -c '. "$1"; test "$OPENSTREAM_TUNNEL_TOKEN" = "$2" && test "$OPENSTREAM_TVPLUS" = true' _ "$env_file" "eyJdummy'quoted"
if grep -F "eyJdummy'quoted" "$tmp/out3" >/dev/null; then
  echo "the tunnel token was printed in installer output" >&2
  exit 1
fi
env_mode=$(stat -c '%a' "$env_file" 2>/dev/null || stat -f '%Lp' "$env_file")
[ "$env_mode" = 600 ]
grep -F "$env_file" "$tmp/out3" >/dev/null
grep -F "ost serve --host 0.0.0.0 --tls --tunnel" "$tmp/out3" >/dev/null
if grep -F "Choose the connection" "$tmp/out3" >/dev/null; then
  echo "a connection picker appeared without any connections" >&2
  exit 1
fi
echo "onboarding JioTV+ without a picker OK"

echo "onboarding persists provider/tunnel settings and prints keyed playlist URL OK"

# ---- declining an update must leave the installed binary untouched --------
mkdir -p "$tmp/install-existing"
printf 'existing executable\n' >"$tmp/install-existing/ost"
chmod +x "$tmp/install-existing/ost"
start_flow "$tmp/install-existing" "$tmp/out4"
wait_for "Which build?" "$tmp/out4"; enter
wait_for "Update it in place and keep the current account?" "$tmp/out4"
send "$(printf '\033[B')"; enter
wait_for "Left as it was" "$tmp/out4"
finish_flow
grep -F "existing executable" "$tmp/install-existing/ost" >/dev/null
if grep -F "Installation complete" "$tmp/out4" >/dev/null; then
  echo "declining an update continued onboarding" >&2
  exit 1
fi
echo "declining an update leaves the installed binary untouched OK"

# ---- an openstream-era install is recognized and migrated without redoing login
mkdir -p "$tmp/install-legacy"
printf 'legacy executable\n' >"$tmp/install-legacy/openstream"
chmod +x "$tmp/install-legacy/openstream"
mkdir -p "$tmp/install-legacy/home/.config/systemd/user"
legacy_unit="$tmp/install-legacy/home/.config/systemd/user/openstream.service"
cat >"$legacy_unit" <<EOF
[Unit]
Description=OpenStream TV

[Service]
ExecStart=$tmp/install-legacy/openstream --config /path/x.toml --skip-update-check serve
EOF
cat >"$tmp/bin/systemctl" <<'EOF'
#!/bin/sh
case "$*" in
  'is-active --quiet openstream'|'--user is-active --quiet openstream') exit 1 ;;
  '--user daemon-reload'|'daemon-reload') exit 0 ;;
esac
exit 0
EOF
chmod +x "$tmp/bin/systemctl"
OPENSTREAM_TEST_SYSTEMD_UNIT=$legacy_unit
export OPENSTREAM_TEST_SYSTEMD_UNIT
start_flow "$tmp/install-legacy" "$tmp/out5"
wait_for "Which build?" "$tmp/out5"; enter
wait_for "Update it in place and keep the current account?" "$tmp/out5"; enter
wait_for "existing account settings were kept" "$tmp/out5"
finish_flow
unset OPENSTREAM_TEST_SYSTEMD_UNIT
rm "$tmp/bin/systemctl"
test -x "$tmp/install-legacy/ost"
if [ -e "$tmp/install-legacy/openstream" ]; then
  echo "legacy executable was not removed after service migration" >&2
  sed 's/\r$//' "$tmp/out5" >&2
  cat "$legacy_unit" >&2
  exit 1
fi
grep -F "ExecStart=$tmp/install-legacy/ost --config /path/x.toml --skip-update-check serve" "$legacy_unit" >/dev/null
if grep -F "Which account are you setting up?" "$tmp/out5" >/dev/null; then
  echo "a migrated openstream install repeated account setup" >&2
  exit 1
fi
echo "legacy openstream install migrates to ost without redoing account setup OK"

# ---- no terminal: the plain install must not prompt ----------------------
OPENSTREAM_INSTALL_DIR="$tmp/install-plain" PATH="$tmp/bin:$PATH" \
  OPENSTREAM_ONBOARD=always sh "$root/scripts/install.sh" >"$tmp/plain" 2>&1
if grep -F "Which build?" "$tmp/plain" >/dev/null; then
  echo "the installer prompted without a terminal" >&2
  exit 1
fi
grep -F "Installed ost (full, aarch64-apple-darwin)" "$tmp/plain" >/dev/null
echo "onboarding stays off without a terminal OK"
