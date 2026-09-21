## 0.369.0

- Run the four CLR strict-path test targets in CI. `clr_long_branches`, `clr_source_typed`, `clr_strict_flow` and `clr_typed_scalars` (19 tests) were added by the CLR09-CLR16 campaign but never listed in `BUILD`, so they compiled in the check step and asserted nothing on any merge gate.
