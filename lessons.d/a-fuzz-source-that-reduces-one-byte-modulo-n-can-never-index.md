---
category: Testing & coverage
---

# A fuzz source that reduces one byte modulo n can never index past 255

The Warrant evidence prelude's fuzz source (`StreamSrc::bounded`) is
`byte() % n`. That is fine for `n <= 256`, which is all the v0 generators
asked of it. The first v1 generator picked a value from a 559-entry edge set
with `src.bounded(len)`, and the fuzz suite could then only ever reach the
*first 256* entries (the most negative ones). A negative control that needed
`256` or `-1` from the middle of the set (`off-by-one-range` on a `try<U8>`
shaped operation) was never caught by the fuzz suite, while the property suite
(a real RNG) caught it: the two suites disagreed and the bug looked like a weak
fuzzer rather than a broken index.

The fix was to index through `src.word()` (eight bytes) whenever the domain can
exceed 256 values. Do differently: when a generator indexes a table, assert (or
test) that every index is reachable from *each* randomness source the table is
used with, not only from the primary one; and when a negative control passes on
one suite kind and fails on another, check the draw's reachability before
raising iteration counts.
