## Unreleased - 2026-09-19 - Computed inert control-recurrence siblings

Finite `step`/`until` control-recurrence analysis now reuses the existing
integer, real, and boolean identity proofs for inert compound-body siblings.
The one-control-assignment rule, effect exclusions, and 4,096-pass cap remain
unchanged.
