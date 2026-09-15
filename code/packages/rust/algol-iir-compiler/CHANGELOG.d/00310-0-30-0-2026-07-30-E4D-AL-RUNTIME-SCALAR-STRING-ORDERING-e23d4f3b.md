## 0.30.0 — 2026-07-30 — E4d-AL: runtime scalar string ordering

Initialized scalar strings carrying a branch-selected `string procedure`
result now participate in lexical ordering. `s := pick(1); if s < 'LO' then ...`
lowers through the shared `str_cmp` operation and its signed comparison against
zero, instead of rejecting the string because its contents are not known at
compile time. The direct backend, generic JIT, AOT snapshot, WASM runtime, real
BEAM runtime, and seven-standard-backend matrix all execute `HI < LO`.

The old literal-provenance set is gone: definite source-order initialization is
the relevant safety invariant for scalar string reads, regardless of how the
value was produced.

