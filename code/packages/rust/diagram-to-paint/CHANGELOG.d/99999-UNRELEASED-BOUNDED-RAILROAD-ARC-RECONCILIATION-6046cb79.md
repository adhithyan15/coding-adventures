## Unreleased - Bounded Railroad arc reconciliation

Railroad paints resolved portable styles and scales declared leaf/rule-name
text boxes with font size. Arc radius zero retains straight polylines; positive
radius uses quadratic corners bounded by adjacent segment lengths. Ten
independently calculated complete command/style/order goldens cover tiny and
duplicate segments, corner clamping and dashed loopbacks. Portable controls
retain complete glyph/path vectors, geometry/styles, all eleven metadata keys,
actual cached font queries and instruction-kind order. Marker/stroke enclosure,
the inherited 4px rule-marker/root-port mismatch, fixed rule-name columns and
typed-empty/large-font fitting remain limitations; arbitrary custom-font and
long-label fitting remain open.

Spec-first import retains producer arc defaults, parser admission, choice
routing, quadratic lowering and existing font/glyph/spacing/style controls.
Test-first typed-error/port goldens expose the hierarchy defects. Typed layout
now admits finite arcRadius 0..65536 before arithmetic; Optional and both
Repetition variants use symmetric 2r top/bottom reserves and matching measure.
Focused parser 13/0, hierarchy 14/0 and paint 4/0 pass. Full fourteen-package
style/parser/paint/downstream run passes 879/0/0 over 35 targets, plus strict
lint. Complete native inputs retain source5ed; no new native execution is
inferred from style success. Broader visual/resource debts and full Closure
lineage remain open. Miri DEFERRED/PENDING, never passed.
