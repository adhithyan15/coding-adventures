## Unreleased — bounded Ishikawa configuration and theme transport

Ishikawa semantic and resolved layout types carry required `IshikawaConfig`:
padding20, useMaxWidthfalse, sans-serif, line/text #333333 and fill #ffffff.
Layout retains the prior fishbone arithmetic and clones the complete config.
Paint uses line color for spine/bones/effect stroke, fill for the effect box,
text color for all labels, and source font family while retaining caller sizes,
weight, italic and stretch. Six exact scene metadata entries carry the config;
useMaxWidth is transported and does not establish complete sizing behavior.

Only direct root themeVariables fields and direct config.ishikawa fields in
leading YAML mapping ancestry can supply settings. A closed single-line init
object after that header replaces its corresponding mapping wholesale, including
empty objects. Nested objects/arrays, unrelated ancestors, sequence/block-scalar
descendants and init-looking YAML literal text cannot supply fields. The active
quote decoder preserves opposite quotes and rejects malformed present values
before the existing lexical sanitizers see them. It does not admit a valid
prefix from a longer invalid value. Source padding retains finite nonnegative
admission without a new upper cap; shared/Venn readers and sanitizers are exact.

Actual source `3e729cea`: full14-package unfiltered suites passed966 tests
across44 targets with zero failures/ignores, plus strict all-target warnings/
unknown-lints denial. Four parser tests assert67 whole causal IR cases. Twenty-
one complete source IR/layout/PaintScene/glyph/metadata/ordered font-query cells
cover16 custom cases and5 literal default/fallback/header/empty-replacement
companions. All232 previous Venn complete scene/query cells and their numeric/
work/style guards remain intact. Tests-first sourceee8 produced1P/3F for nested
origins, opposite-quote truncation and malformed numeric tails; its16 custom
scene cells already passed. The b8 source's965/44 run remains separate; review
withheld spec-complete acceptance until the five companions were added.
Independent exact sourceed49e961 and terminalfcd6517b reviews passed.

Native execution remains actual0cc8fef6:1824P/0F/1unchanged archive ignore over78
targets, compiler887/fullLANG502 plus strict6, and126 focused sources/756 actual
six-backend cells (696 prior exact+60 new; unavailable CLR uncounted). Native166/
2847 input identities retain that source except four named Markdown paths;
21 mapped external static resources remain exact. Historical Closure36/3705
and three conservative static includes retain316e attribution. These input
proofs do not imply new native/Closure execution or a dynamic/toolchain/OS
resource closure. Latest inspected main e6aa15c0 separately raises the native
ALGOL boundary to thirteen; that incoming change remains pending its own bounded
reconstruction/tests/full native validation. These receipts retain their actual
twelve-unit source and557 diagram integration frame. A fresh plan against latest
main is selection only; an unmerged PR update does not establish latest-main
native acceptance or permit merging. Registry376 identities/checksums remain exact; only four local
package versions change in this slice, with no new crates.io dependencies.

Color sanitizers retain their bounded lexical subset, not complete CSS semantics.
Real font fitting, arbitrary forged typed layout/styles, extreme canvas/depth
and general visual parity remain open. The actual hosted macOS compiler-phase
failure remains OPEN; diagnostic4ab4 has13 portable tests/Ruff/review500a passing
and preserves quoted bounded diagnostics only. Required current-head hosted
CI/review/conflict checks and actual fetched merge/payload proof remain delivery
gates. Final-source Miri **DEFERRED/PENDING**, never passed; frozen checkout/run/
inputs/target/receipts stay untouched/unpolled, no duplicate or termination.
Implementation delivery precedes genuine final-source Miri; full Closure parity,
output-source/full transformation lineage/stable IDs/spans/pass-sweep remain OPEN.
