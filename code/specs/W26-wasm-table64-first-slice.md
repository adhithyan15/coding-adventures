# W26 — table64 proposal, first slice: 64-bit table addressing (declarations + import linking)

## Purpose and how this slice was chosen

`W25` (memory64 first slice) deliberately deferred two things at the exact
same pinned corpus SHA (`28864811cf03bdbf880733786148feaba339582d`):
`table64` itself, and `memory64-imports.wast` — because roughly two-thirds
of that file is `table64` declarations/imports, "genuinely entangled" with
memory64's own scope. This session re-fetched both files directly (`gh api
.../git/blobs/<sha>`, not inferred from `W25`'s own description) to
independently re-verify that entanglement claim and assess whether
`table64` is now a viable, separable next slice.

**Re-verified, not trusted:** `table64.wast` (757 bytes) is a real,
self-contained corpus file — read in full. Every directive is a bare
`(module (table i64 <min> [<max>] funcref))`-shaped declaration (plus two
`assert_invalid` "size minimum must not be greater than maximum" cases and
one `(module (table (import "spectest" "table64") i64 0 funcref) (table i64
0 funcref))`). **Zero** `table.get`/`table.set`/`table.grow`/`table.size`/
`table.fill`/`table.copy`/`table.init`/`call_indirect` — no actual table
*operations* at all, only declarations, limits, and one import. This is the
exact same "declarations-and-limits-only" shape `memory64.wast` had for
memory in `W25` — table64's own `call_indirect64.wast`/`table_copy64.wast`/
`table_fill64.wast`/`table_get64.wast`/`table_grow64.wast`/
`table_init64.wast`/`table_set64.wast`/`table_size64.wast` (the files that
actually exercise ops on an `is64` table) are a separate, later slice, out
of scope here — deferred for the identical reason `W25` deferred the bulk-
memory-op `*64.wast` files.

`memory64-imports.wast` (6728 bytes) was also re-read in full this session.
Roughly half of it is `table`/`table64` `register`+`import`
`assert_unlinkable` cases (a 32-bit-table export imported as `i64` must
fail, and vice versa; a table import with incompatible `min`/`max` must
fail); the other half is the *same shape* for `memory`/`memory64`, which
`W25` already built (`wasm-runtime::instantiate`'s `ImportTypeInfo::Memory`
arm already checks `imported_mem.is64() != mem_type.is64` before
`limits_compatible`). This repo's `wasm-conformance::RegistryHost` (the
harness's host implementation) resolves `register`ed sibling modules'
*real* exports generically — not a hardcoded `spectest` stub — the same
machinery `linking.wast`/`throw.wast`/etc. already exercise for
functions/globals/tags. Once `TableType`/`Table` gain the same `is64`
plumbing this slice adds, `memory64-imports.wast`'s table-side
`assert_unlinkable` cases need nothing beyond that plumbing plus a
table-import `is64`-mismatch check mirroring the existing memory one.
**Conclusion: this slice deliberately widens scope to also un-defer
`memory64-imports.wast` entirely** (not partially) — both of `W25`'s two
deferred items resolve together, because the thing that entangled them
(`table64` needing to exist at all) is exactly what this slice builds. This
is a stronger outcome than `W25`'s own first slice (which left both
deferred items untouched); it does not, however, touch actual `is64` table
*operations* (`table.get`/`set`/`grow`/`size`/`call_indirect`/bulk-table-64
ops), which stay out of scope — see below.

### Verifying the real spec ceiling for table64 (not assumed from memory64)

`memory64`'s own spec ceiling is `2^48` pages for an `is64` memory (`W25`).
Naively assuming the identical `2^48` *element* ceiling for an `is64`
*table* would be wrong: `table64.wast` itself declares `(module definition
(table i64 0xffff_ffff_ffff_ffff funcref))` — `u64::MAX`, far past `2^48` —
as a plain (non-`assert_invalid`) directive, meaning it must be spec-valid.
Re-verified live against the actual reference interpreter source
(`WebAssembly/spec`, `interpreter/valid/valid.ml`, `check_memorytype` vs.
`check_tabletype`): memory's `i64` address-type ceiling is `0x1_0000_0000_0000`
(`2^48`) pages, but table's `i64` address-type ceiling is
`0xffff_ffff_ffff_ffffL` — literally `u64::MAX`, i.e. no meaningful
structural ceiling beyond fitting in `u64` at all (table's `i32` ceiling is
likewise `0xffff_ffffL` = `u32::MAX`, not memory's `2^16`-page bound) —
tables aren't measured in byte-multiplying "pages" the way memory is, so
the proposal doesn't impose an equivalent artificial cap. (An earlier,
looser reading of the `memory64` proposal repo's own `Overview.md`, which
describes table and memory limits as "classified by their respective
address types" using the same-shaped judgment rule, could be misread as
"same ceiling" — the actual ceiling *constants* the current reference
interpreter enforces differ per-kind, confirmed by reading `valid.ml`
directly rather than trusting the prose summary.)

`(module definition ...)` was also independently re-verified this session
(`WebAssembly/spec`, `interpreter/text/parser.mly` + `interpreter/script/
runner.ml`): plain `(module ...)` desugars to `(module ...) (instance
...)` (validate *and* instantiate); `(module definition ...)` produces only
the `Module` command (validate only, no instantiation attempted) — the
`Module` command handler in `runner.ml` calls the real validator
unconditionally. This repo's own wast parser doesn't special-case the
`definition` keyword at all (it's an inert atom no field-recognizer
matches, confirmed in `wasm-wast-parser::module::parse_module_expr`'s
`skip_while` + `collect_symbols`/`build`'s per-field dispatch), so a
`(module definition (table i64 <huge> funcref))` directive is currently
still run through `wasm-conformance`'s *full* validate-and-instantiate path
in this repo (matching `W25`'s own pre-existing, documented handling of
`memory64.wast`'s analogous case) — this slice's practical instantiation
cap (below) is what keeps that survivable rather than an allocator abort;
it is not a full fix for the `definition`-means-validate-only semantic gap,
which is out of scope here (a latent, pre-existing behavior difference
from the real reference harness, not introduced by this slice, and not
observable in this file's own outcome either way since the huge-`min`
table's actual instantiation attempt hits this slice's own new graceful
cap either way).

## Scope

### In scope

1. **`wasm_types::TableType` gains `pub is64: bool`** (default `false` at
   every existing call site — mechanical, no behavior change for any
   32-bit table), mirroring `MemoryType::is64` from `W25` exactly.
2. **`wasm-module-parser`/`wasm-module-encoder`**: the table section's and
   table import's `parse_limits`/`encode_table_type` call sites stop
   rejecting the `0x04` (64-bit index) flags bit for tables (previously a
   deliberate, named `W25` rejection — "table64 is not supported by this
   parser") and instead wire it into `TableType.is64`, reusing the exact
   same `parse_limits`/`encode_memory_type`-shaped machinery `W25` already
   built for memory (the shared `parse_limits` binary decoder already
   returns `is64` unconditionally; only the table call sites' own "reject
   if `is64`" branches change). `encode_table_type` becomes `is64`-aware
   the same way `encode_memory_type` already is (`u64leb` `min`/`max` when
   `is64`, `u32leb` otherwise) — it has no `shared` bit (tables never
   carry one), so it stays its own function rather than merging with
   `encode_memory_type`.
3. **`wasm-wast-parser`**: text-form `(table i64 <min> [<max>] funcref)`
   (and the inline-import/inline-export desugared forms) recognized — an
   `i64` keyword atom in the same position `(memory i64 ...)` already
   established (`build_table_limits_and_elements`, mirroring
   `build_memory_limits_and_data`; the import-shorthand `build_import_shell`
   "table" arm, mirroring its "memory" arm's `parse_memory_limits`).
   Reuses `numeric::parse_u64`/`parse_limits64` verbatim (already built for
   memory in `W25`, table-agnostic).
4. **`wasm-validator`**:
   - Check 1c (`min <= max` for tables, already `is64`-agnostic/generic
     since it only compares `u64` fields) needs no change.
   - **New**: Check 2b (`table limits <= MAX_TABLE_ELEMENTS`, previously
     applied unconditionally to every table regardless of width) becomes
     `is64`-aware: unchanged for `is64: false` tables (still the existing
     implementation-defined `MAX_TABLE_ELEMENTS` validation-time cap, so no
     32-bit table's validation outcome changes), but an `is64` table is
     checked against the *real* spec ceiling instead (`u64::MAX`, per the
     "Verifying the real spec ceiling" section above) — which no `u64`
     value can ever exceed, so this is a real, live per-item check that
     simply never fires for `is64` (matching `table64.wast`'s own
     `0xffff_ffff_ffff_ffff` boundary case, which must validate). `is64`
     tables are also excluded from the existing cross-table
     `total_table_elements` aggregate (same rationale `W25`'s Check 1b used
     for `is64` memories: the aggregate was calibrated for the 32-bit case
     where the spec ceiling and the safe-allocation ceiling coincide; nothing
     currently mixes an `is64` and a 32-bit table's `min` inside one
     module in the vendored corpus, but the exclusion keeps the invariant
     true generally rather than by accident).
   - **New**: table import linking's `is64` mismatch — mirrors `W25`'s own
     `imported_mem.is64() != mem_type.is64` check exactly, added to
     `wasm-runtime`'s (not `wasm-validator`'s — the existing memory check
     lives at link time in `wasm-runtime::instantiate`, not at static
     validation, since it depends on the *actual* resolved host value, not
     just the module's own declared import type) table-import arm — see
     below.
5. **`wasm-execution`**:
   - `Table` gains `is64: bool` (mirrors `LinearMemory::is64` from `W25`
     exactly — same doc-comment shape, same "always `false` via the plain
     `new` constructor, only a new `is64`-aware constructor can set it").
     `Table::elements`/`max_size` internal storage stays exactly as-is
     (`u32`/`usize`-based, already practically bounded well under any
     `u64` range by `MAX_TABLE_ELEMENTS` — `is64` only changes what the
     *declared* index width is, not how many elements this interpreter
     will actually try to allocate, the identical "declared width vs.
     practical storage" split `W25` established for `LinearMemory`).
   - **New `Table::new_with_is64`** (mirrors `LinearMemory::new_with_is64`
     exactly): if `is64 && initial_size as u64 > MAX_TABLE_ELEMENTS as u64`,
     returns a real `TrapError` instead of eagerly allocating — reusing the
     *existing* `MAX_TABLE_ELEMENTS` constant as the practical `is64`
     instantiation-time cap (the same "reuse the existing 32-bit-shaped
     practical bound as the new is64 practical bound" move `W25` made with
     `MAX_MEMORY64_INITIAL_PAGES` reusing `65536`). This is the "genuinely
     new DoS consideration" `is64` introduces for tables: the real spec
     ceiling (`u64::MAX`) is validation-time-acceptable but has no relation
     to what this interpreter can actually allocate, exactly the shape
     `W25`'s own "Practical allocation cap" section documents for memory.
6. **`wasm-runtime`**:
   - `instantiate()`'s module-declared-table construction (currently
     `Table::new(table_type.limits.min as u32, ...)` — an outright silent
     *truncating* `as u32` cast for any `min` past `u32::MAX`, which an
     `is64` table's spec-valid range now makes reachable for the first time
     since `Limits` was already widened to `u64` in `W25`) switches to
     `Table::new_with_is64`, propagating its `TrapError` through the same
     `Directive::Module`-handles-a-trap-gracefully path `W25` already
     verified for memory.
   - Table-import linking gains the `is64` mismatch check: `if
     imported_table.is64() != table_type.is64 { return
     Err(link_error(...)) }`, checked *before* `limits_compatible` (which
     stays `is64`-agnostic, comparing only the numeric `u64` fields) — the
     exact same ordering/rationale comment `W25`'s memory-import arm
     already carries.
   - **Security review addition** (not in the original draft of this
     spec): a `total_is64_table_elements` aggregate cap across every
     `is64` table in the module, mirroring `total_is64_pages` (memory64).
     `wasm-validator`'s Check 2b deliberately excludes `is64` tables from
     its OWN 32-bit aggregate (an `is64` table's real spec ceiling has no
     useful per-item bound to aggregate from at validation time) — without
     a matching aggregate here, at instantiation, a module could declare
     up to `MAX_TABLES` (64) separate `is64` tables each individually AT
     the per-table `MAX_TABLE_ELEMENTS` cap and still instantiate all of
     them (~5.1GB of eager allocation from one small module). Uses
     `saturating_add`, not `+=`: unlike memory64's `total_is64_pages`
     (whose addends are already capped at a much smaller `2^48`-page
     validator ceiling), an `is64` table's `min` is validator-uncapped up
     to `u64::MAX` itself, so a plain `+=` could wrap the running total
     back under the cap in a release build and silently defeat the check.
     `Table::new_with_is64`'s own per-table cap check was also made
     unconditional (not only under `is64`) for the same class of reason:
     its safety previously depended entirely on an invariant living in
     `wasm-validator`, a different crate.
7. **Vendor `table64.wast` and `memory64-imports.wast` verbatim** (pinned
   SHA `28864811cf03bdbf880733786148feaba339582d`) into `wasm-conformance`,
   add both to `TESTSUITE_FILES`, regenerate the baseline.

### Explicitly out of scope (deferred to a future slice)

- ~~**Actual operations on an `is64` table**~~ — **implemented in a
  follow-up slice, see the "Addendum: real table64 operations" section
  below.** `table.get`/`table.set`/`table.grow`/`table.size`/`table.fill`/
  `table.copy`/`table.init`/`call_indirect` against an `is64`-indexed
  table originally all assumed an `i32` index unconditionally; the
  follow-up widens each per the TARGET table's own `is64`, mirroring
  memory64's `i32`→`i64` address-operand widening pattern, and vendors
  `call_indirect64.wast`, `table_copy64.wast`, `table_fill64.wast`,
  `table_get64.wast`, `table_grow64.wast`, `table_init64.wast`,
  `table_set64.wast`, `table_size64.wast`, `table_copy_mixed.wast` (mixed
  `is64`/32-bit table pairs).
- **SIMD/atomics against a table** — not applicable to tables at all
  (unaffected either way).
- **The `(module definition ...)` validate-only semantic gap** — noted
  above as a genuine, pre-existing (from `W25`) divergence from the real
  reference harness's script semantics; not fixed here (this slice's own
  practical-cap trap makes the one directive that would otherwise be
  affected (`table64.wast`'s `u64::MAX`-`min` case) behave gracefully
  either way, so it doesn't change this slice's own measured outcome, but
  a module that ONLY needs to validate — never actually allocate — under a
  `definition` tag would, in a fully-correct implementation, never reach
  the practical cap at all). Named as a follow-on, not silently ignored.
- **Non-null concrete reference types / `call_ref` / the GC continuation
  epic** — unchanged from `W20`/`W23`/`W24`; not re-litigated here.

## Verification plan

- **Unit tests**:
  - `wasm-types`: `TableType` construction with `is64: true` and a
    `u64`-range `min`/`max` past `u32::MAX`.
  - `wasm-module-parser`/`wasm-module-encoder`: round-trip a table's
    `flags=0x04`/`0x05` limits encoding (previously the module-parser's own
    `test_table_section_is64_rejected` asserted rejection — replaced with a
    real round-trip test now that it's supported); a table import with the
    same flags round-trips too.
  - `wasm-wast-parser`: `(table i64 1 funcref)`, `(table i64 1 2 funcref)`,
    `(table $t i64 1 funcref)` all parse with `is64: true`; a plain
    `(table 1 funcref)` stays `is64: false`; the inline-import shorthand
    (`(table $t (import "m" "n") i64 1 funcref)`) and explicit
    `(import "m" "n" (table i64 1 funcref))` both produce `is64: true`.
  - `wasm-validator`: Check 2b accepts a table with `min` at `u64::MAX` when
    `is64` and rejects nothing new for `is64: false` tables (regression
    guard); the aggregate `total_table_elements` check ignores an `is64`
    table's `min`.
  - `wasm-execution`: `Table::new_with_is64` accepts up to
    `MAX_TABLE_ELEMENTS` and returns a `TrapError` (not a panic) just past
    it, for `is64: true`; matches plain `Table::new` exactly for
    `is64: false`.
  - `wasm-runtime`: an `is64` module-declared table instantiates correctly
    at a real, in-range size; a table import whose actual `is64` doesn't
    match the declared import type's `is64` fails to link with
    "incompatible import type", mirroring the existing memory-import test
    shape exactly.
- **Real corpus, measured** (`cargo run --bin wasm_conformance_report`, full
  JSON diff against the pre-change baseline, confirming only `table64.wast`
  and `memory64-imports.wast` (both new) moved): report the exact
  `module`/`assert_return`/`assert_invalid`/`assert_unlinkable` counts
  achieved for both files, and confirm zero regressions anywhere else in
  the corpus.
- **Downstream consumers**: `cargo test -p lang-aot` and every other crate
  depending on `wasm-types`/`wasm-module-parser`/`wasm-module-encoder`/
  `wasm-wast-parser`/`wasm-validator`/`wasm-execution`/`wasm-runtime`
  (`wasm-conformance`, `twig-to-wasm`, `nib-wasm-compiler`,
  `brainfuck-wasm-compiler`, `ir-to-wasm-compiler`, `ir-to-wasm-validator`,
  `iir-to-wasm`) built/tested — `wasm_types::TableType` is foundational and
  widely depended on, same rationale `W25` already gave for `Limits`/
  `MemoryType`.
- `/security-review` before push (the practical allocation cap, and the
  now-fixed silent-truncating `as u32` cast on an attacker-controlled
  `u64` `min`, are exactly the kind of finding that review is for).
- Docker (`linux/amd64`) verification of every touched crate's test suite
  plus `wasm-conformance --test testsuite_conformance
  corpus_matches_the_committed_baseline` before pushing.

## Addendum: real table64 operations (follow-up slice)

This slice's own "Explicitly out of scope" section deferred actual
operations against an `is64` table — `table.get`/`table.set`/`table.grow`/
`table.size`/`table.fill`/`table.copy`/`table.init`/`call_indirect` all
still assumed an `i32` index unconditionally. This follow-up implements all
of them, vendoring the 9 real corpus files that exercise them
(`call_indirect64.wast`, `table_copy64.wast`, `table_fill64.wast`,
`table_get64.wast`, `table_grow64.wast`, `table_init64.wast`,
`table_set64.wast`, `table_size64.wast`, `table_copy_mixed.wast`) at the
same pinned SHA.

### Operand-width rules (verified against the real corpus, not assumed)

Each op's index/dest/src/len/delta operand width depends on the TARGET
table's own `is64` — mirroring memory64's `pop_effective_addr` is64/is32
branch (`wasm-execution`, W25) exactly, just for table operands instead of
a memory address:

- **`table.get`/`table.set`**: the index operand is `i64` for an `is64`
  table, `i32` otherwise.
- **`table.grow`**: `delta` and the pushed old-size result are both `i64`
  for an `is64` table (mirrors `memory.grow`'s own `is64` branch).
- **`table.size`**: the pushed result is `i64` for an `is64` table.
- **`table.fill`**: `dest`/`len` are `i64` for an `is64` table (`value`
  stays the table's own reference element type, unaffected by `is64`).
- **`table.init`**: only `dest` (into the target TABLE) widens to `i64`
  for an `is64` target table. `src`/`len` (positions within the PASSIVE
  ELEMENT SEGMENT) always stay `i32` — a segment isn't itself
  address-typed. Verified against `table_init64.wast`: `(table.init $t2 1
  (i64.const 7) (i32.const 0) (i32.const 4))` widens only `dest`.
- **`table.copy`**: `dest` follows the DESTINATION table's own `is64`,
  `src` follows the SOURCE table's own `is64`, independently — a mixed
  is64/is32 copy is legal (`table_copy_mixed.wast`'s `test_64to32`/
  `test_32to64`). `len`'s width is `i64` ONLY when BOTH tables are
  `is64` — otherwise `i32`, even when exactly one side is `is64`.
  Verified against `table_copy_mixed.wast`'s own valid `test_64to32`/
  `test_32to64` cases (both use a plain `i32` `len` despite one table
  being `is64`) and its `bad_size_arg`/`bad_src_idx`/`bad_dst_idx`
  `assert_invalid` cases, each of which mistypes exactly one of these
  three operands. Same "the smaller of the two index types governs a
  shared length/count operand" rule the combined memory64/table64
  proposal defines for a mixed `memory.copy`.
- **`call_indirect`/`return_call_indirect`**: the table-index (element
  lookup) operand is `i64` for an `is64` table.

### Implementation shape

- **`wasm-execution`**: `Table`'s own storage stays `u32`/`usize`-based
  (unchanged from the first slice — already bounded well under any `u64`
  range by `MAX_TABLE_ELEMENTS`); only the operand WIDTH popped off/pushed
  onto the WASM value stack changes. New helpers `pop_table_operand`
  (pops `i64` or `i32` per a `bool is64`, narrowing to `u32` via
  `table_u64_to_u32`) and `push_table_result` (pushes `i64` or `i32`).
  `table_u64_to_u32` maps any `u64` that doesn't fit `u32` to `u32::MAX`
  (never truncates) — a real, attacker-reachable `(i64.const -1)` index
  (`table_get64.wast`/`table_set64.wast`'s own boundary case) must still
  trip every existing bounds check, not silently wrap into a small,
  coincidentally-in-bounds index.
- **`wasm-validator`**: a new `ModuleContext::table_is64: Vec<bool>` field
  (combined imports-first-then-declared index space, mirroring
  `table_element_types`), populated in `build_module_context` from each
  table's own `TableType::is64`. Every affected type-check rule above
  reads it to pick `I64` vs. `I32` for the relevant operand(s).
- **A real, pre-existing bug this follow-up also fixes**: `wasm-runtime`'s
  active ELEMENT segment application (in `instantiate()`) unconditionally
  evaluated its offset expression as `i32`, even though the analogous
  active DATA segment branch right above it was already correctly
  `is64`-aware (W25). Any active element segment targeting an `is64`
  table trapped instantiation instead of applying (found via
  `call_indirect64.wast`'s own `(table $t64 i64 funcref (elem
  $const-i32))` shorthand). Fixed to branch on the target table's own
  `is64`, matching the data-segment code exactly, with the same
  upfront-whole-segment-bounds-check-in-`u64`-before-narrowing discipline
  the rest of this follow-up uses.
- **`wasm-wast-parser`**: one matching bug in the SAME family — the
  `(table $t i64 funcref (elem ...))` inline-elem-shorthand desugaring
  (`build_table_limits_and_elements`) hardcoded its generated active
  segment's offset expression to `i32.const 0` regardless of `is64`,
  mirroring `build_memory_limits_and_data`'s OWN already-`is64`-aware
  default-offset branch. Fixed the same way: `i64.const 0` (`0x42`) for an
  `is64` table, `i32.const 0` (`0x41`) otherwise. No other wast-parser
  changes were needed — every other text-form site (explicit `(elem ...)`
  offset expressions, `table.get`/`set`/`grow`/`fill`/`copy`/`init`
  operand expressions) already encodes whatever const-expr instruction the
  source text names generically, with no `is32` assumption baked in.
- **`wasm-module-parser`/`wasm-module-encoder`**: no changes — these real
  table OPERATIONS carry no table64-specific binary encoding of their own
  (only the table DECLARATION's limits flags, already handled by the
  first slice); an is64 table's operand width is inferred at
  validation/execution time from the TARGET table's own already-decoded
  `is64`, not from anything in the operation's own encoding.

### Real corpus, measured

All 9 files vendor with real, non-trivial pass numbers and ZERO `fail`
anywhere (`cargo run --bin wasm_conformance_report -- --write-baseline`,
diffed programmatically against the pre-change baseline — confirmed zero
existing file's tally changed):

| file | module | assert_return | assert_trap | assert_invalid |
|---|---|---|---|---|
| `call_indirect64.wast` | 1/1 | 1/1 | — | — |
| `table_get64.wast` | 1/1 | 5/5 | 4/4 | — |
| `table_set64.wast` | 1/1 | 10/10 | 8/8 | — |
| `table_grow64.wast` | 1/1 | 15/15 | 6/6 | — |
| `table_size64.wast` | 1/1 | 36/36 | — | — |
| `table_fill64.wast` | 1/1 | 64/64 | 6/6 | 9/9 |
| `table_copy64.wast` | 41/41 (+11 nys) | 334/334 (+109 nys) | 760/760 (+446 nys) | — |
| `table_init64.wast` | 26/26 (+18 nys) | 120/120 (+1 nys) | 158/158 (+476 nys) | 67/67 |
| `table_copy_mixed.wast` | 1/1 | — | — | 3/3 |

("nys" = `not_yet_supported`, a pre-existing capability gap — e.g.
`spectest` imports this crate deliberately has no host for, or, in
`table_init64.wast`'s trailing module, a WasmGC `array`/`arrayref`
construct this crate doesn't support at all — not something this
follow-up's own scope covers or regresses. `table_copy64.wast`'s and
`table_init64.wast`'s numbers are otherwise near-identical to their is32
siblings `table_copy.wast`/`table_init.wast`, confirming both files are
largely mechanical i32→i64 transforms of already-passing corpus.)

Two real, previously-latent bugs surfaced and fixed while vendoring (not
present in any already-vendored file, since no `is64` table existed
before this slice): the `wasm-runtime` active-element-segment `is64` gap
and the `wasm-wast-parser` inline-elem-shorthand default-offset gap, both
described above — each initially surfaced as `call_indirect64.wast`'s sole
module trapping instantiation, root-caused and fixed rather than deferred.

A third correction to this follow-up's OWN first draft: `table.copy`'s
`len` operand width was initially (incorrectly) assumed to always match
the DESTINATION table's `is64`, by analogy with `dest` itself. Re-reading
`table_copy_mixed.wast`'s real `test_32to64` case (destination `is64`,
source `is32`, and `len` is still plain `i32`) falsified that assumption;
the corrected rule (`len` is `i64` only when BOTH tables are `is64`) is
what's implemented and vendored.
