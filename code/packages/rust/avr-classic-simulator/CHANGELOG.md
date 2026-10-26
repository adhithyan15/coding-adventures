# Changelog

- Correct fractional multiply carry to use bit 15 of the unshifted product,
  while zero continues to describe the shifted R1:R0 result.

## 0.1.0

- Add a checked ATmega328P/`avr5` classic AVR integer simulator.
- Model the complete program and data spaces, mapped registers/SREG/SP, stack,
  control flow, exact fetched-byte traces, and typed transactional lifecycle.
- Reject every manual-defined updating-pointer overlap, including `LPM Z+`,
  atomically while preserving legal non-updating and displacement forms.
- Preserve restorable terminal NOP/BREAK post-states while validating every PC
  against the device's 14-bit range, and preflight both return-address bytes
  before CALL, RCALL, or ICALL mutates the candidate stack.
- Validate the completed boundary with 34 release tests, exhaustive
  addition/subtraction flag oracles, bug-sensitive fractional multiply
  carry/zero/unaffected-flag regressions, and all-family terminal post-skip
  target validation with typed atomic rollback, reaching 97.43% package line
  coverage (949/974).
