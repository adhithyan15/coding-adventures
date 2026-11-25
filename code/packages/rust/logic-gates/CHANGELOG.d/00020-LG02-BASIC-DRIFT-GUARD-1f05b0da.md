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

### Hardening (review)

- The guard fails closed on empty parses (exact 31-pair sets), parses `BUILD`, `BUILD_linux` and
  `BUILD_windows` and requires identical pins, rejects duplicate controls and block keys, and
  requires the exact corpus claim form. The fixture checks are gated by `FIXTURE_REQUIRED`
  (False until LGE-S1c, which flips it to True in the PR that adds the fixture). The BUILD files
  run the test file directly so a missing file is an error.
