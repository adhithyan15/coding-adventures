---
category: CI & GitHub Actions
---

# A per-attempt retry budget must fit inside the step timeout that wraps it

**What went wrong.** `code/scripts/ci/apt-install.sh` bounds apt in three
layers: apt's own timeouts, a wall-clock cap on each attempt
(`APT_INSTALL_TIMEOUT`, default 600 s), and three attempts with backoff. The
workflow step that calls it adds a fourth: `timeout-minutes: 20`. For the TeX
Live install in `human-languages-books.yml`, three 600 s attempts need 30
minutes, so the outer cap fired first. On a trickling mirror the first attempt
burned its 10 minutes, the second started, and the step was killed before the
retry layer could reach a healthy mirror. "Build all human-language books"
failed this way twice in one day (#17184, #17186), before any book compiled.

**Fix.** The step sets `APT_UPDATE_TIMEOUT=60` and `APT_INSTALL_TIMEOUT=270`.
In the worst case, with every attempt stalling, the step now takes
3 × (60 + 15) + 30 = 255 s for `update` plus 3 × (270 + 15) + 30 = 885 s for
`install`, 1140 s in total, which is under 1200 s. Normal runs take 2-15 s for
`update` and about 120 s for `install`. A first fix of `300` alone was caught
in review: it fit only if `update` behaved, so the worst case still overran.

**Next time.** When an inner retry loop sits inside an outer timeout, check
the arithmetic for EVERY retried command in the step: the sum of attempts ×
(per-attempt cap + kill grace) + backoff must be less than the outer budget. Otherwise the retry layer exists on paper only.
