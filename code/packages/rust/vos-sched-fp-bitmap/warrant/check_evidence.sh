#!/bin/sh
# Runs the generated evidence suite and rewrites evidence/test-manifest.json from the real run.
#
# EXACTLY three controls are known to fail, and they are named here: the `init` / `field-zero`
# controls of the exhaustive, property and fuzz suites.  `field-zero` is the only fault the
# generator has for a record result and the initial mask is already 0, so no claim can tell the
# faulted `init` from the real one (VOS02 13.4 F4, VOS-D065, VER-BACKLOG WX-SV item 2).
#
# The script demands: at least 61 passing tests, no other failure, and exactly those three.  So
# the day the generator gets a non-inert fault this script fails, and the exception (this file)
# must be deleted and the BUILD line made a plain `cargo test`.
out=$(WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null)
ok=$(printf '%s\n' "$out" | grep -c '^ok ')
bad=$(printf '%s\n' "$out" | grep '^not ok ' | grep -vc '/negctl/[a-z]*/init/field-zero$')
known=$(printf '%s\n' "$out" | grep -c '^not ok .*/negctl/[a-z]*/init/field-zero$')
echo "evidence: $ok ok, $known known inert init controls, $bad other failures"
test "$ok" -ge 61 && test "$bad" -eq 0 && test "$known" -eq 3
