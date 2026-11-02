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

**Open:** fix the range bug in its own change. The pilot's property engine
does not reuse these generators; it pins its own 32-bit generator in a corpus
(VER05).
