# ARMv7-A / Thumb-2 gate-level simulator

Exact gate-level partner for the mixed-width Thumb-2 machine in Spec 07x/07x2.
The completed topology contains 524,833 clocked D flip-flops: 524,288 memory
bits, 512 register bits with PC stored once in R15, all 32 CPSR bits, and one
halt bit. Program installation metadata is validated lifecycle state rather
than architectural storage.

Repository gates implement mixed-width decode, Boolean operations, condition
predicates, flag logic, barrel shifts/rotates, ripple arithmetic, effective
addresses, and multiplication. The public checked lifecycle and complete state,
trace, result, and fault types are shared with `armv7a-simulator`.

Verification includes four topology/datapath unit tests, six lifecycle suites,
six manual Spec 07x suites, and all 417 Python common-surface transitions with
complete trace/state lockstep against the functional Rust oracle. Strict
formatting, Clippy, and rustdoc pass; package line coverage is 97.41% (716/735).
