## Unreleased — bounded Railroad style reconciliation

All four Railroad dialects retain style configuration through init directives
and YAML front matter. Font size must be finite and in (0,65536]; stroke width
and marker radius must be finite and in [0,65536]. Zero font size retains its
positive-number error. Portable tests verify exact refusal domains and complete
accepted config, alongside the preserved spacing-admission matrices.

Spec-first source965 passes the complete affected fourteen-package suites:
871 passed, zero failed/ignored, 34 targets; strict all-target lint EXIT0.
Native63 runtime/test/config inputs remain source-identical apart from four
known README/changelog changes. Hosted CI and merge gates remain separate.
Miri DEFERRED/PENDING, never passed.
