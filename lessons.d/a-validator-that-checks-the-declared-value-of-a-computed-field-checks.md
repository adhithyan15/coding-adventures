---
category: Testing & coverage
---

# A validator that checks the declared value of a computed field checks nothing

Every human-language lesson declares `duration.max_seconds`, and the corpus
enforces a 300-second ceiling. I wrote a pre-flight validator that runs over
draft lessons before they enter the repo, and one of its checks was the
duration ceiling. It read the declared number and compared it to 300.

Twenty-seven drafts passed it. `npm run validate` then failed:

```
ES-C445-repetir: effective duration is 304s (declared 275s, computed 304s)
```

**The declared number was never the thing under test.** The real duration is
computed from the lesson's own content — its word counts and its pause markers —
and `max_seconds` is an author's estimate sitting beside it. My check compared
the estimate to the ceiling and let a 304-second lesson through while reporting
the author's 275.

This is the same trap the corpus already names elsewhere. `modality` is derived
rather than authored precisely because 1,134 authored copies of a computed fact
are 1,134 places for it to go stale, and a lesson's CEFR level is derived from
its spine node so that "a track cannot claim A1 by editing frontmatter". A
declared duration is one more authored copy of a computed fact, and validating
the copy validates the author's intention rather than the artefact.

**The general shape.** When a field is *derived*, its written-down value is a
claim, not the measurement. A check that reads the claim tests whether the
author was honest and self-consistent — which is worth something, but it is not
the check you thought you had, and it passes exactly when the author is
confidently wrong.

**What to do.** Find out whether the field you are validating is authored or
computed before writing the check. If it is computed, either call the same
computation the real gate calls, or do not claim to be checking it — a
pre-flight that silently covers less than the gate is worse than no pre-flight,
because it buys confidence it has not earned. Where re-implementing the
computation is impractical, say so in the validator's own output so the gap is
visible at the moment of use.
