# W35 — WASM cross-instance function-reference identity

## Purpose and how this slice was chosen

`code/specs/W07-wasm-post-mvp-epics.md`'s "Addendum (2026-09-01)" — written
immediately after `W32`/`W33`/`W34` closed the last of this campaign's
type-system correctness gaps, from a fresh full-corpus prioritization pass
— names exactly one item as "the big one" among the genuinely-remaining
(non-"not yet supported") failures: `wasm-execution`'s table entries (and,
as this spec's own investigation confirms, funcref-typed globals too)
store bare `u32` function indices with no instance identity attached, so
`call_indirect`/`table.get`/`call_ref` resolve that index against
whichever instance happens to be EXECUTING right now, not whichever
instance actually wrote it. The addendum names this as the confirmed sole
remaining cause of every real failure in `elem.wast`, `linking.wast`,
`linking0.wast`, and `linking3.wast`, and sizes it **L** ("a real
representation change rippling through `wasm-types`/`wasm-execution`/
`wasm-runtime`") — explicitly calling for a `W35` spec-first PR rather
than a fourth deferral, per this repo's own "no shortcuts, do the hard
core" working principle.

This document re-verifies that claim directly against the pinned corpus
and the actual current Rust source (not re-assumed from the addendum's
prose), and — this is where it goes further than the addendum did —
finds that the addendum's own suggested representation ("likely
`Rc<WasmInstance>` ... everywhere a funcref is stored or compared") is
not directly buildable as stated, for two concrete, source-grounded
reasons (§"Why the naive `Rc<WasmInstance>` sketch doesn't work as
stated" below): `WasmValue` is `Copy`, and `wasm-execution` cannot name
`WasmInstance` at all (it is defined one layer up, in `wasm-runtime`,
which already depends on `wasm-execution` — the reverse dependency would
be circular). The design below (§"Design") works around both constraints
by reusing machinery this codebase has already built and proven for
adjacent problems: the `HostFunction` trait (already the abstraction for
"a callable that may live in another instance"), and the W23
`tag_identities` pattern (already the proof that a process-wide-minted
`u64`, adopted verbatim across an import boundary, gives cheap,
allocation-free cross-instance equality) — generalizing both from
"tags"/"a plain cross-module function call" to "any funcref value that
can be stored, not just called."

## What already exists (grounded in the actual current code)

All claims below were checked directly against
`code/packages/rust/wasm-types/src/lib.rs` (0.1.23),
`code/packages/rust/wasm-execution/src/lib.rs` (0.9.86),
`code/packages/rust/wasm-runtime/src/lib.rs` (0.6.23),
`code/packages/rust/wasm-conformance/src/lib.rs` (0.1.113), and
`code/packages/rust/wasm-validator/src/lib.rs` (0.2.84) as they exist on
this branch — the versions in place immediately after PR #13889 (the
LEB128-strictness fix, the most recent merge on `main` as of this
writing), not re-assumed from the addendum's prose.

### The dependency layering (why this is harder than "add a field")

`Cargo.toml` path-dependencies, read directly:

```
wasm-types    -> wasm-leb128                                   (leaf)
wasm-execution -> wasm-types, wasm-leb128, wasm-opcodes,
                  wasm-module-parser, wasm-wast-parser (dev)
wasm-validator -> wasm-types, wasm-execution, wasm-opcodes,
                  wasm-module-parser, wasm-wast-parser (dev)
wasm-runtime   -> wasm-types, wasm-validator, wasm-execution,
                  wasm-opcodes, wasm-module-parser, wasm-wast-parser (dev)
wasm-conformance -> wasm-runtime, wasm-execution, wasm-validator,
                     wasm-types, wasm-wast-parser, wasm-module-parser
```

`WasmInstance` is defined in `wasm-runtime` (`lib.rs:1106`) — a layer
**above** `wasm-execution`. `WasmExecutionContext`/`WasmValue`/
`HostFunction` live in `wasm-execution`, a layer **below** `wasm-runtime`.
So nothing defined in `wasm-execution` (including `WasmValue` itself) can
ever hold a field of type `WasmInstance` or `Rc<WasmInstance>` directly —
that would make `wasm-execution` depend on `wasm-runtime`, which already
depends on `wasm-execution`, a cycle Cargo rejects outright. Any
"reach back to the owning instance" capability needed **inside**
`wasm-execution`'s own opcode handlers must be expressed as a trait
*defined* in `wasm-execution` and *implemented* by something in
`wasm-runtime`, injected into `WasmExecutionContext` the same way
`type_subtyping`/`canonical_types`/`tag_identities` already are (see
below) — never a concrete `WasmInstance` type name inside
`wasm-execution` itself.

### `WasmValue` is `Copy` (why the addendum's "put an `Rc` in the value" sketch needs refining)

`wasm-execution/src/lib.rs:100-101`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValue {
    I32(i32), I64(i64), F32(f32), F64(f64),
    Ref(Option<u32>),
    V128(u32),
}
```

`Ref(Option<u32>)` is *already* semantically overloaded: for a GC
struct/array-typed reference it is a handle into `ctx.gc_heap`; for a
funcref it "IS the function index directly" (the engine's own comment,
`lib.rs:3679-3684` and `lib.rs:5858-5864`, on `ref.func`'s handler) — only
the *static* type, tracked by `wasm-validator`, disambiguates which
namespace a given `Some(u32)` indexes into. This is the SAME "index into
a side table, not the value itself" convention `V128(u32)` already uses
for its 16-byte payload (`v128_heap`) and `Ref(Some(h))` already uses for
a GC object (`gc_heap`) — precisely because `WasmValue` derives `Copy`,
and neither a `[u8; 16]` nor a full GC object tree can live inline in a
`Copy` enum. An `Rc<dyn HostFunction>` (or an `Rc<WasmInstance>`) is
exactly as un-`Copy`-able as those two payloads — so the fix must follow
the SAME "small `Copy` handle, real payload lives in a side table"
shape `V128`/`Ref`-for-GC already establish, not embed an `Rc` directly
into the enum (which would strip `Copy` from all of `WasmValue` at once
— every `I32`/`I64`/etc. value too — rippling through every implicit
`let x = wasm_value;`/by-value function argument across this crate's
~26,500-line `lib.rs`, an unbounded and unnecessary blast radius when
the un-`Copy`-able payload only ever needs to exist for the `Ref`
variant, and only when that specific reference is a funcref that has
crossed — or could cross — an instance boundary).

### `Table`/`TableStorage`: already shared, already documents this exact gap

`wasm-execution/src/lib.rs:1453-1497` (`Table`'s own doc comment, reproduced
because it is the single most directly relevant piece of prior art in the
whole codebase for this spec):

```rust
struct TableStorage {
    elements: Vec<Option<u32>>,
}
```

> ## Cross-instance sharing (W28)
> ... `elements` ... now lives inside a shared `TableStorage`, reached
> through `inner: Rc<RefCell<TableStorage>>` ...
>
> Known, deliberately out-of-scope remaining gap ...: this fix makes a
> table's raw entries (bare `u32` function indices) and its size genuinely
> shared and observable cross-instance, but `call_indirect` still resolves
> a table entry against the CALLING instance's own `func_bodies`/
> `host_functions` index space. A real funcref written into a SHARED table
> by one module and then `call_indirect`-invoked through a DIFFERENT
> module needs actual cross-instance function IDENTITY for table entries
> (the same class of problem `WasmInstance::tag_identities` already solves
> for exception tags, but requiring genuine cross-instance DISPATCH, not
> just equality comparison) -- a separate, larger follow-on, not part of
> this storage-sharing fix.

This is W28's own author naming this exact gap, in these exact words,
years (in campaign-time) before this addendum re-found it independently —
confirming the addendum's diagnosis is not a fresh guess but a
long-documented, now-finally-being-closed piece of technical debt.
Crucially, `TableStorage` — unlike `WasmValue` — is a private, ordinary
`Rc<RefCell<..>>`-wrapped struct with no `Copy` requirement at all, so it
is free to hold a real, non-`Copy`, `Rc`-containing payload directly, no
handle indirection needed at this layer (see "Design" below).

### `WasmInstance.globals`: the identical bug, one field over, not yet named anywhere

`wasm-runtime/src/lib.rs:1116-1124`:

```rust
/// Global variable values. `Rc<RefCell<WasmValue>>`, not a plain
/// `WasmValue` ... see `HostInterface::resolve_global`'s own doc comment
/// ... for the full cross-instance-sharing rationale, which mirrors
/// `memories`/`tables` above exactly (W28) ...
pub globals: Vec<Rc<RefCell<WasmValue>>>,
```

A funcref-typed global (`(global (export "g") funcref (ref.func $f))`,
importable/exportable exactly like a table) is `Rc<RefCell<WasmValue>>`
-shared cross-instance the same way a table's storage is — and its cell
holds a `WasmValue::Ref(Some(u32))` funcref value with the *exact same*
"which instance's function-index space does this `u32` mean" ambiguity
`Table` has, with no equivalent of `Table`'s own doc-comment flagging it.
Not confirmed as a cause of any CURRENT corpus failure (no vendored
`.wast` file in this campaign's 257-file corpus appears to `register`/
import a funcref-*global* the way `linking.wast`/`elem.wast` do for
tables — see "Corpus grounding" below for the precise scope this spec
targets), but it is the same bug, reachable by the same class of module,
and the design below (§"Design") fixes it as part of the SAME
representation change rather than leaving a twin gap freshly undiscovered
next to the one being fixed.

### `ref.func`, `call_indirect`, `call_ref`: confirmed, by direct read, to assume "index == local index"

`ref.func` (0xD2), `wasm-execution/src/lib.rs:5858-5875`:

```rust
// ref.func (0xD2 <funcidx>) — push a non-null funcref referring to a
// function by index (WASM17). The wrapped `u32` is a function index
// into `ctx.func_types`/`ctx.func_bodies` ...
vm.register_context_opcode(0xD2, |vm, instr, _code, ctx| {
    let ctx = get_ctx(ctx);
    let func_index = operand_int(instr) as usize;
    if func_index >= ctx.func_types.len() { ... }
    push_wasm(vm, WasmValue::Ref(Some(func_index as u32)));
    ...
});
```

`call_indirect` (0x11), `wasm-execution/src/lib.rs:11401-11448` — the
literal confirmed-bug site:

```rust
vm.register_context_opcode(0x11, |vm, instr, _code, ctx| {
    let ctx = get_ctx(ctx);
    let (type_idx, table_idx) = unpack_call_indirect_operand(instr);
    ...
    let table = get_table(ctx, table_idx as usize)?;
    let func_index = table.get(elem_index).map_err(VMError::from)?
        .ok_or_else(|| VMError::GenericError("uninitialized table element".into()))?;
    ...
    call_function(vm, ctx, func_index as usize)?;   // <-- resolved against
    Ok(None)                                          //     THIS instance's
});                                                    //     own func space
```

`call_function_inner` (`lib.rs:11632` onward) dispatches
`ctx.host_functions.get(current_func_index)` (`lib.rs:11708`) first, then
falls through to `ctx.func_bodies` — i.e. `func_index` is looked up in the
CALLER's own "imports first, then module-defined" combined index space,
with **no way to know or record which instance actually populated that
table cell**. `call_ref`/`return_call_ref` (0x14/0x15,
`lib.rs:11522-11576`) pop a `WasmValue::Ref(Some(idx))` and do the exact
same `call_function(vm, ctx, func_index)` dispatch — same gap, currently
unreachable by any corpus case (no vendored directive passes a funcref as
a cross-module `call`/`invoke` boundary argument today — see "Explicitly
out of scope" — but a real, latent soundness gap in the SAME class,
audited here for completeness since the fix touches this exact
dispatch primitive anyway).

`table.set`/`table.fill`/`table.init`/`table.copy`/`table.grow`
(`lib.rs:6668-6700` `table.get`'s own handler pushing `WasmValue::Ref`;
`5809-5829` `table.fill`; `5625-5700` `table.init`/`table.copy`) all move
raw `Option<u32>` payloads between `TableStorage::elements` and either
`WasmValue::Ref` (on the operand stack) or `Element.function_indices`
(`wasm_types::Element`, `lib.rs:1354-1370`, itself a plain `Vec<Option<u32>>`
of the DECLARING module's own local indices) — every one of these is a
call site that currently treats "a `u32` inside a table/element" as
interchangeable with "a `u32` naming a function in whichever instance
happens to be running this opcode," and every one needs the audited fix
below.

### `HostFunction`/`CrossModuleFunction`: the reusable half of the fix, already built for a narrower purpose

`HostFunction` (`wasm-execution/src/lib.rs:1757`) is already this crate's
abstraction for "a callable that may live in another instance" — every
WASI shim AND every already-resolved cross-module import goes through it.
Its `call(&self, args: &[WasmValue], memory: Option<&mut LinearMemory>)
-> Result<Vec<WasmValue>, TrapError>` signature is instance-agnostic by
construction: nothing about it assumes the callee lives in the same
combined index space as the caller.

`wasm-conformance::CrossModuleFunction` (`lib.rs:344-410`+) is the ONE
existing concrete "this callable actually lives in a DIFFERENT
`WasmInstance`" implementation:

```rust
struct CrossModuleFunction {
    instance: Rc<RefCell<WasmInstance>>,
    export_name: String,
    func_type: FuncType,
    group_shape: (u32, u32),
    is_final: bool,
    canonical_type: Option<(Rc<CanonicalGroup>, u32)>,
    type_idx: Option<u32>,
}
impl HostFunction for CrossModuleFunction {
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let mut instance = self.instance.borrow_mut();
        WasmRuntime::new().call_typed(&mut instance, &self.export_name, args)
    }
    ...
}
```

This is exactly the "instance handle + something identifying which
function within it" shape the addendum gestured at — but it is built
around calling an **exported** function **by name** (`call_typed`,
`wasm-runtime/src/lib.rs:2171-2192`, itself an export-name lookup around
the private `call_engine(&self, instance, func_index, args)`,
`lib.rs:2377`). A funcref value written by `ref.func`/an elem segment
routinely names a function that is **not exported at all** (e.g.
`linking.wast`'s own `$Mt` module's `$g`, called via `call_indirect`
through the table but never in `$Mt`'s `exports` list) — so the fix needs
a more general primitive than `CrossModuleFunction`'s own
by-export-name shape: **call by raw combined index**, which
`call_engine` already supports internally but does not currently expose
publicly. `wasm-runtime::instantiate()` (`lib.rs:1397`) itself returns a
bare `WasmInstance` (never `Rc`-wrapped) — it is `wasm-conformance`'s own
`ModuleRegistry` (`lib.rs:106`,
`Rc<RefCell<HashMap<Option<String>, Rc<RefCell<WasmInstance>>>>>`) that
introduces `Rc<RefCell<WasmInstance>>`-wrapping, for its own
register/invoke test-harness purposes — `wasm-runtime` the library does
not itself assume or require it. This spec's design (below) needs
`wasm-runtime::instantiate()` itself, not just `wasm-conformance`'s
harness, to be able to produce self-contained funcref identities (an
elem segment is applied entirely inside `instantiate()`, before
`wasm-conformance` ever sees the resulting `WasmInstance`) — so
`WasmInstance` gains a documented requirement that any embedder wanting
real linking must construct it behind `Rc<RefCell<..>>` from the start
(see "Design" §4), generalizing a convention `wasm-conformance` already
follows today, not inventing a new one.

### `WasmInstance::tag_identities` (W23): proven precedent for cheap cross-instance EQUALITY, explicitly insufficient alone for DISPATCH

`wasm-runtime/src/lib.rs:1176-1195`, `1246`:

```rust
pub tags: Vec<u32>,
pub tag_identities: Vec<u64>,
...
static NEXT_TAG_IDENTITY: AtomicU64 = AtomicU64::new(1);
```

Built once per `instantiate()` call (`lib.rs:1435-1683`): a module-DEFINED
tag mints a fresh, never-repeating `u64` from the process-wide counter; an
IMPORTED tag **adopts the exporting instance's own already-minted
identity verbatim** (via `HostInterface::resolve_tag`, mirrored exactly by
this spec's proposed `HostFunction::identity()`, see "Design" §1) rather
than minting an unrelated new one. Threaded into `wasm-execution` via
`WasmExecutionEngine::set_tag_identities` (`lib.rs:12248-12251`), consulted
by `try_catch_exception` to match a `catch` clause across an instance
boundary. This is the exact "process-wide-unique `u64`, propagated
verbatim across imports, compared by cheap equality" shape this spec
reuses for function identity — but tags only ever need to be **compared**
(does this `throw`'s tag equal this `catch`'s tag?), never **dispatched**
(a tag is never "called"). `Table`'s own doc comment (quoted above)
already names this precise distinction as the reason tag identity alone
doesn't solve the funcref problem: function references need real
**invocation** through the identity, which is `HostFunction`'s job, not
`tag_identities`'s.

## Root cause and the concrete failing trace

### `linking.wast`'s "Tables" section (lines 269-354): the addendum's own cited "4 and -4" swap, traced exactly

```wat
(module $Mt
  (type (func (result i32)))    ;; type 0
  (type (func))                 ;; type 1
  (table (export "tab") 10 funcref)
  (elem (i32.const 2) $g $g $g $g)
  (func $g (result i32) (i32.const 4))          ;; Mt's func index 0
  (func (export "h") (result i32) (i32.const -4)) ;; Mt's func index 1
  (func (export "call") (param i32) (result i32) ;; Mt's func index 2
    (call_indirect (type 0) (local.get 0)))
)
(register "Mt" $Mt)
```

`$Mt` has no imports, so its combined func-index space is trivially
`[$g=0, h=1, call=2]`. Its own `elem` writes `table[2..6) = 0` (`$g`'s
LOCAL index in `$Mt`'s own space).

```wat
(module $Ot
  (type (func (result i32)))                ;; type 0
  (func $h (import "Mt" "h") (result i32))  ;; Ot's func index 0 (an IMPORT)
  (table (import "Mt" "tab") 5 funcref)     ;; the SAME shared table object
  (elem (i32.const 1) $i $h)                ;; writes table[1]=$i(=1), table[2]=$h(=0)
  (func $i (result i32) (i32.const 6))      ;; Ot's func index 1
  (func (export "call") (param i32) (result i32)
    (call_indirect (type 0) (local.get 0))))
