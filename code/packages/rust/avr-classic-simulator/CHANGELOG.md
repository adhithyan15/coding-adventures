# Changelog

## 0.1.0

- Add a checked ATmega328P/`avr5` classic AVR integer simulator.
- Model the complete program and data spaces, mapped registers/SREG/SP, stack,
  control flow, exact fetched-byte traces, and typed transactional lifecycle.
- Reject every manual-defined updating-pointer overlap, including `LPM Z+`,
  atomically while preserving legal non-updating and displacement forms.
- Validate the completed boundary with 30 release tests, exhaustive addition/
  subtraction flag oracles, and 97.11% package line coverage (940/968).
