# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-012b0, VOS-D061)

- `Analysis` gains `op_decls` (names declared with `op`), `rep` (the cone of `model rep` over ALL its
  parameters, as a `BuildShape`) and `claim_shapes` (per claim: the names it mentions and its calls,
  each with arity, the head of its first argument and whether it sits right of an `==`), the facts
  of `vos-link`'s CD07-R lint.  The build-cone walk is generalised to `constructor_shape`; `build`'s
  behaviour is unchanged.  Five tests.

### Changed (VOS-D081)

- `tests/warrant.rs`: the real-block test expects five safety claims and pins `safety_init`
  (`df7b6df1...`). It also repairs the pin of `dispatch_covers_run` (`fc7eb1b7...`), stale since
  the VOS-D079 change of the `any` block (the test had not been run in that PR).

### Changed (VOS-011 PR)

- Doc comments only: the contract package's non-embedded `warrant/lemmas.part` (VOS02 R1.4) is
  admitted by the same `admit_part`.

### Changed (security review round 1)

- The cone of `build` is a **taint walk**: a mention of the abstract state does not count as an operand of `and`/`bit_and`/`*` with a literal false or zero, in `x - x` or `bit_xor(x, x)`, or as an argument to a model that ignores it (`Taint`, `build_shape`).
- `claim_sha256` hashes each token's raw source slice (a literal keeps its prefix and quotes); digests of words, symbols and numbers are unchanged.  All 15 obligation digests are pinned, with `tests/claim_sha256_reference.py` (independent, stdlib Python) as the cross-check.
- Dev-dependency on `vos-sched-contract` so contract edits select these tests; time test is a 4n/n ratio.

### Added

- VOS-033: the pure half of the VOS document tooling (VOS02 sections 2.3, 2.5, 6.8, 10.1).
  - `contract`: `parse_contract` for `contract.vos` over the new `vos_contract` grammar
    pair; caps (64 KiB, 1,024-byte lines, 64 config rows, 1,024 ceiling rows, 4 tiers),
    word classes (`IDENT`, `OPNAME`, `KEY`), `u64` range, sorted and unique rows,
    version 1, and the canonical re-emit comparison; `Contract::emit` and `digest`.
  - `part`: `admit_part` (1 MiB, 4,096-byte lines, ASCII, only LF as a control byte, no
    marker anywhere in a line) with the marker token grammar, so the writer and the reader
    agree on what a marker is.
  - `document`: `assemble` (the generated document of VOS02 R2, with the `assume none`
    trailer) and `parse_layout` over the new `vos_document` grammar pair (marker order,
    each marker once, tiers sorted and unique, digests recorded and recomputed).
  - `warrant`: `analyze_document` and `claims_of_block` over `warrant-grammar`: every
    claim with its `claim_sha256` (VER29 5.3 definition, golden digest pinned), the
    `assume` rows, `bound` declarations, the field count of `Impl` and the shape of
    `build` for the structural cone rule of VOS02 R6.
  - `digest`, `caps`, `grammar`.
- Tests: 4 suites, about 50 tests; line coverage above 95%.
- `vos.toml` (class P), `required_capabilities.json` (empty), BUILD and BUILD_windows.
