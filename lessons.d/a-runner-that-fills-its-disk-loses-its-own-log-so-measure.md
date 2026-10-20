---
category: CI & GitHub Actions
---

# A runner that fills its disk loses its own log, so measure disk usage while there is still room to print it

Shard 5 of main's full build ran out of disk on every run for days (#17652).
Its job log either ended with the runner failing to write
`GITHUB_STEP_SUMMARY` and its own `_diag` page, or was missing entirely: four
of the five most recent shard-5 logs return HTTP 404 from the jobs API. The
Build Report, which is printed at the very end, never appeared, so there was
no record of which package was building or which directory had grown. A
"linker SIGBUS" in another shard of the same run (#17668) was the same event
seen from the inside: `ld` writes its output through mmap, and a full disk
turns that into SIGBUS rather than ENOSPC.

The fix was to print the evidence early and on a schedule:
`code/scripts/ci/disk-watch.sh` reports `df` plus the largest directories
before the build, prints one `df` line every five minutes, and reports in full
the first time usage crosses 70, 80, 90 and 95%. The reclaim step also lists
what the runner image carries, largest first.

What to do differently:
- Treat "the log is gone" as a disk-full signature, not missing data.
- Read a SIGBUS from a linker or any mmap writer as a probable full disk
  before suspecting the toolchain.
- Measure first. Choose what to reclaim from a breakdown taken before the
  disk filled, never from a guess about what is probably big.
