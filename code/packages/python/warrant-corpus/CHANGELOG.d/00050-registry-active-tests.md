## Unreleased

- `test_manifest.py` and `test_cli.py` no longer pin the checked-in consumer registry to `planned` or to a single
  Python row (WD-004b1 flipped it to `active` and added the TypeScript and Go rows). The planned case is built from a
  copy of the registry and the row is found by package. Tests only; `runner.src_sha256` is unchanged.
