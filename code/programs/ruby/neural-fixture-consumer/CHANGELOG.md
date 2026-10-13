# Changelog

## Unreleased

- Fixed: `load_fixture` no longer passes `create_additions: false`. json 3.1
  removed that keyword, so every load raised "unknown keyword:
  create_additions" (X-7). `JSON.parse` never creates additions by
  default, so nothing changes.

- Fixed: a duplicate key in a fixture stopped being rejected. `StrictHash#[]=`
  was the only thing guarding against it, and json 2.21 detects duplicates
  inside the parser -- warning rather than raising, and never assigning the key
  twice, so the guard was never reached. `load_fixture` now passes
  `allow_duplicate_key: false`, which restores the rejection and pins the
  behaviour across the json 3.0 default flip the gem's own warning announces.
  `StrictHash` is kept for any path that builds a hash without the parser.

## 1.0.0

- Add a strict native consumer for the NN03 weighted-neuron fixture.
