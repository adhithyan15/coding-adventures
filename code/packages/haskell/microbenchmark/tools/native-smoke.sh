#!/bin/sh
set -eu

temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
cabal build exe:array-microbenchmark
runner=$(cabal list-bin exe:array-microbenchmark)
env GIT_REVISION= GITHUB_SHA= "$runner" --pairs 40 >"$temporary/report.json"
"$runner" --replay "$temporary/report.json"
dd if=/dev/zero of="$temporary/oversized.json" bs=1048576 count=4 status=none
printf x >>"$temporary/oversized.json"
if "$runner" --replay "$temporary/oversized.json"; then
  echo 'replay adapter accepted a report above 4 MiB' >&2
  exit 1
fi
