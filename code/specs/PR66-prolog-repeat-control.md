# PR66: Prolog Repeat Control

## Status

Implemented for the Python builtin, loader adapter, and VM runtime path.

## Goal

Add Prolog-style `repeat/0` as a reusable control primitive on top of the Logic
VM solving engine.

## Behavior

- `repeato()` succeeds indefinitely by re-yielding the current proof state.
- Host-library callers can bound it with `solve_n(...)` or combine it with cut.
- Source-level `repeat/0` lowers through the shared Prolog loader adapter.
- VM runtime callers can safely use `repeat/0` with query limits, and `!/0`
  commits the repeated search in the expected Prolog shape.

## Scope

This does not add parser syntax. `repeat/0` is a normal zero-arity predicate
adapter backed by the shared builtin layer, so future parser/dialect work can
reuse the same runtime behavior.
