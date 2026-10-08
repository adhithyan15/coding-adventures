---
category: Testing & coverage
---

# A prose rewrite across many lessons needs a book compile before push, not just the generators

PR #17169 rewrote drivable prose in ~760 lessons, moving reading steps into
`[YOU READ: …]` cues. Every generator and check passed locally —
`check:books`, `check:narration`, `check:modality`, `check:doc-shards`, the
drivable-cue test — and CI's "Build all human-language books" job still failed:
`scan_latex_log_warnings.py` found one Japanese overfull `\hbox` (27pt) against
a baseline of 0.

The generators prove the `.tex` matches its lessons; they say nothing about how
TeX breaks the lines. JA-C72-gogo's "Read this without stopping: **もういちど、おねがいします**."
had become "*Read it:* the request without stopping —
**もういちど、おねがいします** (Once more, please.)". In this book a `\ja{…}` run is
one unbreakable box, and inside the quote the English before it was too long to
share a line with the bold Japanese and too short to sit on a line of its own
without exceeding the tolerance, so TeX's final pass chose the overfull line.
Rewording the cue to lead with the Japanese ("[YOU READ: **もういちど、おねがいします** (Once more, please.)
without stopping]") let it break normally.

**What to do:** before pushing a prose rewrite that touches many lessons, compile
at least the most-changed books (count changed lessons per track with
`git diff --name-only $(git merge-base origin/main HEAD) HEAD`) and run
`python3 code/scripts/scan_latex_log_warnings.py --book-root <dir holding
<track>/book/> --baseline
code/learning/human-languages/core/latex-warning-baseline.json` on the logs.
Tracks with long unbreakable script runs (Japanese, Chinese) are the most likely
to regress: lengthening the English in front of one is enough.
