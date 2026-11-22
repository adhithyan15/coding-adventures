# 0.2.0 (VER04r CV2-S3b): the `ir.core` evaluator, enumeration, the 18 v1 digests, the census

The second of four S3 pull requests. ct-compare now reproduces **all eighteen v1 digests**
(`inputs_sha256` and `expected_sha256` of the nine suites), with the same `cases`,
`discarded` and `expected_calls` (572,109 cases), in about 3 s of debug-profile time.
`generate` is still a fail-closed stub (S3c emits).

## Added

- `value`, `eval`: an own `ir.core` evaluator over this crate's lowered IR (not the
  `warrant-grammar` AST evaluator, which is used only as a test-time differential): exact
  `Nat`/`Int` inside the 2^127 window, wrapping fixed-width arithmetic counted as `wraps`
  (VER04r 3.3 rule 1: unmemoised, subject-independent nodes on the pure trace), short-circuit
  `and`/`or`/`implies`/`if`, `MAX_IR_VALUE` checked before allocation, `trace` for
  `expected_calls` (4.4) and `static_calls`. A `#[doc(hidden)]` `Sabotage` makes it wrong on
  purpose for the negative controls.
- `domains`: descriptors, labels and sizes over the S2 `resolve_binders`/`Domain` API; the
  VER04r 16.2 descriptors (`nat_count`, signed `fx_edge`, 16-bit `fx_edge`, list of byte
  strings) are refused with code `domain`; descriptor depth 16, 8 binders, product 2^28.
- `stream`: the 2.4 encoders, context prefix, v1/v2 header choice, `warrant-chunks-v1`
  framing, FNV-1a-64 and the slim base set.
- `suite`: product enumeration by `unrank_product`, admission evaluated once per prefix tuple
  (O(1) `first_admitted`, `admit` wraps counted per product ordinal), chunks, per-suite
  `cases`/`discarded`/`expected_calls`/`wraps`/`ir_cost`, suite and v1 whole-suite digests,
  slim set with `SuiteResult::slim_set(witnesses)` as the S3c seam.
- `enumerate`: `enumerate_document`, the full-run work bounds (IR work, 2^32 ordinals), the v0
  guard rule 2 (`wraps > 0` is `W0701`), profile cross-checks (`slim_chunk`, `slim_stride`),
  and the enumeration parity view.
- `census`: per-claim census (first blocker, product, descriptors, 16.2 items) and its report.
- New error codes `W0706`, `W0707`, `domain`; `GenError::internal`.
- Tests: `enumeration`, `enumeration_parity` (the oracle: counts, digests and every chunk line),
  `census`, `limits`; synthetic documents `enum_forms_v0/v1.warrant`; the oracle snapshot
  script `dump_oracle_enumeration.py` (its `--check` runs in the oracle package's BUILD).

## Changed (S3a review gaps)

- L5: a claim that calls no operation is `W0701` (the synthetic v0 parity document's
  `if_context` claim gained a subject call; the oracle snapshot was regenerated).
- L6: `classify` keeps the operand types of the first typing pass of the top-level `==`.
- L8: the profile reader refuses an exact repeat of a `fault` or `slim_chunk` line, an empty
  token, `budget 0` and `slim_stride 0` (VER04r 16.4 item 7).
- `LoweredExample.reference` is no longer an `Option` (lowering already refuses any other
  example); the dead `None` arm of the view is gone.
