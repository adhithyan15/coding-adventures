## Unreleased - Bounded Railroad arc reconciliation

Railroad configuration carries `arc_radius` (authored `arcRadius`, default 10)
alongside font family/size, portable colors, stroke width and marker settings.
Numeric fields share the finite 65536 cap with spacing; font size is strictly
positive, and stroke width, marker radius and arc radius are nonnegative.
Parsers and checked hierarchy layout retain accepted values without clamping.

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
