# Changelog

## 0.1.0

- Add a pure caller-ticked reducer for 64 button levels and 16 absolute axes.
- Preserve stable Rust v1 validation order, fixed-width semantic errors, and
  full-width BigInt masks and ticks.
- Return fresh frozen frame, axis, result, and error records without mutating
  caller input.
- Consume the shared 21-case fixture and add exhaustive domain, property,
  fuzz, and bounded SAT checks.
