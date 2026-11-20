## Unreleased - CV2-F8: a `match` bind may not shadow a model, op, error type, constructor or built-in

### Fixed

- `lower.py` (`_Lowerer._declare`): a pattern `bind` (`Ok(x)`, `Err(e)`, a name inside a tuple pattern, a whole-value
  name) spelling a declared model, an op, an error type, or a reserved built-in is now a `corpus` error ("a bind name
  shadows ..."), the same refusal as a bind that shadows a binder, parameter, `result` or an outer bind (VER04r 16.10
  R2). The Rust checker already refused these (`check_arm_binder`, W0203); the oracle accepted them. Constructors
  (`Some`, `None`, declared variant constructors, `Ok`, `Err`) were already refused earlier and are unchanged.
- The reserved set is `_RESERVED_V0`/`_RESERVED_V1`, the latter derived from the oracle's own tables
  (`FIXED`, `levels.FIXED_TAGS`, `_LEVEL_DATA`, `_LEVEL_SEQ`, `take`, `drop`); built-ins are version scoped as in the
  checker. `tests/test_bind_names.py` asserts the set equals the checker's `V0_BUILTINS` + `V1_TYPES` + `V1_FUNCTIONS`.

### Tests

- `tests/test_bind_names.py`: one refusal per category (model, op, error type, built-in at v0 and v1, tuple pattern,
  whole-value bind), the constructor controls, version scoping, and an ordinary bind that still lowers. No committed
  fixture or dump changes.
