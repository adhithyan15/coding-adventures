#!/bin/sh
# Runs the generated evidence suite and rewrites evidence/test-manifest.json from the real run.
#
# EXACTLY three controls are known to fail, and they are named here, literally: the `init` /
# `field-flip` controls of the exhaustive, property and fuzz suites.  The only claim whose subject
# is `init` is `rep_init` (`init() == R(init())`), which asserts that the state is in the image of
# `rep`; every `Impl(mask, ptr)` with `ptr < 8` is in that image, and `field-flip` of `mask` (the
# first numeric field) stays in it, so no claim of the control's membership can tell the faulted
# `init` from the real one (VOS-D087).  `safety_init` does catch it, but its subject is `alpha_w`,
# so the generator does not put it in the `init` control (VOS-D098 proposes the rule that would).
# `rep_init` is attacked anyway, through `alpha_w` (BUILD, WARRANT_EVIDENCE_ATTACK=alpha_w).
#
# The script demands: cargo exits with status exactly 1 (the suite ran and reported failures; 0
# would mean the exception is stale, 101 or anything else a crash), exactly 78 passing tests,
# exactly the three failures below and no other.  The day VOS-D098 lands the known count is 0 and
# this script fails: delete it and make the BUILD line a plain `cargo test`.
set -eu

known_ids='warrant/vos_sched_rr/negctl/exhaustive/init/field-flip
warrant/vos_sched_rr/negctl/property/init/field-flip
warrant/vos_sched_rr/negctl/fuzz/init/field-flip'

status=0
out=$(WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null) || status=$?

ok=$(printf '%s\n' "$out" | grep -c '^ok ' || true)
failed=$(printf '%s\n' "$out" | sed -n 's/^not ok [0-9][0-9]* - //p' | sort)
expected=$(printf '%s\n' "$known_ids" | sort)

echo "evidence: cargo status $status, $ok ok"
if [ "$status" -ne 1 ] || [ "$ok" -ne 78 ] || [ "$failed" != "$expected" ]; then
    echo "evidence: expected status 1, 78 ok and exactly these failures:" >&2
    printf '%s\n' "$expected" >&2
    echo "evidence: got these failures:" >&2
    printf '%s\n' "$failed" >&2
    exit 1
fi
