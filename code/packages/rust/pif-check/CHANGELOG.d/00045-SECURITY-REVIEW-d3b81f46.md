## [0.4.1] - 2026-11-24

### Security

- PIF-003d security review: `decode` enforces backward-only operands (R-IDX-2: a record may mention only indices already defined), so cyclic tables are refused (`forward reference`) instead of looping or exhausting memory; every unfolding step spends from one budget for the whole file (`FILE_BUDGET`), and printed text is bounded as a whole; `pif-pp file` runs the same round trip as `check` before printing anything; the profile id and feature words must be plain `[a-z][a-z0-9_.]*` words (the label is always printed quoted), so no control or bidirectional character reaches a terminal; the shadow check is linear (counts of names in scope and the set bits of the loose-variable mask); `pif-pp goal` also prints `goal-digest <sha256 of the binding CTE>` for comparison with the gate's goal digest.
