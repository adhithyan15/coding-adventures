## Unreleased -- LG02 drift guard for the basic-gates Warrant document (LGE-S1b)

### Changed

- `BUILD`, `BUILD_linux` and `BUILD_windows` run `code/scripts/logic_gates_basic_drift.py`
  right after `gen_evidence --check`, then the script's own tests
  (`code/scripts/tests/test_logic_gates_basic_drift.py`). The guard fails the build if
  `code/specs/logic-gates-basic.warrant` (new) is not a byte-identical copy of the eleven
  scalar gates' blocks of `logic-gates.warrant`. Once the LGE-S1c fixture exists it also
  checks the fault list, this BUILD's per-fault claim pins and the corpus digest.
- The build tool selects `rust/logic-gates` when the basic document, its fixture directory,
  the drift script or its tests change (`logicGatesBasicRow`).
- The changelog moved to a sharded `CHANGELOG.d/`; the previous `CHANGELOG.md` is shard 00010, unchanged.

No source, test or recorded evidence change.
