# W09 — Plain Atomic Memory Operations, No Real Concurrency (WASM18)

## Why

`code/specs/W07-wasm-post-mvp-epics.md`'s Epic 2 (Threads and Atomics)
sizes the full threads proposal as **L**, blocked on an architectural
question this repo hasn't answered yet: `wasm-execution`'s `GenericVM` is
a single-threaded tree-walking interpreter, and `memory.atomic.wait`/
`notify` are meaningless without a second real thread able to `notify` a
blocked one (the same architectural question WASM10 — "run WASM on a
dedicated thread with a guaranteed stack" — already set aside as
blocked). But that epic also identifies a genuinely separable,
**already-recommended** partial slice: *"the plain (non-`wait`/`notify`)
atomic ops **are** implementable without real threading (they can just be
regular loads/stores/RMW against the single linear memory, since with one
thread every atomic op is trivially atomic) — a legitimate partial slice
... roughly SIMD-epic-shaped but 5x smaller, and doesn't block on the
threading question at all."* This spec designs exactly that slice.

## Scope: what "plain atomic ops" means here

The WebAssembly threads proposal's `0xFE`-prefixed instruction space (all
byte values verified against the proposal's own binary-encoding table,
confirmed stable/frozen and matching every production implementation)
splits cleanly into three groups:

| Sub-opcode range | Group | This PR |
|---|---|---|
| `0x00`–`0x02` | `memory.atomic.notify`/`wait32`/`wait64` | **Out of scope** — meaningless without real threads |
| `0x03` | `atomic.fence` | **In scope** — a true no-op single-threaded (see below) |
| `0x10`–`0x1D` | atomic load/store family (7 loads + 7 stores, i32/i64 × natural/narrow widths) | **In scope** |
| `0x1E`–`0x4E` | atomic RMW family: `add`/`sub`/`and`/`or`/`xor`/`xchg`/`cmpxchg`, each × 7 width variants (i32, i64, i32-8, i32-16, i64-8, i64-16, i64-32) | **In scope** |

That's **31 in-scope opcodes** (14 load/store + 7×6 RMW/cmpxchg... — see
the exact table in the `wasm-opcodes` section below) against **3
excluded** (`notify`/`wait32`/`wait64`), matching the epic doc's "5x
smaller than SIMD" estimate.

### Why every atomic op is trivially atomic here

`wasm-execution`'s `GenericVM` executes one instruction at a time on one
native thread; nothing else can observe a memory location mid-operation.
So an atomic load/store/RMW/cmpxchg against this repo's single
`LinearMemory` needs **zero synchronization** to satisfy the spec's
atomicity guarantee — it can be implemented as an ordinary read, or an
ordinary read-modify-write sequence with no possibility of a torn or
interleaved observation. This is exactly the epic doc's own reasoning,
not a new decision.

### `atomic.fence` as a true no-op

`atomic.fence` establishes a happens-before ordering between threads.
With one thread, there is nothing to order — it's semantically a no-op
that takes no immediate and touches no memory or stack value. Decoding
and discarding it (matching the existing `0xFC` sub-opcode default arm's
"unknown sub-opcode: no immediates, no stack effect" pattern already used
elsewhere) is correct, not a placeholder.

### The `shared` memory flag: modeled for real, not skipped

The real spec requires atomic instructions to operate only on a memory
declared `shared` (`(memory 1 1 shared)` in text, an extra byte in the
binary limits encoding) — `assert_invalid` cases in the real
`atomic.wast` corpus specifically test that an atomic op on a *non*-shared
memory is rejected. Skipping this would leave those cases graded `Fail`
instead of `Pass` for no real savings (the field is one `bool`, the text
keyword is one `is_some_and` check, the validator guard is one more
`err!` alongside the existing `has_memory` check) — cheap enough to model
for real rather than accept a known, avoidable gap. `shared`-ness has
**no runtime effect** in this repo (single-threaded — there is no second
agent to share memory *with*), so it is purely a static/validation-time
property.

## Scope per crate

### `wasm-types`

`MemoryType` (currently just `{ limits: Limits }`) gains a `shared: bool`
field (default `false`, matching every existing non-atomic module). No
new `ValueType` variants — every atomic op operates on plain `I32`/`I64`.

### `wasm-opcodes`

