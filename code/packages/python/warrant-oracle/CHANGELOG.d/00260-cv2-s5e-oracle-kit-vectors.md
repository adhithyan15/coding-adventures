## Unreleased - CV2-S5e (oracle PR): kit vectors and derivation fixes of VER04r section 16.18

No version bump, no emitted corpus byte changes, no committed fixture file changes (the regenerated `v2-*.json` and
`index.json` are the S5e fixture PR's, as S5a's tables preceded S5b's files). The 734 cases of S5b derive byte for byte
as before; 48 are added to the tables (782 in all: `v2-ir` 248, `v2-domain` 162, `v2-chunk` 74, `v2-fault` 41).

### Fixed (the spec, R68 to R78, wins over the oracle's former behaviour)

- R70: `subject_op` with an empty fourth element on an op without `width_params` is `ill-typed` (the oracle accepted it).
- R72 / CV2-F17: `descriptor_size` refuses a `result` descriptor whose `errs` is empty, has more than 64 names, repeats
  a name or holds a name outside `[A-Za-z_][A-Za-z0-9_]*` (it required only a list of strings).
- R69: the kit's static pass and `requires_of` decide `level-missing` and `requires` over what `ir` reaches (its
  trees, the `env` types, the `model_call` closure, the ops it calls), types included: an `env` entry of a tuple type or
  of `List<U8>` at `["ir.core"]` is `level-missing`, a model with an `ir.fixed` tag that `ir` does not call is not, and
  `requires_of` no longer counts an op or a model that nothing reaches. `type_levels` names `ir.list` for `List<T>`.
  `unknown-tag` stays whole-document.
- R77: every model of a kit case is compiled as a model, called or not, so a chain of 65 that nothing calls is `depth`
  (the derivation counted frames from the caller and accepted 65, refusing 66).
- R75: the `v2-chunk` row check refuses more than 8 binders and a product over `2^28` as `corpus`.
- R72: the `v2-chunk` row check compares each `result` binder's `errs` with the declared `cases` of its error type, order
  included.

### Added

- `conformance.parse_type(widths=..)` and `op_sig` read a named width of an op row (R70); `unstated`, `reach`,
  `reached_levels`, `order_of`.
- `v2-fault` types a row over an op with `width_params` with each named width unstated and requires a common type
  (R51 unify, R94), not assignability, with what each rewrite replaces. The oracle's own fault path
  (`faults._compile_fault`) still compares nothing: more lenient than R94 requires, left to CV2-F17.
- `tests/fixtures/conformance-v2/kit-width.warrant` and `.profile` (a third kit document, so no S5b byte moves),
  the new table rows, `tests/test_s5e_vectors.py` and the README disposition rows.
