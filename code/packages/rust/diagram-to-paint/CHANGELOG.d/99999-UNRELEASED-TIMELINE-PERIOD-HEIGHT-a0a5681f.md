## Timeline period-height product controls

Period cards now estimate vertical space from Unicode scalar counts, owned
width180/190 and positive configured size s. Capacity is floor((width-20)/
(s*0.44)), at least1, with defined saturating usize conversion for positive
quotient overflow/subnormal denominator underflow. Rust hard-line semantics,
integer div_ceil and checked accumulation precede ordered finite checks of
(lines*s)*1.2+12, then max with the existing max(3s,48) card minimum. Overflow
returns TimelineGeometry; no arbitrary cap or wrapped-string allocation.

Title/section/event minima, raw error order, eager empty admission, event
heights/pitch/indexing, grouped LR origin, widths, palette ordinals, authored
strings/IR, metadata/fonts and represented frame/arrow guards remain. Existing
short-label products stay exact. Taller cards update current midpoint and row
dependencies. The painter still uses fixed period text height22: complete
FakeShaper controls retain an oversized unbroken word and three hard-line
glyph runs; neither shaping fit nor containment/raster output is certified.
Event-height estimates and cumulative event stacks remain a separate slice.

Executed source `a0a5681f4295bd8083403f2cb412b9234f95e4bf`: 289 native passes across3 unfiltered harnesses
(Windows e2e0), no failures/ignores/filters. Two doctest invocations contain
0 tests; two strict all-target checks and metadata passed:8 actual commands0.
Fresh1293manifest/1292node/436registry graph gives affected2/forward50.
All407 previous completion identities plus53 new typed and6 complete
source/IR/layout/Scene/query products pass. Compiled semantic red at
`144b1df697f782aef3c7ccec7d82b03a15c973ea` was283P6F before production; both executables and raw streams
are immutable. Exact-source and actual-terminal security reviews PASS/RELEASED.
The EOF whitespace failure (shell1/nativegit2), separate diagnostic assertion
failure and one-LF repair/nativegit0 remain documented in filled lesson shards;
none is relabeled a compiler failure, semantic red or green acceptance.

PR17288 remains unmerged; whole-main/conflicts/current required checks/review
and actual merge remain OPEN. Miri DEFERRED/PENDING, never passed; frozen
run/checkout/inputs/receipts/target untouched and unpolled. Full Closure parity,
stable identities/spans, output-byte/source joins and fold/rename/inline/motion/
merge/delete/pass-sweep lineage remain OPEN. Genuine final-source Miri follows
implementation delivery. No new compiler-wide or Apple rendering run is claimed.
Green receipt `d559f2382dbb398795cc9bb006fe199f45ce9cc4359741cda47093fb36694afa`; immutable manifest `79cb663f73faae8c2e07c12a6ef42bf53aebfa4ad465a9310c93315e08311b5f`.
