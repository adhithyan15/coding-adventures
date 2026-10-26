#!/bin/sh
# Reuse the repository's single ISO compiler scheduler. This front owns cwd.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
d="$SELF"
while [ "$d" != / ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do d=$(dirname "$d"); done
HARNESS="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then echo 'iso-harness not found' >&2; exit 1; fi
ISO_INCLUDE="include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = true ] && [ "$(uname)" = Linux ]; then ISO_REQUIRE='gcc clang'; export ISO_REQUIRE; fi
. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run cpp bounded-json-tests tests/json_test.cpp tests/consumer.cpp
