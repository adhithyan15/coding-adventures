#!/usr/bin/env bash
# Build a Mosaic app through the Flutter backend for a phone, and check what
# came out (UI89 §7).
#
#   build-mosaic-flutter-phone-app.sh <android|ios> <mosaic-program-dir> \
#     <phone-runtime-dir> <output-dir> <org> <project-name>
#
# e.g. for Trestle on Android:
#
#   build-mosaic-flutter-phone-app.sh android code/programs/mosaic/task-app \
#     "$RUNNER_TEMP/trestle-phone-runtime" "$RUNNER_TEMP/trestle-flutter" \
#     dev.codingadventures trestle
#
# <phone-runtime-dir> holds the half for <platform>: android/<abi>/libmosaic_app.so
# (build-mosaic-android-libs.sh) or ios/<sdk>/libmosaic_app.dylib
# (build-mosaic-ios-dylibs.sh). <org> and <project-name> are the app's bundle
# identifier split at its last dot (dev.codingadventures.trestle), the same
# split the generated README makes, which this script checks.
#
# Steps, each of which stops the script when it fails:
#
#   1. mosaic-compile pkg --backend flutter --runtime-library <dir>: the
#      build must be native-complete with no degradations.
#   2. The README's `flutter create` command must be exactly the one derived
#      here; then that command runs. The README's text is checked, never run.
#   3. Android: `android:allowBackup="false"` on <application>, as Mosaic's
#      Compose manifest has it, so the app's state stays on the device.
#   4. flutter pub get (path_provider's resolved versions are printed),
#      flutter analyze, and a debug build: `flutter build apk --debug`, or
#      `flutter build ios --simulator --debug`.
#   5. The built app carries the engine:
#      - Android: the APK's manifest says allowBackup false (aapt2). Each
#        packaged lib/<abi>/libmosaic_app.so has its input's ELF machine and
#        exported symbols, including mosaic_app_create. Not its bytes: the
#        Android Gradle plugin strips debug symbols while packaging. x86_64
#        (the emulator's ABI) and arm64-v8a must be there.
#      - iOS: the Runner.app's bundle identifier is <org>.<project-name>, and
#        one of its frameworks exports _mosaic_app_create.
#
# The app is left at <output-dir>/flutter/build/app/outputs/flutter-apk/app-debug.apk
# (Android) or <output-dir>/flutter/build/ios/iphonesimulator/Runner.app (iOS),
# for the emulator and simulator gates.
#
# Needs: flutter (with native assets enabled), cargo, jq; for Android the SDK
# (ANDROID_HOME) and NDK (ANDROID_NDK_LATEST_HOME or ANDROID_NDK_HOME); for iOS
# Xcode.
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 <android|ios> <mosaic-program-dir> <phone-runtime-dir> <output-dir> <org> <project-name>" >&2
  exit 2
fi
platform="$1"
program="$2"
runtime="$3"
output="$4"
org="$5"
name="$6"
if [[ "$platform" != android && "$platform" != ios ]]; then
  echo "platform must be android or ios: $platform" >&2
  exit 2
fi
# The generated README's own rules (UI89 §7.1). Every token of the create
# command is then free of shell metacharacters.
if [[ ! "$org" =~ ^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$ ]]; then
  echo "invalid org: $org" >&2
  exit 2
fi
if [[ ! "$name" =~ ^[a-z][a-z0-9_]*$ ]]; then
  echo "invalid project name: $name" >&2
  exit 2
fi
if [[ ! -d "$program" || ! -d "$runtime/$platform" ]]; then
  echo "need a Mosaic program directory and $runtime/$platform" >&2
  exit 2
fi
# The output is removed and rebuilt: never the filesystem's root, and never a
# directory this script did not make. A folder that exists and is not empty
# must hold an earlier build's flutter/mosaic-degradations.json, so a mistyped
# path (a home directory, the repository) is refused rather than deleted.
if [[ -z "$output" || "$output" == "/" ]]; then
  echo "invalid output directory: '$output'" >&2
  exit 2
fi
if [[ -e "$output" && -n "$(ls -A -- "$output")" && ! -f "$output/flutter/mosaic-degradations.json" ]]; then
  echo "refusing to replace $output: it is not empty and not an earlier build's output" >&2
  exit 2
