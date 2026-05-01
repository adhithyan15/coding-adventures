# PR49 - Prolog Runtime Flag Introspection

## Goal

Expose a first read-only `current_prolog_flag/2` surface through the Python
logic builtin layer, Prolog loader adapter, and Logic VM stress path.

## Motivation

Real Prolog programs and dialect support code often inspect runtime flags before
choosing portability behavior. Even when flag mutation is not supported yet,
deterministic flag introspection gives parser-backed programs a stable way to
ask what this compatibility layer currently promises.

## Initial Flag Set

The first batch exposes conservative flags that describe existing runtime
behavior:

- `bounded = false`, because Python integers are unbounded.
- `char_conversion = false`, because the frontend does not perform character
  conversion.
- `debug = false`, because there is no source-level debugger mode yet.
- `double_quotes = string`, matching the current string-term behavior.
- `integer_rounding_function = floor`, matching the current floor-division
  arithmetic helper.
- `occurs_check = false`, matching the current unification behavior.
- `unknown = fail`, matching the current unknown-predicate behavior.

## API

Library callers use:

```python
current_prolog_flago(name, value)
```

Parsed Prolog source uses:

```prolog
current_prolog_flag(Name, Value)
```

Both instantiated lookups and full enumeration are supported through ordinary
unification.

## Deliberately Deferred

- `set_prolog_flag/2` and branch-local mutable flag state.
- Dialect-specific flag overlays.
- Erroring on unknown flag names; the first read-only predicate simply fails
  when no known flag unifies.

## Acceptance Tests

- `current_prolog_flago/2` enumerates all supported flag pairs.
- Instantiated flag lookups return the expected value and unknown flags fail.
- `current_prolog_flag/2` is adapted by `prolog-loader`.
- A source-level query using multiple flag lookups runs through
  `prolog-vm-compiler`.
