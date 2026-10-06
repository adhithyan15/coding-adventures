#!/usr/bin/env bash
# Build a Mosaic app's Rust runtime as iOS dynamic libraries, the ios/ half of
# a Flutter phone runtime (UI89 §7.2).
#
#   build-mosaic-ios-dylibs.sh <cargo-package> <phone-runtime-dir>
#
# writes
#
#   <phone-runtime-dir>/ios/iphoneos/libmosaic_app.dylib          arm64
#   <phone-runtime-dir>/ios/iphonesimulator/libmosaic_app.dylib   arm64 + x86_64
#
# and leaves anything else in <phone-runtime-dir> (an android/ half) alone.
#
# Why dynamic, when SwiftUI links the runtime statically
# (build-mosaic-xcframework.sh): Flutter's native assets bundle a dynamic
# library into the app, as a framework, and cannot link a static one. The
# app crates declare `cdylib` + `rlib`, so `cargo rustc --crate-type cdylib`
# builds the library without changing any crate's Cargo.toml.
#
# Slices:
#   aarch64-apple-ios       iPhone and iPad devices         -> iphoneos
#   aarch64-apple-ios-sim   the simulator on Apple silicon  } one fat
#   x86_64-apple-ios        the simulator on Intel Macs     } iphonesimulator
#
# A simulator build for both architectures asks for both, and `hook/build.dart`
# hands each one its own slice.
#
# Needs: macOS with Xcode (lipo), and the Rust targets:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <cargo-package> <phone-runtime-dir>" >&2
  exit 2
fi
package="$1"
output="$2"
# A package name is a cargo identifier; refuse anything else before it reaches
# a command line or a path.
if [[ ! "$package" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]]; then
  echo "invalid cargo package name: $package" >&2
  exit 2
fi
# The output's ios/ is removed and rewritten: never at the filesystem's root.
if [[ -z "$output" || "$output" == "/" ]]; then
  echo "invalid phone runtime directory: '$output'" >&2
  exit 2
fi

# iOS 16, as Mosaic's iOS app target (mosaic-ios-project). It also makes the
# linker write LC_BUILD_VERSION, which names device or simulator, and which the
# builder and the hook both check (UI89 §7.2). An older deployment target gets
# only LC_VERSION_MIN_IPHONEOS, which does not.
export IPHONEOS_DEPLOYMENT_TARGET=16.0

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$here/../packages/rust/Cargo.toml"
target_dir="$(cd "$here/../packages/rust" && pwd)/target"
library="lib${package//-/_}.dylib"

# Each build runs in a command substitution, where bash does not apply
# `set -e`: a failed cargo must still stop the script, or a library an earlier
# build left behind would be picked up instead (hence `|| exit 1`). The
# target directory is named, so a CARGO_TARGET_DIR elsewhere cannot leave
# this script reading a stale file at its default path.
build() {
  local target="$1"
  cargo rustc --manifest-path "$manifest" -p "$package" --release \
    --target "$target" --target-dir "$target_dir" --crate-type cdylib >&2 || exit 1
  local built="$target_dir/$target/release/$library"
  test -f "$built" || { echo "cargo did not produce $built" >&2; exit 1; }
  echo "$built"
}

# Every library is built before anything is removed, so a failed build
# leaves an existing ios/ as it was.
device_built="$(build aarch64-apple-ios)"
arm64_simulator="$(build aarch64-apple-ios-sim)"
x86_64_simulator="$(build x86_64-apple-ios)"

device="$output/ios/iphoneos/libmosaic_app.dylib"
simulator="$output/ios/iphonesimulator/libmosaic_app.dylib"
# Only this script's half is replaced: the runtime directory may already hold
# an android/ half, which is not this script's to touch.
rm -rf -- "${output:?}/ios"
mkdir -p -- "$(dirname -- "$device")" "$(dirname -- "$simulator")"
cp -- "$device_built" "$device"
lipo -create "$arm64_simulator" "$x86_64_simulator" -output "$simulator"
lipo -info "$device" "$simulator"
echo "wrote $output/ios"
