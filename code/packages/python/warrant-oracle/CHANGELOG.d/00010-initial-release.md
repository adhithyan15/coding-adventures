## 0.1.0

- Initial independent Warrant oracle (VER04r section 10, slice S4, part 1):
  a stdlib-only `.warrant` source reader that shares no code with the Rust
  generator (`warrant-grammar`) and never reads the generator's IR as truth.
- Reader: layout-aware lexer and recursive-descent parser for the v0 grammar,
  the VER04j v1 additions (tuples, records, variants, `match`, fixed-width
  literals, static type arguments) and the VER04k WX-G `inout` / `ensures`
  syntax; every adopter file parses (ct-compare, gf256, wasm-leb128,
  logic-gates, lzw, arithmetic, x25519). Fail-closed, with the VER04 section 2
  admission limits enforced before allocation.
- Checker and lowering: type checking and an unoptimised, syntax-directed
  lowering to the model IR (VER04r section 10.3 check 1). Constructs above
  `ir.core` raise a named level error (`ir.fixed`, `ir.data`, `ir.seq`) that is
  recorded per model and per suite, never skipped.
- Evaluator: `ir.core` with the single VER04 section 3 wrapping rule for every
  fixed width, exact `Nat`/`Int` bounded to 2^127-1 (`ir-overflow`),
  `MAX_IR_VALUE`, `MAX_IR_DEPTH`, a static `MAX_IR_COST`, short-circuit,
  memoised pure model calls and call counting on the pure trace.
- Domains and streams: every descriptor of VER04r section 5.1 with `size`/`nth`,
  budget demotion, chunking on product ordinals, admission arithmetic,
  `warrant-stream-v1`/`-v2` encoders, chunk digests, suite digests and the slim
  chunk subset (FNV-1a-64). Witness chunks are not implemented yet.
- CLI `warrant-oracle`: `verify-v1` (the ct-compare gate: all three checks
  against the committed v1 corpus, 572,109 cases, plus the oracle's own v2 chunk
  digests), `emit-v2` (the oracle's enumeration digests for any document) and
  `check` (a level census).
