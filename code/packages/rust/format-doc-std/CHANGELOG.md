# Changelog — format-doc-std

## [0.1.0] — 2026-04-30

Initial release.  Reusable templates over `format-doc`.  The "80%
layer" in the formatter stack.  Rust port of P2D04.

### Added

- `delimited_list(open, items, close)` + `delimited_list_with(...,
  &DelimitedListConfig)` — covers arrays, tuples, parameter
  lists, argument lists, object fields.
- `call_like(callee, args, &CallLikeConfig)` — function and
  constructor calls.
- `block_like(open, body, close)` + `block_like_with(...,
  &BlockLikeConfig)` — braces / `begin … end` / indented block
  bodies.
- `infix_chain(operands, operators, &InfixChainConfig)` —
  arithmetic, boolean, pipeline, type-operator chains.
- `TrailingSeparator` enum (`Never` (default) / `Always` /
  `IfBreak`, `#[non_exhaustive]`).
- `DelimitedListConfig`, `CallLikeConfig`, `BlockLikeConfig`,
  `InfixChainConfig` — all `Default`-implementing.

### Notes

- Pure data → `Doc`.  Single dep on `format-doc` (capability-empty).
  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.
- Templates only call `format-doc` builders; no recursion in this
  crate (the realiser owns recursion).
- Doc is internally `Arc`-shared — `config.separator.clone()` is
  O(1).
- Only panic surface is `infix_chain`'s arity-mismatch `assert_eq!`
  (programmer error, not attacker input).
- 25 unit tests + 1 doctest covering per-template empty / flat /
  broken cases, all `TrailingSeparator` variants, custom
  separators / brackets / open-close, empty-spacing toggle, infix
  break-after vs break-before-operator, arity-mismatch panic,
  composability (nested templates, realistic expression).
- Security review: clean, no findings.
- Filed as follow-ups in README roadmap: more templates
  (`assignment`, `if_then_else`, `triadic_op`), theme-aware
  variants once `format-doc-to-paint` ships, automatic
  annotation pass-through helpers.
