#!/bin/sh
set -eu
self=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
root=$(git -C "$self" rev-parse --show-toplevel)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/ca-c-microbenchmark.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
prefix="$tmp/prefix"
configuration=Release
build_install() {
  name=$1 source=$2; shift 2
  cmake -S "$source" -B "$tmp/$name-build" "-DCMAKE_INSTALL_PREFIX=$prefix" "-DCMAKE_BUILD_TYPE=$configuration" "$@"
  cmake --build "$tmp/$name-build" --config "$configuration"
  cmake --install "$tmp/$name-build" --config "$configuration"
}
build_install bounded-json "$root/code/packages/c/bounded-json"
build_install rank-statistics "$self/../rank-statistics"
build_install microbenchmark "$self" "-DCMAKE_PREFIX_PATH=$prefix"
cmake -S "$self/tests/install" -B "$tmp/consumer-build" "-DCMAKE_PREFIX_PATH=$prefix" "-DCMAKE_BUILD_TYPE=$configuration"
cmake --build "$tmp/consumer-build" --config "$configuration"
consumer_exe="$tmp/consumer-build/$configuration/installed-consumer"
if [ ! -x "$consumer_exe" ] && [ -x "$consumer_exe.exe" ]; then
  consumer_exe="$consumer_exe.exe"
fi
if [ ! -x "$consumer_exe" ]; then
  consumer_exe="$tmp/consumer-build/installed-consumer"
  if [ ! -x "$consumer_exe" ] && [ -x "$consumer_exe.exe" ]; then
    consumer_exe="$consumer_exe.exe"
  fi
fi
"$consumer_exe"
"$prefix/bin/array-microbenchmark" --revision install-smoke --pairs 40 > "$tmp/native-report.json"
"$prefix/bin/array-microbenchmark" --replay "$tmp/native-report.json"
if "$prefix/bin/array-microbenchmark" --replay "$tmp/missing-report.json" \
  > "$tmp/replay-output.txt" 2> "$tmp/replay-error.txt"; then
  echo "benchmark CLI accepted a missing replay file" >&2
  exit 1
fi
if [ -s "$tmp/replay-output.txt" ]; then
  echo "benchmark CLI emitted output for a missing replay file" >&2
  exit 1
fi
printf '{' > "$tmp/malformed-report.json"
if "$prefix/bin/array-microbenchmark" --replay "$tmp/malformed-report.json" \
  > "$tmp/replay-output.txt" 2> "$tmp/replay-error.txt"; then
  echo "benchmark CLI accepted malformed replay JSON" >&2
  exit 1
fi
if [ -s "$tmp/replay-output.txt" ]; then
  echo "benchmark CLI emitted output for malformed replay JSON" >&2
  exit 1
fi
dd if=/dev/zero of="$tmp/oversized-report.json" bs=4194305 count=1 2>/dev/null
if "$prefix/bin/array-microbenchmark" --replay "$tmp/oversized-report.json" \
  > "$tmp/replay-output.txt" 2> "$tmp/replay-error.txt"; then
  echo "benchmark CLI accepted oversized replay input" >&2
  exit 1
fi
if [ -s "$tmp/replay-output.txt" ]; then
  echo "benchmark CLI emitted output for oversized replay input" >&2
  exit 1
fi
for invalid_pairs in -1 -4294967295 -18446744073709551615 0 abc 10001; do
  if "$prefix/bin/array-microbenchmark" --revision bounds-check --pairs "$invalid_pairs" > "$tmp/invalid-pairs.json" 2> "$tmp/invalid-pairs-error.txt"; then
    echo "benchmark CLI accepted invalid pair count: $invalid_pairs" >&2
    exit 1
  fi
  if [ -s "$tmp/invalid-pairs.json" ]; then
    echo "benchmark CLI emitted a report for invalid pair count: $invalid_pairs" >&2
    exit 1
  fi
done
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
  invalid_revision=$(printf '\377')
  if "$prefix/bin/array-microbenchmark" --revision "$invalid_revision" --pairs 40 > "$tmp/invalid-report.json" 2> "$tmp/invalid-report-error.txt"; then
    echo "benchmark CLI accepted invalid UTF-8 metadata" >&2
    exit 1
  fi
  if [ -s "$tmp/invalid-report.json" ]; then
    echo "benchmark CLI emitted an invalid UTF-8 report" >&2
    exit 1
  fi
fi
