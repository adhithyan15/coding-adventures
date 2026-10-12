---
category: CI & GitHub Actions
---

# The first route a Lighthouse gate audits absorbs the runner's first Chrome launch cost

**What happened:** on PR #17185 the Windows "Forme release" job failed the
FM-B070 web-quality gate with `landing: performance median score 0.94 < 0.95`.
The PR changed nothing the landing page ships; its only landing change was a
dev-only `source-map-js` lockfile bump. Linux and macOS passed the same bytes,
both blog routes passed on Windows, and the same page had passed on Windows at
the previous head. Locally the page scored 1.00, and 0.99 with an 8x CPU
slowdown. Every recorded dip (0.85 Windows and 0.92 macOS on 2026-10-05, then
0.94) hit the landing page, which is always the first route audited.

**Why:** the first Chrome launch on a fresh hosted runner pays one-time OS costs:
scanning the new binary, building the font cache, and warming the
disk cache. A fresh process per sample does not avoid them, because they are
per machine, not per process. They land on whichever route goes first, and the
median of three cannot absorb them when two of the three samples are affected.

**Fix:** `release-web-quality.ts` runs one unrecorded warm-up audit
(`discardWarmupAudit`) before the first recorded sample. The budget, the sample
count, the fail-closed aggregation, and the summary schema are all unchanged.

**Do differently:** when a browser gate fails only on the first route it
audits, suspect cold-start cost before the page. Compare that route's score
across hosts and against the later routes in the same job before you touch the
page or the budget.
