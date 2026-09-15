## 0.1.115 — 2026-09-01 — W35 fourth slice, epic closed: cross-module registry fixup pass, `CrossModuleFunction::identity()`, full corpus verification

Slice 4 of 4 for `code/specs/W35-wasm-cross-instance-function-identity.md`
— closes the epic. This slice's own real scope (see `wasm-runtime`'s own
CHANGELOG for the full "the spec's decomposition undersold this slice"
writeup) turned out larger than "mostly verification": a real resolution
fixup pass, a real correctness bug in slice 3's own `owner_instance_
identity` tagging (found here, fixed in `wasm-execution`/`wasm-runtime`),
and a previously-unnamed "ephemeral trap-discarded instance" case
(`wasm-runtime`'s own new `instantiate()` error-path fixup).

### New: `resolve_all_table_funcrefs` call in `instantiate_and_register`

`Executor::instantiate_and_register` now calls `wasm_runtime::
resolve_all_table_funcrefs(&instance)` immediately after wrapping a freshly
instantiated module in its own permanent `Rc<RefCell<WasmInstance>>`, and
BEFORE either registry insertion (`None`/"current module", or an explicit
`$id`) — so no subsequent directive (another module's `import`, or this
module's own `register`) can ever observe a not-yet-fixed-up table. The
actual resolution logic lives in `wasm-runtime` (`pub fn resolve_all_
table_funcrefs`, shared with that crate's own `instantiate()` error-path
fixup — see its own CHANGELOG) rather than being duplicated here; this
crate's own change is purely the one call site plus its own new
`CrossModuleFunction::identity()` (below).

**Three real findings during this slice's own verification, each
documented at length on `resolve_all_table_funcrefs`'s own doc comment in
`wasm-runtime` (not repeated here) — reproduced directly, not
theorized:**
1. Table fixup must cover EVERY table an instance can see (imported ones
   included), not just ones it declares — `linking.wast`'s own `$Ot`
   writes into `$Mt`'s IMPORTED (shared) table via `$Ot`'s own elem
   segment; an "owned tables only" fixup misses exactly this, the spec's
   own motivating case.
2. Funcref-typed GLOBALS must NOT also be eagerly resolved (a real
   regression, backed out) — see the dedicated section below.
3. A table's declared element type must be checked before resolving any
   `Raw` entry — an externref table's `Raw` entries are a real, opaque
   payload (e.g. `ref.extern 42`), never a function index; resolving them
   unconditionally corrupted `elem.wast`'s own externref-table test.

### Deliberately narrower than this slice's own originating task description: funcref-typed GLOBALS are NOT eagerly resolved

A real, reproduced regression, found and reverted during this slice's own
verification — not a theoretical concern. An earlier version of the
fixup pass also resolved every module-DEFINED global whose `GlobalStorage::
func_ref` was still `None`, the same way as a table entry. That version
regressed `return_call_ref.wast`: `31/31 → 30/31`, a NEW `assert_return`
failure, `"func_ref heap limit exceeded (too many funcrefs minted in one
call)"`.

**Root cause, traced directly**: `return_call_ref.wast`'s own `$fac-acc`/
`$count`/`$even`/`$odd` (lines ~108–160) are module-DEFINED `(ref $..)
(ref.func $..)` globals, read every iteration of a deep, genuinely-O(1)-
Rust-stack `return_call_ref` tail-recursion loop via `global.get`.
`wasm-execution`'s own `global.get` opcode handler (0x23) mints a FRESH
`func_ref_heap` handle EVERY time it reads a global whose `func_ref` is
`Some(..)` (`push_func_ref`, unconditionally — no `owner_instance_
identity`-style same-instance fast path the way `call_indirect`/`call_ref`
dispatch already has via `effective_local_index`), and `func_ref_heap` is
reset only "at the start of every call" — NOT between tail-call
iterations of the SAME logical, O(1)-stack call (the whole point of a
real tail call). Before this fixup existed, such a global's `func_ref`
stayed `None` forever, so `global.get` took the cheap `None => value`
branch — zero heap growth per iteration, matching this test's own
explicit intent. Eagerly resolving `func_ref` at fixup time made EVERY
iteration mint one more heap slot, exhausting `MAX_FUNC_REF_HEAP_LEN`
partway through a deep recursion that used to complete cleanly.

Per the spec's own text ("`WasmInstance.globals`: the identical bug, one
field over"), no vendored `.wast` file in this campaign's 257-file corpus
actually `register`s/imports a funcref-typed GLOBAL across instances the
way `linking.wast`/`elem.wast` do for TABLES — confirmed by direct grep.
Given a confirmed regression on one side and zero confirmed benefit on
the other, this slice leaves funcref-typed globals exactly as slice 3
left them (unresolved, resolved lazily and, for a genuinely cross-instance
read, INCORRECTLY — a real, pre-existing, still-open, and now explicitly
documented gap). Fixing this for real needs a `global.get` fast path
mirroring `effective_local_index`'s own same-instance optimization first
— left as a genuine follow-on; see the spec's own closing addendum.

### New: `CrossModuleFunction::identity()`

Implements `HostFunction::identity()` (previously defaulting to `0`) from
a new field, `identity: u64`, snapshotted from the EXPORTING instance's
own `func_identities[index]` at `resolve_function` time — the same
combined function-index space `combined_function_type_idx` already
resolves `index` against. This is what lets an IMPORTING module's own
`func_identities` construction loop (mirroring `tag_identities`'s
"imported adopts the exporter's identity verbatim" rule) give the
imported function the SAME real identity the exporting instance already
minted, rather than a fresh, unrelated one — load-bearing for `wasm-
execution`'s own new `effective_local_index` identity-based fallback (see
that crate's own CHANGELOG) to correctly recognize "this ctx already has
this exact function reachable in its own space."

