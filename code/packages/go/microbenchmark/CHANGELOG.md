# Changelog

## Unreleased

- Bound public `Analyze` input to 10,000 pairs before ratio allocation.
- Add an external Go module consumer check that records the local `replace`
  requirement honestly.
- Add an unsigned capability proposal for monotonic time, standard output, and
  the Windows-only kernel32 performance-counter load/call path.
- Use the Windows high-resolution performance counter for native measurements,
  with exact overflow-checked elapsed-tick conversion and explicit query failures.
- Validate counter arithmetic against an independent big-integer oracle and
  fuzz inputs, and cover the actual Windows API and native array driver.
- Added paired batch collection, ratio analysis, and indexed-array reference command.
