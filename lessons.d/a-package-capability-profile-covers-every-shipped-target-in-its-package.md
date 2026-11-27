---
category: Security boundaries
---

# A package capability profile covers every shipped target in its package

A Rust microbenchmark package had a library and a CLI, but its legacy empty manifest
described only the library's injected-clock collection API. The CLI reads a host clock
and writes its report to stdout, so marking the package capability-free would be false.

**Do instead:** treat the package-root manifest as covering every shipped target. If a
target needs OS capabilities, declare them for the package and obtain the required
hardware-key-signed approval. When that approval is unavailable, leave the inaccurate
manifest unresolved and report the exact blocker; do not add unsigned grants or claim
that the executable is outside the package profile.
