---
category: Rust
---

# Run strict lint on new ownership controls before accepting their native test results

The Windows lifetime repair passed all twelve IOCP/provider tests, but strict
Clippy rejected helper items after its new test module. That native execution
is valid evidence for the tested source, not full lint acceptance. Move the
module after production helpers; preserve every assertion and the original
failed log, then rerun both suites and strict lint on the corrected source.
Do not add a lint suppression merely to turn a validator green.
