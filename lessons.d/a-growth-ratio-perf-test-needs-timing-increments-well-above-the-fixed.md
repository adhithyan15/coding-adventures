---
category: Testing & coverage
---

# A growth-ratio perf test needs timing increments well above the fixed per-run cost and timer jitter: size n accordingly and take the best of several passes

**What went wrong.** The `scales_linearly` performance checks of
`vos-shape-check` (PR #19529) compare the run time at size n with the time at
size 2n and assert a ratio. Each run pays a fixed cost (about 0.8 to 0.9
seconds to build the grammar lexer) that does not grow with n. With small
inputs the fixed cost dominates, so the ratio is close to 1 whatever the
algorithm does: the test is flaky on a noisy runner and blind to a small
quadratic term. Only a quadratic that cost roughly twice the linear work or
more could fail it.

**Fix and rule.** A growth-ratio test measures a *difference*, so the
difference must be larger than the noise:

* choose n so the n-dependent part of the time is well above both the fixed
  per-run cost and timer jitter (a rule of thumb: at least 10 times the fixed
  cost), not merely large enough to be "slow";
* take the best (minimum) of several passes at each size, because noise only
  ever adds time;
* better still, time only the part that scales (reuse one expensive setup
  across runs) or count operations instead of seconds.

If the fixed cost cannot be removed, say in the test what slope it can and
cannot detect. Logged for `vos-shape-check` as VOS-D075.
