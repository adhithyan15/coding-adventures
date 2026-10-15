# Changelog

## Unreleased — isolated cumulative two-unit offsets

Reconcile main's two-unit outer-entier bounds in a dedicated iterative proof.
Keep the shared sign-range helper and legacy one-unit helper exact. Admit one
unshadowed sign root and canonical literal offsets with cumulative absolute
budget at most two; explicit wrappers cannot reset the budget. Preserve the
one-front conditional, exact read-once identity lowering and alias effects.
Refuse excess units, multiple roots, shadowed builtins, generic constant-fold
traps and widened exp/log chains. Focused tests pass 12/0; full ALGOL passes
857/0. All six affected packages pass 1,774 tests over 78 targets with one
existing ignored test, plus strict lint. Miri is DEFERRED/PENDING, never passed.
General recursive composition and full Closure provenance remain open.
