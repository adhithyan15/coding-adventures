## 0.5.0 - S3e: v0 rule 2 on examples [D18], a call outside the op's admit is refused [D19]

### Added

- **[D18]** An `example` is evaluated under v0 rule 2 (VER04r 3.3): in a v0 document an example
  whose arguments or pure side wrap (`f(255u8 + 2u8) == 1u8`) fails generation with `W0701`,
  the code a claim's wrap uses; in a v1 document the same example is accepted.
- **[D19]** During enumeration every subject call on the pure trace of every admitted case is
  checked against the callee's op `admit` (`Program::with_ops`, `Program::trace`). The first case
  outside it fails generation with `W0701` naming the claim, the op and the admitted-case ordinal
  (the `<suite>@<i>` address). An example is checked the same way. A call whose op `admit`
  reads a parameter whose argument depends on a subject result cannot be decided and is refused
  statically (`Program::check_admit_calls`), never skipped; a subject-dependent argument for a
  parameter the `admit` does not read (gf256's `power(add(a, b), 2u64)`) is accepted.
- `tests/admit_calls.rs`: the exact `255u8 + 2u8` example in v0 (refused) and v1 (accepted), a
  claim weaker than the op's admit (refused, case 20), the same claim with a matching refinement
  (accepted), an unrefined binder (case 2560), an example outside the admit, a call under an
  untaken branch, a nested subject argument the admit reads (refused) and one it does not (accepted).

### Changed

- Version 0.5.0. **No emitted byte of any accepted document changes**: both rules are refusals.
  The ct-compare fixture, the oracle's `synth-v2` corpus and the accept-blind fixture differ only
  in `generator.version` (D13), and the `--expect-generator` pin and the fixture README hash moved
  with it. No ct-compare op has an `admit`, so its generation does no extra work; a body that calls
  an op with an `admit` is traced per case (measured: 65,536 cases, 0.15 s before and after).
