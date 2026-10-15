## Unreleased — bounded Railroad style reconciliation

Railroad paints the resolved portable styles and scales declared leaf/rule-name
text boxes with font size. Portable controls compare complete ordered glyph and
path vectors, shape geometry/styles, all ten metadata keys, actual cached font
queries and global instruction-kind order. Marker/stroke visual enclosure,
fixed rule-name columns and typed-empty/large-font fitting remain limitations;
this does not claim arbitrary custom-font or long-label fitting.

Spec-first source965 passes the complete affected fourteen-package suites:
871 passed, zero failed/ignored, 34 targets; strict all-target lint EXIT0.
Native63 runtime/test/config inputs remain source-identical apart from four
known README/changelog changes. Hosted CI and merge gates remain separate.
Miri DEFERRED/PENDING, never passed.
