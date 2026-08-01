#!/usr/bin/env bash

set -euo pipefail

script_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
package_root="$(cd "$script_root/.." && pwd)"
rust_workspace="$(cd "$package_root/../../../packages/rust" && pwd)"
output="target/mosaic-venture-browser"
release=0
emit_only=0
strict=0

usage() {
  cat <<'EOF'
Usage: scripts/build-all.sh [--output PATH] [--release] [--emit-only] [--strict]

Emits Venture's shared Mosaic package for every supported backend, then builds
each project whose native toolchain is available on the current host.

  --output PATH  Generated project root (default: target/mosaic-venture-browser)
  --release      Build mosaic-compile in release mode
  --emit-only    Emit all project shells without invoking backend toolchains
  --strict       Fail when a host-applicable backend toolchain is missing
EOF
}

while (($#)); do
  case "$1" in
    --output)
      [[ $# -ge 2 ]] || { echo "--output requires a path" >&2; exit 2; }
      output="$2"
      shift 2
      ;;
    --release)
      release=1
      shift
      ;;
    --emit-only)
      emit_only=1
      shift
      ;;
    --strict)
      strict=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$output" == /* ]]; then
  output_path="$output"
else
  output_path="$package_root/$output"
fi
mkdir -p "$output_path"
output_root="$(cd "$output_path" && pwd)"
backends=(react electron swiftui qt webcomponent html xaml flutter compose)
cargo_args=(run -q -p mosaic-compile)
if ((release)); then
  cargo_args+=(--release)
fi
cargo_args+=(-- pkg "$package_root")

for backend in "${backends[@]}"; do
  echo "==> Emitting $backend"
  (
    cd "$rust_workspace"
    cargo "${cargo_args[@]}" \
      --backend "$backend" \
      --output "$output_root" \
      --emit-project \
      --theme light
  )
done

if ((emit_only)); then
  echo "Emitted ${#backends[@]} Venture backend projects under $output_root"
  exit 0
fi

skipped=()
deferred=()

skip_backend() {
  local backend="$1"
  local reason="$2"
  echo "==> Skipping $backend: $reason"
  skipped+=("$backend ($reason)")
}

defer_backend() {
  local backend="$1"
  local reason="$2"
  echo "==> Deferring $backend: $reason"
  deferred+=("$backend ($reason)")
}

has_command() {
  command -v "$1" >/dev/null 2>&1
}

build_node_project() {
  local backend="$1"
  if ! has_command npm; then
    skip_backend "$backend" "npm is not installed"
    return
  fi
  echo "==> Building $backend"
  (cd "$output_root/$backend" && npm install --ignore-scripts && npm run build)
}

build_node_project react
build_node_project electron

if has_command node; then
  echo "==> Checking html"
  (cd "$output_root/html" && node --check main.js)
  echo "==> Checking webcomponent"
  (
    cd "$output_root/webcomponent"
    node --check VentureChrome.js
    node --check index.js
    node --check main.js
  )
else
  skip_backend html "node is not installed"
  skip_backend webcomponent "node is not installed"
fi

host_os="$(uname -s)"

if [[ "$host_os" == Darwin ]] && has_command swift; then
  echo "==> Building Venture macOS native bridge"
  bridge_args=(build -p venture-browser-macos)
  bridge_profile=debug
  if ((release)); then
    bridge_args+=(--release)
    bridge_profile=release
  fi
  (cd "$rust_workspace" && cargo "${bridge_args[@]}")
  cp "$rust_workspace/target/$bridge_profile/libventure_browser_macos.dylib" \
    "$output_root/swiftui/libventure_browser_macos.dylib"
  echo "==> Building swiftui"
  (cd "$output_root/swiftui" && swift build)
elif [[ "$host_os" != Darwin ]]; then
  defer_backend swiftui "SwiftUI builds require macOS"
else
  skip_backend swiftui "swift is not installed"
fi

if has_command cmake; then
  echo "==> Building qt"
  if has_command qt-cmake; then
    (cd "$output_root/qt" && qt-cmake -S . -B build && cmake --build build)
  else
    (cd "$output_root/qt" && cmake -S . -B build && cmake --build build)
  fi
else
  skip_backend qt "cmake is not installed"
fi

case "$host_os" in
  MINGW*|MSYS*|CYGWIN*)
    if has_command dotnet; then
      echo "==> Building xaml"
      (cd "$output_root/xaml" && dotnet build VentureChrome.csproj)
    else
      skip_backend xaml "dotnet is not installed"
    fi
    ;;
  *)
    defer_backend xaml "WinUI builds require Windows"
    ;;
esac

flutter_platform=""
case "$host_os" in
  Darwin) flutter_platform=macos ;;
  Linux) flutter_platform=linux ;;
  MINGW*|MSYS*|CYGWIN*) flutter_platform=windows ;;
esac

if [[ -n "$flutter_platform" ]] && has_command flutter; then
  echo "==> Building flutter ($flutter_platform)"
  (
    cd "$output_root/flutter"
    flutter pub get
    flutter analyze lib
    if [[ ! -d "$flutter_platform" ]]; then
      flutter create "--platforms=$flutter_platform" .
    fi
    flutter build "$flutter_platform"
  )
elif [[ -z "$flutter_platform" ]]; then
  defer_backend flutter "unsupported host platform"
else
  skip_backend flutter "flutter is not installed"
fi

java_major=""
if has_command java; then
  java_version="$(java -version 2>&1 || true)"
  java_major="$(printf '%s\n' "$java_version" | sed -nE 's/.*version "([0-9]+).*/\1/p' | head -1)"
fi
if ! has_command gradle; then
  skip_backend compose "gradle is not installed"
elif [[ -z "$java_major" || "$java_major" -lt 21 ]]; then
  skip_backend compose "JDK 21 or newer is not installed"
else
  echo "==> Building compose"
  (cd "$output_root/compose" && gradle --no-daemon build)
fi

echo "Built or checked $((${#backends[@]} - ${#skipped[@]} - ${#deferred[@]})) of ${#backends[@]} Venture backend projects."
if ((${#deferred[@]})); then
  printf 'Deferred to native hosts: %s\n' "${deferred[*]}"
fi
if ((${#skipped[@]})); then
  printf 'Skipped: %s\n' "${skipped[*]}"
  if ((strict)); then
    echo "Strict mode requires every host-applicable backend gate to run." >&2
    exit 1
  fi
fi
