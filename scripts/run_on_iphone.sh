#!/usr/bin/env bash
# Run Runalytics on a connected iPhone with hot reload.
#
#   ./scripts/run_on_iphone.sh              # first physical device found
#   ./scripts/run_on_iphone.sh "iPhone di Francesco"
#   ./scripts/run_on_iphone.sh --build      # archive + install, no dev server
#
# Why this script exists — three things that break a plain `tauri ios dev`:
#
# 1. `npm run dev` has a `predev` hook that runs
#    `lsof -ti tcp:1499 | xargs kill -9`. While Tauri waits for the dev server
#    it holds an open connection to 1499, so lsof lists the Tauri CLI itself and
#    predev kills its own parent. Here Vite is started directly, bypassing the
#    npm script, and beforeDevCommand is cleared for the same reason.
#
# 2. A physical phone cannot reach `localhost` on the Mac, so the dev server
#    must listen on the Mac's LAN address. Tauri rewrites devUrl for us, but
#    Vite only binds that address if TAURI_DEV_HOST is set when it starts.
#
# 3. The generated Xcode project is unsigned and `tauri ios dev` spawns
#    xcodebuild with a filtered environment, so DEVELOPMENT_TEAM on the shell
#    never reaches it. The team has to be written into gen/apple/project.yml.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
APPLE="$REPO/src-tauri/gen/apple"
PORT=1499
VITE_PID_FILE="${TMPDIR:-/tmp}/runalytics_vite.$$.pid"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# --- args -------------------------------------------------------------------
BUILD_ONLY=0
DEVICE_WANT=""
for arg in "$@"; do
  case "$arg" in
    --build) BUILD_ONLY=1 ;;
    -h|--help) sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) die "unknown option $arg (try --help)" ;;
    *) DEVICE_WANT="$arg" ;;
  esac
done

# --- 1. pick a physical device ----------------------------------------------
# devicectl lists simulators too; `reality` separates them. The UDID xcodebuild
# and devicectl want is hardwareProperties.udid, NOT the top-level identifier
# (that is a CoreDevice UUID which xcodebuild rejects).
#
# Fields come back tab-separated: `read` splits on whitespace, so a device name
# containing spaces cannot be recovered by escaping the spaces.
DEV_JSON="$(mktemp)"
trap 'rm -f "$DEV_JSON"' EXIT
xcrun devicectl list devices --json-output "$DEV_JSON" >/dev/null 2>&1 \
  || die "xcrun devicectl failed — is Xcode selected? (xcode-select -p)"

IFS=$'\t' read -r DEVICE_NAME DEVICE_UDID DEVICE_OS < <(
  WANT="${DEVICE_WANT:-}" python3 - "$DEV_JSON" <<'PY'
import json, os, sys
want = os.environ.get("WANT", "")
for dev in json.load(open(sys.argv[1])).get("result", {}).get("devices", []):
    hw = dev.get("hardwareProperties") or {}
    if hw.get("reality") != "physical":
        continue
    dp = dev.get("deviceProperties") or {}
    name = dp.get("name") or ""
    if want and want.lower() not in name.lower():
        continue
    print("\t".join((name, hw.get("udid") or "", dp.get("osVersionNumber") or "")))
    break
PY
) || true

[[ -n "${DEVICE_UDID:-}" ]] || die "no connected iPhone found${DEVICE_WANT:+ matching \"$DEVICE_WANT\"}. Plug it in, unlock it, and trust this Mac."
say "device: $DEVICE_NAME (iOS $DEVICE_OS)"

# Developer mode has to be on or the install is refused with a confusing error.
if ! xcrun devicectl list devices 2>/dev/null | grep -q "available (paired)"; then
  say "warning: device is not 'available (paired)' — unlock it and check Settings → Privacy & Security → Developer Mode"
fi

# --- 2. make sure the iOS scaffold exists -----------------------------------
if [[ ! -f "$APPLE/project.yml" ]]; then
  say "generating the iOS scaffold (tauri ios init)"
  # init prompts "update outdated dependencies? [Y/n]" and looks hung when the
  # prompt is swallowed by a pipe; 'n' is the right answer for revision bumps.
  (cd "$REPO" && printf 'n\n' | npx tauri ios init)
fi

# The CLI emits a Podfile target for a macOS app that project.yml never creates,
# which makes pod install fail with "Unable to find a target named
# runalytics_macOS". Deterministic in tauri-cli 2.12 — strip it.
if grep -q "^target 'runalytics_macOS'" "$APPLE/Podfile" 2>/dev/null; then
  say "removing the phantom runalytics_macOS target from the Podfile"
  ruby -e '
    path = ARGV[0]
    src = File.read(path)
    out = src.sub(/\ntarget \x27runalytics_macOS\x27 do.*?\nend\n/m, "\n")
    abort("could not strip the macOS target") if out == src
    File.write(path, out)
  ' "$APPLE/Podfile"
fi

# --- 3. code signing lives in the project, not the environment --------------
# The team identifier is the certificate subject's OU field. `security
# find-identity` prints the common name, whose parenthesised code is the *key*
# id rather than the team — passing that to Xcode fails with a signing error.
TEAM="${DEVELOPMENT_TEAM:-$(security find-certificate -a -c "Apple Development" -p 2>/dev/null \
  | openssl x509 -noout -subject 2>/dev/null \
  | sed -n 's/.*OU *= *\([A-Z0-9]*\).*/\1/p' | head -1)}"
