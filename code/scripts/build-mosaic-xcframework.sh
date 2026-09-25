#!/usr/bin/env bash
# Build a Mosaic app's Rust runtime as an .xcframework of static libraries,
# for linking into a SwiftUI package on iOS and iPadOS (UI89 §2.1).
#
#   build-mosaic-xcframework.sh <cargo-package> <output.xcframework> [--with-macos]
#
# Why static: iOS and iPadOS do not allow an app to load its own .dylib at run
# time, so the runtime is linked in. The app crates declare `cdylib` + `rlib`;
# `cargo rustc --crate-type staticlib` builds the static library without
# changing any crate's Cargo.toml.
#
# Slices:
#   aarch64-apple-ios       iPhone and iPad devices
#   aarch64-apple-ios-sim   the simulator on Apple silicon  } one fat simulator
#   x86_64-apple-ios        the simulator on Intel Macs     } slice (lipo)
#   aarch64-apple-darwin    macOS, with --with-macos (so one package links the
#                           same way everywhere; macOS can also keep its dylib)
#
# A generic simulator build links both simulator architectures, so the
# simulator slice must carry both.
#
# Needs: macOS with Xcode (xcodebuild, lipo), and the Rust targets:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <cargo-package> <output.xcframework> [--with-macos]" >&2
  exit 2
fi
package="$1"
output="$2"
with_macos="${3:-}"
if [[ -n "$with_macos" && "$with_macos" != "--with-macos" ]]; then
  echo "unknown option: $with_macos" >&2
  exit 2
fi
# A package name is a cargo identifier; refuse anything else before it reaches
# a command line or a path.
if [[ ! "$package" =~ ^[A-Za-z0-9_-]+$ ]]; then
  echo "invalid cargo package name: $package" >&2
  exit 2
fi
if [[ "$output" != *.xcframework ]]; then
  echo "output must end in .xcframework: $output" >&2
  exit 2
fi

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$here/../packages/rust/Cargo.toml"
target_dir="$(cd "$here/../packages/rust" && pwd)/target"
library="lib${package//-/_}.a"

build() {
  local target="$1"
  cargo rustc --manifest-path "$manifest" -p "$package" --release \
    --target "$target" --crate-type staticlib >&2
  local built="$target_dir/$target/release/$library"
  test -f "$built" || { echo "cargo did not produce $built" >&2; exit 1; }
  echo "$built"
}

args=(-library "$(build aarch64-apple-ios)")

simulator="$target_dir/ios-simulator-universal/$library"
mkdir -p "$(dirname "$simulator")"
lipo -create "$(build aarch64-apple-ios-sim)" "$(build x86_64-apple-ios)" -output "$simulator"
args+=(-library "$simulator")

if [[ "$with_macos" == "--with-macos" ]]; then
  args+=(-library "$(build aarch64-apple-darwin)")
fi

rm -rf -- "$output"
xcodebuild -create-xcframework "${args[@]}" -output "$output"
echo "wrote $output"