31 new entries under the `0xFE` prefix, following the exact pattern this
crate already uses for the `0xFC`/`0xFB` prefixes: **not** added to the
flat `OPCODES` table (which is single-byte-keyed and has no room for a
prefix byte), but decoded via the same "read prefix, read sub-opcode,
dispatch" shape `wasm-execution`'s decoder and `wasm-validator`'s type
checker already use for `0xFB`/`0xFC`. A small `atomic_op_name(sub: u8)
-> Option<&'static str>` (or equivalent lookup) belongs here as the one
shared source of truth both consumers dispatch through, mirroring how
`memory_op_shape` in `wasm-validator` and the plain load/store handlers
in `wasm-execution` already share opcode *semantics* without needing a
literal shared function — the important invariant is that the sub-opcode
→ name → (value type, natural alignment) mapping is defined exactly once
and both crates key off the same byte values, not that it lives in one
particular file.

| Sub-opcode | Name | Natural align | Value type |
|---|---|---|---|
| `0x03` | `atomic.fence` | — | — (no operand) |
| `0x10` | `i32.atomic.load` | 4 | I32 |
| `0x11` | `i64.atomic.load` | 8 | I64 |
| `0x12` | `i32.atomic.load8_u` | 1 | I32 |
| `0x13` | `i32.atomic.load16_u` | 2 | I32 |
| `0x14` | `i64.atomic.load8_u` | 1 | I64 |
| `0x15` | `i64.atomic.load16_u` | 2 | I64 |
| `0x16` | `i64.atomic.load32_u` | 4 | I64 |
| `0x17` | `i32.atomic.store` | 4 | I32 |
| `0x18` | `i64.atomic.store` | 8 | I64 |
| `0x19` | `i32.atomic.store8` | 1 | I32 |
| `0x1A` | `i32.atomic.store16` | 2 | I32 |
| `0x1B` | `i64.atomic.store8` | 1 | I64 |
| `0x1C` | `i64.atomic.store16` | 2 | I64 |
| `0x1D` | `i64.atomic.store32` | 4 | I64 |
| `0x1E`–`0x24` | `{i32,i64,i32.rmw8,i32.rmw16,i64.rmw8,i64.rmw16,i64.rmw32}.atomic.rmw*.add[_u]` | 4/8/1/2/1/2/4 | I32/I64 pattern |
| `0x25`–`0x2B` | same 7-slot shape, `sub` | | |
| `0x2C`–`0x32` | same 7-slot shape, `and` | | |
| `0x33`–`0x39` | same 7-slot shape, `or` | | |
| `0x3A`–`0x40` | same 7-slot shape, `xor` | | |
| `0x41`–`0x47` | same 7-slot shape, `xchg` | | |
| `0x48`–`0x4E` | same 7-slot shape, `cmpxchg` (two value operands, not one) | | |

Every RMW/cmpxchg 7-slot block is internally ordered `i32, i64, i32-8,
i32-16, i64-8, i64-16, i64-32`, identical to the load/store family's own
ordering above it. `memory.atomic.notify`/`wait32`/`wait64` (`0x00`–
`0x02`) are deliberately **not** given entries — see "Explicitly out of
scope."

### `wasm-wast-parser`

- `shared` as an optional trailing keyword in a memory's limits list
  (`(memory 1 1 shared)`), parsed alongside the existing `parse_limits`
  digit-scanning pass — the same "scan known trailing tokens, ignore the
  rest" shape `build_table_limits_and_elements` already uses for a
  table's trailing `reftype` keyword.
- All 31 in-scope atomic instruction names as ordinary new instructions
  in both folded and flat form, reusing the *exact* `memarg` (align +
  offset) decode/encode path the 20 existing MVP load/store instructions
  already use — atomic ops take the identical `memarg` immediate shape,
  just under the `0xFE` prefix instead of a bare single byte. `cmpxchg`'s
  two value operands need no special-casing beyond what `i32.store`-style
  "memarg leads, then whatever operands the folded form recurses through"
  already handles.
- `atomic.fence` as a genuinely no-immediate, no-operand instruction
  (the same shape `nop`/`unreachable` already have).

### `wasm-execution`

- `decode_function_body` grows a `0xFE`-prefixed two-byte decode arm,
  structurally identical to the existing `0xFB`/`0xFC` arms: read the
  sub-opcode byte, decode a `memarg` (for every op except `fence`), bundle
  into a `DecodedOperand` carrying the sub-opcode + memarg.
- Load/store handlers are **thin wrappers** around the load/store methods
  `LinearMemory` already has (`load_i32`, `load_i32_8u`,
  `store_i32_16`, etc. — every narrow-width variant this family needs
  already exists, added for the MVP's own `i32.load8_u`-style
  instructions).
- RMW/cmpxchg handlers are genuinely new: read-modify-write in one
  uninterrupted native-Rust sequence (trivially atomic per this spec's
  own "why" section above) — read the current value at the natural
  width, apply the operator (or compare-and-conditionally-write for
  `cmpxchg`), write back, push the **pre-operation** value (the spec's
  own semantics: every RMW op returns the OLD value, not the new one).
