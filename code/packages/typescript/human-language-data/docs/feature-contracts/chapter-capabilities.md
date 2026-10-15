# Chapter capabilities (HL05)

A chapter used to be nothing but an integer on each lesson, so nothing could check
that finishing one left the reader able to do anything. `<track>/chapters.json` is
that missing promise — a first-person `canDo` plus a payoff the reader can deploy
immediately — and `core/chapter-policy.json` holds the thresholds that judge it.

```ts
import { loadTrackChapters, loadChapterPolicy } from "@coding-adventures/human-language-data";

const ledgers = loadTrackChapters();   // tracks WITHOUT a ledger are skipped, not defaulted
const policy = loadChapterPolicy();    // payoff share + HL08 gentle-ramp budgets
```

The skip is deliberate. "Not yet authored" and "authored and empty" are different
kinds of debt, and defaulting the first into the second would erase exactly what the
gap report exists to measure. Ledgers are **authored intent** — unlike
`curriculum.json`'s `omits`/`relocates`, which are recomputed caches, no validator
may rewrite them.
