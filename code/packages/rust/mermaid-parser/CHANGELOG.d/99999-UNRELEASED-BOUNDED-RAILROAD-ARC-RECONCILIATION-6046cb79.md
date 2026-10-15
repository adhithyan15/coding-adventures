## Unreleased - Bounded Railroad arc reconciliation

All four Railroad dialects retain style configuration through init directives
and YAML front matter. Font size must be finite and in (0,65536]; stroke width,
marker radius and arc radius must be finite and in [0,65536]. Zero font size
retains its positive-number error. The existing parser helper already enforces
the new `arcRadius` field's bounds; no parser-production repair was needed.
Controls verify 56 exact arc refusals and 56 accepted complete configurations,
alongside preserved font/style/spacing admission matrices and default radius 10.

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
