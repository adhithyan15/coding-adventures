#!/usr/bin/env bash
# Build a Mosaic app's Rust runtime for Android, one shared library per ABI,
# laid out the way an Android project's jniLibs expects (UI89 §3.2, §3.6).
#
#   build-mosaic-android-libs.sh <cargo-package> <jniLibs-dir> [--release]
#
# writes
#
#   <jniLibs-dir>/arm64-v8a/libmosaic_app.so     phones and tablets
#   <jniLibs-dir>/armeabi-v7a/libmosaic_app.so   older 32-bit devices
#   <jniLibs-dir>/x86_64/libmosaic_app.so        the emulator on Intel/AMD hosts
#   <jniLibs-dir>/x86/libmosaic_app.so           old emulators
#
# Every library is renamed to `libmosaic_app.so`: the shared runtime host loads
# `mosaic_app` by name through JNA, on Android as on desktop. Pass the
# directory to `mosaic-compile pkg --backend compose --emit-project
# --runtime-library <jniLibs-dir>` and the builder installs it into
# `compose/android/src/main/jniLibs`.
#
# Why cargo-ndk: it points cargo at the NDK's clang for each ABI and at the
# right API level, which is otherwise four linker settings per target. The app
# crates already declare `cdylib`, so nothing in any Cargo.toml changes.
#
# Needs: an Android NDK (ANDROID_NDK_HOME), `cargo install cargo-ndk`, and
#   rustup target add aarch64-linux-android armv7-linux-androideabi \
#     x86_64-linux-android i686-linux-android
# Debug by default (CI's speed); --release for a distributable build.
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <cargo-package> <jniLibs-dir> [--release]" >&2
  exit 2
fi
package="$1"
output="$2"
release="${3:-}"
if [[ -n "$release" && "$release" != "--release" ]]; then
  echo "unknown option: $release" >&2
  exit 2
fi
# A package name is a cargo identifier; refuse anything else before it reaches
# a command line or a path.
if [[ ! "$package" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]]; then
  echo "invalid cargo package name: $package" >&2
  exit 2
fi
if [[ -z "$output" ]]; then
  echo "the jniLibs directory must not be empty" >&2
  exit 2
fi
if [[ -z "${ANDROID_NDK_HOME:-}" || ! -d "$ANDROID_NDK_HOME" ]]; then
  echo "ANDROID_NDK_HOME must name an installed Android NDK" >&2
  exit 2
fi
# Absolute for the same reason as the staging directory below.
ANDROID_NDK_HOME="$(CDPATH='' cd -P -- "$ANDROID_NDK_HOME" > /dev/null && pwd -P)"
export ANDROID_NDK_HOME

here="$(CDPATH='' cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" > /dev/null && pwd -P)"
workspace="$here/../packages/rust"
library="lib${package//-/_}.so"
# The minimum SDK the generated Android project declares (UI89 §3.5).
api_level=26
abis=(arm64-v8a armeabi-v7a x86_64 x86)

# Absolute, because cargo-ndk runs from the workspace below: a relative
# TMPDIR would otherwise put its output somewhere this script never looks.
staging="$(mktemp -d)"
staging="$(CDPATH='' cd -P -- "$staging" > /dev/null && pwd -P)"
trap 'rm -rf -- "$staging"' EXIT

ndk_args=()
for abi in "${abis[@]}"; do
  ndk_args+=(-t "$abi")
done
cargo_args=(build -p "$package")
if [[ "$release" == "--release" ]]; then
  cargo_args+=(--release)
fi
# From inside the workspace: cargo-ndk runs its own `cargo metadata` in the
# current directory before it hands the build to cargo, and ignores a
# `--manifest-path` meant for that build -- so called from anywhere else it
# stops at "could not find `Cargo.toml`". Staging and the NDK are absolute.
(CDPATH='' cd -P -- "$workspace" && cargo ndk "${ndk_args[@]}" --platform "$api_level" -o "$staging" "${cargo_args[@]}") >&2

# Replace only the four ABI directories, never the directory the caller
# named: a wrong path costs nothing but four stray folders, and anything else
# found there makes the artifact builder refuse the directory.
mkdir -p -- "$output"
for abi in "${abis[@]}"; do
  built="$staging/$abi/$library"
  test -f "$built" || { echo "cargo-ndk did not produce $built" >&2; exit 1; }
  rm -rf -- "${output:?}/$abi"
  mkdir -p -- "$output/$abi"
  cp -- "$built" "$output/$abi/libmosaic_app.so"
done
echo "wrote $output"
