#!/usr/bin/env bash
# Build a Mosaic app's Rust runtime for Android and install it into a generated
# Android project (UI89 §3.4).
#
#   build-mosaic-android-libs.sh <cargo-package> <android-project-dir> [--abis a,b,...] [--release]
#
# Writes <android-project-dir>/src/main/jniLibs/<abi>/libmosaic_app.so for each
# ABI. The name is the one the Compose runtime host asks JNA for
# (`mosaic_app`), whatever the crate is called, so every app's Android project
# loads its engine the same way.
#
# ABIs (default: all four):
#   arm64-v8a     phones and tablets
#   armeabi-v7a   older 32-bit phones
#   x86_64        the emulator on Intel hosts, Chromebooks
#   x86           old emulators
#
# Needs: the Android NDK (ANDROID_NDK_HOME, or ANDROID_HOME/ndk/<version>),
# `cargo install cargo-ndk`, and the Rust targets:
#   rustup target add aarch64-linux-android armv7-linux-androideabi \
#     x86_64-linux-android i686-linux-android
set -euo pipefail

usage() {
  echo "usage: $0 <cargo-package> <android-project-dir> [--abis a,b,...] [--release]" >&2
  exit 2
}
[[ $# -ge 2 ]] || usage
package="$1"
project="$2"
shift 2
abis="arm64-v8a,armeabi-v7a,x86_64,x86"
profile_flag=""
profile_dir="debug"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --abis) [[ $# -ge 2 ]] || usage; abis="$2"; shift 2 ;;
    --release) profile_flag="--release"; profile_dir="release"; shift ;;
    *) echo "unknown option: $1" >&2; usage ;;
  esac
done

# A package name is a cargo identifier; refuse anything else before it reaches
# a command line or a path.
if [[ ! "$package" =~ ^[A-Za-z0-9_-]+$ ]]; then
  echo "invalid cargo package name: $package" >&2
  exit 2
fi
if [[ ! -f "$project/src/main/AndroidManifest.xml" ]]; then
  echo "not a generated Android project (no src/main/AndroidManifest.xml): $project" >&2
  exit 2
fi
IFS=',' read -r -a abi_list <<< "$abis"
for abi in "${abi_list[@]}"; do
  case "$abi" in
    arm64-v8a|armeabi-v7a|x86_64|x86) ;;
    *) echo "unknown Android ABI: $abi" >&2; exit 2 ;;
  esac
done

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
  if [[ -n "$sdk" && -d "$sdk/ndk" ]]; then
    ANDROID_NDK_HOME="$(find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
    export ANDROID_NDK_HOME
  fi
fi
if [[ -z "${ANDROID_NDK_HOME:-}" || ! -d "$ANDROID_NDK_HOME" ]]; then
  echo "Android NDK not found: set ANDROID_NDK_HOME or ANDROID_HOME" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
workspace="$repo_root/code/packages/rust"
library="lib${package//-/_}.so"
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT

target_args=()
for abi in "${abi_list[@]}"; do
  target_args+=(-t "$abi")
done
(
  cd "$workspace"
  # shellcheck disable=SC2086 # profile_flag is empty or a single flag
  cargo ndk "${target_args[@]}" --platform 26 -o "$staging" build -p "$package" $profile_flag
)

for abi in "${abi_list[@]}"; do
  built="$staging/$abi/$library"
  if [[ ! -f "$built" ]]; then
    echo "cargo-ndk did not produce $abi/$library" >&2
    exit 1
  fi
  destination="$project/src/main/jniLibs/$abi"
  mkdir -p "$destination"
  cp "$built" "$destination/libmosaic_app.so"
  echo "installed $abi/libmosaic_app.so ($profile_dir)"
done
