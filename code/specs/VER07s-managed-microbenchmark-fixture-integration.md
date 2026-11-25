# VER07s — Managed microbenchmark fixture integration

The C#, F#, Java, and Kotlin implementations in VER07k and VER07l add eight
real readers of `code/specs/fixtures/microbenchmark-pairs-v1.json`. Register
each language's `microbenchmark` and `rank-statistics` packages alongside
the sixteen readers already integrated for Go, Python, Rust, TypeScript,
Ruby, C++, C, and Swift. These are twenty-four package consumers, rather
than twenty-four languages or cross-language executions.

A fixture-only edit must select every registered reader. A language filter
must select precisely the two readers in that language. Missing packages
must never be synthesized. Kiln export must declare the canonical fixture
as an exact repository input for every reader, and legacy fallback package
hashing must incorporate its bytes. A byte-only fixture change must change
each reader's fallback hash even when its package sources are unchanged.

Validation extends the existing selection, filtered-selection, missing-package,
graph-export, and fallback-hash tests. Run the affected Go tests and vet.
Keep language-port validation evidence separate from integration checks:
source tests, installed package consumers, native default-setting runs,
and report replay each establish a different part of the acceptance contract.
No registry publication or unbounded correctness proof follows from them.
