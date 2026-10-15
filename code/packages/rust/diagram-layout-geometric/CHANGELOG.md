# Changelog

## Unreleased — reviewed Venn debug geometry and set defaults

Debug layout groups exact ordered set vectors, including embedded NULs, in first
authored order. Preparation indexes preserve last-ID centers, first-ID radii and
any nonempty matching region label. Checked layout validates finite debug
positions and finite nonnegative radii/cell dimensions alongside every previous
numeric guard. Actual scan-visit tests cover 8/32/128 groups without a new cap.

Unstyled sets use opacity 0.1, resolved raw fill as stroke, and stroke width
`5 * width / 1600`. Authored opacity, stroke and width override independently,
including zero; an empty typed palette retains the blue fallback. Paint consumes
explicit debug vectors even when the typed debug flag is false.

Tested source `58a7862f`: 943 tests passed, zero failures/ignores, 41 targets;
strict all-target Clippy passed for all 14 affected packages. The 176 complete
scene/query cases retain 104 earlier oracles through a checked reversible
default adapter and add 56 typed plus 16 parsed-source cases. This establishes
the bounded contracts, not full Mermaid rendering or Closure parity. Required
hosted acceptance remains pending; final-source Miri **DEFERRED/PENDING**.

## Unreleased — preserve bounded Venn palette transport

Transport all eight independently sanitized source palette slots through typed
config and checked layout into ordered circle fills and complete scene metadata.
Preserve scoped source-origin/quote guards, numeric/geometry limits, authored-fill
precedence and arbitrary/empty typed palette semantics. Verify104 whole-scene
cells,932 affected tests/41targets, strict lint and independent source/terminal
reviews. Physical/full-style parity, hosted acceptance and publication remain
separate. Final-source Miri **DEFERRED/PENDING**, never passed.

## 0.1.0 — 2026-04-24

### Added
- Initial release as part of DG04 extended diagram families
- `layout_geometric_diagram(diagram)` — resolve canvas size for geometric diagrams
- Auto-size from element bounding boxes (Box, Circle, Line, Arc, Text)
- Respects explicit `width`/`height` overrides (partial or both)
- 7 unit tests

## Unreleased — bounded Venn canvas configuration

Preserve incoming canvas config and portable metadata while bounding source
scalars and adding a checked typed layout API. Recognize only direct config
members in closed single-line init objects; preserve whole-object front-matter
precedence and field defaults. Keep original supported geometry and every
unrelated production/test path. Add exact geometry and complete scene/query
controls; final affected 14-package strict lint and 918 unfiltered tests pass.
Final-source Miri DEFERRED/PENDING; hosted checks/merge/full parity remain OPEN.

## Unreleased — width-relative Venn typography

`LayoutedVennLabel.font_size` is a required resolved layout field. Generated set
and intersection labels use `48 * width / 1600`, attached labels use
`40 * width / 1600`, and paint titles use `32 * width / 1600`. Paint retains caller
weight, italic and stretch and resolves the source family/colors as before.
Checked layout also refuses nonfinite or negative label font sizes alongside
the existing coordinate checks. Width admission remains finite(0,65536]; zero
font size caused by tiny-width underflow remains supported. No source font
option, new cap, clamp, fallback or crate dependency was added.

At actual source `04712174`, the complete14-package reverse cone passed
960 tests across43 targets with zero failures/ignores, plus strict all-target
warnings/unknown-lints denial. All208 previous complete scene/query cases retain
their non-typography expectations; a narrow one-glyph adapter checks the new
font sizes and full glyph positions before reversing only the old typography.
Twenty-four additional complete scene/query cases cover three widths, two
families, titles and authored colors. Thirty-one required constructor fields
are inventoried; invalid derived fonts and width65537 still refuse.
Independent source and terminal security reviews passed.

Fixed text boxes, long/multiline fitting, real-font rendering, arbitrary forged
layout/fonts/styles and general renderer resource/visual parity remain open.
Native validation is separately attributed to `0cc8fef6`:1,824 passed/zero failed/
one unchanged archive ignore across78 targets, compiler887/fullLANG502 and
strict six-package lint. The126 focused sources produced756 actual backend
cells (696 prior exact+60new), on six available backends; absent CLR is uncounted.
Transfer uses exact166-package/2847-object inputs and21 mapped external static
resources before this documentation carrier. Named Markdown exceptions are
inventoried afterward; no dynamic/toolchain/OS closure or new native execution
is inferred. Historical Closure36/3705 plus three static inputs retains316e
attribution. Registry376 identities/checksums remain exact.

The actual hosted macOS compiler-phase proof is still OPEN. The separate
diagnostic change4ab4 preserves a bounded JSON-quoted child-output tail on
failure;13 portable tests and independent review passed. That is visibility,
not evidence of a successful Apple compiler/Cabal route. Required current-head
CI/review/conflict checks and actual merge remain delivery gates. Final-source
Miri **DEFERRED/PENDING**, never passed; frozen run/source/inputs/target/receipts
remain untouched/unpolled. Full Closure parity and output-source/full lineage
remain OPEN.

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
