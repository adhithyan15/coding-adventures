## 0.29.0 — 2026-07-30 — E4d-AL: runtime scalar string locals

Initialized local scalar strings now carry runtime procedure results instead of
being restricted to direct literals. For example, `s := pick(1); if s = 'HI'
then ...; print(s)` lowers through the portable `str_concat`, `str_eq`, and
`print_str` operations.

- Local string reads require definite source-order initialization. Assigning a
  string procedure result copies it with `str_concat(result, "")`, preserving
  immutable snapshot semantics.
- Equality and output accept initialized runtime values; lexical ordering stays
  literal-backed because it still needs the broader runtime comparison slice.
- The cross-backend regression lowers the program through WASM, JVM, textual
  CIL/CLR, BEAM, and LLVM; AOT and generic-JIT tests cover the shared VM chain.

Captured/`own` strings, string arrays, and Unicode-aware BEAM string operations
remain outside this slice.

