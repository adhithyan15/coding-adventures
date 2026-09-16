# A filter must be proven against a known-positive input, never only a quiet one

Two filters lied in the same session, both by silently matching the wrong thing while looking healthy.

**`gh pr checks` is TAB-separated and check names contain spaces.** So `awk '$2=="fail"'` tests the
*second word of the check name* — `message`, `channel`, `17` — and can never equal `fail`. A PR
monitor built on it reported healthy while a required check was failing. Use `awk -F'\t'`.

**`grep -c $'\r'` does not count carriage returns** where `$'...'` is not expanded: grep receives
the two characters `\r`, and in a basic regex `\r` is just a literal **r**. Every line containing
the letter *r* matches — which yields a count *close to the file's line count* and therefore looks
exactly like a plausible CRLF count. It reported CRLF in all 35 files when only 3 had it, and it
did so with numbers convincing enough to be quoted in a report. Count bytes with a real tool
(`node`, `xxd`, `file`) instead.

The general rule: **a filter that has only ever been observed silent has not been observed
working.** Before trusting one, run it against an input you know is positive and confirm it fires.
Both bugs above would have died instantly under that test, and both survived because the only
evidence collected was "it didn't complain."

### Correction: a known-positive is NOT sufficient, and the remedy above was itself unsound

The paragraph above prescribes one test — run the filter against a known-positive. **That test
passes on a filter that is still broken**, and the same `grep -c $'
'` proves it.

Re-run in Git Bash here, the command fails a *third* way. Not "matches the letter `r`": the
pattern degrades to **empty**, and an empty pattern matches **every line**.

```
printf 'a
b
' > pos.txt ; grep -c $'
' pos.txt   # 2   <- known-positive: looks correct
printf 'a
b
'     > neg.txt ; grep -c $'
' neg.txt   # 2   <- known-NEGATIVE: same answer
```

`neg.txt` contains no carriage return *and no letter `r`*. A known-positive test alone returns 2
and reads as a pass. Only the **known-negative** exposes it. So the rule needs both arms:

> **Prove a filter against a known-positive AND a known-negative, and require the answers to
> DIFFER.** A filter that cannot distinguish the two is measuring something else, whatever it
> prints on the positive case.

This also breaks the earlier entry's reasoning that a bogus count is recognisable because it lands
"*close to* the file's line count". Under the empty-pattern failure it lands **exactly on** the
line count, which is indistinguishable from a file that is genuinely all-CRLF.

**Use a byte-level check instead**, which has no pattern to degrade:

```
python -c "b=open('f','rb').read(); print(len(b), b.count(b'
'), b.count(b'
'))"
```

The wider point, and the reason this is a correction rather than a new entry: **#13190 documented
one failure of this command and then left behind a remediation carrying the same defect.** A wrong
lesson recorded as a lesson is worse than no lesson, because the next reader trusts it instead of
re-deriving it. When an entry prescribes a fix, the fix needs the same evidence the diagnosis got.

**A control that runs AFTER the target measurement cannot stop you reading the target.** The rule
above says prove a filter against a known-positive *and* a known-negative and require the answers to
differ. It does not say *when*, and ordering turns out to matter as much as existence. In one session
the same broken CR-counting command ran three ways, and each time the control came last:

    grep -c $'\r' file        # read as "all CRLF"
    grep -c "$(printf '\r')"  # read as "clean"
    od -An -c | grep '^\r$'   # read as "clean"

**The three fail for three different reasons, and saying otherwise was itself an unchecked claim.** A
first draft of this section called all three "the empty-pattern failure this entry already
documents". Measured on a probe holding two CR bytes: `grep -c ""` returns the line count on a CRLF
file *and* on an LF file — an empty pattern matches **every** line and distinguishes nothing. Two of
the three reported **no** lines matched, which is the opposite signature. The third is an
anchoring mismatch: `od -An -c` prints many escaped bytes per output line, so a whole-line anchor
around a single literal `r` can never match whatever the input contains. What the three share is not
a mechanism but an **ordering** — in each, the control came after the number.

**Both readings reproduce here at once, and they are different PATTERNS rather than different
machines.** Measured in one shell, on probes Python confirms hold 2 CR bytes and 0:

    grep -c $'\r'            -> 0 on the CRLF probe, 0 on the LF probe
    grep -c $'<real newline>' -> 2 on the CRLF probe, 2 on the LF probe
    grep -c ''                -> 2 and 2, identical to the line above

The first is correctly escaped and returns 0 because this grep treats CRLF as the **line
terminator**, so a line-ending CR is gone before matching — though a CR *mid*-line still matches
(`printf 'a\rb\n' | grep -c $'\r'` returns 1), so it is not stripping CR generally. The second is the
form the block above actually contains, where `$'…'` encloses a **real newline byte**: grep splits
that on the newline into two empty patterns, and an empty pattern matches every line. That is where
the "matches every line" signature comes from, and the paragraph recording it is correct for the
command it describes.

*A draft of this section said that signature "does not reproduce in this worktree" and called it
environment-specific. That was wrong in the opposite direction to the error above — it cast doubt on
true text. Re-running the correctly escaped pattern and getting 0 says nothing about the mangled one.
Two wrong causal stories about the same three commands, one asserting a shared mechanism and one
inventing an environmental difference, and both were fixed only because a review asked for the
measurement rather than the explanation.*

The numbers were printed, read and half-believed *before* any probe file existed. A later attempt
finally put a known-CRLF probe first, and it failed instantly — the same probe would have failed
instantly on the first run.

**Why a trailing control is weaker than no control at all.** A control run afterwards is a control
run *only if the answer already looks wrong*. When a broken instrument happens to print a plausible
number — and the empty-pattern form above lands exactly on the file's line count, indistinguishable
from a genuinely all-CRLF file — nothing prompts you to reach for it. It becomes a tiebreaker
consulted when already suspicious, rather than a gate deciding whether the measurement is admissible
at all. It certifies nothing.

**The operational form.** Print the control arms first, and make the script refuse to report when
they do not differ:

    if not (positive_fires and not negative_fires):
        print("INSTRUMENT BROKEN -- no count below is interpretable.")
        raise SystemExit(1)

A sweep written that way aborted on its own first run later in the same session, naming a real bug in
its own needle: the guard forbade a hash *between* the number and the noun, which does nothing when
the hash sits before the digits. That refusal is why the wrong figure did not ship.

**A control probe must reproduce the defect's SHAPE, not merely its subject.** A later probe in that
session was meant to test a phrase wrapping across a line break, and instead put the whole phrase on
line two of a two-line string. The line-anchored arm found it, the arms stopped differing, and the
guard aborted — on a probe that never wrapped anything. That the probe must be able to fail the way
the real defect fails is [[i-built-the-exact-vacuous-check-i-had-spent-the-day-criticising]]'s point;
what this adds is that the failure can be a *layout* mismatch between probe and target, invisible in
a probe that otherwise mentions all the right words.

Related: [[a-probe-that-reports-nothing-where-you-have-already-seen-the-thing]] — there the
instrument contradicted something already seen by eye, which is the one case where the diagnosis is
closed before it begins.
