---
category: Repo policy / workflow reminders
---

# A guard that refuses the fix it just told you to make sends you to the laundering flag

**The shape of the bug.** `check:root-slug-splits` reported seventeen baseline
entries that had *lost* a spelling and said:

> run generate:root-slug-splits to record it

`generate:root-slug-splits` then refused:

> refusing to grow the baseline with 17 new split(s) ... If the split is
> genuinely new and accepted, re-run with `--allow-new` and say why.

Both messages were written deliberately and both are individually correct. The
check knew a shrink from a growth — it had a `shrank` branch and a tailored
sentence for it. The writer did not: it compared a fingerprint, and any entry
whose slug list differed in any way landed in `added`.

**Why that is worse than a plain false positive.** The escape hatch,
`--allow-new`, exists precisely to make accepting a genuine regression visible
in a diff. Routing a *correct* normalisation through it does two things at once:
it puts a laundering flag in the commit of someone who did the right thing, and
it teaches everyone that the flag is routine. The next real regression then
passes without comment.

**The fix.** Separate the directions rather than widening the gate:

- `shrunk` — live slugs are a **strict subset** of the baseline's. `--write`
  accepts it with no flag; `--check` still fails until it is recorded, so the
  baseline cannot go stale.
- `added` — a new key, or any entry carrying a spelling the baseline never had.
  Still needs `--allow-new`.

**Check membership, not length.** `[a,b] -> [a,c]` is the same length and
`[a,b,c] -> [a,x]` is shorter, and both introduce a spelling nobody accepted. A
`newList.length < oldList.length` test would wave both through, which is the
same laundering by the opposite route. Each has its own test.

**The general rule.** When a `--check` and a `--write` disagree about the same
data, they are two implementations of one policy and one of them is wrong. If
the check can name a situation the writer cannot, the writer is the one missing
a case. Look for the disagreement whenever a tool's own instructions do not
survive being followed literally.
