# Generating Unicode/data tables for a zero-dep crate; reading a branch diff when main has advanced

Context: Phase C1 of the Engram zero-dep program replaced `unicode-normalization`
with a from-scratch zero-dep `code/packages/rust/unicode-normalize` (NFD/NFC +
`is_combining_mark`, Unicode 17.0.0).

**Lessons:**
- **Generate large data tables from the real crate, don't transcribe by hand.**
  A throwaway `#[ignore]` test with the upstream crate as a `[dev-dependencies]`
  enumerated every scalar value (`0..=0x10FFFF`) and emitted a compact
  `src/tables.rs` (CCC / recursive decomposition / composition / Mark ranges).
  Then a second throwaway test cross-checked this crate vs the live upstream one
  across ALL code points + 200k random strings (zero mismatches) before deleting
  the generator, the cross-check, and the dev-dep. Same interop-gate discipline
  as protobuf/fsrs. Composition pairs were captured by probing upstream
  `compose(a,b)` over candidate (starter, combiner) sets drawn from the decomp
  table — no need to parse UCD files or hit the network.
- **Two traits with the same method name (`nfd`/`nfc`) on the same receiver make
  calls ambiguous** in the cross-check (both `unicode_normalize` and
  `unicode_normalization` define them). Call via fully-qualified syntax
  `Trait::method(x)` in the gate test.
- **A blanket `impl<I: Iterator<Item=char>> Trait for I` collides with
  `impl Trait for &str`** (coherence can't prove `&str: !Iterator`). Mirror the
  upstream crate: impl for the concrete receivers actually used (`&str` and
  `str::Chars`), not a blanket.
- **`git diff --cached origin/main` on a feature branch shows the OTHER merged
  PRs as spurious "deletions" once origin/main has advanced past your branch
  point** — it compares your index to the newer main, so their newer additions
  read as removed-on-your-side. This looked alarmingly like the stale-worktree
  revert bug. Distinguish with `git status --short` and `git diff --name-only HEAD`
  (working tree vs HEAD) — if those show ONLY your files, you're clean. Fix the
  scary diff by `git rebase origin/main`; afterwards `git diff --name-only
  origin/main` shows exactly your files. Always rebase before pushing so the PR
  diff is only yours.
- **Hangul is algorithmic** (UAX #15 §16) — decompose/compose by arithmetic on
  code points, saving ~11,000 table entries. Only the 11172 syllables; the jamo
  compose via the same arithmetic.

Discovered: 2026-07-03, Engram zero-dep Phase C1 (drop `unicode-normalization`).

---