[[ -n "$TEAM" ]] || die "no Apple Development identity found. Open Xcode → Settings → Accounts and add your Apple ID."
say "signing with team $TEAM"

if ! grep -q "DEVELOPMENT_TEAM" "$APPLE/project.yml"; then
  say "writing DEVELOPMENT_TEAM into project.yml"
  ruby -e '
    path, team = ARGV[0], ARGV[1]
    src = File.read(path)
    needle = "        ENABLE_BITCODE: false\n"
    abort("ENABLE_BITCODE anchor missing") unless src.include?(needle)
    add = needle + "        DEVELOPMENT_TEAM: #{team}\n        CODE_SIGN_STYLE: automatic\n"
    File.write(path, src.sub(needle, add))
  ' "$APPLE/project.yml" "$TEAM"
  # xcodegen owns project.pbxproj, and regenerating it drops the CocoaPods
  # integration, so pod install has to run after every regeneration.
  say "regenerating the Xcode project and re-running pod install"
  (cd "$APPLE" && xcodegen generate >/dev/null && pod install >/dev/null)
fi

# --- 4. archive + install path (no dev server) ------------------------------
if [[ "$BUILD_ONLY" -eq 1 ]]; then
  say "archiving for $DEVICE_NAME (first run compiles every crate for aarch64-apple-ios)"
  # `tauri ios build` has no --devices flag and no --ci flag either (it accepts
  # only --debug/--target/--features/--config/--open), and --no-sign would
  # produce an app the phone refuses to run. So: archive signed, install by hand.
  (cd "$REPO" && npx tauri ios build --debug --target aarch64)
  # Search rather than assume: the export directory differs between tauri-cli
  # versions, so a hardcoded bundle/ipa path can miss a successful build.
  # Picking the newest by hand — `find | xargs ls -t` runs bare `ls` (listing the
  # current directory) when find matches nothing, which would fake a success.
  IPA=""
  while IFS= read -r cand; do
    [[ -z "$IPA" || "$cand" -nt "$IPA" ]] && IPA="$cand"
  done < <(find "$REPO/src-tauri/target" -name '*.ipa' -type f 2>/dev/null)
  [[ -n "$IPA" ]] || die "no .ipa found under src-tauri/target — check the build output above"
  say "installing $(basename "$IPA") on $DEVICE_NAME"
  STAGE="$(mktemp -d)"
  unzip -q "$IPA" -d "$STAGE"
  APP="$(find "$STAGE/Payload" -maxdepth 1 -name '*.app' | head -1)"
  [[ -n "$APP" ]] || die "the .ipa contained no .app under Payload"
  xcrun devicectl device install app --device "$DEVICE_UDID" "$APP"
  xcrun devicectl device process launch --device "$DEVICE_UDID" com.fradone.runalytics
  rm -rf "$STAGE"
  say "done"
  exit 0
fi

# --- 5. dev server on the LAN address ---------------------------------------
IFACE="$(route -n get default 2>/dev/null | awk '/interface:/{print $2}')"
LAN_IP="$(ipconfig getifaddr "$IFACE" 2>/dev/null || true)"
[[ -n "$LAN_IP" ]] || die "could not find the Mac's LAN address — connect to Wi-Fi or Ethernet"
say "dev server: http://$LAN_IP:$PORT (the phone must be on the same network)"

VITE_LOG="$(mktemp)"
# Started directly rather than through `npm run dev` so the predev kill never
# runs. `npx` spawns a child, so killing its pid alone can leave vite behind —
# clean up by pattern too. DEV_JSON is removed here because installing this trap
# replaces the earlier EXIT trap.
(cd "$REPO" && TAURI_DEV_HOST="$LAN_IP" nohup npx vite --port "$PORT" --strictPort \
  > "$VITE_LOG" 2>&1 & echo $! > "$VITE_PID_FILE")
VITE_PID="$(cat "$VITE_PID_FILE")"
cleanup() {
  kill "$VITE_PID" 2>/dev/null || true
  pkill -f "runalytics/node_modules/\.bin/vite" 2>/dev/null || true
  rm -f "$DEV_JSON" "$VITE_LOG" "$VITE_PID_FILE"
}
trap cleanup EXIT INT TERM

for _ in $(seq 1 60); do
  curl -sf -o /dev/null "http://$LAN_IP:$PORT/index.html" && break
  kill -0 "$VITE_PID" 2>/dev/null || { echo "--- vite log ---"; cat "$VITE_LOG"; die "vite exited early"; }
  sleep 0.5
done
curl -sf -o /dev/null "http://$LAN_IP:$PORT/index.html" || die "dev server never answered on $LAN_IP:$PORT"

# --- 6. build, install, launch, watch ---------------------------------------
say "building for $DEVICE_NAME — the first run compiles every crate for aarch64-apple-ios and takes several minutes"
say "if the phone shows an untrusted-developer prompt: Settings → General → VPN & Device Management → trust, then reopen the app"
# beforeDevCommand is emptied because npm's predev would kill this very process.
(cd "$REPO" && exec npx tauri ios dev "$DEVICE_NAME" --host "$LAN_IP" \
  --no-dev-server-wait --config '{"build":{"beforeDevCommand":""}}')