```

`$Ot` imports `$Mt`'s own `h` export as ITS OWN func index **0** (imports
occupy the low end of the combined index space), then imports `$Mt`'s
`tab` (the SAME `Rc<RefCell<TableStorage>>`, per W28). `$Ot`'s own
`elem (i32.const 1) $i $h` OVERWRITES `table[2]` (previously `$g`, i.e.
`0` in `$Mt`'s space, meaning "returns 4") with `$h`'s value in `$Ot`'s
OWN space — which is `0` (because in `$Ot`'s combined space, the import
of `Mt.h` is index 0, not 1).

So after `$Ot` instantiates, `table[2]` holds the raw `u32` value `0` —
written by `$Ot` to mean "my own func index 0, i.e. `Mt.h`, which returns
`-4`." The corpus then asserts:

```wat
(assert_return (invoke $Mt "call" (i32.const 2)) (i32.const -4))
```

`$Mt "call"` runs `call_indirect` against `$Mt`'s OWN table reference
(the SAME shared storage), reads `table.get(2)` = `0`, and — under the
CURRENT, buggy behavior — resolves `0` within **`$Mt`'s own** combined
index space, where `0` means `$g` (returns `4`), not `Mt.h` (returns
`-4`). This is the exact, addendum-cited "expects 4 and -4 ... gets them
swapped" symptom, traced to the literal bit pattern (`0`) two DIFFERENT
instances' index spaces disagree about the meaning of.

### `elem.wast`'s "Element sections across multiple modules change the same table" (lines 926-974): the identical bug, restated with three participants

```wat
(module $module1
  (table (export "shared-table") 10 funcref)
  (elem (i32.const 8) $const-i32-a)   ;; module1's func 0, returns 65
  (elem (i32.const 9) $const-i32-b)   ;; module1's func 1, returns 66
  (func (export "call-8") (call_indirect (type $out-i32) (i32.const 8)))
  (func (export "call-9") (call_indirect (type $out-i32) (i32.const 9))))
