---
category: CI & GitHub Actions
---

# Full commit history does not require every historical blob

PR #18844's native Windows repair passed Linux, macOS and Windows CI, but the
metadata job repeatedly exhausted its unchanged 30-minute budget. The job logs
showed 21-28 minutes fetching Git history; materializing the working tree took
seconds. Retrying the same head preserved the earlier successes but repeatedly
left too little time for the complete contract suite.

Keep `fetch-depth: 0` when provenance and dependency baselines need historical
commits and trees. Add `filter: blob:none` to defer historical file contents,
while checking out every current source file. Full history is a graph of
content-addressed commits and trees; it does not require eagerly downloading
every old file payload.

Validate the fresh filtered checkout, the barcode tree/revision controls, and
lazy loading of an initially absent baseline blob. An unavailable required blob
must fail the baseline check. Preserve the job timeout, read-only permission,
complete source checkout and all contract checks, then require real final-head
CI evidence before calling the repair complete.
