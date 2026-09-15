## 0.106.0 — 2026-08-11 — static real standard-function output

Direct non-overridden `abs` calls over finite literal-only real arithmetic now
use the formatter-free static output path. `sqrt` joins that path when its
finite nonnegative operand has an exactly round-tripping root; inexact roots,
invalid domains, runtime operands, and user overrides continue to fail closed.

