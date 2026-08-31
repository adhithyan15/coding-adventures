# W29 — Extended-const proposal: arithmetic in constant expressions

## Purpose and how this slice was chosen

Vendoring more of the pinned `WebAssembly/testsuite` corpus
(`28864811cf03bdbf880733786148feaba339582d`) toward
`code/packages/rust/wasm-conformance` surfaced `data.wast` as one of a
handful of files with a real, narrow, previously undiagnosed gap: its own
"Extended constant expressions" and "Combining add, sub, mul and
global.get" sections use `i32.add`/`i32.sub`/`i32.mul` **inside** a data
segment's offset expression —

```wat
(module
  (memory 1)
  (data (i32.add (i32.const 0) (i32.const 42)))
)
```

— which is the WASM **extended-const proposal**: a small, self-contained
widening of what's legal inside a constant expression (global
initializers, and data/element segment offsets), beyond WASM 1.0's
original "exactly one `*.const`/`global.get`/`ref.null`/`ref.func`"
restriction. This is not open-ended: the whole proposal is six opcodes
(`i32.add`/`i32.sub`/`i32.mul`/`i64.add`/`i64.sub`/`i64.mul`), reusing the
ordinary arithmetic opcodes' existing encoding, just legalized in one more
context.

## What already existed, and the one real gap

Three components touch a constant expression in this repo, and only one
of them needed a change:

1. **`wasm-wast-parser` (text → bytecode)**: `module.rs`'s global/data/
   element-offset encoding already calls the SAME general folded-
   instruction encoder (`encode_instr_list`) that ordinary function
   bodies use — there was never a "only one instruction allowed here"
   restriction at the text-parsing layer. `(i32.add (i32.const 0)
   (i32.const 42))` already encoded to correct bytecode
   (`i32.const 0, i32.const 42, i32.add, end`) before this change.
2. **`wasm-module-parser` (binary → bytecode)**: `read_expr`'s init-expr
   scanner already pushes every opcode byte onto the returned buffer
   unconditionally, and only special-cases opcodes that carry immediate
   operand bytes (`i32.const`, `i64.const`, `f32.const`, `f64.const`,
   `global.get`). `i32.add`/`i32.sub`/`i32.mul`/`i64.add`/`i64.sub`/
   `i64.mul` have no immediate, so its existing unconditional
   `_ => { /* continue scanning for end */ }` catch-all already
   round-tripped them correctly. No change needed.
3. **`wasm-execution::evaluate_const_expr` (bytecode → value, at
   instantiation time)**: this was the real, and only, gap. The function
   modeled its running value as a single `Option<WasmValue>`
   "accumulator" — correct only because, before this proposal, a
   constant expression was ALWAYS exactly one value-producing opcode.
   Extended-const's arithmetic ops need **two** operands live at once
   right before they run (both `i32.const`s in the example above), which
   one overwritten variable cannot represent, and the accumulator's
   catch-all (`_ => illegal opcode`) rejected `i32.add`/etc. outright.

## The fix: a real operand stack

`Option<WasmValue>` became `Vec<WasmValue>`. Every existing opcode arm
(`i32.const`/`i64.const`/`f32.const`/`f64.const`/`global.get`/
`v128.const`/`ref.i31`/`ref.null`) now `push`es its produced value instead
of overwriting the accumulator; `end` `pop`s exactly the value that must
remain. Three new arm groups:

- `0x6A`/`0x6B`/`0x6C` (`i32.add`/`i32.sub`/`i32.mul`): pop two i32
  operands (`b` first — it was pushed last, so it's the right-hand
  operand — then `a`), apply wrapping arithmetic, push one i32 result.
- `0x7C`/`0x7D`/`0x7E` (`i64.add`/`i64.sub`/`i64.mul`): the i64
  counterpart, identical shape.
- A too-few-operands pop (an empty stack, or a type mismatch — an i64
  operand where an i32 was needed) is a clean `Err`, never a panic on an
  empty `Vec::pop()`.

For the overwhelming majority of real-world constant expressions —
still a lone `i32.const`, a lone `global.get`, etc. — this is
byte-for-byte the same runtime behavior as before: one push, then `end`
pops it right back off. See `wasm-execution`'s own `evaluate_const_expr`
doc comment and its `test_extended_const_*` test group for the concrete
before/after and the full opcode table.

## Deliberately out of scope

- **`wasm-validator` instruction-level type-checking of constant
  expressions.** This repo's validator has never type-checked the
  CONTENTS of a constant expression (only structural things: memory/table
  index bounds, unique exports, etc. — see that crate's own doc comments).
  `data.wast`'s `assert_invalid` cases that specifically probe const-expr
  type errors (an `i64.const` where an i32 offset is required, `(nop)`,
  two instructions in one offset position, a MUTABLE global's
  `global.get`) grade `NotYetSupported` in `wasm-conformance`, the same
  honestly-reported pre-existing gap every other vendored file with
  const-expr type-error `assert_invalid` cases already has. Extended-const
  doesn't widen this gap — it was already there for the single-instruction
  case (e.g. `(data (i64.const 0))` targeting an i32-offset memory
  already validated "successfully" before this change too).
- **`f32`/`f64` arithmetic, or any other opcode family, inside a constant
  expression.** The real extended-const proposal is exactly the six
  integer add/sub/mul opcodes above — nothing else is part of it, and
  nothing else appears in `data.wast` or its sibling files.
- **`ref.func` as a constant-expression opcode.** Unchanged/pre-existing:
  this repo's `evaluate_const_expr` has never accepted it (see that
  function's own allowed-opcode list); not something extended-const
  touches.

## Real corpus evidence

`data.wast` (vendored alongside this change — see
`wasm-conformance/CHANGELOG.md` for the exact before/after tally) is the
motivating file. Its "Extended constant expressions" section (3 modules:
lone `i32.add`, lone `i32.sub`, lone `i32.mul`) and its "Combining add,
sub, mul and global.get" section (1 module nesting all three plus a
`global.get`) all build and instantiate successfully after this change;
none did before it (each previously trapped instantiation with "illegal
opcode 0x6A/0x6B/0x6C in constant expression").
