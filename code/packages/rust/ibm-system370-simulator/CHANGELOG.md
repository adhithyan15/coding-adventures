# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-26

- Add an owned, checked IBM System/370 functional state and transactional
  load, restore, step, run, execute, register, and storage APIs.
- Preserve the modeled IBM System/360 application surface through a private
  checked adapter without exposing predecessor state, trace, or error types.
- Add `MVCL`, `CLCL`, `CLM`, `STCM`, and `ICM`, including long-operation
  overlap, padding, register-progress, masked-byte, and condition-code rules.
- Correct destructive-overlap `MVCL` completion to clear the ignored high byte
  of both address registers while preserving storage, low addresses, and both
  length registers.
- Validate the architecturally designated one-byte storage operand for
  zero-mask `CLM` and `ICM`; zero-mask `STCM` accesses no storage and therefore
  recognizes no protection or addressing exception.
- Cover complete-state transport, inherited differential behavior, every new
  condition-code class, storage boundaries, and typed atomic failures.
- Verify all inherited first-byte classifications across fourteen tests, with
  100% function coverage and 98.79% line coverage (570/577 lines).
