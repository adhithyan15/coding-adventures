#!/bin/sh
set -eu
report="$(mktemp)"
trap 'rm -f "$report"' EXIT
./coding_adventures_microbenchmark >"$report"
mix run tool/native_report_replay.exs "$report"