(register "module1" $module1)
(assert_return (invoke $module1 "call-8") (i32.const 65))   ;; before module2

(module $module2
  (import "module1" "shared-table" (table 10 funcref))
  (elem (i32.const 7) $const-i32-c)   ;; module2's func 0, returns 67
  (elem (i32.const 8) $const-i32-d)   ;; module2's func 1, returns 68
  (func $const-i32-c ...) (func $const-i32-d ...))

(assert_return (invoke $module1 "call-7") (i32.const 67))  ;; module2's c
(assert_return (invoke $module1 "call-8") (i32.const 68))  ;; module2's d, was 65
(assert_return (invoke $module1 "call-9") (i32.const 66))  ;; module1's own b, unchanged
```

`table[8]` is overwritten from `0` (module1's own `$const-i32-a`, `65`) to
`1` (module2's own `$const-i32-d`, `68`) — but `$module1` re-invoking
`call-8` will, under the current bug, resolve `1` in ITS OWN space (its
own func index 1, i.e. `$const-i32-b`, `66`) instead of module2's `d`
(`68`). `linking3.wast` (`$Ms`'s `"table"` export, `$f`'s elem write from
an importing module, `$Ms "get table[0]"` expected to observe the
IMPORTING module's `$f` = `0xdead`) and `linking0.wast` (`$Mt "call"`
observing an IMPORTING module's `$f` written into an imported/shared
table) are the same root cause again, re-confirmed by direct read of both
files against the pinned SHA
(`28864811cf03bdbf880733786148feaba339582d`).

## Why the naive `Rc<WasmInstance>` sketch doesn't work as stated

Restating the two blockers found above, together, because they compound:

1. `wasm-execution` — where `WasmValue`, `Table`, `ref.func`,
   `call_indirect`, `call_ref` all live — **cannot name `WasmInstance` at
   all** (it is defined one layer up, in `wasm-runtime`; the reverse
   dependency is circular). So "put `Rc<WasmInstance>` in the value" is
   not even expressible as a type inside the crate that most needs it.
2. `WasmValue` is `Copy`. Even if `wasm-execution` COULD name
   `WasmInstance`, embedding `Rc<WasmInstance>` (or any `Rc<..>`) directly
   into `WasmValue::Ref`'s payload would strip `Copy` from the WHOLE enum
   — including `I32`/`I64`/`F32`/`F64`/`V128`, none of which have anything
   to do with this bug — forcing every implicit by-value use of a
   `WasmValue` anywhere in this crate's ~26,500-line `lib.rs` to become an
   explicit `.clone()`, an enormous, largely-irrelevant mechanical
   diff for a fix that only concerns funcref values reaching a table or
   global cell.

Both are avoided by NOT touching `WasmValue`'s shape at all, and instead
following the SAME "small `Copy` handle on the stack, real payload lives
in a side table or in already-non-`Copy` shared storage" split this
codebase already uses for `V128`/GC `Ref` handles — landing the new,
non-`Copy`, cross-instance-aware payload in exactly the two places that
are ALREADY not `Copy`-constrained and ALREADY the site of the actual bug:
`TableStorage::elements` and a global's storage cell.

## Design

### 1. `FuncRefTarget`: the real, self-contained, cross-instance-safe payload (new, in `wasm-execution`)

```rust
/// A funcref value's real identity, self-contained enough to be stored in
/// a `Table`/global cell and later invoked by ANY instance, not just the
/// one that produced it — the fix for the gap `Table`'s own W28 doc
/// comment already named. Lives in `wasm-execution` (not `wasm-types`,
/// which cannot depend on anything with a `dyn HostFunction` in it either
/// — `HostFunction` itself is defined here) so `ref.func`/`call_indirect`/
/// `call_ref`/`table.get`/`table.set` can all consume it directly.
#[derive(Clone)]
pub struct FuncRefTarget {
    /// A process-wide-unique, never-repeating identity — the SAME
    /// `AtomicU64`-minted, "imported adopts the exporter's own identity
    /// verbatim" pattern `wasm-runtime::WasmInstance::tag_identities`
    /// (W23) already uses for tags, generalized from tags to functions.
    /// `ref.eq`/table-entry-equality (see §5) compares ONLY this field —
    /// cheap, allocation-free, and never needs to touch `callable`.
    pub identity: u64,
    /// The real dispatch target. Reuses the EXISTING `HostFunction`
    /// abstraction verbatim — `call_indirect`/`call_ref`'s post-fix
    /// dispatch is `target.callable.call(args, memory)`, mechanically
    /// identical to how a plain cross-module `call $imported_func`
    /// already works today. `Rc`, not `Box` (see `HostFunction::call`'s
    /// existing `Box<dyn HostFunction>` storage in `host_functions`,
    /// which this spec also changes to `Rc<dyn HostFunction>` — see §3 —
    /// so a `FuncRefTarget` can cheaply CLONE an existing import's
    /// callable instead of needing to rebuild it).
    pub callable: Rc<dyn HostFunction>,
}
```

`HostFunction` (`wasm-execution/src/lib.rs:1757`) gains one new method,
following the exact "default correct for every pre-existing impl" pattern
`type_group_shape`/`is_final`/`canonical_type` already established for
the W33/W34 additions:

```rust
/// This function's own stable, process-wide-unique identity (W35) —
/// mirrors `wasm_runtime::WasmInstance::tag_identities`'s (W23) own
/// "imported adopts the exporter's identity verbatim" contract, just for
/// functions instead of tags. Defaults to `0` (reserved, "no stable
/// identity" — correct for every pre-existing WASI-shim `HostFunction`,
/// none of which are ever the target of a stored funcref in this corpus;
/// see "Explicitly out of scope"). `CrossModuleFunction` and the new
/// `LocalFunctionRef` (§3) are the two real implementors.
fn identity(&self) -> u64 { 0 }
```

### 2. `WasmInstance::func_identities` (new, in `wasm-runtime`) — mirrors `tag_identities` exactly

```rust
/// Canonical, cross-instance-safe function identity per combined
/// func-index-space entry (W35) -- same shape and same construction-time
/// loop as `tag_identities` (W23) immediately above it in this struct. A
/// module-DEFINED function mints a fresh identity from the SAME
/// process-wide counter `tag_identities` already uses (tags and functions
/// are never compared against each other, so sharing one counter is
/// harmless and avoids a second `AtomicU64`). An IMPORTED function
/// adopts `host_func.identity()` verbatim.
pub func_identities: Vec<u64>,
```

Threaded into `wasm-execution` via a new `set_func_identities`, byte-for-
byte mirroring `set_tag_identities` (`wasm-execution/src/lib.rs:
12248-12251`).

### 3. `LocalFunctionRef` (new, in `wasm-runtime`) — the missing "wrap MY OWN local function" half of `HostFunction`

`CrossModuleFunction` (in `wasm-conformance`) already covers "wrap
ANOTHER instance's EXPORTED function, called by name." The fix needs the
more general "wrap ANY function (exported or not) of an instance, called
by RAW INDEX" primitive — needed because `$g` in the `linking.wast` trace
above is never exported, yet still needs a real cross-instance identity
the moment `$Mt`'s own `elem` segment writes it into a table another
instance can later read. This belongs in `wasm-runtime` (not
`wasm-conformance`) because `wasm-runtime::instantiate()` itself — not
just `wasm-conformance`'s register/invoke harness — is where an active
`elem` segment gets applied (`wasm-runtime/src/lib.rs:1868` onward), so
`wasm-runtime` needs this capability for its OWN internal use, independent
of any particular embedder.

```rust
// wasm-runtime
struct LocalFunctionRef {
    instance: Rc<RefCell<WasmInstance>>,
    func_index: u32,
    func_type: FuncType,     // instance.func_types[func_index], snapshotted
    identity: u64,           // instance.func_identities[func_index], snapshotted
    // group_shape / is_final / canonical_type: same snapshot-at-construction
    // pattern `CrossModuleFunction` already uses, sourced from
    // `instance.module.type_subtyping`/`instance.canonical_types` via
    // `instance.func_type_indices[func_index]`.
}
impl HostFunction for LocalFunctionRef {
    fn func_type(&self) -> &FuncType { &self.func_type }
    fn identity(&self) -> u64 { self.identity }
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let mut instance = self.instance.borrow_mut();
        // Calls the NEW pub-by-index primitive from §4, not `call_typed`
        // (§"CrossModuleFunction" above) -- `func_index` here need not
        // even be exported.
        WasmRuntime::new().call_by_index(&mut instance, self.func_index as usize, args)
    }
    // type_group_shape / is_final / canonical_type: same snapshot fields,
    // same trait impls as `CrossModuleFunction`'s already-shipped ones.
}
```

`WasmRuntime` gains one new `pub` method, exposing the ALREADY-EXISTING
private `call_engine` (`wasm-runtime/src/lib.rs:2377`) under a
by-index (not by-export-name) contract:

```rust
/// Call a function by its raw combined index, whether or not it is
/// exported -- the primitive `call`/`call_typed` (both export-name
/// lookups) don't provide, and `LocalFunctionRef`/a future embedder
/// needing real funcref identity does. Purely additive: `call`/
/// `call_typed` are unchanged, both still resolve a name first and then
/// delegate to this same `call_engine` internally.
pub fn call_by_index(&self, instance: &mut WasmInstance, func_index: usize, args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError> {
    self.call_engine(instance, func_index, args)
}
```

### 4. Wiring `LocalFunctionRef`/`func_identities` into `wasm-execution`'s live opcode handlers

`WasmExecutionContext` cannot name `WasmInstance` (§"Why the naive
sketch doesn't work"), so the "wrap my own local function" capability is
injected as a trait, the same indirection `HostFunction` itself already
provides for the "call a foreign function" capability:

```rust
// wasm-execution (new)
/// Injected by `wasm-runtime::build_engine` (a new `set_self_resolver`,
/// mirroring `set_type_subtyping`'s optional-setter shape) -- lets a
/// LIVE opcode handler (`ref.func`, `table.init`) build a self-contained
/// `FuncRefTarget` for one of the CURRENTLY EXECUTING instance's OWN
/// local (non-imported) functions, without `wasm-execution` ever naming
/// `WasmInstance` directly. Left unset (`None`, the default for every
/// pre-existing hand-built `WasmExecutionContext` in this crate's own
/// unit tests, none of which exercise cross-instance linking), `ref.func`
/// falls back to today's behavior exactly (a bare local index, no
/// `FuncRefTarget` minted at all) -- see §6's compatibility note.
pub trait SelfFunctionResolver {
    fn resolve_local_function(&self, func_index: u32) -> Result<FuncRefTarget, VMError>;
}
```

`wasm-runtime::build_engine` implements this with a thin struct closing
over the `Rc<RefCell<WasmInstance>>` being built (constructed via the
same "self-referential `Rc<RefCell<..>>`" two-phase pattern already
required — see §7 for the exact construction-order fix this needs) and
constructs a `LocalFunctionRef` from §3 for the requested index.

`WasmExecutionContext::resolve_function_ref(&self, idx: u32) ->
Result<FuncRefTarget, VMError>` (new, internal) becomes the ONE place a
`FuncRefTarget` is minted from a bare local index:

```rust
fn resolve_function_ref(&self, idx: u32) -> Result<FuncRefTarget, VMError> {
    if let Some(Some(hf)) = self.host_functions.get(idx as usize) {
        // Already cross-instance-safe (an import) -- clone the Rc, no
        // new identity minted (it already has one, from `func_identities`
        // adopting the exporter's verbatim, per §2).
        return Ok(FuncRefTarget { identity: hf.identity(), callable: hf.clone() });
    }
    let identity = self.func_identities.get(idx as usize).copied().unwrap_or(0);
    let resolver = self.self_resolver.as_ref()
        .ok_or_else(|| VMError::GenericError("funcref crossed an instance boundary with no self-resolver installed".into()))?;
    let mut target = resolver.resolve_local_function(idx)?;
    target.identity = identity; // keep ctx's own view of identity authoritative
    Ok(target)
}
```

(`host_functions: Vec<Option<Box<dyn HostFunction>>>` becomes `Vec<Option<
Rc<dyn HostFunction>>>` — a small, mechanical `Box`→`Rc` swap; every
existing call site only ever calls `&self` methods through it, so this is
behavior-preserving everywhere except the one new `.clone()` this spec
needs.)

### 5. The `Copy`-preserving handle: `func_ref_heap`, transient, per-call

```rust
// wasm-execution: WasmExecutionContext gains
/// Scratch area letting a `Copy` `WasmValue::Ref(Some(handle))` stand in
/// for a non-`Copy` `FuncRefTarget` for the DURATION of one execution --
/// mirrors `gc_heap`/`v128_heap`'s own "index, not embedded value" shape,
/// but — unlike those two — does NOT need to persist across calls: a
/// `FuncRefTarget` handle is always consumed (dispatched, or copied into
/// `TableStorage`/a global cell as a real `FuncRefTarget`, see §6) within
/// the SAME execution that produced it; nothing durable ever stores a
/// raw handle into this Vec. Reset (cleared) at the start of every call,
/// same as the operand stack itself.
pub func_ref_heap: Vec<FuncRefTarget>,
```

`ref.func` (0xD2) changes from pushing `WasmValue::Ref(Some(func_index))`
directly to:

```rust
let target = ctx.resolve_function_ref(func_index as u32)?;
let handle = ctx.func_ref_heap.len() as u32;
ctx.func_ref_heap.push(target);
push_wasm(vm, WasmValue::Ref(Some(handle)));
```

`table.get` (0x25) does the same after reading a `FuncRefTarget` back out
of `TableStorage` (§6). `call_indirect`/`call_ref`/`table.set`/
`table.fill` all resolve a `WasmValue::Ref(Some(handle))` operand via
`ctx.func_ref_heap[handle]` before dispatching (`call_indirect`/
`call_ref`: `target.callable.call(args, memory)`) or storing (`table.set`/
`table.fill`: clone the `FuncRefTarget` into `TableStorage`).

### 6. `TableStorage`/element storage: hold the real payload directly (no handle needed here)

```rust
// wasm-execution
struct TableStorage {
    elements: Vec<Option<FuncRefTarget>>,   // was Vec<Option<u32>>
}
```

Not `Copy`-constrained (a private struct, always accessed through
`Rc<RefCell<..>>`), so it holds the real, self-contained payload with no
indirection. `Table::get`/`set`/`fill`/`grow`/`copy` change from moving
`Option<u32>` to moving `Option<FuncRefTarget>` — mechanical signature
changes, same control flow. `wasm_types::Element.function_indices:
Vec<Option<u32>>` (the DECLARING module's own local indices, as parsed)
is UNCHANGED — it is resolved into `FuncRefTarget`s exactly once, at the
point an active/passive segment is actually applied (`wasm-runtime::
instantiate()`'s elem loop, `lib.rs:1868`, for active segments; `table.init`
's opcode handler, `wasm-execution/src/lib.rs:5625`, for passive ones —
both call the SAME `resolve_function_ref`-shaped conversion, using the
DECLARING instance's own `host_functions`/`self_resolver`, never the
table's or the CALLING instance's).

### 7. Globals: a new `GlobalStorage` wrapper, mirroring `Table`/`TableStorage`'s own split

`WasmValue` staying `Copy` (§"Why the naive sketch doesn't work") means a
funcref-typed global's cell cannot hold a `FuncRefTarget` inline inside
`WasmValue` either. `WasmInstance.globals: Vec<Rc<RefCell<WasmValue>>>`
becomes:

```rust
// wasm-runtime (new)
pub struct GlobalStorage {
    /// `WasmValue::Ref(Some(0))` (a reserved sentinel handle, never a
    /// real `func_ref_heap` index) whenever `func_ref` is `Some` -- the
    /// real payload lives in `func_ref` instead. Every non-funcref global
    /// is completely unaffected: `value` alone is authoritative and
    /// `func_ref` stays `None` forever.
    pub value: WasmValue,
    pub func_ref: Option<FuncRefTarget>,
}
pub globals: Vec<Rc<RefCell<GlobalStorage>>>,   // was Vec<Rc<RefCell<WasmValue>>>
```

`global.get`/`global.set` (and `HostInterface::resolve_global`, whose
return type gains the same wrapper) translate at the SAME boundary
`table.get`/`table.set` do: reading a global with `func_ref: Some(t)`
pushes a fresh `func_ref_heap` handle for `t.clone()`; writing a funcref
value into a global resolves the popped handle back into a real
`FuncRefTarget` and stores it in `func_ref`, alongside the sentinel
`value`. This is a real, mechanical ripple through every existing
globals call site (`instantiate()`'s global-construction loop,
`HostInterface::resolve_global`'s signature, every `wasm-conformance`
`HostInterface` test double) — enumerated precisely in "Call-site audit"
below, not hand-waved.

## Call-site audit (every place a bare funcref-as-`u32` assumption was found, by direct read)

| Site | File:line | Current assumption | Fix |
|---|---|---|---|
| `ref.func` (0xD2) | `wasm-execution/src/lib.rs:5866-5875` | pushes `Ref(Some(local_idx))` raw | mint/reuse a `FuncRefTarget` via `resolve_function_ref`, push a `func_ref_heap` handle |
| `table.get` (0x25) | `wasm-execution/src/lib.rs:6668-6690` | reads `Option<u32>` from `Table`, pushes raw | reads `Option<FuncRefTarget>`, pushes a fresh handle |
| `table.set` (0x26) | `wasm-execution/src/lib.rs` (paired with `table.get`) | pops `Ref(Some(u32))`, writes raw into `Table` | resolves handle -> `FuncRefTarget` via `func_ref_heap`, writes that |
| `call_indirect`/`return_call_indirect` (0x11/0x13) | `wasm-execution/src/lib.rs:11401-11448`, `11475+` | `table.get` result used as a LOCAL func index into `ctx.host_functions`/`func_bodies` | resolve handle -> `FuncRefTarget`, dispatch via `target.callable.call(..)` |
| `call_ref`/`return_call_ref` (0x14/0x15) | `wasm-execution/src/lib.rs:11522-11576`+ | popped `Ref(Some(u32))` used as a LOCAL func index | same handle -> `FuncRefTarget` -> `callable.call(..)` resolution (currently unreached by any corpus case; audited for soundness, see "Explicitly out of scope") |
| `table.fill` (0xFC 0x11) | `wasm-execution/src/lib.rs:5809-5829` | pops `Ref(v)`, calls `Table::fill(dest, v, len)` with raw `Option<u32>` | `v` resolved to `FuncRefTarget` via `func_ref_heap` before `fill` |
| `table.init` (0xFC 0x0C) | `wasm-execution/src/lib.rs:5625-5681` | bulk-copies `Element.function_indices` (`Vec<Option<u32>>`) directly into `TableStorage` | each entry resolved via `resolve_function_ref` (using the segment's OWN declaring-instance context, threaded in the same way `func_type_indices` already is) before the copy |
| `table.copy` (0xFC 0x0E) | `wasm-execution/src/lib.rs:5698-5730` | bulk-copies `Option<u32>` between two `TableStorage`s | unaffected in SHAPE (now copies `Option<FuncRefTarget>`, an already-resolved, self-contained value — no NEW resolution needed, since both sides are already-real `FuncRefTarget`s) |
| `table.grow` (0xFC 0x0F) | around `wasm-execution/src/lib.rs:5780-5799` | grows with an `Option<u32>` init value | grows with an `Option<FuncRefTarget>` init value (already resolved by the caller, e.g. a popped `ref.func` result) |
| Active elem-segment application | `wasm-runtime/src/lib.rs:1868` onward | writes `Element.function_indices` raw `u32`s into `TableStorage` | resolves each via the DECLARING instance's own `host_functions`/local-function-wrap (§3/§4) before writing |
| `WasmInstance.globals` | `wasm-runtime/src/lib.rs:1124` | `Rc<RefCell<WasmValue>>`, funcref value is a raw local index | becomes `Rc<RefCell<GlobalStorage>>` (§7) |
| `HostInterface::resolve_global` | `wasm-runtime/src/lib.rs:290`/`535`(+ 3 test doubles) | returns `(GlobalType, Rc<RefCell<WasmValue>>)` | returns `(GlobalType, Rc<RefCell<GlobalStorage>>)` |
| `host_functions: Vec<Option<Box<dyn HostFunction>>>` | `wasm-execution/src/lib.rs:3687` (+ `WasmEngineConfig` fields at `11938`/`11946`/`11991`) | `Box`, cannot be cheaply cloned into a `FuncRefTarget` | becomes `Vec<Option<Rc<dyn HostFunction>>>` |
| `HostFunction` trait | `wasm-execution/src/lib.rs:1757` | no identity concept | gains `fn identity(&self) -> u64 { 0 }` (default) |
| `CrossModuleFunction` | `wasm-conformance/src/lib.rs:344-410`+ | no identity | implements `identity()` from a NEW field, snapshotted from the exporting instance's `func_identities[type_idx]`-equivalent at `resolve_function` time |
| `WasmInstance` | `wasm-runtime/src/lib.rs:1106` | no `func_identities` field | gains `pub func_identities: Vec<u64>` (§2), populated in `instantiate()` alongside `tag_identities` |
| `WasmExecutionEngine`/`WasmExecutionContext` | `wasm-execution/src/lib.rs` (`WasmEngineConfig`, `~11938`/`11946`/`11991`/`12101`/`12343`) | no `func_identities`/`func_ref_heap`/`self_resolver` fields | gain all three, threaded via new `set_func_identities`/`set_self_resolver` setters mirroring `set_tag_identities`'s exact shape (`lib.rs:12248-12251`) |
| `ref.eq`-style / table-entry equality comparisons | (none currently implemented for funcref specifically — `WasmValue`'s `derive(PartialEq)` on `Ref(Option<u32>)` only ever compares RAW indices, which is already wrong once handles are per-call-transient) | N/A today | any future funcref equality check MUST compare `FuncRefTarget::identity`, never the raw `func_ref_heap` handle or the `Rc` pointer — flagged explicitly since a handle-based `==` would be a silent correctness regression the moment this lands |

## Security and lifetime consideration: reference cycles through `Rc<RefCell<WasmInstance>>`

Once a `FuncRefTarget`/`LocalFunctionRef` can hold `Rc<RefCell<WasmInstance>>`,
and that value can be stored inside the SAME instance's own table/global
(directly, via `ref.func` on one of its own functions) or inside ANOTHER
instance's table/global (via cross-module linking), a genuine reference
cycle is constructible: instance A's table holds a `FuncRefTarget`
pointing into instance B (via B's export), and B's table holds one
pointing back into A. Neither `Rc` ever drops to zero, and neither
instance's memory (`memories`, `gc_heap`, `v128_heap`, everything else it
owns) is ever reclaimed — a real, attacker-triggerable memory-retention
issue if this interpreter is ever embedded somewhere instances are
created and discarded repeatedly at runtime (not just this crate's own
batch conformance-test harness, which creates a bounded, fixed set of
instances per `.wast` file and drops the whole `ModuleRegistry` at once
when the file's directives are exhausted — a cycle within one file's
registry is harmless there, since the WHOLE registry, cycle and all, is
freed together).

This is the same class of concern `wasm-conformance`'s OWN
`ModuleRegistry` (`Rc<RefCell<HashMap<..., Rc<RefCell<WasmInstance>>>>>`)
already has zero present-day mitigation for (an existing, accepted
property of this crate's test-harness usage pattern, not a NEW hole this
spec introduces) — `register`/`invoke` never explicitly drops an instance
mid-file, so a cycle inside one registry is bounded by "at most this many
instances this one `.wast` file registers," never unbounded, and the
whole registry (and everything reachable from it) is dropped together at
end-of-file regardless of internal cycles. This spec's own new
`FuncRefTarget`/`LocalFunctionRef` machinery does not change that
property — a cycle through a funcref is bounded by the same "one
registry, dropped as a unit" lifetime a cycle through
`CrossModuleFunction`'s already-existing `Rc<RefCell<WasmInstance>>`
field could ALREADY construct today (e.g. two modules that `register`
tables/globals holding functions that import EACH OTHER — nothing in
`CrossModuleFunction`'s existing design prevents this cycle either, and
it has shipped since the W10 real-linking work with no reported issue).

**For whoever implements this**: the mitigation this spec recommends is
documentation, not a structural fix — explicitly note, on
`FuncRefTarget`/`LocalFunctionRef`'s own doc comments, that a caller
embedding this crate OUTSIDE the batch-conformance-test usage pattern
(e.g. a long-running host that creates and destroys WASM instances
repeatedly, such as a server handling many short-lived requests) MUST
either (a) avoid ever letting two mutually-linked instances' tables/
globals reference each other in a way that forms a cycle, or (b) use
`Weak<RefCell<WasmInstance>>` in one direction of any such link and
upgrade-or-trap on a stale reference, or (c) implement an explicit
mark-and-sweep/generation-based instance-registry eviction (analogous to
this crate's own existing `gc` module's mark-sweep for the WasmGC object
heap — `wasm-execution/src/gc`) rather than relying on `Rc` refcounting
alone for instance lifetime once cross-instance function references are
real. None of (a)/(b)/(c) is needed to make the CORPUS pass (the
conformance harness's own "one registry, freed as a unit" usage pattern
is provably safe as argued above), so this spec does not mandate
implementing any of them now — it only requires that whichever slice
lands `FuncRefTarget`/`LocalFunctionRef` documents the risk in place, so
a future embedder outside this repo's own test harness doesn't discover
it the hard way.

## Recommended slice decomposition

Following this campaign's own established shape (W33/W34 each shipped as
four dependency-ordered slices; W07's addendum explicitly asks for the
same discipline here):

1. **Representation + `wasm-types`/`wasm-execution` plumbing, no
   behavior change yet.** `FuncRefTarget`, `HostFunction::identity()`
   (default `0`), `host_functions: Box` → `Rc`, `func_ref_heap` field on
   `WasmExecutionContext`, `SelfFunctionResolver` trait (unimplemented by
   anything yet — `self_resolver` stays `None` everywhere). Verify: crate
   compiles, every EXISTING test in `wasm-execution`/`wasm-validator`
   passes byte-for-byte unchanged (nothing wired into any opcode handler
   yet — this slice is purely additive types, exactly W34 first slice's
   own "prove the representation compiles and derives correctly before
   touching any call site" discipline). No corpus tally change expected.

2. **`wasm-execution` opcode call sites.** `ref.func`, `table.get`/`set`/
   `fill`, `call_indirect`/`return_call_indirect`, `call_ref`/
   `return_call_ref`, `table.init`/`table.copy`/`table.grow` all switch to
   the `func_ref_heap`-handle scheme; `TableStorage::elements` becomes
   `Vec<Option<FuncRefTarget>>`. `self_resolver` still unset by every
   test in THIS crate (none of `wasm-execution`'s own hand-built unit
   tests exercise cross-instance linking — that only happens through
   `wasm-runtime`/`wasm-conformance`), so `resolve_function_ref` for a
   LOCAL (non-imported) function returns the "no self-resolver
   installed" error in this slice — acceptable because nothing in
   `wasm-execution`'s own test suite constructs that case; every existing
   `ref.func`/`call_indirect` test in this crate only ever calls
   IMPORTED-or-nonexistent functions through `host_functions`
   (`Rc`-wrapped now) or never crosses an instance at all (whichverify:
   `cargo test -p wasm-execution` unchanged pass rate; no corpus run yet
   (that needs `wasm-runtime`/`wasm-conformance` wiring, slice 3-4).

3. **`wasm-runtime` instantiation/import-wiring.** `WasmInstance::
   func_identities` (mirrors `tag_identities`'s own construction loop
   exactly); `LocalFunctionRef`; `WasmRuntime::call_by_index`;
   `GlobalStorage` replacing bare `Rc<RefCell<WasmValue>>` for globals
   (`HostInterface::resolve_global`'s signature change ripples to every
   `HostInterface` impl in `wasm-runtime`'s own test module — enumerate
   and fix each, per the "Call-site audit" table's `resolve_global` row);
   active elem-segment application (`instantiate()`, `lib.rs:1868`)
   resolves through the OWN instance's `host_functions`/a
   `LocalFunctionRef`-equivalent before writing into `TableStorage`;
   `build_engine` implements and injects `SelfFunctionResolver`. This
   is the slice that requires solving the "`Rc<RefCell<WasmInstance>>`
   must exist BEFORE `instantiate()` finishes building it, because an
   ACTIVE elem segment referencing the instance's OWN local functions
   needs to wrap `Rc<RefCell<WasmInstance>>` + index DURING
   instantiation" construction-order problem — recommended resolution:
   build the `WasmInstance` value first with elem/global application
   DEFERRED, wrap it in `Rc::new(RefCell::new(..))`, THEN run the
   elem/global-application loop against the now-self-referenceable `Rc`,
   finally unwrap via `Rc::try_unwrap` (infallible here — nothing else
   holds a reference yet) before returning the plain `WasmInstance`
   `instantiate()`'s own signature still promises. Verify:
   `cargo test -p wasm-runtime`; still no full corpus run expected to
   move yet (cross-MODULE linking, not just within-instance
   `ref.func`/`call_indirect`, needs slice 4's `wasm-conformance` wiring
   to be exercised by the real testsuite's `register`/`invoke`
   directives).

4. **`wasm-conformance` cross-module registry wiring + full corpus
   verification.** `CrossModuleFunction` implements `identity()` (from
   the exporting instance's `func_identities`); `ModuleRegistry`'s
   `Rc<RefCell<WasmInstance>>` convention (already in place) is exactly
   what `build_engine`'s `SelfFunctionResolver` impl needs, so this slice
   is mostly verification, not new machinery. Re-run
   `wasm_conformance_report --write-baseline` and diff programmatically:
   expect `elem.wast`, `linking.wast`, `linking0.wast`, `linking3.wast`'s
   real (non-"not yet supported") tallies to move to fully passing (per
   the addendum's own "confirmed as the sole remaining cause" claim,
   re-verified above against the actual file contents); expect NO other
   file's tally to regress (every non-linking module's own `table.get`/
   `call_indirect`/`ref.func` usage stays entirely WITHIN one instance,
   so `self_resolver` is exercised but never fails for them, and every
   `FuncRefTarget`'s `identity`/`callable` resolve to exactly the same
   function they always did — this fix is additive identity, not a
   behavior change, for the single-instance case).

Each slice's own tests are gated on the PREVIOUS slice's representation
existing, not its full wiring being complete — slice 2 can be fully unit-
tested (with `self_resolver` unset, exercising only the import-forwarding
half of `resolve_function_ref`) before slice 3 gives it anything real to
resolve against, the same "verify the narrow piece before attempting the
wiring" discipline W33/W34 both used.

## Explicitly out of scope for this spec

- **A funcref value passed as a direct argument or return value across a
  cross-module `call`/`call_indirect` boundary (as opposed to being
  stored in a table or global first).** `HostFunction::call`'s signature
  (`&[WasmValue]` in, `Vec<WasmValue>` out) has no room to carry a
  `FuncRefTarget` through it without EITHER breaking `WasmValue`'s
  `Copy`-ness after all (exactly the blast-radius problem this spec's
  whole design avoids) OR growing a parallel side-channel argument list
  mirroring `func_ref_heap`'s own shape across the call boundary itself —
  a real, separable follow-on. Not required by any corpus file in this
  campaign (per the W32-second-slice-era comment already noting "no
  vendored corpus directive passes [a funcref] as a top-level `invoke`
  argument," and no `.wast` file was found, by direct grep across the
  corpus, passing a funcref as a WASM-to-WASM `call` argument across an
  ACTUAL cross-module import either) — flagged here so a future session
  doesn't assume this spec's fix makes EVERY funcref cross-instance path
  sound, only the table/global-storage paths the corpus actually
  exercises.
- **`call_ref`/`return_call_ref`'s cross-instance soundness specifically**
  is audited and included in the call-site table/design above (so the
  FIX, once slices 1-2 land, covers it structurally for free — no extra
  work needed), but is not independently corpus-verified by this
  campaign (no vendored `.wast` directive currently constructs a funcref
  via `ref.func` in one instance, passes it as a LOCAL/global value
  reachable via `call_ref` in a DIFFERENT instance's own code). Noted as
  "covered by construction, not independently proven" rather than
  "verified."
- **`Weak`-based or mark-and-sweep instance-lifetime management** for the
  `Rc<RefCell<WasmInstance>>` cycle risk (see "Security and lifetime
  consideration" above) — documented as a real risk with concrete
  mitigation options for a future embedder, not implemented, since the
  corpus's own usage pattern (batch-registry, freed as a unit) is
  provably unaffected by it.
- **Interning/deduplicating `FuncRefTarget`'s `Rc<dyn HostFunction>`
  across repeated `ref.func`s of the same function within one call** —
  each `ref.func` execution mints a fresh `func_ref_heap` entry (cheap:
  an `Rc::clone` plus a `Vec::push`) even if the exact same function was
  already pushed earlier in the same call. `identity`-based equality
  (§"Call-site audit," last row) is unaffected by this — two entries for
  the same function always compare equal via `identity` regardless of
  how many separate `func_ref_heap` slots they occupy. Revisit only if a
  real performance problem is measured (this repo's own established
  "correctness first, optimize only if measured" precedent — see W34's
  own "Interning/hash-consing for performance" out-of-scope entry for the
  identical reasoning applied to a different subsystem).
- **`Malformed-binary LEB128 under-strictness`** and **`table.wast`'s
  oversized-declared-minimum case** — the addendum's other two open
  items, unrelated to funcref identity, already scoped as separate,
  independent follow-ons in `W07`'s own addendum (items 2 and 3).

## Verification plan (for whatever session implements this)

- After slice 1: `cargo build --workspace` succeeds; `cargo test -p
  wasm-execution -p wasm-types` passes with a byte-for-byte-unchanged
  test count (purely additive types, nothing wired in).
- After slice 2: new unit tests directly on `WasmExecutionContext` proving
  (a) a `ref.func` of an IMPORTED function reuses that import's existing
  identity/callable (no fresh mint), (b) a `ref.func` of a LOCAL function
  with no `self_resolver` installed produces a clean, non-panicking error
  (not a silent wrong answer), (c) `func_ref_heap` handles are correctly
  intra-call-scoped (two sequential `ref.func`s of the SAME function
  produce two DIFFERENT handles but IDENTICAL `identity` values once
  resolved). `cargo test -p wasm-execution` — existing tests unchanged.
- After slice 3: unit tests on `wasm-runtime::instantiate()` proving (a)
  `func_identities` mirrors `tag_identities`'s own already-tested
  "imported adopts verbatim, module-defined mints fresh" contract exactly
  (reuse/adapt that suite's own test shapes), (b) `LocalFunctionRef`
  actually dispatches to the right function body when invoked through a
  raw index unrelated to any export, (c) the `Rc<RefCell<WasmInstance>>`
  two-phase construction (build, wrap, apply elem/globals, unwrap) never
  panics on `Rc::try_unwrap` (guard this explicitly — a bug that leaks an
  extra `Rc` clone during construction would make this panic loudly,
  which is the correct failure mode, not a silent one). `cargo test -p
  wasm-runtime`.
- After slice 4: re-run `cargo run --bin wasm_conformance_report -p
  wasm-conformance -- --write-baseline` and diff programmatically against
  the pre-slice-4 baseline. Expect exactly `elem.wast`/`linking.wast`/
  `linking0.wast`/`linking3.wast` to move (their real, non-"not yet
  supported" failure counts should all reach zero); expect the total
  pass count across the OTHER 253 files to be byte-for-byte unchanged —
  any other file moving (in either direction) means either an
  unaccounted side effect of this change or a mis-scoped fix, and should
  be treated as a signal to stop and re-diagnose before proceeding, not
  waved through as a bonus.
- Run the FULL `cargo test --workspace` after every slice, not just the
  conformance baseline — `wasm-execution`, `wasm-runtime`, and
  `wasm-conformance` all have extensive existing unit-test suites
  (`Table`'s own grow/fill/copy tests, `WasmInstance`'s own tag-identity
  construction tests, `CrossModuleFunction`'s own resolution tests) that
  a careless `Option<u32>` → `Option<FuncRefTarget>` signature change
  could silently break at a call site this document's own audit missed.
- Specifically re-run `call_ref.wast`/`return_call_ref.wast` (the W32
  second-slice corpus wins) after slice 2 lands, even though no NEW
  behavior is expected there — confirm the `func_ref_heap`-handle
  rewiring of `call_ref`'s operand resolution is a genuinely transparent
  refactor for the single-instance case, not an accidental regression.

## Addendum (2026-09-01): fourth slice shipped, epic CLOSED — `elem.wast`/`linking.wast`/`linking0.wast`/`linking3.wast` all real (non-"not yet supported") failures resolved, except one confirmed pre-existing, unrelated bug

Re-verified this document's own citations fresh before writing any code:
`linking.wast`'s "Tables" section (lines 269-354) and `elem.wast`'s
"Element sections across multiple modules change the same table" section
(lines 926-974) both matched this document's own line-number and content
claims exactly. **This slice's real, delivered scope turned out
substantially larger than this document's own §"Recommended slice
decomposition" item 4 predicted** ("mostly verification, not new
machinery") — closely mirroring how slice 3's own major deviation was
found only once its own literal design was actually attempted. Three
distinct, concrete gaps were found and fixed, each documented at length
in the relevant crate's own CHANGELOG (not repeated in full here):

**What shipped** (`wasm-execution` 0.9.89 → 0.9.90, `wasm-runtime` 0.6.26
→ 0.6.27, `wasm-conformance` 0.1.114 → 0.1.115):

1. **The resolution fixup pass** (`wasm-runtime::resolve_all_table_
   funcrefs`, `pub`), called by `wasm-conformance::Executor::
   instantiate_and_register` immediately after a module is wrapped in its
   own permanent `Rc<RefCell<WasmInstance>>` and before either registry
   insertion — exactly the "real, permanent home" slice 3 identified as
   the missing piece. Deviates from this document's own §"Design"/task
   description in two ways, both evidence-backed: (a) walks EVERY table
   an instance can see (imported ones included), not just ones it
   declares — `linking.wast`'s own `$Ot` writes into `$Mt`'s IMPORTED
   table via `$Ot`'s own elem segment, the document's own motivating case,
   which an "owned tables only" fixup would miss entirely; (b)
   deliberately does NOT also resolve funcref-typed globals, a real,
   reproduced regression (`return_call_ref.wast`, `31/31 → 30/31`, "func_
   ref heap limit exceeded") backed out during this slice's own
   verification — see `wasm-conformance`'s own CHANGELOG for the full
   root-cause trace. Also required a table-element-type check
   (`combined_table_element_type`) that this document's own design never
   named: `Table`/`TableStorage` do not track their declared element type
   at runtime, so an externref table's `Raw` entries (a real, opaque
   payload) must never be reinterpreted as function indices — reproduced
   directly against `elem.wast`'s own "Initializing a table with an
   externref-type element segment" test before this check existed.
2. **A real correctness bug in slice 3's own `owner_instance_identity`
   tagging for IMPORTS**, found by this slice's own corpus verification,
   not anticipated by this document's own design: slice 3 tagged every
   import-branch `FuncRefTarget` `owner_instance_identity: None` ("an
   import is dispatchable via local_index in ANY ctx that holds it").
   That claim was accidentally true for every case reachable before this
   slice and FALSE the moment this slice's own fixup pass makes durable
   cross-instance storage of such a target possible — reproduced directly
   against `linking.wast`'s own `$Mt`/`$Ot`/`h` example (a confirmed
   silent wrong answer: `4` instead of `-4`). Fixed in both
   `wasm-execution::WasmExecutionContext::resolve_function_ref` and
   `wasm-runtime::resolve_func_ref_for_instance`'s own import branches
   (`Some(resolving_ctx's_own_instance_identity)`, mirroring the
   LOCAL-function branch exactly). Fixing this naively reproduced a
   SECOND, worse failure (a guaranteed `RefCell` "already borrowed" panic
   on the SAME motivating example, since the corrected dispatch path
   re-enters `CrossModuleFunction::call`, which borrows the SAME instance
   already mutably borrowed by the call this dispatch happens inside of)
   — fixed by giving `wasm_execution::effective_local_index` a second,
   identity-based fallback (scan the CURRENT ctx's own `func_identities`
   for a match before falling through to `callable.call(..)`), which
   both avoids the re-borrow AND is more efficient for exactly this case.
3. **A previously-unnamed "ephemeral trap-discarded instance" case**,
   found only during full-corpus verification, not predicted by this
   document's §"Design" or §"Recommended slice decomposition" at all:
   `linking0.wast`/`linking3.wast` (and 3 of `linking.wast`'s own
   remaining failures) share a shape where an ANONYMOUS module writes
   into a SHARED table via its own active elem segment and only THEN
   traps (a later data segment, or its own start function) — discarding
   the `WasmInstance` entirely, so `wasm-conformance`'s own registry-level
   fixup never gets a chance to run (the module never reaches
   registration). Fixed inside `wasm-runtime::instantiate()` itself: on
   EITHER trap path, before propagating the error, a TEMPORARY
   `Rc<RefCell<WasmInstance>>` (built from the call's own live state, and
   crucially NEVER `Rc::try_unwrap`ed, since the error path doesn't need a
   bare owned value back) is fixed up the same way, then dropped — kept
   alive afterward purely via the `Rc` clones any resolved `FuncRefTarget`
   embeds in the now-permanently-shared table storage. This does NOT
   reintroduce slice 3's own self-referential-cycle problem: that problem
   was specifically about the SUCCESS path needing `try_unwrap` to
   succeed, which this new code path never attempts.
4. **`CrossModuleFunction::identity()`** (`wasm-conformance`), implemented
   from a new field snapshotted from the exporting instance's own
   `func_identities[index]` at `resolve_function` time — exactly as this
   document's own "Call-site audit" table specified, and load-bearing for
   finding (2)'s own identity-based dispatch fallback to work across a
   real cross-module import.
5. **A deliberate, documented non-implementation**: this document's own
   task description (and, by extension, its intended reading of
   `build_engine`'s own "not this slice's job" comment) suggested wiring
   a real `SelfFunctionResolver` into `wasm-conformance`'s own per-call
   execution (not just the one-time fixup pass). A careful re-entrancy
   analysis proved this UNSOUND, not merely unneeded: it would require
   holding `instance_rc.borrow_mut()` for a call's whole duration while
   ALSO giving a resolver a clone of that SAME `Rc` — the first live
   `ref.func` of a LOCAL function during that call (an ordinary, already-
   tested pattern) would panic on a guaranteed double-borrow. Left
   un-implemented, with the concrete `RefCell`-semantics reasoning
   recorded in `wasm-runtime`'s own CHANGELOG — `build_engine`'s
   pre-existing "no Rc available" justification turns out to be doubly
   correct.

**Corpus outcome** (full baseline diff, 257 files, programmatic,
per-file — see `wasm-conformance`'s own CHANGELOG for the complete
table): `linking.wast` (55/65 → **65/65**), `linking0.wast` (0/1 →
**1/1**), and `linking3.wast` (5/6 → **6/6**) all reach FULL pass.
`elem.wast` reaches 18/19 (was 13/19) — the one remaining failure is
CONFIRMED, via `git stash` A/B against the pre-slice-4 baseline, to be a
PRE-EXISTING, W35-UNRELATED bug (an externref table's null-handling after
a cross-module active-elem overwrite, nothing to do with function-
reference identity) — not a partial fix of this spec's own scope. Zero
other file's tally moved. This closes the epic W07's own addendum opened,
with one honestly-reported exception (`elem.wast`'s pre-existing bug,
now separately visible and fixable on its own, unrelated to this spec).

**Security review (post-implementation, before push)**: a dedicated pass
focused on this slice's own three highest-risk properties (re-entrancy/
borrow-panic risk, cycle-driven resource exhaustion, `owner_instance_
identity` correctness) found two real, actionable issues, both fixed
before merge, both now covered by permanent regression tests in
`wasm-conformance`'s own test suite:
- **HIGH — a NEW, deterministic `RefCell` re-entrant-borrow panic** on an
  entirely ORDINARY (non-circular) linking pattern, distinct from the
  pre-existing, accepted "genuinely mutual cross-instance cycle" risk:
  instance `B` calls into instance `A`; `A`'s own `call_indirect`
  dispatches a table entry `B` itself wrote; the corrected `owner_
  instance_identity` dispatch (finding (2) in the "What shipped" list
  above) correctly falls through to `target.callable.call(..)`, which
  re-enters `B`'s OWN, already-borrowed instance -- a guaranteed panic
  with a bare `borrow_mut()`. Fixed: both `LocalFunctionRef::call`
  (`wasm-runtime`) and `CrossModuleFunction::call` (`wasm-conformance`)
  now use `try_borrow_mut`, converting the conflict into a clean
  `TrapError` instead of a process-aborting panic -- this ALSO updates
  the pre-existing "mutual cycle" risk itself: it now traps cleanly too,
  not just the new case this slice introduced.
- **MEDIUM — silent misattribution of a foreign instance's raw table
  write**: the fixup pass's first version scanned every currently-`Raw`
  table entry visible to an instance, assuming any such entry must be
  something THAT instance itself just wrote. False: a LIVE `table.init`/
  `table.set` on some OTHER, already-registered instance sharing the
  table can leave a `Raw` entry there too, at any later point -- a scan
  could not tell that apart from an entry it should resolve in its OWN
  context, silently dispatching to the wrong instance's function. Fixed:
  `WasmInstance` gained `active_elem_writes` (`Vec<(table_index, offset,
  count)>`), populated ONLY at the exact moment an active elem segment is
  applied; the fixup pass now resolves ONLY those exact, recorded slots,
  never scanning.
- **LOW, not fixed, re-characterized instead**: the fixup pass makes an
  `Rc` self-reference cycle (instance → table → resolved local funcref →
  same instance) the COMMON case for any module with a funcref table plus
  an active elem segment referencing its own function, not the rare case
  this section's own original text described. The section's own
  conclusion still holds unchanged: `wasm-conformance`'s `ModuleRegistry`
  is rebuilt fresh per `.wast` file (`Executor::new()`, in `run_wast_
  source`) and dropped as a whole when that file's directives are
  exhausted, so any such cycle is bounded by "at most this one file's own
  instances," never unbounded within a single script -- but the
  `wasm_conformance_report` binary itself processes all 257 corpus files
  in ONE process, so cyclic garbage from an EARLIER file's own registry
  is never reclaimed (no cycle collector in safe Rust) for the remainder
  of that one report run. Bounded in practice by the corpus's own fixed,
  trusted size (the same "not a concern for THIS crate's actual use"
  reasoning this section already applies to the identical class of risk),
  not by any structural fix -- `Weak`-based or mark-and-sweep instance-
  lifetime management remains explicitly out of scope, per this spec's
  own "Explicitly out of scope" section.

**Genuine, documented follow-on gaps this slice does NOT close** (see
each crate's own CHANGELOG for the full reasoning):
- Funcref-typed GLOBALS still lack real cross-instance resolution (the
  identical bug this spec fixed for TABLES) — deliberately left open
  because eagerly resolving them regresses `return_call_ref.wast`'s own
  deep-recursion tests via `global.get`'s unconditional `func_ref_heap`
  minting. Needs a `global.get` fast path mirroring `effective_local_
  index`'s own same-instance optimization before this can be revisited.
  Not confirmed as a cause of any current corpus failure (this document's
  own original text already flagged this as unconfirmed-by-corpus).
- `elem.wast`'s own one remaining `assert_return` failure (externref
  table null-handling after a cross-module overwrite) is a real,
  separate, unfixed bug — out of this spec's own scope (funcref identity
  only), flagged here so a future session doesn't assume this spec's
  fix makes `elem.wast` fully clean.
- The genuinely-mutual cross-instance call cycle `CrossModuleFunction`'s
  own doc comment already documents now traps cleanly (see the security
  review above), rather than panicking — a strict improvement made as a
  side effect of fixing this slice's own new re-entrancy finding, not a
  deliberate general fix for the cycle case itself, which remains a real,
  accepted limitation (a cross-instance call that traps rather than
  completing) that comment already scopes as
  out-of-scope for this whole campaign).
