### Barcode layout v1 parity for Ruby and Swift

- Adds strict Ruby and Swift `barcode-layout-1d-v1` adapters without changing
  the established legacy APIs.
- Runs the complete language-neutral corpus in both packages, including
  hostile fixture envelopes, stable error precedence, ownership, and
  zero-authority resolver checks.
- Makes the Swift Windows build fail closed and records explicit empty
  capability profiles for both portable implementations.
