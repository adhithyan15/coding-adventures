## Unreleased - three more leniencies closed (CV2-S6a follow-ups R3, R4, R6)

A dual-lane comparison found three inputs this lane accepted that VER04r refuses. The kit is unchanged: 782 cases run,
0 failed, `v2-binding` skipped whole, all mutants reported.

- **R3. Error case names (section 2.2 rule 2, R72).** `wtypes.check_types_table` checked only that each case of an
  `error` type was text and unique, so a case named `"9"` or `"a-b"` was accepted. Each case must now match
  `[A-Za-z_][A-Za-z0-9_]*`, with the same per-context codes as the rest of the table check (`type`, `ill-typed`,
  `descriptor`, `corpus`).
- **R4. Repeated op parameter names.** `Program.op` refuses an op row that names two parameters alike: `ill-typed` in
  kit IR, `corpus` when a suite reads the op, exactly as a repeated model parameter name already was.
- **R6. Reference suite body (section 4.3).** `chunks._compile_oracle` requires a `reference` suite's `body` to be
  `["eq", subject, expected]` over the oracle's two sides, in either order ("reference means `subject == pure`").
  `ne` and any other shape is `corpus`. Sides are compared as canonical JSON, so `true` and `1` stay different. A
  `predicate` suite's body is unconstrained. The test helper `bare` now derives the reference body from its
  `subject` and `expected`; the guarded refinement fixture guards the oracle's subject instead of the body.
