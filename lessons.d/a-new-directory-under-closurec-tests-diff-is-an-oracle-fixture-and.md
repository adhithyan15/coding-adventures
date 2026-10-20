---
category: Compiler / VM / language pipeline
---

# A new directory under closurec tests/diff is an oracle fixture and must be classified in the manifest

While fixing #16008 I put a closurec regression test's input files under
`tests/diff/for-head-shadow/`, like the neighbouring fixtures.
`tests/oracle_manifest.rs` then failed with
`fixture is missing from the manifest: for-head-shadow`.
`discover_fixture_directories` treats every directory under `tests/diff/` as
an oracle fixture, and `tests/oracle/manifest.json` has to classify each one:
an upstream golden, an upstream refusal, a local extension or a local
contract. The test passing on its own proved nothing; only the whole
`cargo test -p coding-adventures-closurec` run caught it.

The fix depends on whether the expected bytes came from the pinned upstream
jar:

- If they did, the fixture belongs in `tests/diff/` (or the ladder), with a
  manifest entry.
- If they are closurec's own output, as for a miscompile regression test with
  no oracle capture, put the inputs outside `tests/diff/` (for example
  `tests/regression/<name>/`). That way the corpus never claims upstream
  provenance it doesn't have.

Before pushing any closurec test change, run the whole closurec suite, not
just the new test target.
