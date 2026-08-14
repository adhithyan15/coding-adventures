# W06 — multi-value block signatures for `block`/`loop`/`if`

> Status: draft — no code. Closes a gap found running the real WebAssembly
> spec testsuite (`wasm-conformance`, W05): `block.wast`, `loop.wast`,
> `if.wast`, and `fac.wast` all fail to even **parse**, because this repo's
> WASM text parser and interpreter only ever understood the WASM 1.0 MVP's
> single-byte `blocktype` shorthand (`0x40` = no result, or one value-type
> byte = one result) — never the multi-value extension's alternative
> encoding, a signed LEB128 index into the module's type section, needed
> the moment a block/loop/if header declares more than one result, or any
> params at all (`(block (param i32 i32) (result i32) ...)`).

## 1. Why this matters, precisely

Multi-value blocks are not an obscure proposal-track feature this repo can
defer — `wat2wasm`/every real toolchain and the official spec testsuite
itself uses `(param ...)`/multi-`(result ...)` block headers routinely
(loop induction variables carried as loop params is the single most common
shape: `fac.wast`'s `fac-ssa` uses `(loop $l (param i64 i64) (result
i64) ...)` for exactly this). Four vendored conformance files are blocked
on this gap alone, unrelated to any other missing feature:

- `block.wast` — param-only and param+multi-result blocks
- `loop.wast` — same, plus `break-multi-value`, the definitive test for
  the param/result branch-target asymmetry (§4)
- `if.wast` — `(if (param i32) (result i32) ... (then ...) (else ...))`
- `fac.wast` — a `loop` header with two i64 params

(`select.wast` also currently fails to parse, but for two *unrelated*
reasons — typed `select` and `funcref`/`externref` value types — not this
gap. Out of scope here; noted so nobody double-counts it as fixed by this
work.)

## 2. What already exists, and the three gaps that shape this design

This is not a green-field feature. Investigation (grounded in reading the
actual current code, not assumption) found the pieces are mostly already
in place, with three specific, well-understood gaps:

### 2.1 The encoder gap (`wasm-wast-parser`)

`encode_structured_instr` (folded `(block ...)`/`(loop ...)`/`(if ...)`)
and `encode_stream_structured_instr` (flat `block ... end`) both compute
the blocktype byte identically:

```rust
let blocktype_byte: Vec<u8> = if let Some(r) = args.get(i).filter(|a| a.is_keyword_list("result")) {
    let items = r.as_list().unwrap();
    i += 1;
    if items.len() == 2 { vec![parse_value_type(&items[1])?.byte_tag().unwrap()] } else { vec![0x40] }
} else {
    vec![0x40]
};
```

Two bugs, same root cause — there is no `(param ...)` branch at all, and
only a *single* result value is ever read:

- A leading `(param i32)` fails the `is_keyword_list("result")` check, `i`
  is never advanced past it, and it's left for the body-encoding call
  right after — which then tries to encode it as an *instruction* named
  `"param"`, and fails with `UnknownInstruction`. This is the literal,
  confirmed cause of `block.wast`'s "unknown instruction \"param\"" parse
  failure at byte 9226.
- `(result i32 i32)` (multiple results, no params) hits the same failure
  mode one step later — the second `i32` result is left unconsumed and
  mis-routed the same way.

**The fix does not need new parsing machinery.** `parse_func_signature`
(already used for `func`/`type` headers and `call_indirect`'s inline
signature) already parses exactly a `(param ...)* (result ...)*` sequence
into a `FuncType`. `dedup_type` (already used the same way for anonymous
`func`/`call_indirect` signatures) already finds-or-inserts a `FuncType`
into the module's type section and returns its index — which is *exactly*
what a multi-value blocktype needs, since the binary format has no notion
of an "anonymous inline blocktype" separate from a real type-section
entry. The change is: detect a leading `(param ...)` (optionally followed
by `(result ...)`) the same way `call_indirect`'s inline-signature scan
already does, and whenever `!params.is_empty() || results.len() > 1`,
build a `FuncType`, `dedup_type` it, and emit the index as a signed
LEB128 instead of a byte. The existing single-byte shorthand stays as a
(non-required, zero-cost) size optimization for the `params.is_empty() &&
results.len() <= 1` case, which covers the overwhelming majority of real
blocks.

An explicit `(type $t)` blocktype reference (rather than an inline
`(param)`/`(result)` list) is also spec-legal and should resolve via
`icx.module.type_names` + `resolve_idx`, the same lookup `call_indirect`
already uses for its own explicit `(type $t)` form. Not exercised by any
of the four blocked fixtures, but cheap to support alongside the inline
form since it reuses the same lookup machinery.

**Confirmed non-issue, so this doesn't need designing:** a block's
`(param ...)` types are NOT new named/indexed locals — WASM semantics
say they're pure operand-stack typing information for validation.
`InstrCtx` (constructed once per function body, threaded by `&mut`
through arbitrarily deep nested blocks with no locals-scope push/pop
anywhere in the recursion) already reflects this correctly: `local.get`/
`local.set` inside a block body already resolve against the enclosing
function's flat local space, unaffected by any blocktype params. No
change needed here — flagged only so a future reader doesn't go looking
for a bug that isn't there.

### 2.2 The decode gap (`wasm-execution`) — already forward-compatible, no change needed

`decode_operand`'s `"blocktype"` case:

```rust
"blocktype" => {
    let byte = code[offset];
    match byte {
        0x40 | 0x7F | 0x7E | 0x7D | 0x7C => (DecodedOperand::Int(byte as i64), 1),
        _ => {
            let (value, consumed) = decode_signed(code, offset).unwrap_or((0, 1));
            (DecodedOperand::Int(value), consumed)
        }
    }
}
```

Already falls through to decoding a signed LEB128 for any byte that isn't
one of the five known shorthands, storing it in the same
`DecodedOperand::Int` variant either way. Once §2.1's encoder emits a real
SLEB128 type index, this needs **zero changes**.

### 2.3 The execute gap — arity resolution reads the wrong table, and only tracks one arity

Two bugs here, one already latent and one structural:

**Bug A — `block_arity` indexes the wrong table (a `call_indirect`-class
mistake, already fixed once for `call_indirect` itself):**

```rust
fn block_arity(block_type: i64, func_types: &[FuncType]) -> usize {
    match block_type {
        0x40 => 0,
        0x7C..=0x7F => 1,
        n if n >= 0 && (n as usize) < func_types.len() => func_types[n as usize].results.len(),
        _ => 0,
    }
}
```

called as `block_arity(block_type, &ctx.func_types)`. `ctx.func_types` is
indexed by **function** index (one entry per function in the module,
whichever type *that function* happens to declare) — not by type-section
index. `call_indirect`'s handler already had this exact bug and already
carries the fix and the doc comment explaining it:

```rust
// type_idx indexes the module's TYPE SECTION (what the call site declared),
// which is a different index space from func_types (indexed by FUNCTION
// index)... this needs ctx.types (the real type section) specifically.
if let Some(expected) = ctx.types.get(type_idx) {
```

`block_arity` needs the identical fix: read from `&ctx.types` (the real,
deduplicated type section, already populated end-to-end for every real
program via `wasm-runtime::call_engine`'s `set_type_section` call — see
§5 for the one place this ISN'T yet wired, wasm-execution's own
hand-built unit-test engines), not `&ctx.func_types`. Today, in any
module with more than one function type declared, a multi-value block's
arity resolves against an unrelated function's signature (or silently
falls back to 0 via the `_ => 0` arm when the index happens to be out of
`func_types`' range) — a real, if currently unreachable (no multi-value
blocktype is ever emitted yet), correctness bug.

**Bug B — `Label` has only one arity field; loops hardcode param-arity to
0.**

```rust
pub struct Label {
    pub arity: usize,        // "results" — how many values on fall-through/`br` to a block/if
    pub target_pc: usize,
    pub stack_height: usize,
    pub is_loop: bool,
}
```

and in `execute_branch`:

```rust
// For loops, arity is 0 (MVP). For blocks, it's the block's result arity.
let arity = if label.is_loop { 0 } else { label.arity };
```

This `// MVP` comment is exactly right about why it was safe before: WASM
1.0's blocktype could only ever encode 0 or 1 value, and a loop's *branch
target* arity is defined by the spec as its **param** types (branching to
a loop re-enters it, re-consuming its params) — which in the byte-only
encoding was always 0, since there was no way to declare loop params at
all. Multi-value breaks this assumption directly: `loop.wast`'s
`break-multi-value` case, `(loop (param i32 i32 i64) ...)`, needs `br 0`
(targeting the loop) to leave exactly 3 values on the stack, not 0.

`execute_branch`'s existing pop/truncate/push sequence for the *result*
arity case is real, already-correct, arity-aware stack bookkeeping — this
is not "no bookkeeping at all," it's specifically missing the second
(param) arity a loop target needs.

## 3. `code/specs/W02-wasm-validator.md` already designed the right abstraction

W02's (unimplemented) type-checker design already anticipates exactly
this, years before this gap was found — its `ControlFrame` already has
both:

```python
class ControlFrame:
    kind: Literal["block", "loop", "if"]
    start_types: tuple[ValueType, ...]   # block/if: params (empty in MVP).
                                          # loop: params (loop targets re-consume them).
    end_types: tuple[ValueType, ...]     # types that must be on the stack when the block ends
    stack_height: int
    unreachable: bool = False
```

and states the exact asymmetry this spec's §4 covers, as prose:

> "This is a key asymmetry: `br` to a `loop` must have the loop's *input*
> types on the stack (because control returns to the loop's start), while
> `br` to a `block` must have the block's *output* types (because control
> exits the block)."

W02 is scoped to the abstract type-checking PASS, not to decoding the
binary `blocktype` immediate into `start_types`/`end_types` in the first
place — that's this crate-layering's job (W02 depends on
`wasm-types`/`wasm-opcodes`/`wasm-module-parser`, never the reverse). This
spec is the encoder/decoder/interpreter-side counterpart W02 was always
missing; W06's `(start_arity, end_arity)` pair is the direct runtime
analogue of W02's `(start_types, end_types)`, chosen deliberately to make
a future W02 implementation's job easier, not to invent a competing
vocabulary.

**Directly reusable, currently-dead code**: `wasm-types::BlockType`
already exists —

```rust
pub enum BlockType {
    Empty,
    Value(ValueType),
    TypeIndex(u32),
}
```

with a doc comment describing the exact byte encoding this spec needs,
including the multi-value `TypeIndex` case. It is currently unused
anywhere in the codebase (grepped — only its own crate's source/README/
CHANGELOG reference it). §6 below decides whether to finally wire it
through, or keep passing blocktypes as raw `i64`/`DecodedOperand::Int`
for consistency with every other index-y operand in `wasm-execution`
today.

## 4. The branch-target asymmetry, precisely

```text
frame kind    a `br N` targeting it jumps to...    what must be on the stack
──────────    ─────────────────────────────────    ──────────────────────────
block         the block's END (falls out)          the block's RESULT values
if            the if's END (falls out)              the if's RESULT values
loop          the loop's START (re-enters)          the loop's PARAM values
```

Concretely, from `loop.wast`'s `break-multi-value` (the single test case
that exercises both halves of this asymmetry in one function):

