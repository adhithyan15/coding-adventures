# W17 — bulk-table ops: enough of the proposal to pass `table_init.wast`/`table_copy.wast`

## Purpose

Logged as task #97 during the post-task-#98 (`table.grow`/`table.size`/
`table.fill`) prioritization scan. `table.init` (`0xFC 0x0C`), `table.copy`
(`0xFC 0x0E`), and `elem.drop` (`0xFC 0x0D`) are entirely unimplemented --
zero opcode decoding, zero interpreter handler, zero validator type-check
rule, zero `wasm-wast-parser` text-form support -- and `wasm_types::Element`
(the element-segment struct, parallel to `DataSegment`) has no `is_passive`
field or equivalent, unlike `DataSegment`, which task #95 already gave a
real 3-mode passive/active design. `wasm-module-parser`'s
`parse_element_section` and `wasm-module-encoder`'s `encode_element` only
ever handle the single oldest binary shape (implicit table 0, active,
funcidx-list) -- structurally earlier than data segments were
pre-task-#95, since they never even read a mode-flags byte at all.

## Scope: narrower than the full bulk-table + reference-types element-segment design, confirmed by direct inspection of the two files that need it

The real spec defines **eight** binary element-segment modes (flag byte
0-7): four "funcidx-list" modes (0=active-implicit-table-funcref,
1=passive-funcref, 2=active-explicit-table-funcref, 3=declarative-funcref)
and four "exprs-list" modes (4-7, the same four kinds but carrying a real
encoded init-expression per entry -- e.g. `ref.func`/`ref.null` -- instead
of a bare `funcidx` LEB128, and a `reftype` byte instead of a fixed
`elemkind`).

Fetched and directly inspected the pinned-SHA (`28864811cf03bdbf88073378614
8feaba339582d`) `table_init.wast` (2286 lines) and `table_copy.wast` (3082
lines) -- the two files this task needs to vendor. Census, by grep/regex
over both files:

- **Zero** `declare` keyword occurrences in either file -- **declarative
  segments (modes 3/7) are entirely out of scope for this spec.** (Their
  ONE real consumer already found, `table_grow.wast`, was already
  deliberately deferred by task #98 to whenever this gap closes -- see
  that PR's own CHANGELOG entry and `wasm-conformance`'s `fetch_testsuite.
  py` comment.)
- **Zero** bare `(elem func idx idx...)` (mode 1, passive-funcidx-list, no
  `funcref` keyword) occurrences -- not exercised by either file, but
  included anyway below since it's the same underlying representation as
  mode 5 minus the `(ref.func N)` expression wrapper, so the marginal cost
  is near zero and it's a real, spec-legal, easily-hit shape.
- **Zero** active-segment-with-exprs-list (modes 4/6) occurrences -- every
  `(elem (table ...) (offset) ...)` and `(elem (offset) ...)` active
  segment in both files uses the bare `func idx idx...` / bare-identifier-
  list funcidx form, confirmed by regex over every active-segment site in
  both files (180 `(table ...)`-qualified + 11 implicit-table sites, zero
  exceptions). **Modes 4 and 6 are out of scope.**
- **280** occurrences of `(elem funcref (ref.func N) (ref.func N) ...)` --
  passive, exprs-list (mode 5) -- **the one exprs-list mode this spec
  covers**, and only `ref.func` expressions appear (`grep -oE '\(ref\.[a-z
  ]+' ... | sort | uniq -c` over both files: 935 `ref.func`, one unrelated
  `ref.eq` inside an ordinary instruction body, elsewhere). **`ref.null`
  is included anyway** below (same near-zero marginal cost reasoning as
  mode 1) since it's the only other expression shape the spec allows in
  practice for a funcref-typed elem list, and every real-world `.wast`
  corpus this repo has vendored so far uses it liberally elsewhere.

So the modes this spec actually implements are **0, 1, 2, 5** (funcidx-
list active-implicit/passive/active-explicit, plus exprs-list passive,
where "exprs" is restricted to `ref.func`/`ref.null` -- never an
arbitrary constant expression). Modes 3, 4, 6, 7 remain `NotYetSupported`
at the binary-parser level (a clean, explicit `WasmParseError`, not a
silent misparse) -- closing them is future work, most naturally paired
with whatever eventually revisits `table_grow.wast`'s `elem declare`
dependency.

### Representation consequence: `function_indices` becomes `Vec<Option<u32>>`, not `Vec<u32>`