- `atomic.fence`: decode and discard, no stack/memory effect (matching
  the existing "0xFC unknown sub-opcode" no-op pattern).

### `wasm-validator`

- New `0xFE`-prefixed case in `type_check_function`'s main match,
  structurally identical to the existing `0xFB`/`0xFC` cases: decode the
  sub-opcode, look up `(value_type, natural_align)` via the same table
  the `wasm-opcodes` section above defines, and:
  - Require `ctx.has_memory` (reusing the existing plain-memory-ops
    guard).
  - **Require the declared memory to be `shared`** — a new check this PR
    adds, the one real semantic gate atomic ops need beyond ordinary
    load/store (`ctx.module.memories[0].shared`, mirroring how
    `has_memory` itself is computed).
  - Enforce natural alignment exactly (the real spec requires atomic
    accesses to be naturally aligned — stricter than plain loads/stores'
    existing check, which only rejects `align > max_align_for(width)`;
    atomic ops must reject `align != max_align_for(width)`, disallowing
    *under*-alignment too, not just over-alignment).
  - Pop/push types matching `memory_op_shape`'s existing convention
    (loads: pop I32 address, push value; stores: pop I32 address + value,
    push nothing; RMW: pop I32 address + value, push the (old) value;
    cmpxchg: pop I32 address + expected + replacement, push the old
    value).
  - `atomic.fence`: no pops, no pushes, no memory requirement at all
    (it's meaningful even with zero declared memories, though that's a
    degenerate case no real module would hit).

### `wasm-conformance`

- Vendor the real testsuite's `proposals/threads/atomic.wast` (same
  pinned commit, no re-pin) — the file this spec's opcode table and
  `shared`-flag design were verified against. `memory.atomic.notify`/
  `wait32`/`wait64` cases within that same file are graded
  `NotYetSupported`, not `Fail`, matching this crate's existing "graded
  `NotYetSupported` only when a real, named capability gap — not a bug —
  explains it" convention (`assert_invalid`/`assert_unlinkable`'s own
  precedent). `exports.wast`/`imports.wast` (shared-memory import/export
  linking) and `memory.wast` (shared-memory growth semantics) are
  deliberately **not** vendored this PR — see "Explicitly out of scope."

## Explicitly out of scope (deferred to a future real-threading slice)

- `memory.atomic.notify`/`wait32`/`wait64` — meaningless without a
  second real thread of execution; correctly graded `NotYetSupported`
  forever until this repo answers the "does WASM ever run concurrently"
  question W07's Epic 2 and WASM10 both already flag as its own
  architectural decision.
- Multi-agent/shared-memory import and export linking
  (`proposals/threads/exports.wast`/`imports.wast`) — this repo's linking
  model (`wasm-runtime::instantiate`) has no cross-instance shared-memory
  concept at all; adding one is a separate, real feature, not a
  consequence of this PR's opcode work.
- Shared-memory `memory.grow` interaction semantics
  (`proposals/threads/memory.wast`) — real single-agent `memory.grow`
  already exists; the shared-memory-specific edge cases in that file
  (growth visibility across agents) don't apply without real multi-agent
  execution.
- SIMD, tail calls, and every other post-MVP proposal — unrelated,
  already tracked separately in `code/specs/W07-wasm-post-mvp-epics.md`.

## Verification plan

- Unit tests in each touched crate for the new opcode family, following
  the established per-crate test style — particularly: a load/store
  round-trip, one RMW op returning the pre-operation value (not the
  post-operation value, the easiest RMW semantics bug to get backwards),
  a `cmpxchg` success and failure case, `atomic.fence` as a genuine
  no-op, and the `shared`-flag validator guard (both a positive case —
  atomic op on a `shared` memory validates — and a negative case — the
  same op on a non-`shared` memory is rejected).
- Vendor `proposals/threads/atomic.wast`, regenerate the conformance
  baseline, and diff against the pre-change baseline exactly like every
  WASM04/06/08/17/19 PR this session — zero regressions on any
  already-parsing file is the primary correctness signal, plus a real,
  non-zero `assert_return`/`assert_invalid` pass count on the newly
  vendored file (not just "it parses").
- `/security-review` before push, per this repo's standing workflow —
  particular attention to the RMW/cmpxchg handlers' address bounds
  checking (an attacker-controlled `i32` address plus a memarg `offset`
  must not overflow past `LinearMemory`'s real bounds check, the same
  class of check the existing plain load/store handlers already get
  right).
