#!/bin/sh
# Runs the generated evidence suite and rewrites evidence/test-manifest.json from the real run.
#
# EXACTLY three controls are known to fail, and they are named here, literally: the `init` /
# `field-zero` controls of the exhaustive, property and fuzz suites.  `field-zero` is the only
# fault the generator has for a record result and the initial mask is already 0, so no claim can
# tell the faulted `init` from the real one (VOS02 13.4 F4, VOS-D065, VER-BACKLOG WX-SV2).
#
# The script demands: cargo exits with status exactly 1 (the suite ran and reported failures; 0
# would mean the exception is stale, 101 or anything else a crash), exactly 61 passing tests,
# exactly the three failures below and no other.  So the day the generator gets a non-inert fault
# the known count is 0 and this script fails: delete the exception and make the BUILD line a
# plain `cargo test`.
set -eu

known_ids='warrant/vos_sched_fp_bitmap/negctl/exhaustive/init/field-zero
warrant/vos_sched_fp_bitmap/negctl/property/init/field-zero
warrant/vos_sched_fp_bitmap/negctl/fuzz/init/field-zero'

status=0
out=$(WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null) || status=$?

ok=$(printf '%s\n' "$out" | grep -c '^ok ' || true)
failed=$(printf '%s\n' "$out" | sed -n 's/^not ok [0-9][0-9]* - //p' | sort)
expected=$(printf '%s\n' "$known_ids" | sort)

echo "evidence: cargo status $status, $ok ok"
if [ "$status" -ne 1 ] || [ "$ok" -ne 61 ] || [ "$failed" != "$expected" ]; then
    echo "evidence: expected status 1, 61 ok and exactly these failures:" >&2
    printf '%s\n' "$expected" >&2
    echo "evidence: got these failures:" >&2
    printf '%s\n' "$failed" >&2
    exit 1
fi
