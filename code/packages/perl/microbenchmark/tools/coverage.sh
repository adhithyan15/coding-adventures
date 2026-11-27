#!/bin/sh
set -eu

package_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$package_root"

cover_bin=$(command -v cover || true)
if [ -z "$cover_bin" ]; then
  echo "Devel::Cover 1.52 is required for package coverage" >&2
  exit 1
fi
cover_version=$(perl -e 'require Devel::Cover; print $Devel::Cover::VERSION')
if [ "$cover_version" != 1.52 ]; then
  echo "expected Devel::Cover 1.52, found $cover_version" >&2
  exit 1
fi

coverage_dir=$(mktemp -d "${TMPDIR:-/tmp}/ca-perl-microbenchmark-coverage.XXXXXX")
trap 'rm -rf "$coverage_dir"' EXIT HUP INT TERM
export DEVEL_COVER_OPTIONS="-db,$coverage_dir/db,-coverage,statement"
export PERL5OPT=-MDevel::Cover
perl -Ilib -I../rank-statistics/lib -I../grammar-tools/lib -I../json-lexer/lib -I../json-parser/lib t/microbenchmark.t >"$coverage_dir/test.log" 2>&1
unset PERL5OPT DEVEL_COVER_OPTIONS
"$cover_bin" -report text -coverage statement -select_re 'CodingAdventures/Microbenchmark\.pm$' "$coverage_dir/db" >"$coverage_dir/report.txt"
perl tools/check_coverage.pl "$coverage_dir/report.txt" lib/CodingAdventures/Microbenchmark.pm