```wat
(func (export "break-multi-value") (result i32 i32 i64)
  (block (result i32 i32 i64)              ;; depth 1 here — 3 RESULT values
    (i32.const 0) (i32.const 0) (i64.const 0)
    (loop (param i32 i32 i64)              ;; depth 0 here — 3 PARAM values
      (block (br 2 (i32.const 18) (i32.const -18) (i64.const 18)))
      (br 0 (i32.const 20) (i32.const -20) (i64.const 20))
    )
    (i32.const 19) (i32.const -19) (i64.const 19)
  )
)
```

`br 2` (targeting the outer `block`) must leave the block's 3 **results**
on the stack — that's `label.arity` today, already correct.
`br 0` (targeting the `loop`) must leave the loop's 3 **params** on the
stack — that's the missing `label.param_arity` (or equivalent) this spec
adds.

## 5. Design

### 5.1 `wasm-types::FuncType` reuse, no new type needed for the blocktype's *contents*

A resolved multi-value blocktype's start/end arities are just
`func_type.params.len()`/`func_type.results.len()` on whichever
`FuncType` the blocktype resolves to (inline-and-deduped, or an explicit
`(type $t)` reference) — no new struct needed beyond what `wasm-types`
already has.

### 5.2 Encoder (`wasm-wast-parser`)

