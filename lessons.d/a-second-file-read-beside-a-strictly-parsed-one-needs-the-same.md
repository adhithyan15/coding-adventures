---
category: Security boundaries
---

# A second file read beside a strictly parsed one needs the same strict-parse gate, and every summed count needs checked arithmetic

The WD-004b1 lane loader parsed the manifest through WD-004a's strict integer-token
gate but read the labels sidecar with the bare f64 JSON parser, so `65537.0`,
`6.5537e4` and `065537` all loaded even though VER04i said "strictly parsed". The
same sidecar summed per-suite case counts with `+=`, so about 2,100 suites at 2^53
panicked in debug and wrapped in release, before any join could reject them.

When a loader adds a second input beside one that already has a strict gate, route
it through the same gate and say so in a test that feeds the three noncanonical
spellings. Any sum over untrusted counts uses `checked_add` and returns a named
refusal; a per-field bound does not bound the total. Also pin spec tables that code
mirrors (here the readiness-check list and its order) with a test that reads the
spec, so a comment claiming they match is not the only evidence.