Since mode 5's exprs-list can contain `ref.null` entries (a real,
uninitialized/null table slot -- not merely absent), `Element`'s existing
`function_indices: Vec<u32>` field can't represent that. Widening it to
`Vec<Option<u32>>` (`Some(idx)` for `ref.func idx`/a bare funcidx-list
entry, `None` for `ref.null`) is the smallest change that covers both
representations uniformly -- the SAME `Some`/`None` shape `Table::
elements: Vec<Option<u32>>` (task #96) and `WasmValue::Ref(Option<u32>)`
(task WASM17) already use for exactly this "funcref, nullable" concept
throughout the interpreter, so this isn't a new pattern, just applying an
existing one to element-segment storage.

## The concrete problem, confirmed by direct inspection

### `wasm-types::Element` (lib.rs:682-689) has no passive concept at all

```rust
pub struct Element {
    pub table_index: u32,
    pub offset_expr: Vec<u8>,
    pub function_indices: Vec<u32>,
}
```

Becomes:

```rust
pub struct Element {
    pub table_index: u32,       // unused (0) when is_passive
    pub offset_expr: Vec<u8>,   // unused (empty) when is_passive
    pub function_indices: Vec<Option<u32>>,
    pub is_passive: bool,
}
```
Additive/widening only (existing `Vec<u32>` construction sites need
`Some(..)`-wrapping + `is_passive: false`, same mechanical fixup task #95
needed for `DataSegment.is_passive` across ir-to-wasm-compiler/
iir-to-wasm/wasm-module-parser/wasm-module-encoder/wasm-validator/
wasm-runtime).

### `wasm-module-parser::parse_element_section` (lib.rs:838-855) never reads a mode-flags byte

Unconditionally reads `table_index → offset_expr → func_count → indices`
-- for a real mode-1/3/5 segment (no offset expr in the binary at all)
this would misparse the passive segment's own vec-count/reftype bytes as
an offset expression, corrupting every subsequent byte read in the
section. Needs the same real-flag-then-branch rewrite task #95 gave
`parse_data_section`, but branching on 4 supported modes (+ 4 clean-error
modes) instead of 3.

### `wasm-module-encoder::encode_element` (lib.rs:311-319) hardcodes the old format

Same shape as `encode_data_segment` before task #95 -- needs the matching
flag-byte-then-branch rewrite, is_passive-aware.

### `wasm-runtime::instantiate()` (lib.rs:~1360-1368) unconditionally applies every element segment

No `is_passive` check (unlike the parallel, already-correct data-segment
loop at lib.rs:1350/1377-1380) -- applying a passive segment automatically
at instantiate time would defeat the entire point of `table.init`, same
"applying one automatically would defeat the entire point" reasoning
task #95 already established for `memory.init`. Needs a
`dropped_elements: Vec<bool>` field on `WasmInstance`, threaded through
`build_engine`/`call_engine` exactly like `dropped_data_segments`.

### `table.init`/`table.copy`/`elem.drop` entirely unimplemented

- `wasm-execution`'s `0xFC` dispatch (lib.rs:2853) falls into the
  catch-all `Err("unsupported bulk-memory opcode 0xFC 0x{other:02X}")`
  for `0x0C`/`0x0D`/`0x0E`. Needs new `WasmExecutionContext::
  elements: Vec<Vec<Option<u32>>>` (immutable content, mirrors
  `data_segments`) and `dropped_elements: Vec<bool>` (mutable, persistent
  across calls, mirrors `dropped_data_segments`), plus handlers modeled
  directly on `memory.init`/`data.drop`'s own task #95 shape (stack pop
  order `[dest, src, len]` for `table.init`, matching `memory.init`; a
  dropped segment behaves as length-0 for bounds-checking, matching
  `memory.init`'s own "dropped segment can never be initialized from
  again" rule) and `table.copy` on `table.fill`/`memory.copy`'s shape
  (`[dest, src, len]`, `Table`-to-`Table` `copy_within`-style, overlap-
  safe, zero-length-still-bounds-checked per task #94's lesson).
  `table.copy`'s two table operands (dst/src) are both real decoded
  indices (unlike `memory.copy`'s discarded-to-0 memory operands per W16
  -- this repo already supports `MAX_TABLES` real tables, task #96, so
  hardcoding either side to table 0 would be a real, avoidable
  regression, not a defensible scope cut).
- `wasm-validator`'s `0xFC` dispatch (type_check.rs:708) hits the same
  catch-all. New arms bounds-check `data_idx`/`elem_idx` against
  `ctx.module.elements.len()` (mirroring `memory.init`/`data.drop`'s own
  out-of-bounds checks) and both table operands against `ctx.table_count`
  (mirroring `table.grow`/`table.size`/`table.fill`'s task #98 checks).
- `wasm-wast-parser` needs: (a) real element-segment name tracking
  (`elem_names: HashMap<String, u32>` on `ModuleCtx`, parallel to
  `data_names` -- doesn't exist today, `elem.drop $e` needs it exactly
  like `data.drop $d` needed `data_names`); (b) `build_elem`'s real
  rewrite to detect the passive shape (no offset expr, matching
  `build_data`'s own `is_passive` detection logic) and parse BOTH the
  bare-funcidx-list and the `(ref.func N)`/`(ref.null <type>)` exprs-list
  forms into the same `Vec<Option<u32>>`; (c) new `table.init`/
  `table.copy`/`elem.drop` text-form interception (both flat and folded,
  same `#[inline(never)]`-factored-out shape task #98 established after
  its own stack-frame-bloat regression) with `table.init`'s data-idx-then-
  table-idx immediate order and `table.copy`'s dst-table-then-src-table
  order, per the real binary encoding
  (`table.init <elemidx:u32leb> <tableidx:u32leb>`,
  `table.copy <dst_tableidx:u32leb> <src_tableidx:u32leb>`,
  `elem.drop <elemidx:u32leb>`).

## Non-goals (explicit, not silent gaps)

- Declarative element segments (modes 3/7, `(elem declare ...)`) --
  `table_grow.wast` remains deferred; closing this is its own future
  slice, most naturally paired with whatever finally unblocks it.
- Active-segment exprs-list forms (modes 4/6) -- zero real-corpus
  consumer found across either target file; if a future vendored file
  needs one, extending the already-widened `Vec<Option<u32>>`
  representation to cover it is a small, well-understood follow-up, not
  a redesign.
- Arbitrary constant expressions inside an elem exprs-list (only
  `ref.func`/`ref.null` are parsed) -- matches every real corpus
  consumer found; a segment using anything else fails with a clean,
  explicit parse error, not a silent misparse.
- `table.wast` (task #99) -- separate, already-logged gaps (hex-literal
  table limits, `spectest` import) unrelated to this spec.

## Staged commits

1. **This spec-only sign-off PR.**
2. **`wasm-types`/`wasm-module-parser`/`wasm-module-encoder`**: `Element.
   is_passive` + `Vec<Option<u32>>` widening, real 4-mode (0/1/2/5) binary
   decode/encode with clean errors for 3/4/6/7, mechanical fixup at every
   existing `Element` construction site.
3. **`wasm-wast-parser`**: `elem_names`, `build_elem` passive-detection +
   dual funcidx-list/exprs-list parsing, `table.init`/`table.copy`/
   `elem.drop` text-form encoding (flat + folded, `#[inline(never)]`-
   factored).
4. **`wasm-execution`/`wasm-validator`/`wasm-runtime`**: new `0xFC`
   sub-opcode handlers/type-check arms, `elements`/`dropped_elements`
   context/instance state threaded through instantiate/call, passive-
   segment skip in `instantiate()`.
5. **`wasm-conformance`**: vendor `table_init.wast`/`table_copy.wast`,
   baseline regen, CHANGELOGs, `/security-review`, push, babysit PR --
   same workflow every prior WASM task in this session has followed.

Each stage lands its own PR (or the smallest coherent grouping that keeps
CI green at every commit), per this repo's own established multi-PR
pattern for features this size (W09/W10/W12/W13/W16 all preceded their
implementation with exactly this kind of spec-only sign-off first).

## Verification

- `cargo test -p wasm-types -p wasm-module-parser -p wasm-module-encoder
  -p wasm-wast-parser -p wasm-execution -p wasm-validator -p wasm-runtime`
  green at every stage, including new tests for: each of the 4 supported
  binary modes round-tripping through parser+encoder; `table.init`
  copying from a passive segment, including the `ref.null` case (copies a
  real null table slot, not a decode error); a dropped segment behaving
  as length-0 for `table.init` (zero-length still succeeds, nonzero
  traps, mirroring `memory.init`'s own task #95 test); `table.copy`
  between two DIFFERENT real tables (not just the same table, proving
  neither side is hardcoded); out-of-range `elem_idx`/table operands
  rejected by both the validator (compile-time) and the interpreter's own
  defensive runtime check (never trusting a decoded index, task #95's
  established discipline) with a regression test for each, following the
  TEMP-REVERT-CHECK discipline this session has used throughout.
- `cargo run --bin wasm_conformance_report -p wasm-conformance --
  --write-baseline` after vendoring `table_init.wast`/`table_copy.wast`
  -- confirm the aggregate deltas match exactly what the two new files
  contribute (same before/after diff discipline every prior vendoring PR
  this session has used), zero regressions elsewhere in the corpus.
- `cargo clippy --all-targets` clean across every touched + downstream
  crate (nib-wasm-compiler, brainfuck-wasm-compiler, twig-to-wasm).
- `/security-review` on the full diff before each push, iterated to
  PASSED -- with particular attention (given this session's own recent
  findings on `table.grow`/`memory.grow`) to `table.copy`'s bounds
  arithmetic between two independently-sized tables, and to whether the
  new `elements`/`dropped_elements` state needs the same "index out of
  range is always a hard error, dropped-but-valid degrades softly"
  separation task #95's `memory.init` security fix established.
