---
category: CI & GitHub Actions
---

# nm piped into grep -q under pipefail fails whenever grep exits before nm finishes writing

The macOS lane's iOS checks confirm the Rust engine is linked in with
`nm -gU "$binary" | grep -q ' _mosaic_app_create$'`. GitHub's `bash` shell
runs with `-eo pipefail`. `grep -q` exits as soon as it finds the symbol; if
`nm` is still writing, it gets SIGPIPE, prints `LLVM ERROR: IO failure on
output stream: Broken pipe` and exits non-zero, and pipefail fails the step
right after `** BUILD SUCCEEDED **`.

Whether it fires depends only on where the symbol falls in `nm`'s output and
how much follows it. It passed for weeks, then failed on #16391, a PR that
changed `MosaicRuntimeHost.swift` and so changed what the binary exports.

**Fix:** write `nm`'s output to a file, then `grep -q` the file. The Swift CI
acceptance test now refuses a line that pipes `nm` into `grep -q` in those
blocks.

**Do instead:** never pipe a producer that keeps writing into `grep -q` under
`pipefail` (or into `head`). Either drop `-q` (`grep ... >/dev/null` reads to
EOF), or write the producer's output to a file first.