In both `encode_structured_instr` and `encode_stream_structured_instr`:

1. After the optional `$label`, scan for `(type $t)` and/or `(param
   ...)`/`(result ...)` forms the same way `call_indirect`'s inline-
   signature scan already does (`is_type_or_param_or_result` +
   `parse_func_signature`).
2. If nothing matched: emit `0x40` (unchanged behavior).
3. If exactly one `(result T)` and no params/type-ref matched: emit the
   single value-type byte (unchanged behavior — this is the common case,
   kept as the size-optimized encoding the spec explicitly allows).
4. Otherwise (any params, `(type $t)` present, or `results.len() != 1`):
   resolve or `dedup_type` a `FuncType`, emit its index as a signed
   LEB128.

Advance `i`/the following-elements cursor correctly past whatever was
consumed in each case — this is the actual bug being fixed, so the new
test suite (§7) must include a case immediately after a multi-value
header to confirm the body is encoded starting from the right position
(a regression here would silently re-introduce a variant of the current
bug: the block's real first body instruction misread as more blocktype).

### 5.3 Decoder (`wasm-execution`) — no changes (§2.2)

### 5.4 Interpreter (`wasm-execution`)

- Fix `block_arity` to read `&ctx.types` instead of `&ctx.func_types`
  (§2.3 Bug A) — this alone is a real correctness fix independent of
  everything else here, matching the exact class of bug `call_indirect`
  already had fixed.
- Extend arity resolution to return **both** counts. Either widen
  `block_arity`'s signature to `(usize, usize)` (`(param_count,
  result_count)`) or add a sibling function — implementer's choice, but
  keep the single-source-of-truth property `call_indirect`'s fix already
  established (one function, one correct table, no duplicated index-space
  confusion possible).
- Add `param_arity: usize` to `Label`, populated at the same three call
  sites that push a `Label` today (`block`/`loop`/`if` handlers).
- `execute_branch`: change
  `let arity = if label.is_loop { 0 } else { label.arity };`
  to
  `let arity = if label.is_loop { label.param_arity } else { label.arity };`
  — the rest of `execute_branch`'s pop/truncate/push sequence is already
  correct and unchanged by this.

### 5.5 `wasm-types::BlockType` — decision needed, not required for correctness

Wiring the existing (currently dead) `BlockType` enum through
`decode_operand`'s blocktype case and `Label`/`block_arity` would make
the "what kind of blocktype is this" distinction self-documenting in the
type system instead of an `i64` with implicit sentinel meanings (`0x40`,
`0x7C..=0x7F`, "anything else is a type index"). This is a genuine
readability improvement but is **not required** for correctness — the
existing `DecodedOperand::Int` + raw-byte-range-matching pattern already
works and is how every other index-y operand in this interpreter is
represented today (staying consistent with that has its own value).
Recommendation: **defer** — implement with the existing `i64`
representation first (matches existing patterns, smallest diff, easiest
to review), note this as a follow-up cleanup, not block correctness work
on a refactor.

## 6. Explicitly out of scope

- **`select.wast`'s two failures** (typed `select`'s `(result T)`
  immediate; `funcref`/`externref` value types) — confirmed unrelated to
  this gap, tracked separately.
- **W02's actual type-checker implementation** — this spec makes W02's
  eventual implementation easier (shared `(start, end)` arity vocabulary)
  but does not implement it. `wasm-validator` remains structural-only
  after this work; `assert_invalid` grading is unaffected.
- **Validating blocktype-vs-actual-stack-contents at parse/instantiate
  time** — this repo's text parser and interpreter don't type-check
  blocktypes against what's actually pushed before them (that's W02's
  job, unimplemented). This spec only makes the ENCODING and BRANCH-
  ARITY-BOOKKEEPING correct for well-formed input, matching this crate's
  existing division of responsibility (documented throughout
  `wasm-wast-parser`: malformed-but-structurally-parseable input is
  `wasm-validator`'s problem, not this parser's).

## 7. Verification

- `wasm-wast-parser`: new unit tests for both folded and flat multi-value
  block/loop/if headers — param-only, multi-result-only, and
  param+multi-result combinations, for all three of `block`/`loop`/`if`
  (mirroring `call_indirect`'s existing inline-signature test coverage);
  a test confirming the body-position bug class described in §5.2 doesn't
  regress (first real body instruction after a multi-value header decodes
  correctly, not swallowed as more blocktype).
- `wasm-execution`: unit tests for the `block_arity` `ctx.types` vs
  `ctx.func_types` fix (a module with 2+ distinct function types, a
  multi-value block whose type-index would resolve to the WRONG arity if
  `func_types` were used); unit tests for `Label.param_arity` /
  `execute_branch`'s loop-branch fix, directly reproducing `loop.wast`'s
  `break-multi-value` shape end-to-end through the real interpreter.
- Regenerate the `wasm-conformance` baseline once `block.wast`/
  `loop.wast`/`if.wast`/`fac.wast` parse and run; verify via a full
  per-file diff against the previous baseline that no other file's tally
  regresses (matching this whole arc's established verification
  discipline).
- `cargo clippy` clean across `wasm-wast-parser`/`wasm-execution`.

## 8. Staged commits

Following this arc's established pattern (see W05 §6 / its own 4-PR
sequence):

1. **This spec-only sign-off PR** — no code.
2. **Encoder fix** (`wasm-wast-parser`) — §5.2, with its own regression
   tests; does not by itself unblock any conformance file yet if pushed
   alone (the interpreter-side arity fix is also required for the
   *execution* directives, `assert_return`, in these files to pass, only
   `module`/parse-level directives would improve) — implementer's call
   whether to land encoder+interpreter together in one PR (simpler
   review story, avoids a half-fixed intermediate baseline state) or
   split further; given the WASM02 precedent in this same arc (an
   apparently-small parser fix that turned out to require a paired
   interpreter-side fix to avoid landing a regression), landing them
   together in one PR is the safer default here too.
3. **Baseline regeneration**, folded into the same PR as #2 per the
   above.
