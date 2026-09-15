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
