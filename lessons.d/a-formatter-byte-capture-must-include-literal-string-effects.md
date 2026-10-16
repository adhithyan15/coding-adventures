---
category: Testing & coverage
---

# A formatter byte capture must include literal string effects

The native25 full compiler run at0b9e8bd0 compiled and finished972P/1F. Its
new exact-source test captured putchar/print_i64 but left print_str writing to
host stdout. The effectful original source visibly emitted P, while its buffer
contained only24 and correctly failed the expectedP24 assertion. Compilation
and all numeric outputs do not establish complete ordered output capture.

Capture print_str UTF8 bytes in the same bounded ordered buffer, with exact
argument arity/type checks. Retain the complete effectful source, expectedP24
and typed main result; never remove the marker or change the oracle to24.
Keep the failed receipt and rerun affected suites after the test-only fix.
