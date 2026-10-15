## Unreleased - Bounded Railroad themes and lexical styles

Railroad paints resolved portable styles and scales declared leaf/rule-name
text boxes with font size. Portable controls preserve 40 explicit-style scenes,
40 theme-fallback scenes and 40 sanitized-style scenes with complete ordered
glyph/path/shape/marker traces, real font queries and all eleven metadata keys.
Arc radius zero retains straight polylines; positive radius uses quadratic
corners bounded by adjacent segment lengths, with independent complete goldens.
Marker/stroke enclosure, the inherited 4px rule-marker/root-port mismatch,
fixed rule-name columns, typed-empty/large-font fitting, arbitrary custom-font
and long-label fitting remain open. Apple raster execution is a separate gate.

Integrate producer theme fallback and lexical sanitization without changing
its string policy. Repair only the selected inherited-font admission bypass;
the 72-case test-first cap failure remains recorded. Retain all explicit
numeric, spacing, font, glyph, arc, style and resource controls. New full-config
tests cover 2,112 accepted colors, 1,188 explicit-to-theme fallbacks, 120 font
cases and 12 selected-invalid-primary default cases across four notations and
three source forms. Full fourteen-package validation passes 890/0/0 over
37 targets, plus strict lint; focused parser22/layout14/paint6 pass. Complete
native inputs retain actual c0f2 source acceptance; this is not new native
execution. Broader visual/resource and full Closure provenance debts remain
open. Final-source Miri DEFERRED/PENDING, never passed.
