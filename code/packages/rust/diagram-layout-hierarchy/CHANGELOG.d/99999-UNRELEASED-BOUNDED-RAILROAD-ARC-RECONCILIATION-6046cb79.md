## Unreleased - Bounded Railroad arc reconciliation

`try_layout_railroad` validates all seven numeric spacing/style values before
font clamp or derived arithmetic, including empty diagrams. Invalid font size,
stroke width, marker radius or arc radius returns `UnsupportedStyle`; spacing
errors stay unchanged. Font measurement retains the producer's 14px default
(44.1px minimum leaf content width and 16.1px height before padding).

Radius-driven Optional and both Repetition variants reserve twice the arc
radius above and below child content. Matching measurement and placement keep
their main ports on the containing box's center through nested sequences and
choices. Independent wrapper geometry and complete path goldens cover zero,
tiny, default and nondefault radii. Checked derived coordinates remain finite
and rectangle sizes nonnegative; no global derived-layout cap is introduced.
The inherited 4px rule-marker/root-port mismatch remains open.

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
