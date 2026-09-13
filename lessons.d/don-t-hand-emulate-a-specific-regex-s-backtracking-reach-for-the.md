# Don't hand-emulate a specific regex's backtracking — reach for the engine

Context: Phase C2 of the Engram zero-dep program aimed to hand-write scanners
replacing the two HTML-tag regexes in `engram-core` search-text rendering.

**Lessons:**
- **A hand scanner can replace a *simple* regex, but not a backtracking one.**
  The tag-strip `(?is)<[^>]+>` has no alternation/backreference/overlap, so a
  two-pointer scanner reproduces it byte-for-byte (verified vs live `regex` on
  300k random strings). But the media pattern
  `<(?:img|…)[^>]*(?:src|data)\s*=\s*(?:"([^"]+)"|'([^']+)'|([^ >]+))[^>]*>`
  hides layered backtracking that a fuzz cross-check exposed one corner at a
  time: (1) a **quoted value `[^"]+` crosses `>`**, so the tag end isn't the
  first `>`; (2) when the quoted alt yields no trailing `>`, the regex
  **backtracks through the alternation** to the bare `[^ >]+`; (3) `\s*` and the
  value class `[^ >]` **overlap** (tab ∈ both), so greedy `\s*` gives a tab back
  to the value. Each fix surfaced the next corner — a sign you're reimplementing
  a regex engine badly.
- **When you catch yourself hand-emulating quantifier/alternation backtracking,
  stop and build/adopt the general engine.** The right move was to descope: ship
  only the trivially-exact tag-strip scanner now, and move the media pattern (plus
  glob/whole-word/`re:`) to the planned zero-dep regex *engine*, where correct
  semantics give byte-exactness for free instead of per-pattern emulation.
- **The randomized cross-check earned its keep**: every divergence was a
  malformed-HTML input a human would never think to unit-test. Fuzz vs the real
  library before trusting a reimplementation — and read the *first* divergence
  carefully; it usually reveals a structural misunderstanding, not an off-by-one.

Discovered: 2026-07-03, Engram zero-dep Phase C2 (HTML tag scanners).

---
