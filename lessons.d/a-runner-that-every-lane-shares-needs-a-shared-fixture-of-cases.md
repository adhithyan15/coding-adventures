---
category: Testing & coverage
---

# A runner that every lane shares needs a shared fixture of cases, not a per-lane review of the same bugs

The Python and TypeScript Warrant lanes each re-discovered, in review, the same
runner bugs: duplicate JSON keys accepted, integers above 2^53 rounded, lone
surrogate escapes accepted, a `__proto__` key dropped, Windows cross-drive paths
accepted, a hard-coded toolchain name in a digest, and output that differed
between runtimes. Fixing each in one lane left the other (and the 16 lanes to
come) free to repeat it.

Pin the behaviour once as data: `code/specs/fixtures/warrant-corpus-conformance-v1/`
(VER04i 9.1), with an independent checker that derives every expectation with
its own reference code. Each lane runs every case through its own reader,
renderer, containment helper and digest, and compares per-category counts with
`index.json` so a new case cannot be skipped. Running it against the two existing
lanes found five more divergences nobody had reported (UTF-16 key order, numeric
key order, raw DEL, no nesting limit in Python, floats accepted in Python).
