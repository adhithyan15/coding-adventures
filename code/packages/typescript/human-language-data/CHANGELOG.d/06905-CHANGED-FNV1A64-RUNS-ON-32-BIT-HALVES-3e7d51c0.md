### Changed — `fnv1a64` runs on two 32-bit halves instead of BigInt

- `src/hash.ts`: FNV-1a 64 now keeps its state as two unsigned 32-bit halves. It
  splits the multiply by the prime 2^40 + 0x1b3 into `lo * 0x1b3`, `Math.imul(hi, 0x1b3)`
  and `lo << 8`. Before, it allocated BigInts for every byte.
- The output is byte-identical:
  - it matched the BigInt version on 20,000 random strings, including astral code
    points;
  - it matched on every one of the corpus's 20,149 lessons;
  - the published test vectors and the generated hash ledgers still pin it.
- Hashing every lesson went from ~5.2s to ~1.7s, and one curriculum gap report from
  ~24s to ~19s locally.
- At 20,317 lessons, `tests/cli.test.ts`'s json report case overran its 35s budget
  on CI. Its comment asks for a profile rather than a raised budget, and this was the
  largest single cost the profile found. The comment now records that history.
