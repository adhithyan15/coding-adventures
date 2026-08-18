# W18 — real multi-memory `memarg`: enough of the proposal to pass `memory-multi.wast`

## Purpose

Logged as task #92 during the WASM17/multi-memory (task #85-91) prioritization
scan, and re-scoped this session after tasks #97/#99/#107 landed. `memory.size`/
`memory.grow` already carry a real, decoded, bounds-checked memory index
(task #87/#90) -- but every OTHER memory instruction still hardcodes memory 0:
the entire `memarg`-carrying load/store family (`i32.load`, `i64.store`, etc.,
opcodes `0x28`-`0x3E`), plus `memory.copy`/`memory.fill`/`memory.init` (the
bulk-memory family, `0xFC 0x0A`/`0x0B`/`0x08`). This is a real, admitted gap:
`wasm-validator/src/type_check.rs`'s own doc comment for `ModuleContext.
has_memory` says outright: *"a plain bool, since every memory op hardcodes
memory index 0 and ignores its reserved-byte immediate."*

## Scope: real memarg opcodes, confirmed by direct inspection of `memory-multi.wast`

### Binary encoding (confirmed against the real multi-memory proposal text)

`memarg`'s `align` field is reinterpreted as a bitfield, not widened: bits
0-5 (low 6 bits) still encode `log2(alignment)` exactly as MVP. **Bit 6
(`0x40`)**, previously required to be zero, is now a sentinel -- when set, a
third LEB128 `u32` **memidx** immediately follows the `offset` LEB128. When
clear, memory defaults to 0 and no memidx byte is present at all -- byte-
identical to MVP for the overwhelmingly common single-memory case. `offset`
itself stays a plain LEB128 `u32` (this repo has no memory64 support, so 32
bits is the real ceiling for this spec).

`memory.copy`/`memory.fill`/`memory.init` are a separate mechanism (already
real LEB128 memidx immediates per the bulk-memory proposal, task #94/#95) --
this repo's interpreter already DECODES those correctly but discards them,
hardcoding `get_memory(ctx)` (memory 0) regardless. Same class of bug as
`call_indirect`'s task #107 fix (immediate decoded correctly, silently
dropped downstream), not a new decode gap.

### Real corpus census (fetched, not guessed): `memory-multi.wast` at the pinned SHA `28864811cf03bdbf880733786148feaba339582d`

A small, focused 42-line file, not the full multi-memory conformance suite:

- Declares two memories, `$mem1`/`$mem2`. No `spectest` import, no `shared`
  memory, no threads/atomics, no SIMD.
- Exercises exactly **one** memarg opcode with an explicit, non-default
  memory: `i32.load`, in the text form `(i32.load $mem1 (i32.const 1))` /
  `(i32.load $mem2 (i32.const 1))` -- a **leading identifier token**
  immediately after the opcode mnemonic, not an `offset=`/`align=`
  attribute combined with a memory reference. No `offset=`/`align=` +
  explicit-memidx combination appears anywhere in this file.
- Also exercises `memory.init $mem1 $d ...` and `memory.fill $mem1 ...`
  (bulk-memory, already-decoded-but-discarded memidx per above) with a
  leading memidx token in the SAME shape as `memory.size`/`memory.grow`'s
  own already-working precedent.
- No binary-quoted (`(module binary ...)`) content -- the file is pure text
  form, so this spec's binary encoding work is validated by round-tripping
  whatever the text encoder emits, not by anything pre-encoded in the file
  itself.

### What gets real binary support vs. what gets real text-form + test coverage

The binary decode/encode (flags-bit `0x40` + memidx) is built generally
across **all 23** memarg opcodes (`0x28`-`0x3E`) -- one shared decode/execute
path per opcode family, cheap to do uniformly, and future multi-memory
`.wast` files beyond this one may exercise other opcodes. The `wasm-wast-
parser` TEXT-form work and the test coverage THIS vendoring pass needs to
pass are scoped down to what `memory-multi.wast` actually exercises:
`i32.load`'s leading-memidx-token form, plus `memory.init`/`memory.fill`'s
existing memidx wiring -- the same "spec allows N modes, corpus only needs a
subset" narrowing this session already applied to W17's bulk-table element-
segment modes (8 modes in the spec, 4 needed by the real corpus).

## The concrete problem, confirmed by direct inspection

### `wasm-execution::DecodedOperand::MemArg` has no memidx field, and the decoder never checks bit `0x40`

```rust
MemArg { _align: u32, offset: u32 }
```

`_align`'s leading underscore already marks it dead (never read downstream).
The binary decode block reads two plain LEB128s (align, offset) with **no
bit-0x40 check anywhere** -- this is the actual root bug; the decoder
doesn't even look at bit 6, so a real multi-memory-encoded `memarg` would
already misparse the trailing memidx LEB128 as the START of the next
instruction's own bytes if one were ever fed to it.

Becomes:

```rust
MemArg { _align: u32, offset: u32, memidx: u32 }
```

Decoded conditionally: if `align & 0x40 != 0`, mask that bit back out of
`align` and read a third LEB128 into `memidx`; otherwise `memidx = 0`.

**Packing into `Operand::Index`**: given `MAX_MEMORIES = 64`, `memidx` fits
comfortably in the high bits of the packed `usize`, exactly like
`CallIndirect`'s own `((table_idx as usize) << 32) | type_idx` pattern (task
#107) -- NOT the `BrTable`/`Gc`/`V128Const` side-table pattern, since
`offset` (a genuine 32-bit value, no memory64 here) needs the full low 32
bits and `memidx` needs only ~6, so `((memidx as usize) << 32) | (offset as
usize)` loses nothing. A new `unpack_memarg_operand` helper mirrors
`unpack_call_indirect_operand`.

All 23 load/store handlers (`register_memory()`, opcodes `0x28`-`0x3E`)
currently call `get_memory(ctx)` (= `get_memory_at(ctx, 0)`, hardcoded).
Each becomes `get_memory_at(ctx, memidx as usize)`.

### `memory.copy`/`memory.fill`/`memory.init` already decode a memidx-shaped immediate and discard it

Same class of bug as `call_indirect`'s task #107 fix: the bulk-memory
proposal's own memidx immediate is read but never used, and every one of
these three handlers hardcodes `get_memory(ctx)` regardless of what was
actually decoded. Fixed the same way: thread the already-decoded value
through to the real `get_memory_at(ctx, memidx)` call instead of discarding
it.

### `wasm-wast-parser`'s shared load/store encoder arm has zero leading-memidx awareness

The single match arm covering all 23 load/store opcode names calls
`parse_memarg(args, 0)`, which only scans for `offset=`/`align=` atoms
starting at `args[0]` -- it has no concept of a leading `$mem1` identifier at
all. As written today, `(i32.load $mem1 (i32.const 1))` would break
`parse_memarg`'s scan immediately (`$mem1` matches neither prefix) and then
try to `encode_instr_list` on `$mem1` as if it were a nested instruction --
a real, reachable parse failure today, not merely a silent wrong-answer.

The fix splices the same leading-atom check `memory.size`/`memory.grow`'s
own already-working memidx handling uses (`args.first()` as a bare
`SExpr::Atom`, resolved via `resolve_idx(&icx.module.memory_names, expr,
"memory")`) in front of `parse_memarg`'s own call, then sets bit `0x40` on
the encoded align byte and emits the memidx LEB128 only when a non-default
memory was actually named -- byte-identical to MVP when memory 0 is
implicit, matching every other multi-memory-aware encoder this session has
written (`table.grow`'s own optional leading `$t` token, task #98, is the
closest sibling precedent).

### `wasm-validator`'s memarg type-check rule has no memidx bounds check at all

`ModuleContext` currently has no `memory_count` field, only the boolean
`has_memory`. The `0x28`-`0x3E` type-check arms decode align/offset and
check `has_memory` (a module has at least one memory), never a real per-
instruction index against a real count. `memory.size`/`memory.grow`
(`0x3F`/`0x40`) already decode a real memidx into a discarded `_reserved`
binding and do no bounds check either -- the identical gap, just currently
unreachable in practice since nothing in this repo emits a non-zero memidx
there yet.

Fixed by adding `memory_count: u32` to `ModuleContext` (computed the same
way `table_count` already is) and bounds-checking `memidx >= ctx.
memory_count` at both `0x28`-`0x3E` and `0x3F`/`0x40`, mirroring
`table_idx >= ctx.table_count`'s own established shape (task #96/#98/#107).

## Non-goals (explicit, not silent gaps)

- `offset=`/`align=` attributes combined with an explicit memory index in
  the SAME instruction -- zero real-corpus consumer found in
  `memory-multi.wast`; the binary encoding supports it (bit 0x40 is
  orthogonal to the low 6 alignment bits), only the text-form encoder's
  test coverage is scoped to the bare leading-memidx-token shape the real
  corpus actually uses.
- Atomic memory ops (`0xFE`-prefixed, `register_atomics`) sharing the
  identical memory-0-hardcoding gap -- confirmed present, but
  `memory-multi.wast` doesn't exercise atomics with multi-memory; a future
  vendored file that does is its own small follow-up reusing the same
  `get_memory_at`/bounds-check machinery this spec builds.
- SIMD `v128.load`/`v128.store` -- no handlers exist yet at all (unrelated
  gap, out of scope regardless of memidx).
- memory64 (64-bit offsets/memidx-adjacent addressing) -- not a real spec
  dependency of this task; `offset`/`memidx` both stay `u32` throughout.

## Staged commits

1. **This spec-only sign-off PR.**
2. **`wasm-execution`**: `DecodedOperand::MemArg.memidx`, bit-0x40-aware
   decode, `unpack_memarg_operand`, all 23 load/store handlers switched to
   `get_memory_at(ctx, memidx)`, `memory.copy`/`memory.fill`/`memory.init`
   threaded through to their own already-decoded memidx instead of
   discarding it.
3. **`wasm-wast-parser`**: leading-memidx-token parsing spliced into the
   shared load/store encoder arm, scoped test coverage to `i32.load` per
   the real corpus.
4. **`wasm-validator`**: `memory_count` field, bounds-check arms for
   `0x28`-`0x3E` and `0x3F`/`0x40`.
5. **`wasm-conformance`**: vendor `memory-multi.wast`, baseline regen,
   CHANGELOGs, `/security-review`, push, babysit PR -- same workflow every
   prior WASM task this session has followed.

Each stage lands its own PR (or the smallest coherent grouping that keeps
CI green at every commit), per this repo's established multi-PR pattern for
features this size (W09/W10/W12/W13/W16/W17 all preceded their
implementation with exactly this kind of spec-only sign-off first).

## Verification

- `cargo test -p wasm-execution -p wasm-wast-parser -p wasm-validator -p
  wasm-runtime` green at every stage, including new tests for: an
  end-to-end execution test with two memories holding DIFFERENT bytes at
  the same offset, proving `i32.load $mem1`/`$mem1` actually reach the
  named memory (not memory 0), mirroring task #107's own two-table
  `call_indirect` proof-test pattern; a folded-form wast-parser byte-
  encoding test confirming the `0x40` bit and trailing memidx LEB128 are
  emitted correctly (and NOT emitted at all when no memory token is
  present -- backward-compatibility regression guard); an out-of-range
  memidx rejected by both the validator (compile-time) and the
  interpreter's own defensive runtime bounds check, following the TEMP-
  REVERT-CHECK discipline this session has used throughout.
- `cargo run --bin wasm_conformance_report -p wasm-conformance --
  --write-baseline` after vendoring `memory-multi.wast` -- confirm the
  aggregate deltas match exactly what the new file contributes, zero
  regressions elsewhere in the corpus (same before/after diff discipline
  every prior vendoring PR this session has used).
- `cargo clippy --all-targets` clean across every touched + downstream
  crate.
- `/security-review` on the full diff before each push, iterated to
  PASSED -- with particular attention (given this session's own recent
  `table.copy` self-copy aliasing finding) to whether the packed-operand
  bit-shifting for `memidx`/`offset` can overflow or truncate on a
  crafted `memarg`, and to whether the bounds-check ordering leaves any
  window where an out-of-range memidx reaches a raw memory-buffer index
  before being rejected.
