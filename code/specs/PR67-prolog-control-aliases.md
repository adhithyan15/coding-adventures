# PR67: Prolog Control Aliases

## Status

Implemented for the Python builtin layer, source adapter, and VM stress path.

## Goal

Round out everyday Prolog control with `false/0` and `ignore/1`.

## Behavior

- `falseo()` is an explicit alias for logical failure.
- Source-level `false/0` adapts to `falseo()`.
- `ignoreo(Goal)` runs `Goal` at most once.
- If `Goal` succeeds, `ignoreo/1` preserves the first solution's bindings.
- If `Goal` fails or cannot be interpreted as a callable goal at runtime,
  `ignoreo/1` succeeds once with the original proof state.

## Scope

These are library-backed builtins, not new syntax. They compose with the same
solver, loader adapter, and Logic VM runtime used by the rest of the Prolog
control surface.
