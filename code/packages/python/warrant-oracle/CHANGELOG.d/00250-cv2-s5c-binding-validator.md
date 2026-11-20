## Unreleased - CV2-S5c: the reference binding validator and the `v2-binding` tables

No version bump, no emitted byte changes: the six committed `v2-*.json` files and the four v1 category files are
byte-identical, and `v2-binding.json` is not written into `code/specs/fixtures` (S5d commits it).

### Added

- `src/coding_adventures_warrant_oracle/bindings_v2.py`: the **reference binding validator** of VER04r section 7
  ([D43]). Lane-neutral and pure: bytes of an `evidence.bindings`, the `types` and `ops` of a corpus and a `host`
  object (four closed lists: `values` per Warrant value name, `exports`, `discriminators`, `immutable`) in; the
  normalized binding or `BindingInvalid` with one `reason` out. `validate_binding` raises the first violation,
  `violations` returns all of them in the order of the checks (bytes, lines, model).
- The **closed `reason` set** (R20), 39 values, listed in the module docstring and in the package README:
  `too-large invalid-utf8 bom cr control-character syntax unknown-directive duplicate-directive bad-value
  missing-directive level-unknown level-implication values-name values-representation values-missing type-unknown
  type-shape type-missing op-unknown op-missing op-key export-missing export-unknown args ret width errors-form
  errors-missing errors-unexpected errors-case errors-incomplete discriminator-shared discriminator-bare
  discriminator-unknown err-any-multi panics-form inout-param inout-form fresh`.
- `tests/fixtures/conformance-v2/cases_binding.py`: the `v2-binding` table, 176 cases (30 accepted with the normalized
  form written by hand and compared by equality, 146 refused, every reason at least once). Every case requires `["ir.core"]`. The mandatory
  vectors of section 11: two cases of one op sharing a discriminator, `err_any` on a two-case error type, CR, a BOM and
  invalid UTF-8, and the directive-level aliasing cases (`fresh=immutable` on a mutable `Bytes`, the `inout` forms).
- `make_conformance.py --write` writes `v2-binding.json`; `--check` holds it to the derivation as soon as the file or a
  `v2-binding` row of `index.json` exists and reports it as not committed yet while neither does, so the committed S5b
  family stays green.
- `tests/test_bindings_v2.py` (the validator: order of checks, shapes, schema errors, branches the kit corpora do not
  reach) and `tests/test_conformance_binding.py` (the table, the mandatory vectors, the README list of the reason set,
  `make_conformance.py`, and 39 + 11 mutants: a validator that never reports a reason, a lattice or size cap off, a
  reader that normalizes CRLF or skips a BOM ... each of which changes the derived outcome of a case).

### Review fixes (F1-F10)

- F1: a `seed` is bounded **before** `int()`: `_` and leading zeros (after `0x` for hex) are stripped, then more than 20
  significant decimal digits or more than 16 significant hex digits is `bad-value`. A 5,000-digit seed used to crash
  with Python's 4,300-digit `ValueError`; kit case `bad-value-seed-too-many-digits`. The grammar stays strict
  (single `_` between digit groups, lowercase `0x`, leading zeros allowed: `accept-seed-*-leading-zeros`).
- F2: the package name regex is `[A-Za-z0-9][A-Za-z0-9_.-]*` (`elixir/gf256_native`, `swift/Barcode1D`,
  `csharp/Mosaic.Flux` accepted; `bad-value-package-name-leading-dot`).
- F4: kit cases for a U+2028 and a U+0085 comment, a file with no final LF, an indented comment
  (`syntax`), `values-missing-for-a-tuple-component` and `inout-form-growable-for-an-integer`; mutant tests for each.
- F5: the BOM test is written `"\ufeff"`, not an invisible literal; the new case table has no raw non-ASCII.
- F6: `fresh=immutable` checks the host class of a record or variant result and, for a tuple, list or option, the
  container and every component against `host.immutable` (two reject and two accept cases).
- F7: the four accepted cases that lacked a hand-written `normalized` now have one, and `build_binding` demands every
  accepted case's `normalized` be equal to the derived one (it was a sub-object test).
- F8: `make_conformance.py` says S5d must delete the pending branch.
- F9: the kit-side corpus reader refuses a one-component tuple, `Bytes<0999>` / `Bytes<4097>`, a `Bytes<M>` whose `M` is
  not a `width_params` entry, and `fresh` together with `inout` (kit errors, not vectors).
- F10: this note, and the module docstring's phase-3 order, are corrected.
- Owner decisions: kit case `inout-form-reserved-record-form` is dropped (the validator still refuses `struct:`,
  `slice-of:struct:`, `vec-of:struct:` and `codec:`, pinned by a unit test); declaring a reserved level stays accepted;
  every case keeps `requires` `["ir.core"]`.

### Changed

- `conformance.py`: `ERROR_CODES` gains `"v2-binding": {"BindingInvalid"}` (`CATEGORIES`, the six S5a categories and
  `build_files` are untouched).
- `tests/test_conformance_tables.py`: the scratch `--write` now yields the six files plus `v2-binding.json`.
- README and the fixture README: the validator, the reason set and the pending rule.

### Notes

- Number 00230 is S11a's (merged), 00240 is S5b's; this is 00250.
