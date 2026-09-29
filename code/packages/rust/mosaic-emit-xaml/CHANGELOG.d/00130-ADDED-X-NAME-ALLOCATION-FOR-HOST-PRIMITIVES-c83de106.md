### Added — `x:Name` allocation for Host* primitives

- When the node has a `part_name`, the `x:Name` is the part name
  PascalCased (`formula-field` → `FormulaField`). Matches the spec's
  examples and the convention React/SwiftUI use for code-behind refs.
- When the node lacks a `part_name`, the emitter allocates a
  monotonically-increasing per-component counter (`HostInput_1`,
  `HostInput_2`, ...). Stable across rebuilds.

