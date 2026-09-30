#!/usr/bin/env bash
# Boot a headless x86_64 Android emulator for the Mosaic Android gate
# (UI89 §3.7), and wait until Android says it has finished booting.
#
#   start-mosaic-android-emulator.sh <log-file>
#
# Needs ANDROID_HOME with `cmdline-tools/latest` (GitHub's Ubuntu images carry
# it) and hardware virtualisation (/dev/kvm): an x86_64 guest without KVM
# boots in tens of minutes, not one or two. The emulator keeps running in the
# background, writing to <log-file>; stop it with `adb emu kill`.
#
# The system image's API level, variant and ABI are pinned; its revision, like
# the emulator's, is whatever the SDK repository serves (logged below):
#
#   API 34    well above the app's minimum (26), and a level the emulator's
#             software renderer is widely run headless on in CI
#   default   plain AOSP: no Google APIs, nothing to sign in to, smallest image
#   x86_64    the ABI the host runs natively under KVM, and one of the four
#             the app's runtime is built for (build-mosaic-android-libs.sh)
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <log-file>" >&2
  exit 2
fi
log="$1"
if [[ -z "${ANDROID_HOME:-}" || ! -d "$ANDROID_HOME" ]]; then
  echo "ANDROID_HOME must name an Android SDK" >&2
  exit 2
fi
if [[ ! -e /dev/kvm ]]; then
  echo "no /dev/kvm: this machine cannot run an x86_64 emulator at usable speed" >&2
  exit 1
fi

image="system-images;android-34;default;x86_64"
avd="mosaic-gate"
sdkmanager="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
avdmanager="$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager"
emulator="$ANDROID_HOME/emulator/emulator"

# `yes` is killed by SIGPIPE once sdkmanager stops reading, which pipefail
# would report as a failure; only sdkmanager's own status matters.
(yes || true) | "$sdkmanager" --licenses > /dev/null
"$sdkmanager" --install emulator platform-tools "$image" > /dev/null
# The revisions this run got, for comparing a failing run with a passing one.
"$sdkmanager" --list_installed | grep -E "emulator|platform-tools|system-images" || true
# One directory for the virtual device, named for both tools. Left to
# themselves they can disagree: avdmanager has been seen to write under
# $XDG_CONFIG_HOME/.android/avd (which GitHub's runners set) while the
# emulator only searches $ANDROID_AVD_HOME, $ANDROID_SDK_HOME/avd and
# $HOME/.android/avd -- the first CI run created the device and then booted
# nothing ("Unknown AVD name [mosaic-gate]").
export ANDROID_AVD_HOME="${ANDROID_AVD_HOME:-$HOME/.android/avd}"
mkdir -p -- "$ANDROID_AVD_HOME"
# avdmanager asks whether to write a custom hardware profile; answer no.
echo no | "$avdmanager" create avd --force --name "$avd" --package "$image" --device pixel_6
# Proof before a two-minute wait: the emulator itself must list the device.
# If avdmanager ignored ANDROID_AVD_HOME and wrote somewhere under $HOME
# instead, point the emulator at the directory it actually used (and say so).
# Captured, then searched: `grep -q` on the pipe would exit at the first match
# and, under pipefail, turn the emulator's SIGPIPE into "not listed".
listed() {
  local avds
  avds="$("$emulator" -list-avds 2>/dev/null || true)"
  grep -Fxq -- "$avd" <<< "$avds"
}
if ! listed; then
  written="$(find "$HOME" -maxdepth 5 -name "$avd.ini" -print -quit 2>/dev/null || true)"
  if [[ -n "$written" ]]; then
    echo "avdmanager wrote $avd to $(dirname -- "$written"), not $ANDROID_AVD_HOME; using that" >&2
    ANDROID_AVD_HOME="$(dirname -- "$written")"
    export ANDROID_AVD_HOME
  fi
fi
if ! listed; then
  echo "avdmanager created $avd, but the emulator does not list it (ANDROID_AVD_HOME=$ANDROID_AVD_HOME):" >&2
  "$emulator" -list-avds >&2 || true
  exit 1
fi

# Headless, silent, cold every time (no snapshot to resume a stale state from),
# rendered in software: runners have no GPU.
nohup "$emulator" -avd "$avd" -no-window -no-audio -no-boot-anim -no-snapshot \
  -gpu swiftshader_indirect -camera-back none -camera-front none \
  > "$log" 2>&1 &
emulator_pid=$!

adb start-server > /dev/null
# Wait for adb to see the device, but stop at once if the emulator exits: a
# refusal (a bad AVD, no KVM) otherwise costs the whole timeout.
appeared=0
for _ in $(seq 1 60); do
  if ! kill -0 "$emulator_pid" 2>/dev/null; then
    echo "the emulator exited before it appeared to adb" >&2
    tail -n 80 "$log" >&2
    exit 1
  fi
  status=0
  timeout 2 adb wait-for-device || status=$?
  if [[ "$status" -eq 0 ]]; then
    appeared=1
    break
  fi
  # Only a timeout (124) means "not yet"; adb failing fast must not burn the
  # sixty tries in a few seconds.
  if [[ "$status" -ne 124 ]]; then
    sleep 2
  fi
done
if [[ "$appeared" -ne 1 ]]; then
  echo "the emulator never appeared to adb" >&2
  tail -n 80 "$log" >&2
  exit 1
fi
# `wait-for-device` returns as soon as adb can talk to it; the system is ready
# for `am start` only once it sets sys.boot_completed.
for _ in $(seq 1 120); do
  if [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == 1 ]]; then
    # Animations only slow a test down and can hold `am start -W`.
    adb shell settings put global window_animation_scale 0
    adb shell settings put global transition_animation_scale 0
    adb shell settings put global animator_duration_scale 0
    echo "emulator booted"
    exit 0
  fi
  sleep 5
done
echo "the emulator did not finish booting in ten minutes" >&2
tail -n 80 "$log" >&2
exit 1
