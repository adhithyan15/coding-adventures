### Added — styles follow a component mounted more than once

- Composition gives each part the resolver renamed for a second (third, …)
  mount a copy of the original part's style, every state included, so the
  n-th mount renders like the first.
- A style the consumer wrote for the renamed part itself (for example
  `part empty-state-m2 { … }`) wins, and no copy is made.
- With every component mounted once, nothing is renamed and the output is
  byte-identical to before.
- `tests/multi_mount.rs` composes the toolkit's `EmptyState` twice, checks
  the copied styles, and checks that a consumer override and a single mount
  are unchanged.

