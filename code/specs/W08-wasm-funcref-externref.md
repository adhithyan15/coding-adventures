# W08 — `funcref`/`externref` as First-Class Value Types (WASM17)

## Why

`code/specs/W07-wasm-post-mvp-epics.md`'s Epic 4 identified this as the
highest-value-per-effort next slice: four already-vendored conformance
files (`global.wast`, `select.wast`, `br_table.wast`, `call_indirect.wast`)
currently fail to parse entirely because `funcref`/`externref` aren't
real `ValueType`s — they exist today only as the implicit, hardcoded
element type of WASM 1.0's one table. This spec designs the minimal
extension that lets those values move through locals, globals, function
signatures, and the `select`/`ref.null`/`ref.func`/`ref.is_null`/
`table.get`/`table.set` instructions that manipulate them, matching W07's
own explicit exclusion of the harder GC/reference-types pieces
(`br_on_null`/`br_on_cast`, recursive types, `array`).

## Design: reuse the existing uniform-handle representation

`wasm-execution::WasmValue` already has exactly one variant for every
kind of reference this repo's WasmGC slice produces:

```rust
pub enum WasmValue {
    I32(i32), I64(i64), F32(f32), F64(f64),
    Ref(Option<u32>),   // None = ref.null; Some(handle) = a live reference
}
```

`Ref(Option<u32>)` already carries `anyref`, `i31ref`-boxed-as-`I32` (an
`i31ref` is stack-identical to its `I32` payload, per that variant's own
doc comment), and `StructRef` heap handles — the *static* `ValueType`,
not a runtime tag, is what distinguishes them, matching how a real
statically-typed WASM engine works (this repo's `wasm-validator` already
enforces that statically; the runtime trusts it, same as every other
already-implemented instruction family).

**Decision: `funcref` and `externref` reuse `WasmValue::Ref(Option<u32>)`
too, rather than adding new `WasmValue` variants.** For `funcref`, the
wrapped `u32` is a function index into `ctx.func_types`/`ctx.func_bodies`
— not a GC heap handle, but the same "opaque `u32` handle, `None` for
null" shape. For `externref`, the wrapped `u32` is treated as an opaque
host-supplied cookie (see the `ref.extern N` script literal under
`wasm-wast-parser` below) — this repo has no host environment that
produces real external references, so the only externref values that
will ever exist are the testsuite's own `ref.extern N` script literals.

This is a **small, additive** change precisely because it reuses the
existing typed-stack tagging scheme (`REF_TAG = 0x6E` for the round-trip
through `GenericVM`'s typed stack — see `WasmValue::to_typed`'s own doc
comment) rather than inventing a second reference representation. The
alternative (separate `Funcref(Option<u32>)`/`Externref(Option<u32>)`
variants) would require a second `REF_TAG`-style constant and doubles
the match arms in `to_typed`/`from_typed` for no behavioral difference,
since nothing in this repo's runtime needs to distinguish a `funcref`
from an `anyref` from a `structref` at the *value* level — only
`wasm-validator`'s static types need to, and those are tracked
separately already.

## Scope: what this PR adds

### `wasm-types`

Two new `ValueType` variants, matching the real WASM binary encoding
(confirmed against the byte range already reserved by this repo's own
`ValueType::byte_tag`/`encode`, which currently claims `0x7F`/`0x7E`/
`0x7D`/`0x7C`/`0x6E`/`0x6C`/`0x63` — `0x70` and `0x6F` are free):

```rust
Funcref,     // byte_tag() = Some(0x70)
Externref,   // byte_tag() = Some(0x6F)
```

### `wasm-opcodes`

Three new single-byte entries, added to the crate's normal metadata
table exactly like any other MVP opcode (`immediates: &["tableidx"]`
etc. — no special-casing needed, since `table.get`/`table.set`/
`ref.func` each take a single plain LEB128 index immediate). Per this
crate's own existing test comment (`"The gaps (e.g. 0x06–0x0A,
0x12–0x1F, 0x25–0x27) are reserved/unassigned in the MVP"`), `0x25`–
`0x27` is exactly where the reference-types proposal's `table.get`/
`table.set` live — confirming these are genuinely still single-byte
MVP-table gaps, not a new prefix scheme:

