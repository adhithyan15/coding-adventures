#!/usr/bin/env bash
# Install, verified and with retries, the Android NDK that this Flutter SDK
# pins, before a Flutter Android build needs it.
#
#   ensure-flutter-android-ndk.sh
#
# Why: a Flutter app's Android build compiles its plugins' native code with
# `ndkVersion = flutter.ndkVersion`. path_provider_android pulls in
# package:jni, whose C library is built by CMake under the `:jni` project.
# Flutter declares that version read-only (`val ndkVersion: String = "..."`
# in FlutterExtension.kt), so an app can't point its plugins at the NDK the
# runner already has. build-mosaic-flutter-phone-app.sh does that for `:app`
# only (X-7). When the pinned NDK is missing, the Android Gradle plugin
# downloads it (about 1 GB) in the middle of the build, once, with no retry
# and no integrity check. A truncated download fails the whole build:
#
#   Warning: An error occurred while preparing SDK package NDK (Side by side)
#   28.2.13676358: Error reading Zip content from a SeekableByteChannel.
#   > Failed to install the following SDK components:
#       ndk;28.2.13676358 NDK (Side by side) 28.2.13676358
#
# (PR #17914, `build (shard-4-of-15)`, "trestle (Flutter, Android)").
#
# So this script installs it up front with sdkmanager, which checks the
# archive's published checksum. It retries up to three times, and accepts
# the result only when `ndk/<version>/source.properties` names exactly that
# version. A failed attempt's partial directory is removed before the next.
#
# Truth table:
#
#   pinned NDK already installed, with the right Pkg.Revision -> exit 0, no download
#   not installed, sdkmanager found, an attempt succeeds      -> exit 0
#   not installed, sdkmanager found, three attempts fail      -> exit 1
#   not installed, no sdkmanager (a developer machine)        -> exit 0, Gradle
#                                                                downloads it as before
#   version unreadable from the Flutter SDK                   -> exit 1 (fail loudly:
#                                                                Flutter moved it)
#
# Needs: flutter on PATH (or FLUTTER_ROOT), ANDROID_HOME. Optional:
# ENSURE_NDK_RETRY_DELAY, the seconds to wait before the second attempt
# (doubled before the third; default 10), and ENSURE_NDK_ATTEMPT_TIMEOUT, the
# limit on one sdkmanager run (a `timeout` duration; default 12m), so a hung
# download can't eat the whole CI step's budget.
set -euo pipefail

: "${ANDROID_HOME:?ANDROID_HOME must name the Android SDK}"

# The Flutter SDK root: FLUTTER_ROOT if set, else two levels above the real
# `flutter` script (<root>/bin/flutter).
if [[ -z "${FLUTTER_ROOT:-}" ]]; then
  flutter_bin="$(command -v flutter)" || {
    echo "flutter is not on PATH and FLUTTER_ROOT is unset" >&2
    exit 1
  }
  FLUTTER_ROOT="$(cd "$(dirname "$(readlink -f "$flutter_bin")")/.." && pwd)"
fi
extension="$FLUTTER_ROOT/packages/flutter_tools/gradle/src/main/kotlin/FlutterExtension.kt"
if [[ ! -f "$extension" ]]; then
  echo "no FlutterExtension.kt under $FLUTTER_ROOT; cannot read Flutter's pinned NDK" >&2
  exit 1
fi
version="$(sed -n 's/^ *val ndkVersion: String = "\([^"]*\)" *$/\1/p' "$extension" | head -n 1)"
# A version is dotted digits and nothing else: it becomes a path component
# under ANDROID_HOME below, and a directory this script may delete.
if [[ ! "$version" =~ ^[0-9]+(\.[0-9]+)+$ ]]; then
  echo "could not read Flutter's pinned NDK from $extension (got '$version')" >&2
  exit 1
fi

ndk_dir="$ANDROID_HOME/ndk/$version"
installed() {
  [[ -f "$ndk_dir/source.properties" ]] &&
    [[ "$(sed -n 's/^Pkg\.Revision *= *//p' "$ndk_dir/source.properties" | tr -d '\r')" == "$version" ]]
}

if installed; then
  echo "NDK $version (Flutter's pin) is already installed at $ndk_dir"
  exit 0
fi

sdkmanager="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
if [[ ! -x "$sdkmanager" ]]; then
  sdkmanager="$(command -v sdkmanager || true)"
fi
if [[ -z "$sdkmanager" ]]; then
  echo "no sdkmanager: leaving NDK $version for the Android Gradle plugin to download" >&2
  exit 0
fi

delay="${ENSURE_NDK_RETRY_DELAY:-10}"
attempt_timeout="${ENSURE_NDK_ATTEMPT_TIMEOUT:-12m}"
# Both reach arithmetic or `timeout`: plain numbers (and a unit) only.
if [[ ! "$delay" =~ ^[0-9]+$ || ! "$attempt_timeout" =~ ^[0-9]+[smh]?$ ]]; then
  echo "ENSURE_NDK_RETRY_DELAY must be seconds and ENSURE_NDK_ATTEMPT_TIMEOUT a duration" >&2
  exit 2
fi

log="$(mktemp)"
trap 'rm -f -- "$log"' EXIT
for attempt in 1 2 3; do
  if (( attempt > 1 )); then
    echo "NDK $version: attempt $((attempt - 1)) failed; retrying in ${delay}s" >&2
    sleep "$delay"
    delay=$((delay * 2))
  fi
  # A half-unpacked directory from a failed attempt would make sdkmanager
  # skip or trip over the package; start each attempt clean.
  rm -rf -- "$ndk_dir"
  # sdkmanager can print a corrupt-archive warning and still exit 0, so its
  # status is not the verdict: source.properties is. --sdk_root keeps a
  # sdkmanager found on PATH installing into this ANDROID_HOME, not its own.
  yes | timeout "$attempt_timeout" "$sdkmanager" --sdk_root="$ANDROID_HOME" \
    --install "ndk;$version" > "$log" 2>&1 || true
  if installed; then
    echo "NDK $version (Flutter's pin) installed at $ndk_dir (attempt $attempt)"
    exit 0
  fi
done
echo "could not install ndk;$version after 3 attempts; last sdkmanager output:" >&2
tail -n 40 "$log" >&2
exit 1
