---
package: */rng
lane: *
rung: infra
status: open
---

# The rng packages draw 32 bits for ranges up to 2^64

The per-language `rng` packages' bounded draw (`next_int_in_range` and its
equivalents) draws a single 32-bit value but accepts ranges up to 2^64, so it
can never return a value more than 2^32 above the minimum. These packages also
use 64-bit arithmetic on every draw, which is BigInt in TypeScript and Perl.

**Worse than first recorded:** lanes that take the rejection threshold modulo
2^64 while drawing 32 bits (Rust before 0.2.0, and possibly others) **never
return** for a span such as 2^63 + 1: the threshold exceeds every possible
draw. The whole signed 64-bit domain divides by zero in fixed-width lanes.

**Handling (in progress):** `code/specs/rng.md` defines `next_int_in_range` v2
in three regimes, keeping the 32-bit regime exactly as v1 so small ranges keep
their sequences. The shared corpus `code/specs/fixtures/rng-v1/cases.json` is
checked by the independent reference `code/scripts/rng_conformance.py`.
Fixed and running the corpus: Python, Rust, Go, C, C++ and Swift. Go, C and
C++ had the hang; Swift also trapped on any range wider than half the domain.
Still open: C#, Dart, Elixir, F#, Haskell, Java, Kotlin, Lua, Perl, Ruby and
TypeScript. Lanes without a JSON reader can use the generated flat form
`code/specs/fixtures/rng-v1/cases.txt`.

The pilot's property engine does not reuse these generators; it pins its own
32-bit generator in a corpus (VER05).
