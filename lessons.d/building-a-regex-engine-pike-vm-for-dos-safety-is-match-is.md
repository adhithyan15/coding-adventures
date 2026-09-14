# Building a regex engine: Pike VM for DoS-safety; is_match is far easier than match-extent

Context: Phase D of the Engram zero-dep program replaces the third-party `regex`
crate with a from-scratch engine. D0 shipped the `is_match` core.

**Lessons:**
- **Use a Pike VM (Thompson NFA), not backtracking.** Engram matches
  *user-supplied* `re:` patterns; a backtracker is exponential on inputs like
  `(a*)*b` — a DoS. The Pike VM advances all threads in lockstep: O(pattern ×
  input), immune. It also matches the `regex` crate's leftmost-first + greedy
  semantics if you order Split's targets by priority (greedy tries "take it"
  first).
- **Make the epsilon-closure iterative, not recursive.** A long chain of
  epsilon transitions (`a?a?a?…`) recursed would overflow the stack — another
  DoS. An explicit stack that pushes Split's second target before its first
  preserves DFS priority order without recursion.
- **Cap the compiled program size.** `{0,10000000}` expands to millions of
  instructions; reject over a cap (as `regex` does) to bound memory + match time.
- **`is_match` is dramatically easier than `find`/`captures`/`replace_all`.**
  Boolean existence doesn't care about *which* match or its extent, so the basic
  Pike VM nails it (cross-verified vs `regex` `(?-u)` on 100k+ random pairs, zero
  divergences). But exact match **extent** requires solving the **nullable-loop**
  priority problem: `(b??)+` on `"b"` — `regex` returns the empty match `(0,0)`
  (the greedy outer loop must not take a zero-progress iteration), but a naive
  Pike VM consumes the `b` → `(0,1)`. This is a known-hard sub-problem (RE2
  handles it with empty-width tracking). So: **ship `is_match` first**, and give
  match-extents their own PR. Most of engram's regex use is boolean anyway.
- **Force the reference into ASCII mode for a fair cross-check.** The `regex`
  crate's `\w\d\s\b` are Unicode by default; prefix the reference pattern with
  `(?-u)` so the corpus compares against ASCII-class behaviour while the engine's
  Unicode classes are still a follow-up.

Discovered: 2026-07-03, Engram zero-dep Phase D0 (regex engine `is_match` core).
