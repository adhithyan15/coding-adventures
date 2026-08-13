# Changelog — wasm-wast-parser

## 0.1.0 — 2026-08-12 — initial release (W05 PR-2)

New crate. Parses the WebAssembly text format — both plain `.wat` modules
and the official spec testsuite's `.wast` script dialect — into
`wasm-types::WasmModule` and a sequence of test directives. Phase A of the
`wasm-execution` conformance-harness arc; see
`code/specs/W05-wasm-conformance-harness.md`.

- **`tokenizer`**: S-expression tokenizer — atoms, parens, quoted strings
  (with standard, `\u{XXXX}`, and raw `\XX` hex-byte escapes for embedding
  intentionally-invalid bytes), line comments, and **nestable** block
  comments (`(; a (; b ;) c ;)` is one comment, not two).
- **`sexpr`**: generic S-expression tree the tokenizer's flat stream is
  grouped into — folded instruction syntax (`(i32.add (i32.const 1) ...)`)
  is structurally identical to any other nested list, so there is no
  separate "folded vs. flat" parsing code path at this layer.
- **`numeric`**: WAT numeric literal parsing beyond what `str::parse`
  offers — hex integers, hex floats (`0x1.8p3`, computed bit-exact via
  digit-by-digit mantissa accumulation scaled by an exact power of two,
  not an approximate float parse), `inf`/`nan`, and `nan:0x<payload>` (an
  *exact* NaN bit pattern). `i32`/`i64` literals accept the WAT-permitted
  range union of both the signed and unsigned spelling of the same bit
  pattern (`-1` and `0xffffffff` both denote the identical i32 bits).
- **`module`**: the core — two-pass `(module ...)` parsing (pass 1 collects
  every symbolic name in every index space, imports always occupying the
  lowest indices regardless of textual interleaving with non-import
  definitions per the WAT spec; pass 2 encodes function bodies, globals'
  init expressions, and element/data segments straight to raw WASM
  bytecode). Supports both **folded** and **flat** instruction syntax for
  every MVP opcode with immediates (control flow, local/global access,
  calls, memory load/store, the four `*.const`s) via two structurally
  distinct encoders (`encode_flat_instr` for folded-list operand/immediate
  splitting, `encode_stream_instr`/`encode_stream_structured_instr` for a
  bare-atom instruction consuming however many *following* stream elements
  its own immediates need) — the two forms have different immediate
  ordering rules (folded: instruction's own index/label leads, operand
  sub-expressions trail; flat: operands were already pushed by whatever
  came before in the stream, so only trailing immediate atoms belong to
  this instruction) that a single shared code path could not represent
  correctly; the crate's own tests were written specifically to catch this
  after development first got it backwards for `br`/`call`/`local.set`/
  `local.tee`/`global.set` (immediate-first, not immediate-last).
- **`script`**: `.wast` script-directive parsing — `module`, `register`,
  `invoke`/`get`, `assert_return` (including `nan:canonical`/
  `nan:arithmetic` NaN-class result forms), `assert_trap`,
  `assert_exhaustion`, `assert_invalid`, `assert_malformed`,
  `assert_unlinkable`. A plain `module` directive is built eagerly
  (propagating a real syntax error immediately, since `assert_return`/
  `assert_trap` need an already-valid module to invoke against);
  `assert_invalid`/`assert_malformed`'s module is captured as a **raw,
  unparsed S-expression** instead, since failing to build it is exactly
  what those two directives test for — eagerly building it here would turn
  every legitimate fixture into a hard error aborting the whole script.
  Also supports the `(module binary "...")`/`(module quote "...")` module
  variants, concatenating their string-literal bytes for the caller.
- **Hardening pass** (pre-merge security review): this crate will eventually
  process the official testsuite's `assert_malformed`/`assert_invalid`
  fixtures, which are deliberately adversarial — so every reachable panic
  on malformed-but-syntactically-parseable input was replaced with a clean
  `Result::Err`. Fixed: `parse_i32` overflow on an extreme-magnitude
  negative literal (unary negation panicking in debug builds — switched to
  `wrapping_neg`); `parse_limits` panicking via `.unwrap()` on a
  non-numeric or out-of-`u32`-range limit; folded `br_table` underflowing
  `labels.len() - 1` on an empty label list; multiple `script.rs` directive
  parsers and `module.rs`'s `build()`/`build_elem`/`build_data` indexing
  past the end of a too-short field list (`(register)`, `(export "e")`,
  an empty `elem`/`data` segment, etc.) — all now go through a shared
  `sexpr::expect_get` helper instead of `items[N]`; and unbounded `(...)`
  nesting recursion, now capped by `sexpr::MAX_NESTING_DEPTH` (512) with a
  new `WastParseError::TooDeeplyNested` variant instead of a stack
  overflow. Each fix has a dedicated regression test proving the old code
  path would have panicked.
- 70 unit tests across all five modules, ~95%+ line coverage.