### New integration tests

Three new, hand-built, `linking.wast`-shaped tests directly exercising the
fix (not just relying on the corpus pass, per this slice's own
verification plan), plus two more added after the security-review pass
below (permanent regression tests for its two real findings):
- `a_funcref_written_by_one_instance_into_a_table_shared_with_another_
  dispatches_to_the_writers_own_function` — a LOCAL function written into
  a shared table by a DIFFERENT instance, dispatched correctly.
- `a_table_entry_written_via_an_imported_function_dispatches_through_the_
  real_exporter_not_the_readers_own_index_space` — the `owner_instance_
  identity`-for-imports bug, isolated and proven fixed.
- `a_funcref_written_by_a_module_whose_own_instantiation_later_traps_
  still_dispatches_correctly` — the "ephemeral trap-discarded instance"
  case (`wasm-runtime`'s own `instantiate()` error-path fixup),
  mirroring `linking3.wast`'s own `$Ms`/`"get table[0]"` example.
- `a_reentrant_dispatch_back_into_the_caller_traps_cleanly_instead_of_
  panicking` — the security review's HIGH finding (a real, deterministic
  `RefCell` re-entrant-borrow panic on an ordinary, non-circular linking
  pattern), proving the process survives and a directive still grades,
  rather than the whole test binary aborting.
- `a_raw_entry_written_by_a_live_table_init_is_not_reattributed_to_a_
  later_importing_instance` — the security review's MEDIUM finding (a
  live `table.init`'s raw write silently misattributed to an unrelated,
  later-importing instance's own fixup pass), proving `wasm-runtime`'s
  precise `active_elem_writes` tracking (not a scan) fixes it.

### Full corpus baseline diff (257 files, programmatic, per-file)

Re-ran `cargo run --release --bin wasm_conformance_report -p
wasm-conformance -- --write-baseline` and diffed `tests/fixtures/
testsuite-status.json` against the pre-slice-4 committed baseline. Exactly
4 files changed — the 4 the spec named — and NONE of the other 253:

| File | Before | After | Outcome |
|---|---|---|---|
| `linking.wast` | `assert_return` 55/65 (10 fail) | **65/65 (0 fail)** | FULLY FIXED |
| `linking0.wast` | `assert_return` 0/1 (1 fail) | **1/1 (0 fail)** | FULLY FIXED |
| `linking3.wast` | `assert_return` 5/6 (1 fail) | **6/6 (0 fail)** | FULLY FIXED |
| `elem.wast` | `assert_return` 13/19 (6 fail) | 18/19 (1 fail) | 5/6 fixed — see below |

`elem.wast`'s ONE remaining failure (`(assert_return (invoke $m "get"
(i32.const 0)) (ref.null extern))`, in the "Initializing a table with an
externref-type element segment" section) is CONFIRMED, via `git stash`
A/B against the pre-slice-4 baseline, to be a PRE-EXISTING failure,
UNRELATED to W35 — it already failed identically before any of this
slice's changes. It concerns an EXTERNREF table's null-handling after a
cross-module active-elem overwrite, not function-reference cross-instance
identity at all; out of scope for this spec, not a partial fix. `elem.
wast`'s own 5 OTHER previously-failing `assert_return`s (the "Element
sections across multiple modules change the same table" section) are the
real W35 fix, and are now fully passing.

### Security review: two real findings, both fixed

See `wasm-runtime`'s own CHANGELOG for the full writeup of both findings
(a HIGH re-entrant-borrow panic, a MEDIUM silent-misattribution bug) and
their fixes. This crate's own share of the fix: `CrossModuleFunction::
call` now uses `try_borrow_mut` instead of a bare `borrow_mut()` (the
same fix `wasm_runtime::LocalFunctionRef::call` needed, for the identical
reason), and its own doc comment updated to reflect that the pre-existing
"genuinely mutual cross-instance cycle" risk now traps cleanly instead of
panicking. Two new permanent regression tests (above) directly reproduce
both findings and prove them fixed.

### Verification

- `cargo build --workspace`: clean.
- `cargo test -p wasm-execution -p wasm-runtime -p wasm-validator -p
  wasm-conformance`: all green (`wasm-conformance`'s own lib suite grew
  from 60 to 65 tests, the 5 new ones above); `git stash` A/B against a
  totally clean base showed IDENTICAL test counts and outcomes before and
  after this slice's combined changes (both before and after the
  security-review fixes).
- Full corpus baseline diff: see the table above — 3 of 4 target files
  fully closed, the 4th (`elem.wast`) closed except for one confirmed
  pre-existing, unrelated bug. Re-confirmed byte-for-byte identical after
  the security-review fixes landed (the fixes change failure MODE --
  panic vs. clean trap, and precision of what gets resolved -- not any
  currently-passing directive's outcome).
- `cargo clippy -p wasm-execution -p wasm-runtime -p wasm-validator -p
  wasm-conformance --all-targets -- -D warnings`: clean.
- Downstream consumer sweep (workspace-wide grep for `HostInterface`/
  `HostFunction` implementors and `WasmInstance` constructors): confirmed
  no crate outside `wasm-execution`/`wasm-runtime`/`wasm-validator`/
  `wasm-conformance`/`lang-aot`'s own test doubles (already fixed by
  earlier slices) touches any of this slice's changed surface.