| Byte | Name | immediates | pop | push |
|---|---|---|---|---|
| `0x25` | `table.get` | `tableidx` | 1 (i32 index) | 1 (funcref) |
| `0x26` | `table.set` | `tableidx` | 2 (i32 index, funcref) | 0 |
| `0xD2` | `ref.func` | `funcidx` | 0 | 1 (funcref) |

`0xD0` (`ref.null`) and `0xD1` (`ref.is_null`) need **no**
`wasm-opcodes` changes — they're already fully implemented end to end
(see `wasm-execution` and `wasm-validator` below), and like the
existing `0xFB`/`0xFC` GC/misc-prefix opcodes they were never entries in
this crate's generic metadata table to begin with; `wasm-execution`'s
decoder already special-cases `0xD0`'s heap-type-byte immediate outside
that table. Listed here only for completeness, not as new table rows:

| Byte | Name | immediates | pop | push |
|---|---|---|---|---|
| `0xD0` | `ref.null` | `heaptype` (1 byte) | 0 | 1 |
| `0xD1` | `ref.is_null` | — | 1 (any ref) | 1 (i32) |

`ref.null`'s **decode** side is not new: `wasm-execution`'s
`decode_function_body` already special-cases `0xD0` (see its own comment:
*"a single-byte primary opcode... followed by a one-byte heap-type
immediate"*), reads and discards the heap-type byte, and always pushes
`Ref(None)` — that code needs no changes, since the runtime genuinely
doesn't need to distinguish a null funcref from a null externref from a
null anyref, only the validator does.

`ref.null`'s **encode** side is a different story depending on which of
this repo's two independent WASM-emitting crates is meant: the
builder-style `wasm-module-encoder` (used by `ir-to-wasm-compiler`
etc.) already has a `ref_null_none()`-style method emitting `0x0F`
("none", `anyref`'s bottom type) — that crate is untouched by this PR.
**`wasm-wast-parser`** — the *text*-format parser this PR's scope
actually touches, and the one `wasm-conformance` runs against the real
testsuite — has **no `ref.null`/`ref.is_null`/`ref.func` support at all
today** (confirmed: no match on any `ref.` instruction name anywhere in
`module.rs`). This PR adds all three to `wasm-wast-parser` from
scratch, choosing the emitted heap-type byte (`0x70` `func`, `0x6F`
`extern`, `0x0F` bare `ref.null` with no keyword, matching this repo's
existing anyref-null convention) based on the parsed keyword, and adds
a real type rule to `wasm-validator` that reads that byte (unlike
`wasm-execution`'s decoder, the validator needs to know *which* null it
is, to push the right static type).

### `wasm-wast-parser`

- `funcref`/`externref` as value-type keywords wherever `ValueType`
  already parses (params, results, locals, globals) — same dispatch
  point that already recognizes `i32`/`i64`/`f32`/`f64`/`anyref`.
- `ref.null func`/`ref.null extern`/bare `ref.null` (defaulting to the
  existing `0x0F` anyref-null convention) as a genuinely new
  instruction, added from scratch per the finding above.
- `ref.func $x`, `ref.is_null`, `table.get`/`table.set` as ordinary new
  instruction names, following the exact same pattern every opcode
  family in this crate already uses (folded + flat forms both route
  through the shared `get_opcode_by_name` lookup once `wasm-opcodes` has
  the entries).
- **Script-level `ref.extern N` literal** (used only inside `(invoke
  ...)` argument position and `assert_return` expected-value position in
  the vendored corpus — confirmed by fetching and grepping the real
  `global.wast`/`select.wast`/`br_table.wast`/`call_indirect.wast` at
  the pinned commit; it never appears inside a function body). This is
  **not a real WASM instruction** — it's the official testsuite's own
  script-syntax convenience for constructing an externref test value
  from a literal `i32`. Parses into `WasmValue::Ref(Some(n as u32))` at
  the `Action`/`Expected` layer, the same layer that already parses
  `i32.const`/`f64.const`/etc. as script-literal values.

### `wasm-execution`

- `ref.null` (`0xD0`) and `ref.is_null` (`0xD1`) **already have real,
  tested handlers registered** (confirmed: `register_context_opcode`
  calls for both already exist, with unit tests exercising them via
  hand-built `FunctionBody` byte sequences) — this repo's existing GC
  slice already needed them for `nil`/null checks. Nothing to add here;
  they just become *reachable* once `wasm-wast-parser` can emit them
  from text.
- `ref.func F` (`0xD2`) is genuinely new: bounds-check `F` against the
  combined function index space (same check `call`'s handler already
  does), push `WasmValue::Ref(Some(F))`.
- `table.get`/`table.set` (`0x25`/`0x26`) are genuinely new opcode
  handlers, but thin wrappers around the already-existing
  `Table::get`/`Table::set` *methods* (confirmed: both already have
  exactly the right `Option<u32>` signature and full unit test coverage
  as plain Rust methods — today only `call_indirect` and element-segment
  initialization call them directly; no WASM *instruction* reaches them
  yet).

### `wasm-validator`

`type_check.rs` (WASM06) already has real, tested cases for both `0xD0`
(`ref.null`) and `0xD1`(`ref.is_null`) — confirmed by reading the file.
`0xD1` needs no changes (`pop_val` + push `I32` is already exactly
right, and already correctly treats any reference type polymorphically,
matching how `drop` handles any type). `0xD0` currently reads the
heap-type byte only to bounds-check it's present, then unconditionally
pushes `StackType::Unknown` — its own comment explains why (*"reference
subtyping remains outside this validator phase"*). This PR **upgrades**
that one case to push a real static type instead of `Unknown`: `Funcref`
for heap-type byte `0x70`, `Externref` for `0x6F`, `Anyref` for `0x0F`
(the pre-existing convention) — still not full subtyping, just enough
to make `select`/`global.set`/etc.'s existing type-mismatch checks
actually catch a `funcref`-vs-`externref` mixup, which they can't do
today since both currently look like the same `Unknown`.

- `ref.func F`: bounds-check `F` against `ctx.func_types.len()` (same
  pattern `call`'s existing type rule already uses), push `Funcref`.
- `table.get`/`table.set`: pop/push `I32` + `Funcref`, matching the
  memory-instruction-family's existing "require a table to exist" guard
  shape (`ctx.has_memory` has a direct analogue: add `ctx.has_table`,
  computed the same way `has_memory` already is).

### `wasm-conformance`

- `value_matches_expected` (or wherever `assert_return`'s comparison
  lives) needs a `Ref == Ref` arm — likely already falls out for free
  since `WasmValue` already derives `PartialEq`, but verify the
  `ref.extern`-literal parsing round-trips correctly through a real
  `assert_return (invoke "select-externref" (ref.extern 1) (ref.extern
  2) (i32.const 1)) (ref.extern 1)` case (comparing the *handle number*,
  not comparing WASM engine identity, which is the right semantics here
  since this repo has no separate host-object identity to preserve).

## Explicitly out of scope (deferred to a future GC/reference-types slice)

- `br_on_null`/`br_on_non_null`/`br_on_cast`/`br_on_cast_fail` — new
  control-flow shapes (branch based on a dynamic type test), not just
  new opcodes. `W07`'s own Epic 4 already flags these as the harder
  follow-on slice.
- `table.grow`/`table.size`/`table.fill`/`table.copy`/`table.init`/
  `elem.drop` — the bulk-memory-adjacent table operations. `Table`'s own
  `max_size` field is already `#[allow(dead_code)]` (*"table growth is
  not yet enforced against it"*) — `table.grow` would need to actually
  respect it, which this PR doesn't touch.
- Tables of element type other than `funcref` (WASM 1.0 still allows
  exactly one table, always `funcref` — `externref`-typed tables are
  themselves a reference-types-proposal extension this repo's existing
  `TableType`/module-level table handling doesn't model yet).
- Recursive/mutually-recursive type definitions and `funcref`
  sub-typing by signature (`(ref $t)` for a specific function type,
  distinct from bare `funcref`) — out of scope; every `funcref` in this
  slice is the same untyped "reference to *some* function" `ValueType`.

## Verification plan

- Unit tests in each touched crate for the new opcodes/value type,
  following the established per-crate test style (`wasm-wast-parser`'s
  own `tests` module, `wasm-validator`'s `tests/type_check.rs`).
- Vendor `global.wast`, `select.wast`, `br_table.wast`,
  `call_indirect.wast` are **already vendored** (from W05/PR3) but
  currently recorded as "failed to parse" in the baseline — this PR
  should flip them to real, gradable pass/fail entries. A full baseline
  regen + per-file diff against the pre-change baseline is the primary
  correctness signal, same as every WASM04/06/08 PR this session.
- `/security-review` before push, per this repo's standing workflow.
