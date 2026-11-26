---
category: Security boundaries
---

# Never read a caller container twice between validation and use (TOCTOU); copy once and cap before the copy

Validating a caller container and then re-reading it for the computation lets a lying or concurrently mutated container pass validation with one value and be used with another (CWE-367). It hit java/kotlin/csharp N-ary gates, dart `xorN`/`muxN` (a one-element list returned 7), and python mux. Copy once (length once, each index once), cap the length before the copy, validate and compute from the copy, and build `inputs[i]` names only on failure. Growing-length lists loop forever unless the length is read once.
