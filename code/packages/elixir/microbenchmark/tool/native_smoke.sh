#!/bin/sh
set -eu
report="$(mktemp)"
trap 'rm -f "$report" "$report.oversized"' EXIT
env GIT_REVISION= ./coding_adventures_microbenchmark >"$report"
mix run tool/native_report_replay.exs "$report"
oversized="${report}.oversized"
dd if=/dev/zero of="$oversized" bs=1048576 count=4 status=none
printf x >>"$oversized"
if mix run tool/native_report_replay.exs "$oversized"; then
  echo 'replay adapter accepted a report above 4 MiB' >&2
  exit 1
fi
