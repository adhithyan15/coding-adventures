#!/bin/sh
set -eu

package_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$package_root"
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi
cabal test

tix_file=$(find dist-newstyle -type f -path '*/hpc/vanilla/tix/spec/spec.tix' -print | head -n 1)
mix_file=$(find dist-newstyle -type f -path '*/hpc/vanilla/mix/*/Microbenchmark.mix' -print | head -n 1)
if [ -z "$tix_file" ] || [ -z "$mix_file" ]; then
  echo "Cabal HPC output is missing for microbenchmark" >&2
  exit 1
fi
hpc_dir=$(dirname "$(dirname "$mix_file")")
report=$(mktemp "${TMPDIR:-/tmp}/ca-microbenchmark-hpc.XXXXXX")
trap 'rm -f "$report"' EXIT HUP INT TERM
hpc report --per-module --hpcdir="$hpc_dir" --include=Microbenchmark --include=ArrayWorkload "$tix_file" >"$report"
awk -v expected=Microbenchmark,ArrayWorkload -f tools/check_coverage.awk "$report"
