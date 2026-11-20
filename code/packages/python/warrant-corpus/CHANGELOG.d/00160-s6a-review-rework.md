## Unreleased - S6a review rework (CV2-S6a)

The review of the v2 `ir.core` library (entry 00150) failed it; this is the rework.

- **B1, independence.** Five pieces had been adapted from the oracle's kit code. They were deleted and rewritten
  from VER04r sections 2.1 to 2.4, 5.1 to 5.6 and 11 alone: `wtypes.parse_type` (a tokenizer and a reader class),
  the value codec in `values` (one codec object per type), the chunk-row key set, `build_suite`,
  `check_slim_chunks`, `check_chunk_file` (fields split at spaces, no pattern over hostile text), `domains`
  descriptor/type matching (`is_descriptor_for`, an explicit work list), and the `v2-domain` outcome. Entry 00150 and
  the README no longer say more than is true: the library imports nothing from the oracle and was written from the
  spec, but the repository cannot prove independence; the kit is the cross-check.
- **B2** static widths: the fourth `subject_op` element binds `width_params` into the signature
  (`Bytes<N>`); missing, extra, out-of-range, non-integer and any list on an op without `width_params` (empty
  included, R70) are `ill-typed`. The committed `ct-compare-warrant-v2` corpus loads and its 9 suites (572,109
  cases) reproduce every chunk's cases, calls and both digests.
- **B3** a bytes `lit` has static type `Bytes<len>` and keeps the written type `Bytes`; a written `Bytes<N>` or
  `{"type":"Bool"}` literal is `ill-typed`.
- **[D63]** the `list` descriptor and `List<T>` are refused as `ir.list`; the list formula is gone.
- **[D64] / R69 / R77** levels by reach. `Program` no longer refuses a corpus for a model or op of another level:
  such models and ops are classified (`not_live_models`, `not_live_ops`; no manifest row, [D65]); models and ops
  inside the lane's levels are compiled and validated in full whether reached or not; tag-name checks (unknown tag,
  subject in a model body, call order, depth, a chain over 64) apply to every model. `build_suite` passes
  `level-missing` through, and makes it `corpus` when the row's `requires` is inside the lane's levels. The `lzw`,
  `x25519` and `wasm-leb128` corpora's `ir.core` suites (4, 6, 4) run and reproduce their chunk digests.
- **F4 / R71** an overflow is `case` under a guard whose condition calls the subject (branch, right operand), and
  inside any model entered through a call with a subject in an argument; otherwise `corpus`.
- **F5** the `oracle` key set is closed, `subject` is compiled, `expected` must flow into `result_type`.
  **F6** fault rows: key set, rewritten parameter exists and is unique, expression type, no `KeyError` on a missing
  `op`; a named width is plain `Bytes` in a fault (R70); `Nat + 1` may stand for a `Nat` result.
  **F7** chunk files: 2 MiB cap, fields split at spaces, digit runs bounded, no `ValueError` escapes.
  **F9** `model_order` is a permutation of `models`. **F11** `{"none": true}` only. **F12 / R72** `errs` is 1 to 64
  distinct names equal to the declared cases. **F13** argument arrays are walked positionally. **F14** the
  recursion guard wraps one IR evaluation and only when the nesting needs it, not the chunk loop. **R74/R75** a
  fixed-width `take`/`drop` count is `ill-typed`; a suite has 0 to 8 binders (a ground claim has one empty case).
  **F16** README `STANDARD`. **F17** `coverage>=7.9` in the dev dependencies.
- `runner.src_sha256` moves again; the `python/ct-compare` v1 manifest is regenerated.
