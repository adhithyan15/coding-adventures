---
category: Testing & coverage
---

# The Malayalam exam-coverage total is pinned in FOUR places, not three

A previous unit learned that Malayalam carries *three* stale-number sites and
wrote that down. Chapter 75 found a fourth, the expensive way: after `validate`,
all twelve gates and the five suite-only tests were green.

The four:

1. `tests/corpus/malayalam.test.ts` — `coverage.covered` / `coverage.unmapped`
2. `tests/corpus/malayalam.test.ts` — the `formatExamCoverage` string
3. `tests/exam-inventory.test.ts` — the joining count, **written into the
   `it(...)` test NAME** as well as into its assertion
4. `tests/exam-inventory.test.ts` — **its own** `coverage.covered` /
   `coverage.unmapped` pair, in the same `it` block as the format string

Number 4 is the one that hides. The `it` block that owns it runs from roughly
line 1950 to 2078 and contains *five* separately-updatable numbers — the test
name, `covered`, `unmapped`, the joining tuple, and the format string. Updating
the format string in that file feels like updating "the exam-inventory site",
and it is not: the `covered` assertion sits ninety lines earlier under a
different comment block, and searching for the old total by its formatted
spelling (`171/243`) does not find it, because it is written `toBe(171)`.

## What to do instead

Do not search for the formatted string. Before touching a track's coverage,
run:

```
grep -rn "toBe(<old total>)\|toBe(<old unmapped>)\|<old total>/<enumerated>" tests/
```

and also grep the *column* count for any category the change touches, including
inside `it(...)` titles. Then re-run **both** pinned files together and read the
failure rather than trusting the count of sites recorded last time.

The general rule, which is the reusable part: **a per-track "how many sites"
number is itself a stale number.** It is worth recording as a floor — never as a
total. Re-derive it by grep on every tranche.
