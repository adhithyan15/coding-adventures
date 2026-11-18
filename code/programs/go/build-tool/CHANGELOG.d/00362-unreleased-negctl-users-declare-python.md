## Unreleased — every user of the negative-control checker declares Python (WD-HARN1c)

- `TestWarrantGrammarDeclaresPythonOnBothBuildFronts` is replaced by
  `TestEveryBuildFileUsingTheNegctlCheckerDeclaresPython`. It scans every `BUILD*` file under
  `code/packages` and `code/programs` for `warrant_negctl.py` and requires each one to declare
  `# needs-toolchain: python` for the platform that file serves (`_windows` as windows, `_mac` as
  darwin, anything else as linux). No list is hard-coded: a package migrated later is pinned as soon
  as its BUILD calls the checker. It fails if the scan finds no file at all.
- Covers `rust/warrant-grammar` (as before) plus `rust/lzw`, `rust/wasm-leb128` and
  `rust/hash-functions`, which now call the checker. The `vos-sched-*` packages are not covered yet
  because they do not call it yet; they are picked up automatically when VOS migrates them.
