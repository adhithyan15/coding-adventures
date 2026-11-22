## Unreleased - S10c-1: `ir.fixed` evaluation, the domains of D21/D25/D26, the fault templates and the `fixed-eval-v1` dumps (oracle half)

No version bump, and no emitted byte of an existing fixture changes: `synth-v2`, `accept-blind-v2`,
`omit-example-v2`, the ct-compare v1 and v2 fixtures, `fixed-forms-v1` (document and lowering dump) and
`tuple-forms-v1` verify unchanged. The **fault-evaluation gate stays closed** (`faults.IMPLEMENTED` is
`("ir.core",)`), so ct-compare's `eq_u64/ignore-high-bit` control stays `deferred ir.fixed` (`not-live` in
every lane) until the atomic flip (S10c, D29). The generator follows in its own PR (VER04r section 13,
S10c-2), against the committed dumps below.

### Added

- **The `ir.fixed` evaluator** (`ir_eval._fixed_fn`): all 35 built-ins of Appendix A.1 on Python integers
  with explicit masking. Bit operations on the `w`-bit pattern re-read in the type; shift counts are exact
  `Nat`s compared with `w` before any shift (`shl`/`shr` give 0, `sar` gives `-1` or 0, `bit` gives false),
  `rotl`/`rotr` reduce `n mod w`; `popcount`/`clz`/`ctz` of the pattern (`clz`/`ctz` of 0 are `w`);
  `widen`/`wrap`/`try`/`to_int`/`to_nat`/`max_value`/`min_value`; `checked_*`; `div`/`rem` (floor on
  `Int`/`Nat`, plain on unsigned fixed), `div_trunc`/`rem_trunc` (`None` for `b == 0` and `MIN / -1`),
  `imin`/`imax`; `be_bytes`/`le_bytes`/`from_be`/`from_le` (a runtime width other than the target's is an
  `ir` error, never a truncation); `at`; `unwrap_or` (both operands evaluated). `wraps` counts only
  `add sub mul neg` [D31]; cost is as S10a defined it. `Program.compile_node` compiles a producer node
  without the top-level `Option` check, for dumps and tests.
- **Signed `fx_edge`** [D21]: `wrap_S(E_U(w)) | {-255..-1}`, de-duplicated and ascending (I16 527, I32 559,
  I64 623), the generator's set; the unsigned sets are unchanged. **16-bit `fx_edge` by demotion only; 8-bit
  refused** [D25]. **`nat_shift`** [D26], `{"d":"nat_shift","size":73}`: `0..=64`, then `65 127 128 129 255
  256 65535 65536`, chosen (`domains.refine_counts`, before any demotion, v1 documents) for a `Nat` binder
  that is directly the count operand of `shl shr sar rotl rotr bit` in the claim body; R10: not a count that
  is an expression of the binder, not a `model_call` argument, not a count only in an `admit`.
  `domains.descriptor_size` is the consumer-side check of VER04r 5.1 (sizes recomputed in checked
  arithmetic, 16 levels, every leaf's legality).
- **Emission**: `derive`, `emit-v2` and `verify-v2` accept `ir.fixed` suites (`emit-v2` reports
  `levels_implemented: ["ir.core", "ir.fixed", "ir.tuple"]`); the lowerer no longer marks a unit that needs
  `ir.fixed` unsupported. A v0 document still refuses the built-ins at lowering. `lzw.warrant` evaluates:
  with six `omit_example` lines its bomb examples classify four `value-over-limit` and two
  `not-op-eq-pure`, it needs `max_ir_cost 16384` (not the 8192 the S10 plan guessed) and the 16-bit
  `fx_edge`; a full derive took about 21 seconds.
- **Fault templates and the gate**: the `ir.fixed` templates (`xor-lsb`, `ignore-high-bit`,
  `flip-result-bit`, `drop-last-result-byte`, `truncate-last`, bare and in the `Ok` form) were instantiated
  as data in S10t-1; they are pinned here by exact JSON and evaluated by brute force. `parity_dumps.GATE_OPEN`
  (`ir.core`, `ir.fixed`, `ir.tuple`) is the test-only open gate, reached only through
  `derive(..., implemented=...)`; under it the ct-compare witness is chunk 0 (`a = 0`, `b = 2**63`,
  ordinal 311).
- **`tests/fixtures/fixed-eval-v1/`** (`make_fixed_eval.py`, `BUILD` and `BUILD_windows` run `--check`):
  the generated document, its profile (`budget 2^20`), and the lowering, evaluation, fault, value and domain
  dumps the generator's S10c-2 must reproduce byte for byte (R11: where 8.2 is loose, the dump is
  normative). The value table (`eval_dumps.values_view`) has about 9,800 rows, every built-in at every
  width on edge values; the domain table (`eval_dumps.domain_view`) has the full `fx_edge` and `nat_shift`
  value lists and the descriptors a consumer must reject.
- Tests: `test_fixed_eval` (every built-in against an independent bit-list reference),
  `test_fixed_domains` (R10 end to end, D25, the descriptor check), `test_fixed_faults`, `test_fixed_lzw`,
  `test_fixed_eval_dump` (three-way pins and re-checks of every row of the value table).
- Lessons: a test that emits an adopter becomes a full enumeration when its level is evaluated; a rule
  keyed on the claim body is blind to shifts behind a `model_call`.

### Changed

- **Census pins** (`tests/test_lower.py::CENSUS`, `emit.census`): suites whose evaluation is not blocked
  were `ir.core`, the rest named a level; now only `ir.data` blocks. wasm-leb128 core/fixed 18/24 -> 42/0,
  logic-gates 28/57/data 3 -> 85/0/3, lzw 4/17 -> 21/0, arithmetic 7/11/data 45 -> 18/0/45, x25519 6/2 ->
  8/0, hash-functions 4/31/data 21 -> 35/0/21, zeroize 8/1 -> 9/0. `test_later_adopters_report_not_live_by_level`
  now reads the census (it must not enumerate wasm-leb128).
- The tests that asserted "`ir.fixed` is typed but not evaluated" assert evaluation; `test_domains` pins the
  signed sets of D21.