fi

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
program="$(cd "$program" && pwd)"
runtime="$(cd "$runtime" && pwd)"

rm -rf -- "${output:?}"
mkdir -p -- "$output"
output="$(cd "$output" && pwd)"
cargo run --manifest-path "$repo/code/packages/rust/mosaic-compile/Cargo.toml" -- \
  pkg "$program" --backend flutter --output "$output" --emit-project \
  --profile native-complete --runtime-library "$runtime"
jq -e '.nativeComplete == true and (.degradations | type == "array" and length == 0)' \
  "$output/flutter/mosaic-degradations.json"

cd "$output/flutter"
create=(flutter create "--platforms=$platform" --org "$org" --project-name "$name" .)
grep -qxF "    ${create[*]}" README.md || {
  echo "README.md does not give: ${create[*]}" >&2
  exit 1
}
"${create[@]}"

if [[ "$platform" == android ]]; then
  manifest=android/app/src/main/AndroidManifest.xml
  if grep -q 'android:allowBackup=' "$manifest"; then
    sed -i 's/android:allowBackup="[a-z]*"/android:allowBackup="false"/' "$manifest"
  else
    sed -i 's/<application/<application android:allowBackup="false"/' "$manifest"
  fi
  test "$(grep -c 'android:allowBackup="false"' "$manifest")" = 1
fi

flutter pub get
# What path_provider's implementations resolved to (UI89 §7.4).
grep -A3 -E '^  path_provider(_android|_foundation|_platform_interface)?:' pubspec.lock
flutter analyze

checks="$(mktemp -d)"
trap 'rm -rf -- "$checks"' EXIT

if [[ "$platform" == android ]]; then
  flutter build apk --debug
  apk=build/app/outputs/flutter-apk/app-debug.apk
  test -f "$apk"
  aapt2="$(find "$ANDROID_HOME/build-tools" -name aapt2 -type f | sort -V | tail -n 1)"
  test -n "$aapt2"
  "$aapt2" dump xmltree --file AndroidManifest.xml "$apk" > "$checks/manifest.txt"
  grep -Eq 'allowBackup\(0x01010280\)=false' "$checks/manifest.txt"
  ndk="${ANDROID_NDK_LATEST_HOME:-${ANDROID_NDK_HOME:-}}"
  test -n "$ndk"
  test -d "$ndk"
  llvm_bin="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
  test -x "$llvm_bin/llvm-nm"
  test -x "$llvm_bin/llvm-readelf"
  unzip -q -o "$apk" 'lib/*' -d "$checks/apk"
  packaged=0
  for library in "$checks"/apk/lib/*/libmosaic_app.so; do
    abi="$(basename "$(dirname "$library")")"
    input="$runtime/android/$abi/libmosaic_app.so"
    test -f "$input"
    # Each one is that ABI's own engine, never another's.
    for side in packaged input; do
      file="$library"
      [[ "$side" == input ]] && file="$input"
      "$llvm_bin/llvm-readelf" -h "$file" | grep -E '^ +Machine:' > "$checks/$abi.$side.machine"
      "$llvm_bin/llvm-nm" -D --defined-only "$file" > "$checks/$abi.$side.symbols"
    done
    diff "$checks/$abi.packaged.machine" "$checks/$abi.input.machine"
    diff "$checks/$abi.packaged.symbols" "$checks/$abi.input.symbols"
    grep -Eq ' T mosaic_app_create$' "$checks/$abi.packaged.symbols"
    packaged=$((packaged + 1))
  done
  test "$packaged" -ge 2
  for abi in x86_64 arm64-v8a; do
    test -f "$checks/apk/lib/$abi/libmosaic_app.so"
  done
  echo "built $output/flutter/$apk"
else
  flutter build ios --simulator --debug
  app=build/ios/iphonesimulator/Runner.app
  test -d "$app"
  test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Info.plist")" = "$org.$name"
  found=0
  for binary in "$app"/Frameworks/*.framework/*; do
    [[ -f "$binary" ]] || continue
    if nm -gU "$binary" 2>/dev/null > "$checks/symbols.txt" \
      && grep -q ' _mosaic_app_create$' "$checks/symbols.txt"; then
      echo "engine: $binary"
      found=1
    fi
  done
  test "$found" = 1
  echo "built $output/flutter/$app"
fi
