## Unreleased — bounded Railroad style reconciliation

Railroad configuration also carries font family/size, portable colors, stroke
width and marker visibility/radius. Numeric style fields share the finite
65536 cap with spacing; font size is strictly positive, and stroke/radius are
nonnegative. Defaults and accepted values are retained without clamping.

Spec-first source965 passes the complete affected fourteen-package suites:
871 passed, zero failed/ignored, 34 targets; strict all-target lint EXIT0.
Native63 runtime/test/config inputs remain source-identical apart from four
known README/changelog changes. Hosted CI and merge gates remain separate.
Miri DEFERRED/PENDING, never passed.
