---
category: TypeScript / JavaScript
---

# A per-byte BigInt hash is the hot loop once every lesson is fingerprinted; profile before raising a corpus-scaled budget

**What happened:** on the French A2 spine PR, CI's `build (ubuntu-latest)` failed with
`Test timed out in 35000ms` at `human-language-data/tests/cli.test.ts`. That test
builds the whole curriculum gap report, and its comment forbids just raising the
number: profile it first.

A `node --cpu-prof` run of `runCurriculumGapReport` found nothing superlinear. The
corpus had grown to 20,317 lessons, about 7x the size the 35s budget was sized for.
The single largest self-time entry was `fnv1a64` in `src/hash.ts`, at about 4s of a
24s report. It hashed every lesson with BigInt arithmetic, allocating several BigInts
per byte.

**Fix:** hold the 64-bit FNV state as two unsigned 32-bit halves. The FNV prime is
2^40 + 0x1b3, so the multiply splits into three parts:

- `lo * 0x1b3`, which is exact in a double and carries into `hi`;
- `Math.imul(hi, 0x1b3)`;
- `lo << 8` added into `hi`.

The output was checked byte-identical against the old implementation on 20,000
random strings and on every lesson in the corpus. Hashing all lessons went from
~5.2s to ~1.7s, and one report from ~24s to ~19s.

**What to do differently:**

1. Avoid BigInt in a per-byte loop that runs over the whole corpus. Use 32-bit halves
   (`Math.imul`, `>>> 0`) when the arithmetic allows it.
2. When a corpus-scaled test budget trips, profile it the way its comment says
   (`node --cpu-prof` over the built `dist/`, then sum self time per function). The
   hot spot may be a utility nobody thinks of as expensive.
3. A rewrite of a fingerprint function must be proven byte-identical before it
   ships. Every checked-in hash ledger depends on it.
