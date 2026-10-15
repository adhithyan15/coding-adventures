## Unreleased — bounded Railroad style reconciliation

`try_layout_railroad` validates all six numeric spacing/style values before
font clamp or derived arithmetic, including empty diagrams. Invalid font size,
stroke width or marker radius returns `UnsupportedStyle`; spacing errors stay
unchanged. Font measurement retains the producer's 14px default (44.1px minimum
leaf content width and 16.1px height before padding).

Spec-first source965 passes the complete affected fourteen-package suites:
871 passed, zero failed/ignored, 34 targets; strict all-target lint EXIT0.
Native63 runtime/test/config inputs remain source-identical apart from four
known README/changelog changes. Hosted CI and merge gates remain separate.
Miri DEFERRED/PENDING, never passed.
