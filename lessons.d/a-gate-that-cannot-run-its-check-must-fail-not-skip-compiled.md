# A gate that cannot run its check must fail, not skip — "compiled 0, skipped 1, failed 0" and exit 0

`check-book-compile.sh` skips any track whose SVG figures it cannot convert, because a
missing figure PDF is a compile failure that says nothing about the LaTeX under test.
That is right on a laptop. Wired into CI unchanged it is a gate that reports success
having verified nothing — on a runner without `rsvg-convert`, every illustrated track
skips, the summary reads `compiled 0, skipped 1, failed 0`, and the exit status is 0.

This is the same absent-vs-could-not-determine conflation as #12731 and #12734, one
layer up: there it was a bare `catch → "missing"`, here it is `skip → pass`.

**The shape of the fix, reusable:** keep the lenient behaviour, add `--strict` that
turns every "could not verify" into a failure **naming the missing dependency**, and
additionally fail when the run verified *zero* items — because a selection typo or a
renamed directory produces a clean-looking green with an empty work list. Then make the
lenient path announce its own weakness in its output (`"CI runs this script with
--strict, where each of those is a failure"`), so a local pass is never quoted as
evidence the gate would pass.

**And prove the red before trusting the green.** A gate that has only ever been observed
passing has not been observed working. Both directions were run here: `--strict` on a
track with a figure and no converter fails with the dependency named; the same command
with a converter on `PATH` compiles the book and reports `compiled 1, skipped 0,
failed 0`.
