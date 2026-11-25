#!/bin/sh
set -eu
self=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
root=$(git -C "$self" rev-parse --show-toplevel)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/ca-c-microbenchmark.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
prefix="$tmp/prefix"
build_install() {
  name=$1 source=$2; shift 2
  cmake -S "$source" -B "$tmp/$name-build" "-DCMAKE_INSTALL_PREFIX=$prefix" "$@"
  cmake --build "$tmp/$name-build"
  cmake --install "$tmp/$name-build"
}
build_install bounded-json "$root/code/packages/c/bounded-json"
build_install rank-statistics "$self/../rank-statistics"
build_install microbenchmark "$self" "-DCMAKE_PREFIX_PATH=$prefix"
cmake -S "$self/tests/install" -B "$tmp/consumer-build" "-DCMAKE_PREFIX_PATH=$prefix"
cmake --build "$tmp/consumer-build"
"$tmp/consumer-build/installed-consumer"
"$prefix/bin/array-microbenchmark" --revision install-smoke --pairs 40 > "$tmp/native-report.json"
"$prefix/bin/array-microbenchmark" --replay "$tmp/native-report.json"
host_os=$(uname -s)
case "$host_os" in
  MSYS*|MINGW*|CYGWIN*) has_native_full_sink=false ;;
  *) [ -e /dev/full ] && has_native_full_sink=true || has_native_full_sink=false ;;
esac
if [ "$has_native_full_sink" = true ]; then
  if "$prefix/bin/array-microbenchmark" --revision install-sink-check --pairs 40 > /dev/full; then
    echo "benchmark CLI accepted a failed stdout sink" >&2
    exit 1
  fi
fi
