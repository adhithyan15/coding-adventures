## Unreleased - Checked Railroad display label geometry

Reserve the displayed rule-label width and emit explicit root-port marker
points; decorate special display labels without mutating semantic inputs.
Extract the unchanged complete derived-layout predicate and extend it to both
marker points. Twelve nonfinite-coordinate controls and exact connector/layout
goldens pass; all prior admission limits and circular controls remain intact.
Full fourteen-package validation passes 902 tests/38 targets and strict lint.
Large-radius connector reversal/clearance and general font fitting remain debt.
Miri DEFERRED/PENDING, never passed.
