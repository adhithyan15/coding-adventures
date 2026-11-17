## 0.4.0 - S3e: v0 rule 2 on examples [D18], a call outside the op's admit is refused [D19]

### Added

- **[D18]** `derive` counts the wraps of an example's evaluation; in a v0 document a wrap is the
  generation error `W0701` (the claim rule's code), in a v1 document it is accepted
  (`f(255u8 + 2u8) == 1u8`).
- **[D19]** `SuitePlan.check_admits`: one dedicated, unmetered pass over the admitted cases in
  order (whatever the run depth) evaluates each subject call on the pure trace
  (`ir_eval.compile_admit_check`, which mirrors `compile_trace`) against the callee's op `admit`;
  the first case outside it is `W0701` naming the claim, the op and the admitted-case ordinal.
  Examples are checked the same way. A call whose op `admit` reads a parameter whose
  argument depends on a subject result is refused statically; an argument the `admit` does not
  read (gf256's `power(add(a, b), 2u64)`) is accepted. The pass is skipped when no call is of an
  op with an `admit`.
- `tests/test_admit_calls.py`, the same documents as the generator's `tests/admit_calls.rs`; the
  two implementations agree on all thirteen (refusals with the same ordinal, accepted documents
  byte for byte).

### Review fixes (Fable, CV2-S3e)

- A wrap while evaluating a callee's `admit` is `W0701` in a v0 document (claim and example),
  discarded from `wraps` (`ir_eval.run_admit`, `WrapsInAdmit`).
- The D19 check is charged to the IR-work meter when a suite is planned (`Work.charge_ir`,
  `ir_eval.admit_check_cost`); the recorded `ir_cost` is unchanged.
- The `synth-v2` fixture is regenerated with generator 0.5.0 (only `generator.version` changed)
  and its BUILD line passes `--expect-generator warrant-corpus-gen@0.5.0`, so it cannot go stale.

### Changed

- Version 0.4.0; `--expect-generator warrant-corpus-gen@0.5.0` in BUILD and BUILD_windows and in
  the README, the test pins and the accept-blind fixture follow the generator's 0.5.0 (D13).
  No emitted byte changes.
