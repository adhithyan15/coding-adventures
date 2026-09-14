# Count what you inspected, or the loop that inspects nothing reports success

Also from the same review, in — of all places — the step added to *prove* the hardening
was in effect:

```bash
offenders=0
while IFS= read -r log; do ... done < <(find … -name book.log | sort)
test "$offenders" -eq 0
echo "Shell escape disabled in every book.log"     # printed after reading zero files
```

`find` inside a process substitution has an exit status `set -o pipefail` cannot see, and
a loop body that never executes leaves the counter at its initial value. Zero matches is
indistinguishable from a clean corpus. Any later change that adds `-outdir=` to the
latexmk line disarms the check silently, and the job stays green while asserting the
opposite.

**Every verification loop needs a `checked` counter and a floor**, and the floor should
come from the producer (`wc -l < manifest`) rather than a hardcoded number that goes stale
when a track is added. This is the same failure as `compiled 0, skipped 1, failed 0` two
lessons up — which is the point: the shape recurs, including inside the fix for itself.
