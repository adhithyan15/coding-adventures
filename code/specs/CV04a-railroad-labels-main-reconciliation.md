# CV04a: Railroad display labels and explicit marker geometry

Main4a721b2c (#17561) adds explicit start/end marker points to each layouted rule,
dynamic rule-label reservation, bold `name =` display and special `? text ?`
decoration. Commit this spec before integration and tests. Preserve semantic
labels, owned checked layout/font/spacing limits, cubic controls, selected font
cap, metric-aware leaf/rule-name heights, no-wrap behavior, registry policy and
complete style/glyph/font-query/order/metadata oracles. Do not blindly accept
incoming fixed28 rule-name height where existing valid large fonts need more.

Retain all incoming fields and display semantics, reconstruct merge overlaps
explicitly and capture actual raw style compilation/test failures. Two owned
typed test constructors need explicit marker points after the public struct
change; change those fixtures to named declared points, preserving all other
geometry/metadata. This constructor migration is not successful test execution.
Before production repairs, add authored complete glyph/font-query controls for
both rule and special labels, all four dialects, explicit/theme typography,
small/large valid font sizes, both compact/marker states, long rule names and
marker radii. Compare complete glyph sequences, font weight/family/size, element
dimensions, marker centers, connector endpoints and complete12-key metadata.

Update only old assertions whose documented display/marker semantics changed:
rule glyphs now include suffix and bold font, special glyphs include decoration,
rule width/position is label-derived, marker y follows the actual root port,
and paths attach at declared marker radii. Enumerate reversible old-oracle
changes and retain every other shape/style/source field assertion. Retain
circular control shapes/radial tests; translate authored absolute route goldens
only by the independently calculated new label origin and replace the two root
connectors with independently specified point endpoints. Require exact new
absolute geometry rather than accepting mere cubic/marker presence. Existing
duplicate/zero/collinear controls remain. Preserve all work/resource controls.

Test real metric bounds before proposing layout repair. Keep rule-name height
max(28,fontSize*1.15), resolved bold typography and no-wrap semantics, unless an
executed failing control justifies a narrower repair. Explicit root/marker port
agreement can close the former4px mismatch only with exact y/endpoint evidence.
Do not claim arbitrary custom-font/long-label fitting, marker/stroke enclosure,
forged/extreme/oblique geometry safety, browser typography or full Mermaid
parity. Preserve the bounded circular-corner qualification in DG04 while adding
the new label/marker contract.

After focused green freeze the labels checkout and run full affected14-package
style/paint/downstream tests plus strict all-target lint. Independently compare
all native166 package inputs with the separately tested seven-unit source,
including named documentation exceptions, before retaining actual terminal
native acceptance. Never infer it while its run is active. Use separate targets
and preserve R2 source while native validation runs. Actual docs/lessons/checker,
corrected Closure36/3705, registry376, fresh current-main unforced build plan and
independent final-carrier review gate same-PR publication. Hosted terminal
acceptable conflict-free/review-clean exact-head checks still gate auto/merge.

Miri DEFERRED/PENDING, never passed; frozen run/checkout/target/inputs/receipts
untouched/unpolled. Base ESTree/owned types/no new crates.io dependencies remain.
Full Closure parity/output-source/full lineage/CV04b remain OPEN; deliver first,
then genuine final-source Miri; heartbeat ACTIVE.
