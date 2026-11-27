## Unreleased - VER04r section 16.23 / R100, R124: the stand-in of an op `ir` does not call is ignored

The kit is unchanged: 782 cases run, 0 failed, the mutants pass, and no committed kit case changes outcome. The
`ct-compare` v1 manifest is regenerated (the runner digest moves). No dependency added.

- `SubjectTable` walked `Program.reached_ops`, but that map holds every op whose signature was *read*, and the corpus
  check reads every op's (R122). So the stand-in of an op the tree never calls was compiled and its signature compared,
  and a stand-in needing a missing level (or with the wrong signature) was a `KitError`, which `run_kit` counts as a
  failure, for a case R100 says is a value. `Program` now records `called_ops` (the ops a compiled tree calls, taken
  branches or not), and only those stand-ins are compiled and compared.
- A called op whose stand-in needs a missing level, or has the wrong signature, is still a `KitError` (R100: such a
  kit row fails the kit build), never `level-missing`.
- Tests: `tests/test_v2_erratum_16_23.py`; `tests/test_v2_review.py` now compiles a call before it compares a stand-in.
