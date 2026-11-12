---
category: Security boundaries
---

# A passing verdict that omits the banned constructs it waived lets consumers read 'excepted' as 'absent'

`ca-capability-analyzer` printed `PASS ... No ambient OS capabilities detected
(pure computation)` for `rust/ct-compare`, which contains two `unsafe` blocks
that a `banned_construct_exceptions` entry waives. The assurance report then
cited `analyzer::no-unsafe-or-ffi` for the same package: a waived construct
and an absent one were indistinguishable in the output (audit finding AF-22).

Fix: the report now carries `excepted_constructs`, prints them on every PASS
(not only `--verbose`), reserves the words "pure computation" for packages with
no excepted construct, and ends every run with a machine-readable
`SUMMARY unsafe_code=present-excepted(2) ...` line
(`AnalysisReport::construct_status`). VER08a section 6.1 makes this a contract.

Do differently: whenever a checker can waive something, its output must say
"present and waived" as a separate state from "absent", and any claim derived
from the checker must read that state. A boolean `is_clean()` is not enough.
