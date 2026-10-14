---
category: CI & GitHub Actions
---

# A stalled TeX mirror is a toolchain setup failure rather than a book build failure

Human Languages Books run 38213744282 passed the hygiene, scanner, catalog,
syllabary, shard and curriculum-report checks on head 8d472b2d. It then failed
Install focused TeX toolchain: the Ubuntu archive mirror stalled across all
three bounded apt attempts, ending with exit 124 after a 270-second attempt.
Book compilation and publication verification never started.

Keep toolchain installation separate from build/runtime conclusions. Do not
attribute this failure to curriculum content or dependency code, and do not
increase thresholds to conceal a stalled mirror. Rerun the failed workflow on
the same source head once GitHub permits it, then require actual compilation,
publication-bundle verification and the protected Books gate to pass.

Attempt 2 on the same 8d472b2d head passed TeX installation, compiled every book,
verified the publication bundle and passed the protected Books gate. No source
or installation-budget change was needed to clear the mirror failure.
