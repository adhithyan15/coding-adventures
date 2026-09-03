## HL-C310 — de-pin the corpus exam-point total in `plan-cli.test.ts`

`tests/plan-cli.test.ts` pinned an absolute corpus figure — `851 uncovered
point(s) across 10 written` — inside a test whose actual subject is that a
duplicated inventory is counted once. The number was only standing in for "the
duplicate did not double the count".

In a single day it read 529 → 686 → 793 → 792 → 775 → 774 → 786 → 839 → 851. It
rises when an inventory lands (Telugu +157, Tamil +107) and falls when a tranche
covers points (Telugu −44, Tamil −19) or a retirement closes one. Every parallel
author had to edit the same line; four PRs sat DIRTY on it at once, and only one
human-languages PR merged in six hours.

Worse than the churn: two branches that both **lower** it merge quietly, because
git sees them agreeing — and the agreed value is wrong. Composing it by
arithmetic was wrong every time it was tried, including once where the arithmetic
happened to agree, which is why agreement is not validation.

A ratchet is not available. The number legitimately **rises** when an inventory
lands, because an inventory enumerates points that were previously unmeasurable,
so a ceiling would fail on exactly the work we most want.

**Resolved** by asserting what the test owns: run the CLI on a clean corpus and
on the duplicated one and require both figures identical, then cross-check the
written-inventory count against the distinct (language, level) pairs read from
the `core/` directory listing, which the plan engine does not produce. Not
weakened — duplication changing either figure still fails, and a dropped or
double-counted inventory still fails.
