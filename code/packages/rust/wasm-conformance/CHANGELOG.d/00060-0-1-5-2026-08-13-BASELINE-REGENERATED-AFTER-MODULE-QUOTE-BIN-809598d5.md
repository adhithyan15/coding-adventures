## 0.1.5 — 2026-08-13 — baseline regenerated after (module quote/binary ...) directives were fixed (WASM12)

No code changes in this crate — `wasm-wast-parser` 0.1.3 fixed a real bug
(`(module quote/binary ...)` **directives** silently built an empty
module instead of the module the source actually described) surfaced
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 12215/12238 (99.8%) → 12219/12238 (99.8%,
+4); `assert_malformed` 145/147 → 33/35 graded (46 → 158
`NotYetSupported`) — a real, understood reclassification, not a
regression: many quote-module `assert_malformed` cases were previously
`Pass` only because the quote text failed to parse for the WRONG reason
(the missing-wrapper bug, not the case's actual intended malformation);
now that it parses correctly, this repo's still-missing instruction-level
type-checker genuinely can't tell those specific cases apart from a valid
module, so they honestly report `NotYetSupported` instead. Verified via a
full per-file diff against the previous baseline: every changed file's
`fail` count went down or stayed the same, never up. See
`wasm-wast-parser`'s own `0.1.3` changelog entry for the full bug writeup.

