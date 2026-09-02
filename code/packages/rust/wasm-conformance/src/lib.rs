//! # wasm-conformance
//!
//! Runs the official WebAssembly spec testsuite's `.wast` scripts against
//! this repo's `wasm-execution` interpreter (via `wasm-runtime` and
//! `wasm-wast-parser`) and reports a real, git-pinned conformance baseline.
//! Phase A of the `wasm-execution`-as-good-as-wasmtime arc; see
//! `code/specs/W05-wasm-conformance-harness.md`.
//!
//! ## Pipeline
//!
//! ```text
//! .wast source text
//!   │  wasm_wast_parser::parse_script
//!   ▼
//! Vec<Directive>              (module / register / invoke / assert_* ...)
//!   │  Executor::execute, one directive at a time, in file order
//!   ▼
//! Vec<(DirectiveKind, DirectiveOutcome)>
//!   │  report::tally_results
//!   ▼
//! report::KindTallies         (pass/fail/trap/not_yet_supported per kind)
//! ```
//!
//! ## Why some directives are graded `NotYetSupported`, never `Fail`
//!
//! Grading a directive `Fail` is a claim that `wasm-execution` got something
//! wrong. Specific gaps in this repo's WASM stack make that claim
//! impossible to back up honestly for certain directive kinds — see each
//! one's own doc comment on [`Executor::execute`] for the reasoning, and
//! `code/specs/W05-wasm-conformance-harness.md` section 4.3 for the design
//! rationale. In short:
//! - `assert_invalid` needs an instruction-level type-checker
//!   `wasm-validator` doesn't have yet (`W02` designs it, isn't implemented).
//! - `assert_unlinkable`/any module import: `WasmRuntime::instantiate`
//!   genuinely fails on an unresolved or type-mismatched import (WASM05,
//!   see `code/specs/W10-wasm-real-linking-and-unlinkable.md`), and this
//!   crate's own `RegistryHost` resolves imports from a `register`ed
//!   sibling module for real. It ALSO resolves the well-known `spectest`
//!   host module (W07 addendum 2 item 4) -- the informal fixture module
//!   many upstream `.wast` files assume is implicitly available, providing
//!   `print*`/`global_*`/`table`/`table64`/`memory` -- via a small built-in
//!   [`SpectestModule`], not a real parsed WASM module. An import from any
//!   OTHER unregistered module name still correctly grades
//!   `NotYetSupported`, not `Fail`.
//!
//! `assert_exhaustion` USED to be a third, unconditional case — this
//! crate never ran it at all, because `wasm-execution` had no call-depth
//! guard and the deliberately-unbounded recursion these cases trigger
//! would have overflowed the real host thread stack (an uncatchable
//! process abort, not a gradeable trap). WASM01 added that guard
//! (`wasm_execution`'s `MAX_CALL_DEPTH`), so `assert_exhaustion` is now
//! graded for real, the same way `assert_trap` is.

pub mod report;

use report::{DirectiveKind, DirectiveOutcome};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_execution::{GlobalStorage, HostFunction, HostInterface, LinearMemory, Table, TrapError, V128Bytes, WasmValue};
use wasm_module_parser::WasmModuleParser;
use wasm_runtime::{resolve_all_table_funcrefs, resolve_exported_global_funcrefs, WasmInstance, WasmRuntime};
use wasm_types::{CanonicalGroup, ExternalKind, FuncType, GlobalType, WasmModule};
use wasm_wast_parser::script::{Action, ConstValue, Directive, Expected, F32LaneExpected, F64LaneExpected, ModuleSource};
use wasm_wast_parser::{parse_script, WastParseError};

/// Run every directive in one `.wast` file's source text, in order, and
/// report the outcome of each. This is the crate's single entry point —
/// both the report CLI and the data-driven fixture test call this and
/// nothing lower-level.
pub fn run_wast_source(source: &str) -> Result<Vec<(DirectiveKind, DirectiveOutcome)>, WastParseError> {
    let directives = parse_script(source)?;
    let mut executor = Executor::new();
    Ok(directives
        .into_iter()
        .map(|d| {
            let kind = directive_kind(&d);
            (kind, executor.execute(d))
        })
        .collect())
}

fn directive_kind(d: &Directive) -> DirectiveKind {
    match d {
        Directive::Module { .. } => DirectiveKind::Module,
        // `module definition`/`module instance` (real corpus vendoring
        // pass, `instance.wast`) tally into the SAME `Module` bucket as a
        // plain `(module ...)` -- they're conceptually still "did a module
        // build/instantiate correctly," just split into two directives
        // instead of one; adding a whole new `DirectiveKind` for them would
        // only fragment that one existing category without changing what
        // question it answers.
        Directive::ModuleDefinition { .. } => DirectiveKind::Module,
        Directive::ModuleInstance { .. } => DirectiveKind::Module,
        Directive::Register { .. } => DirectiveKind::Register,
        Directive::Action(_) => DirectiveKind::Action,
        Directive::AssertReturn { .. } => DirectiveKind::AssertReturn,
        Directive::AssertTrap { .. } => DirectiveKind::AssertTrap,
        Directive::AssertExhaustion { .. } => DirectiveKind::AssertExhaustion,
        Directive::AssertInvalid { .. } => DirectiveKind::AssertInvalid,
        Directive::AssertMalformed { .. } => DirectiveKind::AssertMalformed,
        Directive::AssertUnlinkable { .. } => DirectiveKind::AssertUnlinkable,
        Directive::AssertException { .. } => DirectiveKind::AssertException,
    }
}

/// Keyed by `register` name, or `None` for "the current module". Shared
/// (not borrowed) between `Executor` and `RegistryHost` -- see each's own
/// doc comment for why.
type ModuleRegistry = Rc<RefCell<HashMap<Option<String>, Rc<RefCell<WasmInstance>>>>>;

/// A no-op host function for the `spectest` fixture module's `print*`
/// exports (W07 addendum 2 item 4). The real upstream reference
/// interpreter's own `spectest` host prints its arguments to a log for a
/// human running the interpreter interactively -- irrelevant here, since
/// no corpus directive ever asserts on printed output, only on the import
/// resolving at all and the call succeeding with no trap and the right
/// arity of (zero) results. `func_type` is the only thing that varies
/// across the seven `print`/`print_i32`/`print_i64`/`print_f32`/
/// `print_f64`/`print_i32_f32`/`print_f64_f64` exports -- see
/// `SpectestModule::resolve_function`'s own match arms for each one's
/// real upstream-verified signature.
struct SpectestPrintFunction {
    func_type: FuncType,
}

impl HostFunction for SpectestPrintFunction {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }

    fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        Ok(Vec::new())
    }
}

/// The well-known `spectest` fixture module (W07 addendum 2 item 4,
/// `code/specs/W07-wasm-post-mvp-epics.md`): an informal convention the
/// official `WebAssembly/spec` testsuite's OWN interpreter test harness
/// documents (`interpreter/host/spectest.ml` in that repo) -- many
/// upstream `.wast` files assume a host module literally named
/// `"spectest"` is available to import from, without ever `register`ing
/// it themselves in-script (the real interpreter's own test runner
/// registers it once, unconditionally, before running ANY script, via
/// its `-h`/`Import.register` call -- confirmed live against
/// `WebAssembly/spec`'s own source rather than guessed). This struct is
/// the cheap, built-in stand-in for that registration: a handful of
/// fixed exports, backed by real `wasm-execution` primitives
/// (`LinearMemory`/`Table`/`GlobalStorage`), never a real parsed WASM
/// module or a full `WasmInstance` -- exactly the "missing, cheap,
/// one-time host-stub addition to `wasm-conformance`'s own test harness"
/// the addendum predicted this gap would turn out to be, not a genuine
/// interpreter capability gap.
///
/// Every export below is EXACTLY what a live probe of this crate's own
/// vendored corpus (`grep -oh '(import "spectest" "[a-zA-Z0-9_]*"'
/// tests/fixtures/testsuite/*.wast`) shows is actually imported
/// somewhere in the 257-file corpus -- no unused export was added "for
/// completeness." The corpus also imports `spectest.unknown` in five
/// places (`imports.wast`/`imports2.wast`), but every one of those is a
/// deliberate `assert_unlinkable` case whose whole point is that
/// `"unknown"` is NOT a real `spectest` export -- adding it here would
/// make those five directives regress from `Pass` to a real `Fail`
/// (linking would unexpectedly succeed), so it is deliberately absent.
///
/// Values (globals, table/memory limits) are taken verbatim from the
/// real upstream `spectest.ml` source, fetched and read directly rather
/// than guessed:
/// ```ocaml
/// "global_i32", _ -> global (GlobalT (Cons, NumT I32T))   (* value: 666 *)
/// "global_i64", _ -> global (GlobalT (Cons, NumT I64T))   (* value: 666 *)
/// "global_f32", _ -> global (GlobalT (Cons, NumT F32T))   (* value: 666.6 *)
/// "global_f64", _ -> global (GlobalT (Cons, NumT F64T))   (* value: 666.6 *)
/// "table"   -> TableT (I32AT, {min = 10L; max = Some 20L}, (Null, FuncHT))
/// "table64" -> TableT (I64AT, {min = 10L; max = Some 20L}, (Null, FuncHT))
/// "memory"  -> MemoryT (I32AT, {min = 1L; max = Some 2L})
/// ```
/// (`Cons` = immutable/`const`; none of these globals are ever imported
/// as `mut` anywhere in the corpus, matching the real upstream module,
/// which declares them all immutable too.)
///
/// # Why cloning this struct is cheap and shares live state
///
/// `#[derive(Clone)]` here is a SHALLOW, field-wise clone -- and every
/// field already carries its own real storage behind an `Rc`:
/// `LinearMemory`/`Table` hold `Rc<RefCell<..>>` internally (see either's
/// own doc comment in `wasm-execution`, the same W28 fix `RegistryHost::
/// resolve_memory`/`resolve_table` rely on for registered sibling
/// modules), and each global is stored as `Rc<RefCell<GlobalStorage>>`
/// directly, the exact type `HostInterface::resolve_global` already
/// returns. So cloning a `SpectestModule` -- once per `RegistryHost`
/// construction, itself once per module instantiation (`Executor::
/// instantiate_and_register`) -- clones seven cheap `Rc` pointers, NOT
/// seven megabytes of memory or thousands of table slots, and every
/// clone still shares the SAME underlying live storage. This is what
/// makes `spectest.table`/`spectest.memory` behave like a real
/// persistently-registered module across an entire script's multiple
/// `(module ...)` directives (a `table.set`/`memory.grow`/`i32.store`
/// against one importer's copy is visible through every other importer's
/// copy, and through `SpectestModule::new`'s own original) -- `Executor`
/// constructs exactly ONE `SpectestModule` (in `Executor::new`) and every
/// `RegistryHost` for the lifetime of that `Executor` clones FROM that
/// one original, never re-constructing fresh (differently-seeded, but
/// here it wouldn't matter since the values are fixed) storage.
#[derive(Clone)]
struct SpectestModule {
    memory: LinearMemory,
    /// 32-bit-indexed funcref table, min 10 / max 20 (see this struct's
    /// own doc comment for the upstream-verified limits).
    table: Table,
    /// 64-bit-indexed (table64 proposal, W26) counterpart to `table`,
    /// same min/max -- only `table64.wast` imports this one.
    table64: Table,
    global_i32: Rc<RefCell<GlobalStorage>>,
    global_i64: Rc<RefCell<GlobalStorage>>,
    global_f32: Rc<RefCell<GlobalStorage>>,
    global_f64: Rc<RefCell<GlobalStorage>>,
}

impl SpectestModule {
    fn new() -> Self {
        SpectestModule {
            memory: LinearMemory::new(1, Some(2)),
            table: Table::new(10, Some(20)),
            // `expect`: a fixed, hardcoded 10-element table64 stub is
            // nowhere near `wasm-execution`'s own `MAX_TABLE_ELEMENTS`
            // practical-allocation cap -- `new_with_is64` can only ever
            // fail for a caller-supplied size that approaches that cap,
            // which this literal `10` never does.
            table64: Table::new_with_is64(10, Some(20), true).expect("fixed-size spectest table64 stub (10 elements) never exceeds the practical allocation cap"),
            global_i32: Rc::new(RefCell::new(GlobalStorage { value: WasmValue::I32(666), func_ref: None })),
            global_i64: Rc::new(RefCell::new(GlobalStorage { value: WasmValue::I64(666), func_ref: None })),
            global_f32: Rc::new(RefCell::new(GlobalStorage { value: WasmValue::F32(666.6), func_ref: None })),
            global_f64: Rc::new(RefCell::new(GlobalStorage { value: WasmValue::F64(666.6), func_ref: None })),
        }
    }

    /// `None` for any name other than the seven real `print*` exports
    /// (see this struct's own doc comment for why `"unknown"` is
    /// deliberately not among them) -- surfaces as a link failure exactly
    /// like an unresolved import from any other module, which is exactly
    /// right for the corpus's own `spectest.unknown` `assert_unlinkable`
    /// cases.
    fn resolve_function(&self, name: &str) -> Option<Box<dyn HostFunction>> {
        use wasm_types::ValueType::{F32, F64, I32, I64};
        let params = match name {
            "print" => vec![],
            "print_i32" => vec![I32],
            "print_i64" => vec![I64],
            "print_f32" => vec![F32],
            "print_f64" => vec![F64],
            "print_i32_f32" => vec![I32, F32],
            "print_f64_f64" => vec![F64, F64],
            _ => return None,
        };
        Some(Box::new(SpectestPrintFunction { func_type: FuncType { params, results: vec![] } }))
    }

    fn resolve_global(&self, name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        use wasm_types::ValueType::{F32, F64, I32, I64};
        let (value_type, cell) = match name {
            "global_i32" => (I32, &self.global_i32),
            "global_i64" => (I64, &self.global_i64),
            "global_f32" => (F32, &self.global_f32),
            "global_f64" => (F64, &self.global_f64),
            _ => return None,
        };
        // Every real `spectest` global is immutable (`Cons` in the
        // upstream OCaml source quoted in this struct's own doc comment)
        // -- `mutable: false` here matches that exactly, for spec
        // fidelity. No corpus file actually declares an IMPORTED
        // `spectest.global_*` as `(mut ...)`, so nothing in this crate's
        // own graded output currently distinguishes `true` from `false`
        // here.
        Some((GlobalType { value_type, mutable: false }, Rc::clone(cell)))
    }

    fn resolve_memory(&self, name: &str) -> Option<LinearMemory> {
        match name {
            "memory" => Some(self.memory.clone()),
            _ => None,
        }
    }

    fn resolve_table(&self, name: &str) -> Option<Table> {
        match name {
            "table" => Some(self.table.clone()),
            "table64" => Some(self.table64.clone()),
            _ => None,
        }
    }
}

/// A `HostInterface` backed by the `Executor`'s own module registry
/// (WASM05/W10) -- lets a module import a function/memory/table/global
/// from a `register`ed sibling module in the same script, exactly the
/// shape the real corpus's own `assert_unlinkable`/linking cases use
/// (`register "test"` earlier in the script, then `(import "test" ...)`
/// later). ALSO resolves the well-known `spectest` fixture module (W07
/// addendum 2 item 4) via `spectest`, a small built-in [`SpectestModule`]
/// -- see that struct's own doc comment for why this is a cheap host
/// stub, not a real parsed WASM module. Registry lookups are tried FIRST
/// in every `resolve_*` method below, so a script that (hypothetically)
/// `register`ed something under the literal name `"spectest"` would
/// shadow the built-in stub -- matching how the real upstream interpreter
/// shares one namespace between host-registered and script-registered
/// modules. No corpus file in this vendored testsuite actually does this.
/// `resolve_*` returns `None` for any OTHER module name not found in the
/// registry, which correctly surfaces as a link failure without needing a
/// real host for it.
struct RegistryHost {
    /// `Rc<RefCell<..>>`, not a borrowed reference: `HostInterface` (like
    /// any trait consumed as `Box<dyn HostInterface>`) is implicitly
    /// `'static`, so a `RegistryHost` can't hold a borrow of `Executor`'s
    /// own fields -- it needs owned, shared access to the SAME
    /// underlying registry `Executor` itself reads/writes.
    registry: ModuleRegistry,
    /// The built-in `spectest` fixture module (W07 addendum 2 item 4) --
    /// cloned from `Executor`'s own persistent copy (see [`SpectestModule`]'s
    /// doc comment for why cloning it is cheap and shares live storage,
    /// not an independent copy).
    spectest: SpectestModule,
}

impl RegistryHost {
    fn find_export(&self, module_name: &str, name: &str, kind: ExternalKind) -> Option<(Rc<RefCell<WasmInstance>>, u32)> {
        let instance_rc = Rc::clone(self.registry.borrow().get(&Some(module_name.to_string()))?);
        let index = instance_rc
            .borrow()
            .exports
            .iter()
            .find(|(n, k, _)| n == name && *k == kind)
            .map(|(_, _, idx)| *idx)?;
        Some((instance_rc, index))
    }
}

impl HostInterface for RegistryHost {
    fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        // Registry (real `register`ed sibling module) takes precedence --
        // see this struct's own doc comment for why -- falling back to
        // the built-in `spectest` stub (W07 addendum 2 item 4) only when
        // `module_name` is literally `"spectest"` and no real registered
        // module shadows it.
        let Some((instance_rc, index)) = self.find_export(module_name, name, ExternalKind::Function) else {
            return if module_name == "spectest" { self.spectest.resolve_function(name) } else { None };
        };
        let func_type = instance_rc.borrow().func_types.get(index as usize)?.clone();
        // W35 fourth slice: the exporting instance's own already-minted
        // real identity for this function (`instance.func_identities`,
        // the SAME combined function-index space `index` already lives
        // in -- see `WasmInstance::func_identities`'s own doc comment).
        // `unwrap_or(0)` mirrors `HostFunction::identity`'s own documented
        // "0 == no stable identity" default -- unreachable in practice for
        // any function actually returned by a successful `find_export`
        // (every entry in a validated, instantiated module's combined
        // function-index space gets a real identity, imported or not),
        // kept only so an out-of-range index degrades to the same safe
        // default every other pre-`identity()` `HostFunction` already used.
        let identity = instance_rc.borrow().func_identities.get(index as usize).copied().unwrap_or(0);
        let (group_shape, is_final, canonical_type, type_idx) = {
            let instance = instance_rc.borrow();
            match combined_function_type_idx(&instance, index) {
                Some(t) => (
                    instance.module.type_group_shape(t),
                    instance.module.type_subtyping_at(t).is_final,
                    // W34 fourth slice: the EXPORTING module's own already-
                    // computed canonical form for this function's type-
                    // section index, cloned (cheap, `Rc`-backed) at
                    // resolution time -- see `HostFunction::canonical_type`'s
                    // own doc comment. `instance.canonical_types` (NOT
                    // `instance.module`) is the same "cloned once at
                    // `instantiate()` time from `ValidatedModule::
                    // canonical_types()`" field `wasm-runtime`'s own
                    // `WasmExecutionEngine` wiring already uses (W34 third
                    // slice) -- reusing it here means a `CrossModuleFunction`
                    // never needs to re-derive canonicalization itself.
                    instance.canonical_types.get(t as usize).cloned().flatten(),
                    // W34 fourth slice: this function's own flat type-
                    // section index, kept (not just consumed above) so
                    // `HostFunction::canonically_matches` can climb this
                    // function's own module-LOCAL nominal `sub` chain
                    // lazily at match time, re-borrowing `instance` --
                    // see that method's own doc comment for why a declared
                    // supertype relationship can only ever be walked
                    // within the module that declared it.
                    Some(t),
                ),
                None => ((1, 0), true, None, None),
            }
        };
        Some(Box::new(CrossModuleFunction { instance: instance_rc, export_name: name.to_string(), func_type, identity, group_shape, is_final, canonical_type, type_idx }))
    }

    fn resolve_global(&self, module_name: &str, name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        let Some((instance_rc, index)) = self.find_export(module_name, name, ExternalKind::Global) else {
            return if module_name == "spectest" { self.spectest.resolve_global(name) } else { None };
        };
        let instance = instance_rc.borrow();
        let gtype = instance.global_types.get(index as usize)?.clone();
        // Real cross-instance global sharing (real corpus vendoring pass,
        // `instance.wast`'s "Import is not generative" tests / `linking.
        // wast`'s `mut_glob` tests): `.clone()` here clones the `Rc`
        // pointer, NOT the `WasmValue` it points to (mirrors `resolve_
        // memory`/`resolve_table`'s own W28 `.cloned()` fix immediately
        // below/above -- see either's doc comment for the full
        // rationale). Before this fix this line was `*instance.globals.
        // get(index as usize)?` -- a genuine VALUE copy of whatever the
        // global held at THIS EXACT MOMENT -- so the importing instance's
        // own `globals` slot and the exporting instance's original
        // silently diverged the moment either side executed a
        // `global.set`, exactly the shape `LinearMemory`/`Table` had
        // before W28.
        let gval = instance.globals.get(index as usize)?.clone();
        Some((gtype, gval))
    }

    fn resolve_memory(&self, module_name: &str, name: &str) -> Option<LinearMemory> {
        let Some((instance_rc, index)) = self.find_export(module_name, name, ExternalKind::Memory) else {
            return if module_name == "spectest" { self.spectest.resolve_memory(name) } else { None };
        };
        // W28: `.cloned()` here is a genuine SHARED live view, not an
        // independent copy -- `LinearMemory`'s mutable storage
        // (`data`/`current_pages`) lives behind an `Rc<RefCell<..>>` (see
        // that struct's own doc comment in `wasm-execution`), so cloning
        // it clones the `Rc` pointer: a write through the IMPORTING
        // instance's copy is immediately visible through the EXPORTING
        // instance's own `memories[index]`, and vice versa. Before W28,
        // `LinearMemory` derived `Clone` over a plain `data: Vec<u8>`
        // field, making this an independent, byte-for-byte COPY -- a
        // real interpreter correctness bug for the common "one module
        // shares its memory with several consumers" pattern, confirmed
        // and fixed via `linking.wast`'s own `assert_return` tally
        // improving (48/65 -> 54/65) plus five newly-vendored corpus
        // files (`elem.wast`/`linking0.wast`/`linking1.wast`/
        // `linking3.wast`/`load1.wast`) that specifically exercise this.
        //
        // `index` (multi-memory, W16, task #85) selects WHICH of the
        // exporting instance's memories this export refers to -- before
        // W16, an instance had at most one memory, so discarding it was
        // harmless; now it must be used, same as `resolve_global` already
        // does with its own export index.
        let memory = instance_rc.borrow().memories.get(index as usize).cloned();
        memory
    }

    fn resolve_table(&self, module_name: &str, name: &str) -> Option<Table> {
        let Some((instance_rc, index)) = self.find_export(module_name, name, ExternalKind::Table) else {
            return if module_name == "spectest" { self.spectest.resolve_table(name) } else { None };
        };
        // W28: same shared-live-view fix as `resolve_memory` above --
        // `Table`'s `elements` also lives behind an `Rc<RefCell<..>>` now,
        // so `.cloned()` shares storage rather than copying it. Still
        // does NOT give a table entry real cross-instance function
        // IDENTITY -- see `Table`'s own doc comment in `wasm-execution`
        // for the deliberately out-of-scope follow-on that remains.
        // Confirmed (real corpus bug-hunt pass, W-next) to be the root
        // cause of every remaining WRONG-VALUE (not not-yet-supported)
        // `assert_return` failure in `elem.wast`, `linking.wast`, and
        // `linking3.wast` -- NOT `linking0.wast`, whose own one
        // remaining failure turned out to be a completely different,
        // since-fixed bug (active element segments were being applied
        // AFTER data segments instead of before -- see `wasm-runtime::
        // instantiate()`'s own comment on that fix).
        let table = instance_rc.borrow().tables.get(index as usize).cloned();
        table
    }

    fn resolve_tag(&self, module_name: &str, name: &str) -> Option<(FuncType, u64)> {
        let (instance_rc, index) = self.find_export(module_name, name, ExternalKind::Tag)?;
        let instance = instance_rc.borrow();
        // `instance.tags[index]` -- the COMBINED imported+defined tag
        // index space `WasmInstance::tags` builds at instantiation time,
        // NOT `instance.module.tags` (module-DEFINED tags only, like
        // `module.functions`; see that field's own doc comment). Using
        // the module-only field here was a real bug (W22): any
        // exporting module with tag imports of its own would resolve an
        // exported LOCAL tag at the wrong, off-by-import-count slot.
        // Resolved through the EXPORTING module's own type section
        // (`instance.module.types`), not the importing module's -- a
        // different index space entirely (see `wasm-runtime::
        // instantiate`'s own `ImportTypeInfo::Tag` arm, which compares
        // this against ITS OWN `module.types[type_idx]`).
        let type_idx = *instance.tags.get(index as usize)?;
        let func_type = instance.module.types.get(type_idx as usize).cloned()?;
        // The exporting instance's own already-minted canonical identity
        // (W23) for this tag, same index space as `instance.tags` above
        // -- returned alongside the type so the IMPORTING instance adopts
        // it verbatim (see `HostInterface::resolve_tag`'s own doc
        // comment), letting a `throw` in this ("test") instance be
        // caught by a `try_table` in the instance that imports it.
        let identity = *instance.tag_identities.get(index as usize)?;
        Some((func_type, identity))
    }

    /// W33 first slice: the `(rec_group_size, rec_group_position)`
    /// counterpart to `resolve_tag` above, for `wasm-runtime`'s
    /// cross-module import-compatibility guard (`HostInterface::
    /// resolve_tag_group_shape`'s own doc comment). Re-resolves the same
    /// export rather than caching it alongside `resolve_tag`'s own
    /// result -- registry lookups here are simple `HashMap`/`Vec`
    /// indexing, cheap enough that a second lookup is not worth the
    /// complexity of threading a combined return type through
    /// `HostInterface`'s existing, already-stable `resolve_tag` shape.
    fn resolve_tag_group_shape(&self, module_name: &str, name: &str) -> (u32, u32) {
        let Some((instance_rc, index)) = self.find_export(module_name, name, ExternalKind::Tag) else {
            return (1, 0);
        };
        let instance = instance_rc.borrow();
        let Some(&type_idx) = instance.tags.get(index as usize) else {
            return (1, 0);
        };
        instance.module.type_group_shape(type_idx)
    }

    /// W33 first slice: the finality counterpart to `resolve_tag_group_shape`
    /// above — see `HostFunction::is_final`'s own doc comment.
    fn resolve_tag_is_final(&self, module_name: &str, name: &str) -> bool {
        let Some((instance_rc, index)) = self.find_export(module_name, name, ExternalKind::Tag) else {
            return true;
        };
        let instance = instance_rc.borrow();
        let Some(&type_idx) = instance.tags.get(index as usize) else {
            return true;
        };
        instance.module.type_subtyping_at(type_idx).is_final
    }
}

/// The type-SECTION index (into `instance.module.types`) for the
/// COMBINED-index-space function `index` in `instance` (W33 first slice)
/// -- `index` follows the same "imports first, then module-defined"
/// convention `instance.func_types` itself uses. Mirrors `wasm-validator`'s
/// own `build_module_context`'s identical imports-first-then-declared
/// resolution for `func_type_indices`.
fn combined_function_type_idx(instance: &WasmInstance, index: u32) -> Option<u32> {
    let imported_function_count = instance.module.imports.iter().filter(|i| i.kind == ExternalKind::Function).count() as u32;
    if index < imported_function_count {
        instance
            .module
            .imports
            .iter()
            .filter(|i| i.kind == ExternalKind::Function)
            .nth(index as usize)
            .and_then(|imp| match &imp.type_info {
                wasm_types::ImportTypeInfo::Function(t) => Some(*t),
                _ => None,
            })
    } else {
        instance.module.functions.get((index - imported_function_count) as usize).copied()
    }
}

/// A resolved cross-module function import (WASM05/W10): calling it
/// re-enters `WasmRuntime::call_typed` against the CALLEE's own instance
/// state (its own memory/tables/globals/func_bodies), not the caller's --
/// reusing already-tested machinery rather than new interpreter
/// internals. `HostFunction::call`'s `memory` parameter (normally the
/// *caller's* memory, used by e.g. WASI's `fd_write` to read/write guest
/// pointers) is unused here for exactly that reason: a cross-module call
/// operates entirely on the callee's own state, not the caller's.
///
/// Known limitation, not silently allowed to corrupt anything: a
/// MUTUAL/circular cross-instance call (this function's own callee
/// instance, reached via a DIFFERENT import, calls back into the
/// original caller instance) traps cleanly on a `RefCell` re-borrow
/// conflict (W35 fourth slice, security-review finding: `call`'s own
/// `try_borrow_mut` -- see its doc comment; PRE-fourth-slice this was a
/// bare `borrow_mut()` panic instead) -- a `TrapError`, not a
/// memory-safety issue, and not a process abort. None of the corpus
/// vendored so far is circular.
struct CrossModuleFunction {
    instance: Rc<RefCell<WasmInstance>>,
    export_name: String,
    func_type: FuncType,
    /// W35 fourth slice (`code/specs/W35-wasm-cross-instance-function-
    /// identity.md`): this function's own real, process-wide-unique
    /// identity, snapshotted from the EXPORTING instance's own
    /// `func_identities[index]` at `resolve_function` time (the same
    /// combined function-index space `combined_function_type_idx` already
    /// resolves `index` against, immediately below) -- see
    /// `HostFunction::identity`'s own doc comment. This is what lets an
    /// IMPORTING module's own `WasmInstance::func_identities` construction
    /// loop (`instantiate()`, mirroring `tag_identities`'s "imported
    /// adopts the exporter's identity verbatim" rule) give the imported
    /// function the SAME real identity the exporting instance already
    /// minted for it, rather than a fresh, unrelated one.
    identity: u64,
    /// W33 first slice: this function's own `(rec_group_size,
    /// rec_group_position)`, computed once at `resolve_function` time
    /// (see `combined_function_type_idx`) — see `HostFunction::
    /// type_group_shape`'s own doc comment.
    group_shape: (u32, u32),
    /// W33 first slice: this function's own declared finality, computed
    /// alongside `group_shape` — see `HostFunction::is_final`'s own doc
    /// comment.
    is_final: bool,
    /// W34 fourth slice: this function's own real canonical type-group
    /// identity, computed alongside `group_shape`/`is_final` above (from
    /// the EXPORTING instance's own `canonical_types` table) — see
    /// `HostFunction::canonical_type`'s own doc comment. `None` whenever
    /// the exporting module's own type-section index wasn't canonicalized
    /// (out of range, or the type it belongs to has an internally-
    /// inconsistent `rec`-group shape — see `wasm_types::canonicalize_types`'s
    /// own doc comment), in which case `wasm-runtime`'s import check falls
    /// back to the pre-existing three-part conservative guard, unchanged.
    canonical_type: Option<(Rc<CanonicalGroup>, u32)>,
    /// W34 fourth slice: this function's own flat type-section index in
    /// the EXPORTING instance's own module, kept so `canonically_matches`
    /// can climb that module's own `type_subtyping` chain lazily (via a
    /// fresh borrow of `instance`) rather than needing the whole chain
    /// eagerly precomputed here. `None` in exactly the same case
    /// `canonical_type` is `None` (an unresolvable combined type index).
    type_idx: Option<u32>,
}

impl HostFunction for CrossModuleFunction {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }

    /// **W35 fourth slice, security-review finding**: `try_borrow_mut`,
    /// not a bare `borrow_mut()` -- see `wasm_runtime::LocalFunctionRef::
    /// call`'s own doc comment for the full rationale (a real, reproduced,
    /// non-circular re-entrant-dispatch panic this slice's own fixup pass
    /// made newly reachable: instance `B` calls into instance `A`, whose
    /// `call_indirect` dispatches a stored funcref pointing back to `B`
    /// itself, which is still mutably borrowed by the outer call). This
    /// struct's own doc comment already names the SEPARATE, pre-existing,
    /// accepted "genuinely mutual cross-instance call cycle" panic risk;
    /// this fix is specifically about the NEW, non-cyclic case this
    /// slice's own table-fixup pass introduced.
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let mut instance = self.instance.try_borrow_mut().map_err(|_| {
            TrapError::new(
                "cross-instance call failed: the target instance is already executing (a re-entrant \
                 call back into an instance already on the call stack) -- this trap, not a panic, is \
                 the correct failure mode for this shape"
                    .to_string(),
            )
        })?;
        WasmRuntime::new().call_typed(&mut instance, &self.export_name, args)
    }

    /// W35 fourth slice: see this struct's own `identity` field doc
    /// comment.
    fn identity(&self) -> u64 {
        self.identity
    }

    fn type_group_shape(&self) -> (u32, u32) {
        self.group_shape
    }

    fn is_final(&self) -> bool {
        self.is_final
    }

    fn canonical_type(&self) -> Option<(Rc<CanonicalGroup>, u32)> {
        self.canonical_type.clone()
    }

    fn canonically_matches(&self, target: &(Rc<CanonicalGroup>, u32), budget: &mut wasm_types::CrossModuleComparisonBudget) -> bool {
        let Some(type_idx) = self.type_idx else {
            return false;
        };
        let instance = self.instance.borrow();
        wasm_types::canonical_chain_reaches(&instance.module.type_subtyping, &instance.canonical_types, type_idx, Some(target), budget)
    }
}

/// Walks a script's directives in order, maintaining the module registry
/// `invoke`/`register` need to resolve "the current module" vs. a
/// previously `register`ed one.
struct Executor {
    runtime: WasmRuntime,
    /// Keyed by `register` name, or `None` for "the current module" (the
    /// most recently processed `(module ...)` directive). A script that
    /// never uses `register` only ever touches the `None` entry.
    ///
    /// `Rc<RefCell<..>>` VALUES, not an owned `WasmInstance`, because a
    /// `register`ed module IS the same live instance as "current" -- same
    /// memory, same globals, same subsequent mutations -- not an
    /// independent copy (and `WasmInstance` isn't `Clone` anyway: it holds
    /// a `Box<dyn HostFunction>`). The MAP itself is also wrapped in
    /// `Rc<RefCell<..>>` so `RegistryHost` (WASM05/W10) can hold shared,
    /// owned access to the exact same registry `Executor` reads/writes,
    /// satisfying `HostInterface`'s implicit `'static` bound without a
    /// borrow.
    registry: ModuleRegistry,
    /// Set (W14) whenever the current `(module ...)` directive did NOT
    /// leave a live instance registered under `None` -- for any of 3
    /// reasons that are genuine capability gaps, not bugs: the module's
    /// own instruction stream failed to BUILD (an opcode this repo
    /// doesn't implement yet, `wasm-wast-parser`'s
    /// `Directive::Module(Err(_))`), or it failed to INSTANTIATE because
    /// an import references `spectest` or another module this crate's
    /// `RegistryHost` doesn't know about (WASM05/W10's link-failure
    /// path). Any directive run against "the current module" while this
    /// is `Some` is graded `NotYetSupported`, not `Fail`/`Trap` -- a wrong
    /// answer here would be "we haven't built/linked this yet," not "the
    /// interpreter is broken." (Renamed and broadened from the
    /// link-failure-only `current_link_failed` -- a real, genuine
    /// structural-validation failure or instantiation TRAP is a
    /// different, non-capability-gap kind of failure and does NOT set
    /// this field, even though it also leaves no live instance
    /// registered; see the `Directive::Module` match arm below for how
    /// the registry's `None` slot is kept consistent across all 4
    /// possible outcomes regardless.)
    current_module_status: Option<String>,
    /// `(module definition $M ...)` bodies (real corpus vendoring pass,
    /// `instance.wast`), keyed by `$M` -- stored as a plain, un-instantiated
    /// [`WasmModule`] template. A later `(module instance $I $M)` clones
    /// this entry and validates+instantiates the CLONE, so instantiating
    /// the same `$M` twice (`instance.wast`'s "Instantiation is generative"
    /// tests) gives two independent live instances, each with its own
    /// fresh globals/tables/memories -- exactly what a `WasmModule`'s own
    /// `Clone` + a fresh `instantiate()` call naturally provides, with no
    /// extra bookkeeping needed.
    definitions: HashMap<String, WasmModule>,
    /// A generalization of `current_module_status` to ANY registry key
    /// (not just `None`/"the current module"): whenever a
    /// `Directive::Module`/`ModuleDefinition`/`ModuleInstance` with an
    /// explicit `$id` hits a genuine capability gap (fails to build, or
    /// fails to link/resolve its definition) instead of leaving a live
    /// instance registered under that `$id`, the reason is recorded here
    /// too. `Directive::Register`'s own "target not found" fallback checks
    /// this map before falling back to a hard `Fail` -- so `(register
    /// "name" $id)` naming an `$id` that never built for a real capability-
    /// gap reason (the real corpus's own `instance.wast`/`type-rec.wast`:
    /// `(register "I1" $I1)` where `$I1` came from an unsupported `(module
    /// instance ...)`, or `(register "M" $M)` where `$M` used a `(rec ...)`
    /// type group this crate can't build yet) is graded the same honest
    /// `NotYetSupported` a directive against "the current module" already
    /// gets, not a `Fail` that looks like a genuine harness/script bug.
    unavailable_reasons: HashMap<Option<String>, String>,
    /// The ONE built-in `spectest` fixture module (W07 addendum 2 item 4)
    /// for this `Executor`'s entire run -- constructed once here, then
    /// cloned (cheaply -- see [`SpectestModule`]'s own doc comment) into
    /// every `RegistryHost` this `Executor` builds, so `spectest.table`/
    /// `spectest.memory` behave like a real persistently-registered
    /// module shared across every `(module ...)` directive in the
    /// script, exactly matching the real upstream interpreter's own
    /// "register spectest once, before running the script" behavior.
    spectest: SpectestModule,
}

impl Executor {
    fn new() -> Self {
        Executor {
            runtime: WasmRuntime::new(),
            registry: Rc::new(RefCell::new(HashMap::new())),
            current_module_status: None,
            definitions: HashMap::new(),
            unavailable_reasons: HashMap::new(),
            spectest: SpectestModule::new(),
        }
    }

    /// Shared tail of `Directive::Module`'s success path and `Directive::
    /// ModuleInstance` (real corpus vendoring pass, `instance.wast`):
    /// validate `module`, instantiate it fresh, and register the live
    /// instance under `id` -- `set_current` additionally registers it under
    /// `None` ("the current module"), which a plain `(module ...)`
    /// directive does but a NAMED `(module instance $I $M)` deliberately
    /// does NOT (an instance is only ever reachable by its own `$I`, never
    /// implicitly "current" -- matching the real corpus's own `instance.
    /// wast`, which always addresses `$I1`/`$I2`/`$I` by name).
    ///
    /// Security note (flagged in this feature's own review): a single
    /// `(module definition $M ...)` can now be instantiated an arbitrary
    /// number of times via repeated short `(module instance $I_k $M)`
    /// lines, each triggering a REAL, eager allocation (this crate's
    /// `instantiate()` has never been lazy about memory/table sizing --
    /// see `memory64.wast`/`table64.wast`'s own already-existing boundary-
    /// case tests). That's a cheaper allocation-amplification primitive
    /// than existed before this feature (previously, triggering N real
    /// instantiations needed N full module bodies, not N one-line
    /// references to one shared template). Not a concern for THIS crate's
    /// actual use (`run_wast_source` only ever runs the pinned, trusted
    /// `WebAssembly/testsuite` corpus fetched by `fetch_testsuite.py`, never
    /// arbitrary/untrusted `.wast` text) -- but if this parser/executor is
    /// ever pointed at untrusted `.wast` input, cap either the number of
    /// `Directive::ModuleInstance` directives or cumulative allocated
    /// memory/table bytes per script before doing so.
    fn instantiate_and_register(&mut self, module: &WasmModule, id: Option<String>, set_current: bool) -> DirectiveOutcome {
        match self.runtime.validate(module) {
            Err(e) => DirectiveOutcome::Fail(format!("module failed structural validation: {e}")),
            Ok(validated) => {
                let host = RegistryHost { registry: Rc::clone(&self.registry), spectest: self.spectest.clone() };
                match WasmRuntime::with_host(Box::new(host)).instantiate(&validated) {
                    Ok(instance) => {
                        let instance = Rc::new(RefCell::new(instance));
                        // W35 fourth slice (`code/specs/
                        // W35-wasm-cross-instance-function-identity.md`):
                        // resolve every `TableElement::Raw` entry THIS
                        // instance's own `instantiate()` call just wrote
                        // into any table it can see (owned or imported),
                        // now that a real, PERMANENT `Rc<RefCell<
                        // WasmInstance>>` finally exists for it -- see
                        // `wasm_runtime::resolve_all_table_funcrefs`'s own
                        // doc comment for the full rationale, including
                        // why this was originally TABLES only (a broader
                        // attempt at globals too caused a real, reproduced
                        // regression this slice found and backed out of).
                        // Run BEFORE either registry insertion below, so
                        // no subsequent directive (another module's
                        // `import`, or this module's own `register`) can
                        // ever observe a not-yet-fixed-up table.
                        if let Err(e) = resolve_all_table_funcrefs(&instance) {
                            return DirectiveOutcome::Trap(format!(
                                "internal error: post-instantiation cross-instance funcref resolution failed: {e}"
                            ));
                        }
                        // W35 fifth slice: the analogous fixup for
                        // EXPORTED funcref-typed GLOBALS specifically --
                        // see `wasm_runtime::resolve_exported_global_
                        // funcrefs`'s own doc comment for why "exported
                        // only" (not every module-defined funcref global,
                        // which is exactly the shape of the earlier
                        // regression the comment above refers to) is both
                        // sufficient to fix `elem.wast`'s own
                        // "Initializing a table with imported funcref
                        // global" case and provably safe against
                        // reintroducing that regression. Same placement
                        // rationale as the table fixup immediately above:
                        // must run before this instance is ever
                        // `register`ed/imported from.
                        if let Err(e) = resolve_exported_global_funcrefs(&instance) {
                            return DirectiveOutcome::Trap(format!(
                                "internal error: post-instantiation cross-instance global funcref resolution failed: {e}"
                            ));
                        }
                        if set_current {
                            self.registry.borrow_mut().insert(None, Rc::clone(&instance));
                        }
                        if let Some(id) = id {
                            self.unavailable_reasons.remove(&Some(id.clone()));
                            self.registry.borrow_mut().insert(Some(id), instance);
                        }
                        DirectiveOutcome::Pass
                    }
                    Err(e) if is_link_error(&e) => {
                        let reason = format!("module failed to link (real capability gap, not a bug): {e}");
                        if set_current {
                            self.current_module_status = Some(reason.clone());
                            self.unavailable_reasons.insert(None, reason.clone());
                        }
                        if let Some(id) = &id {
                            self.unavailable_reasons.insert(Some(id.clone()), reason.clone());
                        }
                        DirectiveOutcome::NotYetSupported(reason)
                    }
                    Err(e) => DirectiveOutcome::Trap(format!("instantiation trapped: {e}")),
                }
            }
        }
    }

}

impl Executor {
    fn execute(&mut self, directive: Directive) -> DirectiveOutcome {
        match directive {
            Directive::Module { id, result: module_result } => {
                // W14: clear BOTH the status flag and the registry's
                // "current module" slot unconditionally, before looking at
                // which of this directive's 4 possible outcomes actually
                // happened -- a module that fails to build/validate/link/
                // instantiate must never leave a STALE PREVIOUS module
                // registered as "current" (a real, previously rarely-
                // exercised bug: only the success path ever wrote this
                // slot, so a broken module used to silently inherit
                // whatever instance came before it).
                self.current_module_status = None;
                self.unavailable_reasons.remove(&None);
                self.registry.borrow_mut().remove(&None);
                let module = match *module_result {
                    Err(e) => {
                        let reason =
                            format!("module failed to parse/build (real capability gap, not a bug): {e}");
                        self.current_module_status = Some(reason.clone());
                        self.unavailable_reasons.insert(None, reason.clone());
                        if let Some(id) = &id {
                            self.unavailable_reasons.insert(Some(id.clone()), reason.clone());
                        }
                        return DirectiveOutcome::NotYetSupported(reason);
                    }
                    Ok(module) => module,
                };
                // Task #93 (linking.wast): also registers the live
                // instance under the module's own `$id`, if it has one --
                // the SAME instance as "current" (`set_current: true`), so
                // a LATER `(invoke $id ...)`/`(register "M" $id)` can
                // resolve back to this specific module even after other
                // `(module ...)` directives have since become "the current
                // module".
                self.instantiate_and_register(&module, id, true)
            }

            Directive::ModuleDefinition { id, result: module_result } => {
                if let Some(id) = &id {
                    self.unavailable_reasons.remove(&Some(id.clone()));
                }
                match *module_result {
                    Err(e) => {
                        let reason = format!(
                            "module definition failed to parse/build (real capability gap, not a bug): {e}"
                        );
                        if let Some(id) = id {
                            self.unavailable_reasons.insert(Some(id), reason.clone());
                        }
                        DirectiveOutcome::NotYetSupported(reason)
                    }
                    // A "definition" is validated (a real structural bug in
                    // it is a genuine `Fail`, exactly like a plain `(module
                    // ...)`), but deliberately NOT instantiated -- only a
                    // later `(module instance $I $M)` naming it does that,
                    // and possibly more than once. See `Self::definitions`'
                    // own doc comment for why storing the raw template (not
                    // a `ValidatedModule`/live instance) is what makes that
                    // "instantiate twice, independently" shape possible. An
                    // ANONYMOUS definition (`id: None` -- see `Directive::
                    // ModuleDefinition`'s own doc comment) is validated the
                    // same way but has nowhere to be stored, since nothing
                    // could ever name it in a later `module instance`.
                    Ok(module) => match self.runtime.validate(&module) {
                        Err(e) => {
                            DirectiveOutcome::Fail(format!("module definition failed structural validation: {e}"))
                        }
                        Ok(_) => {
                            if let Some(id) = id {
                                self.definitions.insert(id, module);
                            }
                            DirectiveOutcome::Pass
                        }
                    },
                }
            }

            Directive::ModuleInstance { id, definition_id } => {
                let module = match self.definitions.get(&definition_id).cloned() {
                    Some(m) => m,
                    // The named definition never became available -- either
                    // it doesn't exist at all (a genuine script bug), or
                    // (the real corpus's own case) it hit a capability gap
                    // recorded by `ModuleDefinition`'s own `Err` arm above.
                    // Either way this is a capability gap FROM THIS
                    // DIRECTIVE's perspective too: it can't instantiate a
                    // definition that was never built, so it propagates the
                    // same reason (falling back to a generic one if the
                    // definition simply was never declared) rather than
                    // failing hard.
                    None => {
                        let reason = self.unavailable_reasons.get(&Some(definition_id.clone())).cloned().unwrap_or_else(|| {
                            format!(
                                "module instance: no definition registered as ${definition_id} \
                                 (real capability gap, not a bug)"
                            )
                        });
                        if let Some(instance_id) = &id {
                            self.unavailable_reasons.insert(Some(instance_id.clone()), reason.clone());
                        }
                        return DirectiveOutcome::NotYetSupported(reason);
                    }
                };
                // `set_current: false` -- see `instantiate_and_register`'s
                // own doc comment for why a named instance never becomes
                // "the current module".
                self.instantiate_and_register(&module, id, false)
            }

            Directive::Register { name, module_name } => {
                // `module_name` (task #93/linking.wast): a `$id` referencing
                // an earlier `(module $id ...)`, now resolvable since that
                // id is captured at parse time and registered in
                // `Directive::Module`'s own arm above. Falls back to "the
                // current module" (`None`) when `module_name` is absent --
                // the plain `(register "name")` form real WAT scripts also
                // use.
                let key = module_name;
                let target = self.registry.borrow().get(&key).cloned();
                match target {
                    Some(target) => {
                        self.registry.borrow_mut().insert(Some(name), target);
                        DirectiveOutcome::Pass
                    }
                    // W14, generalized beyond just "the current module"
                    // (real corpus vendoring pass, `instance.wast`/`type-
                    // rec.wast`): if there's no live instance under `key`
                    // BECAUSE building/linking/instantiating it hit a
                    // genuine capability gap -- tracked in
                    // `unavailable_reasons` for EITHER the `None`/"current
                    // module" key (unchanged from before) OR an explicit
                    // `$id` key (new: e.g. `$I1` from an unsupported
                    // `(module instance ...)`, or `$M` from a `(rec ...)`
                    // type group this crate can't build yet) -- that gap
                    // should propagate as `NotYetSupported` here too, not
                    // get flattened into a hard `Fail` that looks like a
                    // real test-script bug. Only when `key` has NEVER been
                    // the target of any module directive at all (capability
                    // gap or otherwise) is this a genuine script-level bug.
                    None => match self.unavailable_reasons.get(&key) {
                        Some(reason) => DirectiveOutcome::NotYetSupported(reason.clone()),
                        None if key.is_none() => {
                            DirectiveOutcome::Fail("register: no current module to register".to_string())
                        }
                        None => DirectiveOutcome::Fail(format!("register: no module registered as {key:?}")),
                    },
                }
            }

            Directive::Action(action) => match self.run_action(&action) {
                Ok(_) => DirectiveOutcome::Pass,
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
                Err(ActionError::Trap(m)) => DirectiveOutcome::Trap(m),
                Err(ActionError::Exception(m)) => DirectiveOutcome::Trap(m),
            },

            Directive::AssertReturn { action, expected } => match self.run_action(&action) {
                Ok((results, v128_bytes)) => {
                    if results.len() != expected.len() {
                        DirectiveOutcome::Fail(format!(
                            "expected {} result(s), got {}",
                            expected.len(),
                            results.len()
                        ))
                    } else {
                        let mismatch = results
                            .iter()
                            .zip(v128_bytes.iter())
                            .zip(expected.iter())
                            .find(|((r, vb), e)| !value_matches_expected(r, **vb, e));
                        match mismatch {
                            None => DirectiveOutcome::Pass,
                            Some(((r, _), e)) => DirectiveOutcome::Fail(format!("expected {e:?}, got {r:?}")),
                        }
                    }
                }
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
                Err(ActionError::Trap(m)) => DirectiveOutcome::Fail(format!("expected a return value but action trapped: {m}")),
                // W21: an uncaught WASM exception is just as much "not a
                // return value" as an ordinary trap is -- same Fail shape,
                // distinct wording so a reader can tell which happened.
                Err(ActionError::Exception(m)) => {
                    DirectiveOutcome::Fail(format!("expected a return value but action raised an uncaught exception: {m}"))
                }
            },

            Directive::AssertTrap { action, .. } => match self.run_action(&action) {
                // The official testsuite's own reference runners do not
                // match trap MESSAGE text against `message` -- only that
                // some trap occurred. Matching this repo's error strings
                // against the spec's human-oriented ones would be testing
                // string formatting, not conformance.
                Ok(_) => DirectiveOutcome::Fail("expected a trap, action returned normally".to_string()),
                Err(ActionError::Trap(_)) => DirectiveOutcome::Pass,
                // W21: a trap and an uncaught exception are different
                // outcomes per the real spec (`try_table` never catches a
                // trap, only an exception) -- `assert_trap` must not
                // accept an exception in place of a real trap.
                Err(ActionError::Exception(m)) => {
                    DirectiveOutcome::Fail(format!("expected a trap, action raised an uncaught exception instead: {m}"))
                }
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
            },

            // `wasm-execution` (WASM01) now has a call-depth guard
            // (`MAX_CALL_DEPTH`), so the deliberately unbounded recursion
            // these cases exist to trigger traps cleanly with a real "call
            // stack exhausted" error instead of overflowing the host
            // stack -- graded the same way `assert_trap` is (any trap
            // counts; the spec's own reference runners don't strict-match
            // trap message text either).
            Directive::AssertExhaustion { action, .. } => match self.run_action(&action) {
                Ok(_) => DirectiveOutcome::Fail("expected exhaustion, action returned normally".to_string()),
                Err(ActionError::Trap(_)) => DirectiveOutcome::Pass,
                Err(ActionError::Exception(m)) => {
                    DirectiveOutcome::Fail(format!("expected exhaustion, action raised an uncaught exception instead: {m}"))
                }
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
            },

            Directive::AssertInvalid { module, .. } => self.grade_assert_invalid(module),
            Directive::AssertMalformed { module, .. } => self.grade_assert_malformed(module),
            Directive::AssertUnlinkable { module, .. } => self.grade_assert_unlinkable(module),

            // W21 (exceptions proposal): `assert_exception` passes ONLY on
            // a real uncaught exception -- neither a normal return NOR an
            // ordinary trap satisfies it (see `ActionError::Exception`'s
            // own doc comment for why the two are graded as genuinely
            // different outcomes, not interchangeable "something went
            // wrong" signals).
            Directive::AssertException { action } => match self.run_action(&action) {
                Ok(_) => DirectiveOutcome::Fail("expected an uncaught exception, action returned normally".to_string()),
                Err(ActionError::Exception(_)) => DirectiveOutcome::Pass,
                Err(ActionError::Trap(m)) => {
                    DirectiveOutcome::Fail(format!("expected an uncaught exception, action trapped instead (ordinary trap, not an exception): {m}"))
                }
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
            },
        }
    }

    /// `assert_invalid` expects the module to be *rejected*. This repo's
    /// validator only checks module-level structure (index bounds, unique
    /// exports, segment validity) -- not instruction-level types. So:
    /// - The module fails to even build (a text-level parse error) or fails
    ///   structural validation -> we DID correctly reject it -> `Pass`
    ///   (structural rejection is still a legitimate form of "invalid",
    ///   even when the specific case was designed to probe a type error).
    /// - The module builds and validates fine -> we can't tell this case
    ///   apart from a valid module without a type-checker -> `NotYetSupported`.
    fn grade_assert_invalid(&self, module: ModuleSource) -> DirectiveOutcome {
        match build_module(module) {
            Err(_) => DirectiveOutcome::Pass,
            Ok(built) => match self.runtime.validate(&built) {
                Err(_) => DirectiveOutcome::Pass,
                Ok(_) => DirectiveOutcome::NotYetSupported(
                    "no instruction-level type-checker; module structurally validates".to_string(),
                ),
            },
        }
    }

    /// `assert_malformed` expects the module to fail to even PARSE (a
    /// syntax-level defect, distinct from `assert_invalid`'s semantic
    /// defect). For the `binary` variant, `wasm-module-parser` already has
    /// real error paths (bad magic, LEB128 overflow, truncation, bad
    /// section order) -- graded for real. For the `quote` (text) variant,
    /// this repo's own `wasm-wast-parser` can now attempt the same
    /// re-parse: rejecting it is a real `Pass`; but if it unexpectedly
    /// accepts the text, we can't tell whether that specific case needed
    /// type-checking knowledge our parser doesn't have -- `NotYetSupported`,
    /// never `Fail`, since blaming the wrong layer would send a future
    /// reader chasing a bug in the wrong crate.
    fn grade_assert_malformed(&self, module: ModuleSource) -> DirectiveOutcome {
        match module {
            ModuleSource::Binary(bytes) => match WasmModuleParser::parse(&bytes) {
                Err(_) => DirectiveOutcome::Pass,
                // A module that parses can still be malformed by a rule this
                // crate's decoder doesn't check at PARSE time but its
                // instruction-level type-checker (`wasm-validator`) DOES walk
                // through, incidentally, while decoding operands -- e.g. a
                // memop's align flags encoded with the reserved top bit set
                // (`align.wast`'s "malformed memop flags" cases) decode as an
                // absurdly large LEB128 value that `wasm-validator`'s existing
                // `align > max_align` check already rejects, just under a
                // different error message than the spec's own "malformed
                // memop flags" wording. Real bug, found via a prioritization
                // scan after task #80 (PR #11844): this path never asked
                // `wasm-validator` at all, so any malformed-ness it happened
                // to already catch was invisible here. Same "outcome
                // category, not the specific reason" precedent as
                // `grade_assert_unlinkable` below -- the spec's malformed-vs-
                // invalid split is about WHERE a real engine catches the
                // error, not a promise this harness must replicate that exact
                // pipeline stage.
                Ok(built) => match self.runtime.validate(&built) {
                    Ok(_) => DirectiveOutcome::Fail(
                        "binary module parsed but should have been rejected as malformed".to_string(),
                    ),
                    Err(_) => DirectiveOutcome::Pass,
                },
            },
            ModuleSource::Quote(bytes) => match std::str::from_utf8(&bytes) {
                Err(_) => DirectiveOutcome::Pass,
                Ok(text) => match wasm_wast_parser::parse_module(text) {
                    Err(_) => DirectiveOutcome::Pass,
                    Ok(_) => DirectiveOutcome::NotYetSupported(
                        "text parsed without error; this case may need type-checking knowledge to reject".to_string(),
                    ),
                },
            },
            ModuleSource::Text(_) => DirectiveOutcome::NotYetSupported(
                "assert_malformed module captured as an already-built form, not raw text/bytes".to_string(),
            ),
        }
    }

    /// `assert_unlinkable` expects the module to fail to be usable at all
    /// -- whether because it doesn't even build, doesn't structurally
    /// validate, or (WASM05/W10) genuinely fails to LINK. Like
    /// `grade_assert_invalid`'s own precedent: the harness only needs the
    /// OUTCOME category (rejected) to match, not the specific reason --
    /// `assert_trap`'s grading doesn't string-match trap text either.
    fn grade_assert_unlinkable(&self, module: ModuleSource) -> DirectiveOutcome {
        match build_module(module) {
            Err(_) => DirectiveOutcome::Pass,
            Ok(built) => match self.runtime.validate(&built) {
                Err(_) => DirectiveOutcome::Pass,
                Ok(validated) => {
                    let host = RegistryHost { registry: Rc::clone(&self.registry), spectest: self.spectest.clone() };
                    match WasmRuntime::with_host(Box::new(host)).instantiate(&validated) {
                        Ok(_) => DirectiveOutcome::Fail("module linked successfully; expected unlinkable".to_string()),
                        Err(_) => DirectiveOutcome::Pass,
                    }
                }
            },
        }
    }

    /// Returns each result `WasmValue` alongside its resolved v128 bytes
    /// (`Some` only for a `WasmValue::V128` result -- see
    /// `wasm_execution::V128Bytes`'s own doc comment for why a bare
    /// post-call `V128` handle can't be compared directly: the engine
    /// that produced it, and the heap the handle indexes into, are both
    /// gone by the time this function returns). `Action::Get` (a global
    /// read, not a call) never produces resolved v128 bytes -- see this
    /// method's own note on that arm.
    fn run_action(&mut self, action: &Action) -> Result<(Vec<WasmValue>, Vec<Option<V128Bytes>>), ActionError> {
        match action {
            Action::Invoke { module, name, args } => {
                let key = module.clone();
                if key.is_none() {
                    if let Some(reason) = &self.current_module_status {
                        return Err(ActionError::NotYetSupported(reason.clone()));
                    }
                }
                let instance_rc = self
                    .registry
                    .borrow()
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| ActionError::Trap(format!("no module registered as {key:?}")))?;
                let mut instance = instance_rc.borrow_mut();
                let mut wasm_args = Vec::with_capacity(args.len());
                for a in args {
                    wasm_args.push(match a {
                        // v128 invoke arguments (task #86, W15 follow-up):
                        // `WasmInstance.v128_heap` is now a persistent
                        // field that exists BEFORE any call runs (see
                        // `code/specs/W15-wasm-v128-persistent-storage.md`),
                        // so a `v128.const` argument can allocate directly
                        // into it here, exactly like `evaluate_const_expr`
                        // already does for a global initializer -- this
                        // used to be impossible (no heap existed yet at
                        // this point at all), now it's the same
                        // "push and return the new index" shape used
                        // everywhere else a v128 value is created.
                        ConstValue::V128(bytes) => {
                            if instance.v128_heap.len() >= wasm_execution::MAX_V128_HEAP_LEN {
                                return Err(ActionError::Trap(
                                    "v128 heap limit exceeded (too many SIMD values created)".to_string(),
                                ));
                            }
                            let handle = instance.v128_heap.len() as u32;
                            instance.v128_heap.push(*bytes);
                            WasmValue::V128(handle)
                        }
                        _ => const_value_to_wasm_value(a).ok_or_else(|| {
                            ActionError::NotYetSupported(format!(
                                "invoke argument {a:?} has no WasmValue representation (real capability gap, not a bug)"
                            ))
                        })?,
                    });
                }
                self.runtime
                    .call_typed_with_v128(&mut instance, name, &wasm_args)
                    // W21: an uncaught exception (`TrapError::is_exception`)
                    // becomes `ActionError::Exception`, not `Trap` -- the
                    // one place a `TrapError` crosses into this harness's
                    // own `ActionError`, so the one place that distinction
                    // needs preserving.
                    .map_err(|e: TrapError| {
                        if e.is_exception {
                            ActionError::Exception(e.to_string())
                        } else {
                            ActionError::Trap(e.to_string())
                        }
                    })
            }
            Action::Get { module, name } => {
                let key = module.clone();
                if key.is_none() {
                    if let Some(reason) = &self.current_module_status {
                        return Err(ActionError::NotYetSupported(reason.clone()));
                    }
                }
                let instance_rc = self
                    .registry
                    .borrow()
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| ActionError::Trap(format!("no module registered as {key:?}")))?;
                let instance = instance_rc.borrow();
                instance
                    .exports
                    .iter()
                    .find(|(n, kind, _)| n == name && *kind == ExternalKind::Global)
                    // W35 third slice: `GlobalStorage::value`, not the
                    // whole `GlobalStorage` -- a real funcref global's
                    // `value` here is the reserved `WasmValue::Ref(Some(0))`
                    // sentinel (see that struct's own doc comment); its
                    // real identity lives in `func_ref`, unused by this
                    // action (mechanical fallout only, per this slice's
                    // own scope -- `wasm-conformance`'s own funcref-
                    // equality/cross-module propagation is slice 4's job).
                    .and_then(|(_, _, idx)| instance.globals.get(*idx as usize).map(|g| g.borrow().value))
                    // A global read is not a call -- there's no engine
                    // `ctx` involved at all -- but UNLIKE at the time this
                    // comment was first written (SIMD PR1b-3), a
                    // `WasmValue::V128` global's handle CAN now be
                    // resolved here: `code/specs/
                    // W15-wasm-v128-persistent-storage.md` moved
                    // `v128_heap` onto the persistent `WasmInstance`
                    // itself, so the same `Vec` a call's `ctx.v128_heap`
                    // clones from/restores to is directly readable here
                    // too, no engine required.
                    .map(|v| {
                        let bytes = match v {
                            WasmValue::V128(handle) => {
                                instance.v128_heap.get(handle as usize).copied().map(V128Bytes)
                            }
                            _ => None,
                        };
                        (vec![v], vec![bytes])
                    })
                    .ok_or_else(|| ActionError::Trap(format!("no global export named {name:?}")))
            }
        }
    }
}

enum ActionError {
    Trap(String),
    NotYetSupported(String),
    /// An uncaught WASM **exception** (W21 -- the exceptions proposal's
    /// `throw`, propagated all the way out), distinct from an ordinary
    /// `Trap` -- see `wasm_execution::TrapError::is_exception`'s own doc
    /// comment for why the real spec treats these as genuinely different
    /// outcomes.
    Exception(String),
}

/// `None` only for `ConstValue::V128` -- see this function's one caller
/// (`run_action`'s `Action::Invoke` arm) for why a v128 invoke ARGUMENT
/// can't be represented as a real `WasmValue::V128` handle at all: no
/// engine/heap exists yet at the point arguments are being built, before
/// the call that would own one even starts.
fn const_value_to_wasm_value(c: &ConstValue) -> Option<WasmValue> {
    match *c {
        ConstValue::I32(v) => Some(WasmValue::I32(v)),
        ConstValue::I64(v) => Some(WasmValue::I64(v)),
        ConstValue::F32Bits(bits) => Some(WasmValue::F32(f32::from_bits(bits))),
        ConstValue::F64Bits(bits) => Some(WasmValue::F64(f64::from_bits(bits))),
        // WASM17: `(ref.null func/extern)` -> Ref(None); `(ref.extern n)`
        // -> Ref(Some(n)). Falls out for free since `WasmValue::Ref` already
        // wraps the identical `Option<u32>` shape `ConstValue::Ref` does.
        ConstValue::Ref(v) => Some(WasmValue::Ref(v)),
        ConstValue::V128(_) => None,
    }
}

/// Distinguishes a real LINK failure (an unresolved or type-mismatched
/// import) from a genuine RUNTIME fault during `instantiate`'s data/
/// element-segment initialization -- `WasmRuntime::instantiate` reuses
/// `TrapError` for both rather than a new error type (this crate's own
/// convention of self-authored, capability-gap-shaped error text; see
/// `wasm-runtime`'s `link_error` helper), so the message's own prefix is
/// the only signal. This is matching OUR OWN self-authored text, not the
/// spec's expected message wording -- a different thing from the
/// trap-grading discipline elsewhere in this crate that deliberately
/// never string-matches the SPEC's own trap messages.
fn is_link_error(e: &TrapError) -> bool {
    let msg = e.to_string();
    msg.contains("unknown import") || msg.contains("incompatible import type")
}

fn build_module(source: ModuleSource) -> Result<wasm_types::WasmModule, String> {
    match source {
        ModuleSource::Text(sexpr) => wasm_wast_parser::module::parse_module_expr(&sexpr).map_err(|e| e.to_string()),
        ModuleSource::Binary(bytes) => WasmModuleParser::parse(&bytes).map_err(|e| e.to_string()),
        ModuleSource::Quote(bytes) => {
            let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            wasm_wast_parser::parse_module(text).map_err(|e| e.to_string())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Bit-exact `assert_return` comparison
// ═══════════════════════════════════════════════════════════════════════

/// WASM's "canonical NaN": exponent all-ones, only the quiet bit (the
/// mantissa's top bit) set, every other mantissa bit zero. Sign is
/// unconstrained -- `nan:canonical` accepts either sign, exactly this
/// payload.
const F32_CANONICAL_NAN_UNSIGNED: u32 = 0x7FC0_0000;
const F32_SIGN_BIT: u32 = 0x8000_0000;
const F32_QUIET_BIT: u32 = 0x0040_0000;

const F64_CANONICAL_NAN_UNSIGNED: u64 = 0x7FF8_0000_0000_0000;
const F64_SIGN_BIT: u64 = 0x8000_0000_0000_0000;
const F64_QUIET_BIT: u64 = 0x0008_0000_0000_0000;

fn value_matches_expected(actual: &WasmValue, v128_bytes: Option<V128Bytes>, expected: &Expected) -> bool {
    match expected {
        Expected::Value(ConstValue::I32(v)) => matches!(actual, WasmValue::I32(a) if a == v),
        Expected::Value(ConstValue::I64(v)) => matches!(actual, WasmValue::I64(a) if a == v),
        Expected::Value(ConstValue::F32Bits(bits)) => matches!(actual, WasmValue::F32(a) if a.to_bits() == *bits),
        Expected::Value(ConstValue::F64Bits(bits)) => matches!(actual, WasmValue::F64(a) if a.to_bits() == *bits),
        // SIMD PR1b-3: byte-exact v128 comparison. `actual` must actually
        // be a `WasmValue::V128` AND its resolved bytes (threaded through
        // from `run_action`'s `call_typed_with_v128`) must equal the
        // expected literal exactly -- both checks matter: `v128_bytes`
        // alone can't distinguish "this result wasn't a v128 at all" from
        // "it was, and happened to resolve to bytes that don't match" (the
        // former should never occur for a real vendored fixture, but a
        // defensive check here costs nothing and documents the invariant
        // `call_function_impl`'s resolution logic guarantees).
        Expected::Value(ConstValue::V128(expected_bytes)) => {
            matches!(actual, WasmValue::V128(_)) && v128_bytes == Some(V128Bytes(*expected_bytes))
        }
        // WASM17: exact `(ref.null func/extern)` (None) / `(ref.extern n)`
        // (Some(n)) -- compares the *handle number*, not host-object
        // identity, matching this repo's own script-literal design (no
        // real host environment produces externref values to preserve
        // identity for).
        Expected::Value(ConstValue::Ref(v)) => matches!(actual, WasmValue::Ref(a) if a == v),
        // Bare `(ref.null)` -- matches the null reference of ANY reference
        // type. This repo's `WasmValue::Ref(Option<u32>)` doesn't carry a
        // runtime type tag (only the *static* type distinguishes a funcref
        // from an externref from an anyref -- see `code/specs/
        // W08-wasm-funcref-externref.md`), so "null of any type" and
        // "null" are the same check at this layer.
        Expected::RefNullAny => matches!(actual, WasmValue::Ref(None)),
        // Bare `(ref.func)` -- matches ANY non-null funcref. Same
        // representation limitation as above: this crate can't distinguish
        // "non-null funcref" from "non-null externref" at the value level,
        // so this accepts any non-null reference. In practice this is only
        // ever used where the real testsuite itself expects a funcref
        // result, so the type ambiguity doesn't cause false passes against
        // the vendored corpus.
        Expected::RefFuncAny => matches!(actual, WasmValue::Ref(Some(_))),
        // Bare `(ref.i31)` (W20) -- matches ANY i31ref. This repo carries
        // an i31ref as its plain (already 31-bit-masked) `i32` payload on
        // the value stack (never a `WasmValue::Ref`, see `wasm-execution`'s
        // own `0xFB` handler doc comment), so "is this some i31ref at all"
        // is just "is this an I32" at this layer -- the real testsuite only
        // ever uses this wildcard where the static result type is already
        // known to be an i31ref, so the representation ambiguity (an I32
        // result that ISN'T meant to be an i31 would also match) doesn't
        // cause a false pass against the vendored corpus.
        Expected::RefI31Any => matches!(actual, WasmValue::I32(_)),
        // Bare `(ref.array)`/`(ref.struct)` (GC proposal, real corpus
        // vendoring pass) -- same "any non-null ref handle" grading as
        // `RefFuncAny` above, and the same representation caveat: this
        // crate's `WasmValue::Ref` carries no per-kind tag distinguishing
        // "some array ref" from "some struct ref" from "some funcref", so
        // this accepts any of them. Only used where the real testsuite
        // already expects specifically an array/struct ref, so the
        // ambiguity doesn't cause a false pass against the vendored corpus.
        Expected::RefArrayAny => matches!(actual, WasmValue::Ref(Some(_))),
        Expected::RefStructAny => matches!(actual, WasmValue::Ref(Some(_))),
        // Bare `(ref.eq)` -- `eqref`'s members are `i31ref` (this crate's
        // `WasmValue::I32`, see `RefI31Any` above) plus every non-null
        // struct/array ref (`WasmValue::Ref(Some(_))`) -- NOT `funcref`/
        // `externref`, but this layer can't tell a non-null funcref/
        // externref apart from a non-null struct/array ref either (same
        // representation limitation as `RefFuncAny`), so this is the same
        // conservative "any non-null ref OR any i31" superset every other
        // wildcard here already accepts.
        Expected::RefEqAny => matches!(actual, WasmValue::Ref(Some(_)) | WasmValue::I32(_)),
        Expected::NanCanonicalF32 => {
            matches!(actual, WasmValue::F32(a) if (a.to_bits() & !F32_SIGN_BIT) == F32_CANONICAL_NAN_UNSIGNED)
        }
        Expected::NanArithmeticF32 => {
            matches!(actual, WasmValue::F32(a) if a.is_nan() && (a.to_bits() & F32_QUIET_BIT) != 0)
        }
        Expected::NanCanonicalF64 => {
            matches!(actual, WasmValue::F64(a) if (a.to_bits() & !F64_SIGN_BIT) == F64_CANONICAL_NAN_UNSIGNED)
        }
        Expected::NanArithmeticF64 => {
            matches!(actual, WasmValue::F64(a) if a.is_nan() && (a.to_bits() & F64_QUIET_BIT) != 0)
        }
        // SIMD widen PR28: `v128.const f32x4`/`f64x2` expected values with
        // at least one NaN-class lane (see `Expected::V128F32x4`/
        // `V128F64x2`'s own doc comments in wasm-wast-parser). Every lane
        // must match independently -- an exact lane compares bits, a
        // NaN-class lane reuses exactly the same canonical/arithmetic
        // check as the scalar `NanCanonicalF32`/`NanArithmeticF32` arms
        // above, just applied per-lane instead of to one whole value.
        Expected::V128F32x4(lanes) => match (actual, v128_bytes) {
            (WasmValue::V128(_), Some(bytes)) => (0..4).all(|i| {
                let bits = u32::from_le_bytes(bytes.0[i * 4..i * 4 + 4].try_into().unwrap());
                match lanes[i] {
                    F32LaneExpected::Exact(want) => bits == want,
                    F32LaneExpected::NanCanonical => (bits & !F32_SIGN_BIT) == F32_CANONICAL_NAN_UNSIGNED,
                    F32LaneExpected::NanArithmetic => f32::from_bits(bits).is_nan() && (bits & F32_QUIET_BIT) != 0,
                }
            }),
            _ => false,
        },
        Expected::V128F64x2(lanes) => match (actual, v128_bytes) {
            (WasmValue::V128(_), Some(bytes)) => (0..2).all(|i| {
                let bits = u64::from_le_bytes(bytes.0[i * 8..i * 8 + 8].try_into().unwrap());
                match lanes[i] {
                    F64LaneExpected::Exact(want) => bits == want,
                    F64LaneExpected::NanCanonical => (bits & !F64_SIGN_BIT) == F64_CANONICAL_NAN_UNSIGNED,
                    F64LaneExpected::NanArithmetic => f64::from_bits(bits).is_nan() && (bits & F64_QUIET_BIT) != 0,
                }
            }),
            _ => false,
        },
        // `(either A B)` (relaxed SIMD epic PR1 -- see `code/specs/
        // W19-wasm-relaxed-simd-first-slice.md` and `Expected::Either`'s
        // own doc comment in wasm-wast-parser): the relaxed-simd spec
        // deliberately leaves certain ops implementation-defined for
        // certain inputs, and the upstream corpus grades them with this
        // "match A OR B" combinator instead of one exact value. Recurses
        // through this same function for both children -- `A`/`B` can in
        // principle be any other `Expected` shape (a NaN class, a nested
        // `Either`, etc.), not just a plain `v128.const`, so this is NOT
        // limited to the specific opcode this PR implements.
        Expected::Either(a, b) => value_matches_expected(actual, v128_bytes, a) || value_matches_expected(actual, v128_bytes, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use report::DirectiveOutcome;

    fn outcomes(source: &str) -> Vec<(DirectiveKind, DirectiveOutcome)> {
        run_wast_source(source).expect("script should parse")
    }

    #[test]
    fn assert_return_i32_passes_on_exact_match() {
        let results = outcomes(
            r#"
            (module (func (export "add") (param i32 i32) (result i32)
              local.get 0 local.get 1 i32.add))
            (assert_return (invoke "add" (i32.const 1) (i32.const 2)) (i32.const 3))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_return_fails_on_wrong_value() {
        let results = outcomes(
            r#"
            (module (func (export "add") (param i32 i32) (result i32)
              local.get 0 local.get 1 i32.add))
            (assert_return (invoke "add" (i32.const 1) (i32.const 2)) (i32.const 4))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_return_ref_extern_compares_the_handle_number() {
        let results = outcomes(
            r#"
            (module (func (export "id") (param externref) (result externref) local.get 0))
            (assert_return (invoke "id" (ref.extern 1)) (ref.extern 1))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));

        let results = outcomes(
            r#"
            (module (func (export "id") (param externref) (result externref) local.get 0))
            (assert_return (invoke "id" (ref.extern 1)) (ref.extern 2))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_return_ref_null_func_and_bare_ref_null_both_accept_a_null_funcref() {
        let module = r#"(module (func (export "n") (result funcref) (ref.null func)))"#;
        let exact = outcomes(&format!(r#"{module} (assert_return (invoke "n") (ref.null func))"#));
        assert_eq!(exact[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
        let wildcard = outcomes(&format!(r#"{module} (assert_return (invoke "n") (ref.null))"#));
        assert_eq!(wildcard[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_return_bare_ref_func_accepts_any_non_null_funcref_but_rejects_null() {
        let module = r#"(module (func $f) (func (export "get") (result funcref) (ref.func $f)))"#;
        let results = outcomes(&format!(r#"{module} (assert_return (invoke "get") (ref.func))"#));
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));

        let null_module = r#"(module (func (export "get") (result funcref) (ref.null func)))"#;
        let results = outcomes(&format!(r#"{null_module} (assert_return (invoke "get") (ref.func))"#));
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    /// Regression test for a real, previously-shipped bug found while
    /// prioritizing the vendored testsuite corpus: `br_table.wast` (a
    /// foundational MVP-level control-flow file, no GC-proposal syntax
    /// involved) was TOTALLY failing -- `module 0/1`, `assert_return
    /// 0/161` -- with `ValidationError: TypeMismatch: expected
    /// ConcreteFuncRef(1), found Funcref` on an entirely ordinary `(table
    /// funcref (elem $f))`-style construct
    /// (`code/packages/rust/wasm-conformance/tests/fixtures/testsuite/
    /// br_table.wast`'s own `meet-funcref-1`, vendored unchanged from the
    /// official spec testsuite).
    ///
    /// This is the simplest possible reproduction of the FIRST of the
    /// bug's two root causes: `(table $t (ref null $t) (elem $tf))`
    /// declares a table whose element type is the CONCRETE function type
    /// `$t`, not generic `funcref` -- but `wasm-wast-parser` used to
    /// silently discard that reftype entirely (see the removed comment
    /// this fix replaced, "this crate only tracks FUNCREF tables"),
    /// leaving `table.get $t` with no way to know the table was anything
    /// but generic `funcref`. A `br_table` branching to a label that
    /// genuinely requires the NARROWER `$t` type then failed even though
    /// the actual value in the table (`$tf`, of type `$t`) is exactly
    /// right.
    #[test]
    fn table_get_on_a_concrete_funcref_table_keeps_its_concrete_type() {
        // The exported function's declared result is the CONCRETE type
        // `(ref null $t)`, not generic `funcref` -- its implicit-return
        // check (every function body's own final assignability check
        // against its declared results) only passes if `table.get $t`
        // really did push `$t`'s concrete type. Before this fix it pushed
        // generic `Funcref` instead, which is NOT assignable to a
        // concrete-typed result slot (the opposite direction from
        // `ConcreteFuncRef <: Funcref`), so this alone reproduces the
        // regression without needing a value-comparing `assert_return`.
        let results = outcomes(
            r#"
            (module
              (type $t (func))
              (func $tf)
              (table $t (ref null $t) (elem $tf))
              (func (export "get-as-concrete") (result (ref null $t))
                (table.get $t (i32.const 0))
              )
            )
            "#,
        );
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
    }

    /// The SECOND of the two root causes behind the `br_table.wast`
    /// regression `table_get_on_a_concrete_funcref_table_keeps_its_
    /// concrete_type` above documents: even once `table.get` correctly
    /// pushes the table's own concrete type, `br_table`'s multi-target
    /// type check was ORDER-DEPENDENT, which the real spec is not.
    ///
    /// `br_table`'s typing rule requires that the SAME operand value(s)
    /// be simultaneously assignable to every listed target AND the
    /// default target -- a "meet" over all of them, not a left-to-right
    /// chain. The old implementation instead re-pushed each target's OWN
    /// declared type after checking it, so checking a WIDER target (here,
    /// `$l1`'s generic `(ref null func)`) before a NARROWER one (`$l2`'s
    /// concrete `(ref null $t)`) irreversibly widened the value away,
    /// and the narrower target's check then failed spuriously -- even
    /// though the actual value is perfectly assignable to BOTH.
    ///
    /// This is `br_table.wast`'s own `meet-funcref-1` (vendored
    /// unchanged): its label list `$l1 $l1 $l2` deliberately checks the
    /// generic target ($l1, twice) before the concrete one ($l2, the
    /// default) -- exactly the ordering the old chain-based algorithm got
    /// wrong. `meet-funcref-2` (`$l2 $l2 $l1`, concrete first) already
    /// passed even before this fix, which is WHY this needed a dedicated
    /// order-sensitive test rather than trusting one passing permutation.
    #[test]
    fn br_table_targets_type_check_regardless_of_which_order_they_are_listed_in() {
        let module = r#"
            (module
              (type $t (func))
              (func $tf)
              (table $t (ref null $t) (elem $tf))
              (func (export "meet-wide-target-first") (param i32) (result (ref null func))
                (block $l1 (result (ref null func))
                  (block $l2 (result (ref null $t))
                    (br_table $l1 $l1 $l2 (table.get $t (i32.const 0)) (local.get 0))
                  )
                )
              )
              (func (export "meet-narrow-target-first") (param i32) (result (ref null func))
                (block $l1 (result (ref null func))
                  (block $l2 (result (ref null $t))
                    (br_table $l2 $l2 $l1 (table.get $t (i32.const 0)) (local.get 0))
                  )
                )
              )
            )
        "#;
        let results = outcomes(module);
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_return_grades_bit_exact_not_lossy() {
        // 3.5 truncates to 3 via `as i64` -- if this harness went through
        // wasm-runtime::call() instead of call_typed, this would wrongly
        // pass by accident (3 == 3) rather than genuinely comparing floats.
        let results = outcomes(
            r#"
            (module (func (export "half") (result f64) f64.const 3.5))
            (assert_return (invoke "half") (f64.const 3.5))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));

        let results = outcomes(
            r#"
            (module (func (export "half") (result f64) f64.const 3.5))
            (assert_return (invoke "half") (f64.const 3.6))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    // ── SIMD PR1b-3: v128 byte-exact assert_return grading ──────────────
    //
    // These are hand-written, not vendored -- every real root-level
    // `simd_*.wast` file (`simd_const.wast`/`simd_splat.wast`/
    // `simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast`, all checked against
    // the pinned corpus commit) exercises SIMD opcodes well beyond this
    // repo's current 5-opcode slice (`v128.const`/`i32x4.splat`/`add`/
    // `eq`/`extract_lane`) -- e.g. `simd_const.wast`'s sole
    // `i64x2.add` use (its "i64x2.inc_smin" test) or `simd_splat.wast`'s
    // `i8x16.add`/`f32x4.min`/`v128.and`/`v128.load`/etc. And because this
    // crate's `Directive::Module` is built EAGERLY at `parse_script` time
    // (see this file's own module doc comment on why), even one
    // unsupported instruction ANYWHERE in a file -- not just in a directive
    // that would actually run -- aborts parsing the WHOLE FILE, not just
    // that one test. So no real corpus file can be vendored yet without
    // either widening opcode coverage first or making per-module build
    // failures degrade gracefully (a real, separate follow-up either way
    // -- see the SIMD PR1b-3 CHANGELOG entry and the backlog items logged
    // alongside it). These tests instead prove the comparison MACHINERY
    // itself is correct, restricted to opcodes this slice really executes.

    #[test]
    fn assert_return_v128_const_passes_on_exact_byte_match_and_fails_on_mismatch() {
        let results = outcomes(
            r#"
            (module (func (export "f") (result v128) (v128.const i32x4 1 2 3 4)))
            (assert_return (invoke "f") (v128.const i32x4 1 2 3 4))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));

        let results = outcomes(
            r#"
            (module (func (export "f") (result v128) (v128.const i32x4 1 2 3 4)))
            (assert_return (invoke "f") (v128.const i32x4 1 2 3 5))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_return_v128_grades_the_actual_computation_not_just_any_v128() {
        // If grading only checked "is this a V128 result", a WRONG
        // computed sum would wrongly pass -- this proves the resolved
        // BYTES are what's actually compared, mirroring the same
        // load-bearing check `wasm-execution`'s own SIMD PR1b-1 tests make
        // one layer down.
        let results = outcomes(
            r#"
            (module (func (export "add") (result v128)
              (i32x4.add (v128.const i32x4 1 2 3 4) (v128.const i32x4 10 20 30 40))))
            (assert_return (invoke "add") (v128.const i32x4 11 22 33 44))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));

        let results = outcomes(
            r#"
            (module (func (export "add") (result v128)
              (i32x4.add (v128.const i32x4 1 2 3 4) (v128.const i32x4 10 20 30 40))))
            (assert_return (invoke "add") (v128.const i32x4 999 22 33 44))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_return_v128_eq_boolean_mask_result_is_itself_a_v128() {
        // The SIMD comparison convention: `i32x4.eq`'s result is an
        // all-1s/-1 (equal) or all-0s (not-equal) per-lane MASK, still a
        // v128 -- not a plain i32 `1`/`0` the way scalar comparisons work.
        let results = outcomes(
            r#"
            (module (func (export "eq") (result v128)
              (i32x4.eq (v128.const i32x4 5 6 7 8) (v128.const i32x4 5 0 7 0))))
            (assert_return (invoke "eq") (v128.const i32x4 -1 0 -1 0))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_return_i32x4_splat_and_extract_lane_round_trip() {
        let results = outcomes(
            r#"
            (module (func (export "roundtrip") (param i32) (result i32)
              (i32x4.extract_lane 2 (i32x4.splat (local.get 0)))))
            (assert_return (invoke "roundtrip" (i32.const 42)) (i32.const 42))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn invoke_with_v128_arguments_passes_for_real() {
        // Task #86 (W15 follow-up): `WasmInstance.v128_heap` is now a
        // persistent field that exists BEFORE any call runs, so a
        // `(v128.const ...)` invoke ARGUMENT allocates directly into it --
        // this used to be a hard capability gap (`NotYetSupported`, no
        // heap existed yet at this point at all); now it's a real,
        // byte-exact `Pass`, same as a v128 RESULT already was.
        let results = outcomes(
            r#"
            (module (func (export "add") (param v128 v128) (result v128) (i32x4.add (local.get 0) (local.get 1))))
            (assert_return (invoke "add" (v128.const i32x4 1 2 3 4) (v128.const i32x4 10 20 30 40)) (v128.const i32x4 11 22 33 44))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn ref_i31_any_matches_i32_but_not_a_ref_or_other_numeric_type() {
        // W20 -- an i31ref is carried as its plain (masked) i32 payload on
        // this repo's value stack, never a `WasmValue::Ref` (see
        // `wasm-execution`'s own `0xFB` handler doc comment).
        assert!(value_matches_expected(&WasmValue::I32(0), None, &Expected::RefI31Any));
        assert!(value_matches_expected(&WasmValue::I32(-1), None, &Expected::RefI31Any));
        assert!(!value_matches_expected(&WasmValue::I64(0), None, &Expected::RefI31Any));
        assert!(!value_matches_expected(&WasmValue::Ref(Some(0)), None, &Expected::RefI31Any));
        assert!(!value_matches_expected(&WasmValue::Ref(None), None, &Expected::RefI31Any));
    }

    #[test]
    fn ref_array_and_ref_struct_any_match_any_non_null_ref_only() {
        // GC proposal -- same "any non-null ref handle" grading as
        // `RefFuncAny`, since this crate's `WasmValue::Ref` carries no
        // per-kind tag.
        assert!(value_matches_expected(&WasmValue::Ref(Some(0)), None, &Expected::RefArrayAny));
        assert!(!value_matches_expected(&WasmValue::Ref(None), None, &Expected::RefArrayAny));
        assert!(!value_matches_expected(&WasmValue::I32(0), None, &Expected::RefArrayAny));
        assert!(value_matches_expected(&WasmValue::Ref(Some(0)), None, &Expected::RefStructAny));
        assert!(!value_matches_expected(&WasmValue::Ref(None), None, &Expected::RefStructAny));
        assert!(!value_matches_expected(&WasmValue::I32(0), None, &Expected::RefStructAny));
    }

    #[test]
    fn ref_eq_any_matches_a_non_null_ref_or_an_i31_but_not_null_or_other_numerics() {
        // `eqref`'s members are `i31ref` (`WasmValue::I32`) plus every
        // non-null struct/array ref (`WasmValue::Ref(Some(_))`).
        assert!(value_matches_expected(&WasmValue::Ref(Some(0)), None, &Expected::RefEqAny));
        assert!(value_matches_expected(&WasmValue::I32(-1), None, &Expected::RefEqAny));
        assert!(!value_matches_expected(&WasmValue::Ref(None), None, &Expected::RefEqAny));
        assert!(!value_matches_expected(&WasmValue::I64(0), None, &Expected::RefEqAny));
    }

    #[test]
    fn assert_return_nan_canonical_accepts_either_sign_exact_payload() {
        assert!(value_matches_expected(&WasmValue::F32(f32::from_bits(0x7FC0_0000)), None, &Expected::NanCanonicalF32));
        assert!(value_matches_expected(&WasmValue::F32(f32::from_bits(0xFFC0_0000)), None, &Expected::NanCanonicalF32));
        assert!(!value_matches_expected(&WasmValue::F32(f32::from_bits(0x7FC0_0001)), None, &Expected::NanCanonicalF32));
    }

    #[test]
    fn assert_return_nan_arithmetic_accepts_any_payload_with_quiet_bit() {
        assert!(value_matches_expected(&WasmValue::F64(f64::from_bits(0x7FF8_0000_0000_002A)), None, &Expected::NanArithmeticF64));
        assert!(!value_matches_expected(
            &WasmValue::F64(f64::from_bits(0x7FF0_0000_0000_002A)), // quiet bit clear
            None,
            &Expected::NanArithmeticF64
        ));
    }

    // ── Relaxed SIMD epic PR1: `(either A B)` grading -- see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md ───────────────────────────────

    #[test]
    fn either_accepts_a_value_matching_the_first_alternative() {
        let expected = Expected::Either(
            Box::new(Expected::Value(ConstValue::I32(1))),
            Box::new(Expected::Value(ConstValue::I32(2))),
        );
        assert!(value_matches_expected(&WasmValue::I32(1), None, &expected));
    }

    #[test]
    fn either_accepts_a_value_matching_the_second_alternative() {
        let expected = Expected::Either(
            Box::new(Expected::Value(ConstValue::I32(1))),
            Box::new(Expected::Value(ConstValue::I32(2))),
        );
        assert!(value_matches_expected(&WasmValue::I32(2), None, &expected));
    }

    #[test]
    fn either_rejects_a_value_matching_neither_alternative() {
        let expected = Expected::Either(
            Box::new(Expected::Value(ConstValue::I32(1))),
            Box::new(Expected::Value(ConstValue::I32(2))),
        );
        assert!(!value_matches_expected(&WasmValue::I32(3), None, &expected));
    }

    #[test]
    fn either_v128_matches_the_real_relaxed_swizzle_out_of_range_shape() {
        // Mirrors `i8x16_relaxed_swizzle.wast`'s own out-of-range case:
        // accepts either the all-zero clamp-to-zero result (this repo's
        // actual choice) or the modulo-16-wrapped alternative.
        let zeros = [0u8; 16];
        let mut identity = [0u8; 16];
        for (i, b) in identity.iter_mut().enumerate() {
            *b = i as u8;
        }
        let expected =
            Expected::Either(Box::new(Expected::Value(ConstValue::V128(zeros))), Box::new(Expected::Value(ConstValue::V128(identity))));
        assert!(value_matches_expected(&WasmValue::V128(0), Some(V128Bytes(zeros)), &expected));
        assert!(value_matches_expected(&WasmValue::V128(0), Some(V128Bytes(identity)), &expected));
        let mut neither = [0u8; 16];
        neither[0] = 0xFF;
        assert!(!value_matches_expected(&WasmValue::V128(0), Some(V128Bytes(neither)), &expected));
    }

    #[test]
    fn either_nested_four_way_matches_any_of_the_four_alternatives() {
        // Relaxed SIMD epic PR3: the real `relaxed_min_max.wast` corpus
        // is the first relaxed-simd file whose `either` groups carry FOUR
        // alternatives (see `wasm-wast-parser`'s generalized `either`
        // parsing arm, which folds N children into a right-leaning chain
        // of nested `Expected::Either`s). This confirms grading itself --
        // the existing recursive `||` in `value_matches_expected`, which
        // needed NO code changes for this -- correctly accepts a match on
        // the 3RD or 4TH alternative too, not just the first two the
        // original binary-only `either` arm would have exposed.
        let nested = Expected::Either(
            Box::new(Expected::Either(
                Box::new(Expected::Either(
                    Box::new(Expected::Value(ConstValue::I32(0))),
                    Box::new(Expected::Value(ConstValue::I32(1))),
                )),
                Box::new(Expected::Value(ConstValue::I32(2))),
            )),
            Box::new(Expected::Value(ConstValue::I32(3))),
        );
        assert!(value_matches_expected(&WasmValue::I32(0), None, &nested));
        assert!(value_matches_expected(&WasmValue::I32(1), None, &nested));
        assert!(value_matches_expected(&WasmValue::I32(2), None, &nested));
        assert!(value_matches_expected(&WasmValue::I32(3), None, &nested));
        assert!(!value_matches_expected(&WasmValue::I32(4), None, &nested));
    }

    #[test]
    fn assert_trap_passes_on_real_trap_fails_on_normal_return() {
        let results = outcomes(
            r#"
            (module (func (export "div0") (result i32) i32.const 1 i32.const 0 i32.div_s))
            (assert_trap (invoke "div0") "integer divide by zero")
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertTrap, DirectiveOutcome::Pass));

        let results = outcomes(
            r#"
            (module (func (export "one") (result i32) i32.const 1))
            (assert_trap (invoke "one") "some trap")
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_exception_passes_on_a_real_uncaught_throw_fails_on_normal_return_and_on_a_plain_trap() {
        // W21 (exceptions proposal): `assert_exception` must accept ONLY a
        // real uncaught exception -- neither a normal return NOR an
        // ordinary trap (this repo distinguishes the two via `TrapError::
        // is_exception`, and the real spec treats them as genuinely
        // different outcomes: `try_table` never catches a trap).
        let results = outcomes(
            r#"
            (module (tag $e) (func (export "boom") (throw $e)))
            (assert_exception (invoke "boom"))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertException, DirectiveOutcome::Pass));

        let results = outcomes(
            r#"
            (module (func (export "one") (result i32) i32.const 1))
            (assert_exception (invoke "one"))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)), "a normal return must not satisfy assert_exception");

        let results = outcomes(
            r#"
            (module (func (export "div0") (result i32) i32.const 1 i32.const 0 i32.div_s))
            (assert_exception (invoke "div0"))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)), "an ordinary trap must not satisfy assert_exception");
    }

    #[test]
    fn assert_trap_and_assert_exception_do_not_accept_each_others_outcome() {
        // The converse of the above, from `assert_trap`'s side: a real
        // uncaught exception must NOT satisfy `assert_trap` either.
        let results = outcomes(
            r#"
            (module (tag $e) (func (export "boom") (throw $e)))
            (assert_trap (invoke "boom") "some trap")
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)), "an uncaught exception must not satisfy assert_trap");
    }

    #[test]
    fn assert_exhaustion_passes_on_real_unbounded_recursion() {
        // WASM01: wasm-execution's call-depth guard turns unbounded
        // recursion into a real, gradeable trap instead of a host-crash
        // risk this crate had to route around entirely.
        let results = outcomes(
            r#"
            (module (func $loop (export "loop") (result i32) call $loop))
            (assert_exhaustion (invoke "loop") "call stack exhausted")
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertExhaustion, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_exhaustion_fails_if_the_action_returns_normally() {
        let results = outcomes(
            r#"
            (module (func (export "one") (result i32) i32.const 1))
            (assert_exhaustion (invoke "one") "call stack exhausted")
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_invalid_structurally_rejected_module_passes() {
        // A duplicate export name is a real structural violation this
        // repo's validator already catches -- a legitimate `Pass`.
        let results = outcomes(
            r#"
            (module
              (func (export "f") (result i32) i32.const 0)
              (func (export "f") (result i32) i32.const 1))
            "#,
        );
        // The module directive's OWN outcome should reflect the rejection.
        assert!(matches!(results[0].1, DirectiveOutcome::Fail(_)));
    }

    #[test]
    fn assert_invalid_rejected_by_the_instruction_level_type_checker_is_a_real_pass() {
        // Before WASM06 (the instruction-level type checker), a module
        // like this one -- structurally fine (valid index bounds, etc.)
        // but semantically ill-typed (declares `(result i32)` with an
        // empty body, a real stack underflow) -- passed the old
        // structural-only validator, so this case graded
        // `NotYetSupported` (this repo couldn't tell it apart from a
        // genuinely valid module). `wasm-validator` now runs a real
        // per-instruction type checker (see its `type_check` module), so
        // this exact case is correctly rejected -- a real `Pass`.
        let results = outcomes(
            r#"(assert_invalid (module (func (result i32))) "type mismatch")"#,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (DirectiveKind::AssertInvalid, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_malformed_binary_bad_magic_is_correctly_rejected() {
        let results = outcomes(r#"(assert_malformed (module binary "\00\00\00\00") "bad magic")"#);
        assert_eq!(results[0], (DirectiveKind::AssertMalformed, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_malformed_quote_unparseable_text_is_correctly_rejected() {
        let results = outcomes(r#"(assert_malformed (module quote "(module (func (") "unexpected token")"#);
        assert_eq!(results[0], (DirectiveKind::AssertMalformed, DirectiveOutcome::Pass));
    }

    /// Task #83 (prioritization scan after task #80, PR #11844): a memop's
    /// align immediate with the reserved top bit set decodes as an absurdly
    /// large LEB128 value that `wasm-module-parser::parse` alone never
    /// notices (it stores code-section bytes raw, undecoded) but
    /// `wasm-validator`'s instruction-level type-checker DOES reject via
    /// its existing `align > max_align` rule -- `grade_assert_malformed`'s
    /// binary path used to only call `parse`, never `validate`, so this
    /// real corpus case (`align.wast`'s "memop flags" cases) wrongly
    /// graded `Fail` even though the module genuinely IS unusable.
    #[test]
    fn assert_malformed_binary_reserved_align_bit_is_caught_via_validate() {
        let src = concat!(
            r#"(assert_malformed (module binary "#,
            r#""\00asm" "\01\00\00\00""#,
            r#""\01\04\01\60\00\00""#, // Type section: 1 type
            r#""\03\02\01\00""#,       // Function section: 1 function
            r#""\05\03\01\00\01""#,    // Memory section: 1 memory
            r#""\0a\0b\01""#,          // Code section: 1 function
            r#""\09\00""#,
            r#""\41\00""#,           // i32.const 0
            r#""\28\80\01\00""#,     // i32.load offset=0 align="2**128" (malformed)
            r#""\1a""#,              // drop
            r#""\0b""#,              // end
            r#") "malformed memop flags")"#,
        );
        let results = outcomes(src);
        assert_eq!(results[0], (DirectiveKind::AssertMalformed, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_unlinkable_passes_for_real_on_a_totally_unknown_module() {
        // WASM05/W10: `instantiate` now genuinely fails to link when no
        // host resolves the import at all -- no `spectest` support
        // needed for this specific case, since `RegistryHost` correctly
        // returns `None` for any module name it has never `register`ed.
        let results = outcomes(r#"(assert_unlinkable (module (import "m" "f" (func))) "unknown import")"#);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (DirectiveKind::AssertUnlinkable, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_unlinkable_passes_for_real_on_an_unknown_export_within_a_known_module() {
        let results = outcomes(
            r#"
            (module (func (export "func")))
            (register "test")
            (assert_unlinkable (module (import "test" "unknown" (func))) "unknown import")
            "#,
        );
        assert_eq!(results[2], (DirectiveKind::AssertUnlinkable, DirectiveOutcome::Pass));
    }

    #[test]
    fn assert_unlinkable_passes_for_real_on_a_function_type_mismatch() {
        let results = outcomes(
            r#"
            (module (func (export "func-i32") (param i32)))
            (register "test")
            (assert_unlinkable (module (import "test" "func-i32" (func))) "incompatible import type")
            "#,
        );
        assert_eq!(results[2], (DirectiveKind::AssertUnlinkable, DirectiveOutcome::Pass));
    }

    #[test]
    fn a_module_importing_a_registered_siblings_function_links_and_calls_across_instances() {
        // The positive counterpart to the assert_unlinkable cases above:
        // a real cross-instance function call, exercising
        // `CrossModuleFunction::call`'s `WasmRuntime::call_typed`
        // reentrance against the CALLEE's own instance state.
        let results = outcomes(
            r#"
            (module (func (export "double") (param i32) (result i32) local.get 0 local.get 0 i32.add))
            (register "test")
            (module
              (import "test" "double" (func $double (param i32) (result i32)))
              (func (export "quadruple") (param i32) (result i32)
                (call $double (call $double (local.get 0)))))
            (assert_return (invoke "quadruple" (i32.const 3)) (i32.const 12))
            "#,
        );
        assert_eq!(results[3], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn register_does_not_disturb_current_module_addressing() {
        // `register "name"` exposes the current module for a LATER module's
        // `(import "name" ...)` to resolve against -- it is not itself an
        // addressing mechanism `invoke` uses (real WAT syntax addresses a
        // specific earlier module via a bare `$id` right after `invoke`,
        // never via the register string). A plain, unqualified `(invoke
        // "answer")` after `register` must still resolve against "the
        // current module," unaffected by the registration.
        let results = outcomes(
            r#"
            (module (func (export "answer") (result i32) i32.const 42))
            (register "M")
            (assert_return (invoke "answer") (i32.const 42))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    /// Task #93 (linking.wast): real WAT scripts address a SPECIFIC
    /// earlier module by its own `$id` -- `(invoke $Mf "f" ...)` -- not
    /// just "the current module." Before this fix, `(module $id ...)`'s
    /// own name was discarded during parsing, so `$id` never resolved to
    /// anything: this is the single root cause behind ALL 65 `assert_
    /// return` failures the real, vendored `linking.wast` corpus file had
    /// before this fix (confirmed via a direct diagnostic run -- every one
    /// of them failed with the identical "no module registered as
    /// Some($id)" message).
    #[test]
    fn invoke_addresses_a_specific_earlier_module_by_its_own_id_not_just_the_current_one() {
        let results = outcomes(
            r#"
            (module $Mf (func (export "get") (result i32) i32.const 1))
            (module $Mg (func (export "get") (result i32) i32.const 2))
            (assert_return (invoke $Mf "get") (i32.const 1))
            (assert_return (invoke $Mg "get") (i32.const 2))
            "#,
        );
        assert_eq!(results[2], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
        assert_eq!(results[3], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    /// `(register "name" $id)` -- the explicit-target form, as opposed to
    /// the "register the current module" form already covered above --
    /// needs the identical `$id` resolution.
    #[test]
    fn register_with_an_explicit_module_id_targets_that_module_not_the_current_one() {
        let results = outcomes(
            r#"
            (module $Earlier (func (export "answer") (result i32) i32.const 42))
            (module (func (export "unrelated") (result i32) i32.const 0))
            (register "E" $Earlier)
            (module
              (import "E" "answer" (func $answer (result i32))))
            (assert_return (invoke $Earlier "answer") (i32.const 42))
            "#,
        );
        assert_eq!(results[2], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(results[3], (DirectiveKind::Module, DirectiveOutcome::Pass), "import against the EARLIER module must link");
        assert_eq!(results[4], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    /// W35 fourth slice (`code/specs/W35-wasm-cross-instance-function-
    /// identity.md`): a hand-built, minimal version of `linking.wast`'s
    /// own motivating case, proving the fix end-to-end without relying on
    /// the corpus. `$A` exports a table and writes ITS OWN local function
    /// into slot 0 via an active elem segment; `$B` imports that SAME
    /// table (the shared `Rc<RefCell<TableStorage>>`, per W28) and
    /// OVERWRITES slot 0 with a DIFFERENT local function of its own, via
    /// `$B`'s OWN active elem segment (in `$B`'s own combined function-
    /// index space, unrelated to `$A`'s). `$A`'s own `call_indirect`
    /// through that same slot must then observe `$B`'s write (222), not
    /// `$A`'s own original one (111) -- the exact bug `resolve_owned_
    /// funcrefs`/`resolve_all_table_funcrefs`'s post-instantiation fixup
    /// pass exists to close (before this slice, `$A`'s own `call_indirect`
    /// would resolve the raw index `$B` wrote against `$A`'s OWN
    /// combined index space instead, silently returning the WRONG
    /// function's result).
    #[test]
    fn a_funcref_written_by_one_instance_into_a_table_shared_with_another_dispatches_to_the_writers_own_function() {
        let results = outcomes(
            r#"
            (module $A
              (type $t (func (result i32)))
              (table (export "tab") 2 funcref)
              (elem (i32.const 0) $a_func)
              (func $a_func (result i32) (i32.const 111))
              (func (export "call0") (result i32) (call_indirect (type $t) (i32.const 0))))
            (register "A" $A)
            (module $B
              (type $t (func (result i32)))
              (table (import "A" "tab") 2 funcref)
              (elem (i32.const 0) $b_func)
              (func $b_func (result i32) (i32.const 222)))
            (assert_return (invoke $A "call0") (i32.const 222))
            "#,
        );
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::Module, DirectiveOutcome::Pass), "$B must link against $A's exported table");
        assert_eq!(
            results[3],
            (DirectiveKind::AssertReturn, DirectiveOutcome::Pass),
            "expected $A's own call_indirect to observe $B's OVERWRITE (222), not $A's original write (111) -- \
             got: {:?}",
            results[3].1
        );
    }

    /// W35 fourth slice: the `owner_instance_identity`-for-IMPORTS
    /// correctness gap this slice's own corpus verification found and
    /// fixed, isolated into a minimal, hand-built reproduction -- mirrors
    /// `linking.wast`'s own `$Mt`/`$Ot`/`h` example exactly (`$Mt` exports
    /// `h`; `$Ot` imports it as `$Ot`'s OWN combined-index-space slot 0,
    /// then writes THAT slot into `$Mt`'s shared table via `$Ot`'s own
    /// active elem segment). Before this slice's fix, `resolve_func_ref_
    /// for_instance`'s import branch tagged the resulting `FuncRefTarget`
    /// with `owner_instance_identity: None` ("dispatchable via local_index
    /// in ANY ctx"), which is FALSE the moment that target is written into
    /// a table `$Mt`'s own ctx later reads: `$Mt` has no imports of its
    /// own, so `local_index: Some(0)` in `$Mt`'s combined space names an
    /// entirely different (`$Mt`'s own local) function -- confirmed to
    /// silently produce the WRONG result (this exact test previously
    /// returned `$Mt`'s own `other` function's value, `999`, instead of
    /// `h`'s `-4`-shaped value, `77`, before the fix landed).
    #[test]
    fn a_table_entry_written_via_an_imported_function_dispatches_through_the_real_exporter_not_the_readers_own_index_space() {
        let results = outcomes(
            r#"
            (module $Mt
              (type $t (func (result i32)))
              (table (export "tab") 2 funcref)
              (func $other (result i32) (i32.const 999))
              (func (export "h") (result i32) (i32.const 77))
              (func (export "call0") (result i32) (call_indirect (type $t) (i32.const 0))))
            (register "Mt" $Mt)
            (module $Ot
              (type $t (func (result i32)))
              (func $h (import "Mt" "h") (result i32))
              (table (import "Mt" "tab") 2 funcref)
              (elem (i32.const 0) $h))
            (assert_return (invoke $Mt "call0") (i32.const 77))
            "#,
        );
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::Module, DirectiveOutcome::Pass), "$Ot must link against $Mt's exports");
        assert_eq!(
            results[3],
            (DirectiveKind::AssertReturn, DirectiveOutcome::Pass),
            "expected $Mt's own call_indirect to reach the REAL h (77) via $Ot's import-derived write, not \
             misinterpret local_index 0 in $Mt's OWN space ($other, 999) -- got: {:?}",
            results[3].1
        );
    }

    /// W35 fifth slice (`code/specs/W35-wasm-cross-instance-function-
    /// identity.md`): a hand-built, minimal reproduction of `elem.wast`'s
    /// own "Initializing a table with imported funcref global" case --
    /// this crate's own corpus baseline's LAST remaining real (non-"not
    /// yet supported") failure anywhere in the 257-file testsuite before
    /// this slice. `$module4` exports a funcref-typed GLOBAL, populated
    /// via `ref.func` on one of ITS OWN local functions (`$const-i32`,
    /// which returns 42); the importer imports that global and writes it
    /// into ITS OWN table via an active elem segment whose item is
    /// `(global.get 0)`, not a literal `ref.func`/`ref.null` -- exactly
    /// the shape `resolve_exported_global_funcrefs`/`element_func_refs`
    /// exist to carry a real cross-instance-safe `FuncRefTarget` through.
    /// `call_imported_elem`'s own `call_indirect` through that table slot
    /// must then invoke `$module4`'s real function (42), not misdispatch
    /// to `call_imported_elem`'s OWN local index 0 (itself) -- which, at
    /// the exact byte-level coincidence this corpus case has, is the
    /// unbounded self-recursion this fix closes: before it, this same
    /// scenario traps with "call stack exhausted" (a stack overflow, not
    /// merely a wrong numeric answer) instead of returning 42.
    #[test]
    fn a_table_entry_populated_via_an_imported_funcref_global_dispatches_to_the_exporters_own_function() {
        let results = outcomes(
            r#"
            (module $module4
              (func $const-i32 (result i32) (i32.const 42))
              (global (export "f") funcref (ref.func $const-i32)))
            (register "module4" $module4)
            (module
              (import "module4" "f" (global funcref))
              (type $out-i32 (func (result i32)))
              (table 10 funcref)
              (elem (offset (i32.const 0)) funcref (global.get 0))
              (func (export "call_imported_elem") (type $out-i32)
                (call_indirect (type $out-i32) (i32.const 0))))
            (assert_return (invoke "call_imported_elem") (i32.const 42))
            "#,
        );
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(
            results[2],
            (DirectiveKind::Module, DirectiveOutcome::Pass),
            "the importer must link against $module4's exported funcref global"
        );
        assert_eq!(
            results[3],
            (DirectiveKind::AssertReturn, DirectiveOutcome::Pass),
            "expected call_imported_elem's own call_indirect to reach $module4's REAL function (42) via the \
             imported global's resolved FuncRefTarget, not misdispatch to its own local index 0 (itself) -- \
             got: {:?}",
            results[3].1
        );
    }

    /// W35 fourth slice: the "ephemeral trap-discarded instance" case --
    /// `linking3.wast`'s own `$Ms`/`"get table[0]"` example, hand-built.
    /// An anonymous module (wrapped in `assert_trap`, so it goes through
    /// `grade_assert_unlinkable`'s throwaway `instantiate()` call, never
    /// registered anywhere) imports `$M`'s shared table, writes its OWN
    /// local `$f` into slot 0 via an ACTIVE elem segment (which succeeds),
    /// then its `(start $main)` calls `unreachable`, discarding the
    /// `WasmInstance` `instantiate()` would otherwise have returned. `$M`'s
    /// own LATER `call_indirect` through that same slot must still observe
    /// `$f`'s real value -- proving `wasm_runtime::instantiate()`'s own
    /// error-path fixup (a TEMPORARY `Rc<RefCell<WasmInstance>>`, built
    /// from this call's live state and never `try_unwrap`ed, just before
    /// propagating the trap) correctly keeps the ephemeral instance alive
    /// via the `FuncRefTarget`'s own `Rc` clone, embedded in the SHARED
    /// table before the trap ever discarded anything.
    #[test]
    fn a_funcref_written_by_a_module_whose_own_instantiation_later_traps_still_dispatches_correctly() {
        let results = outcomes(
            r#"
            (module $M
              (type $t (func (result i32)))
              (table (export "tab") 1 funcref)
              (func (export "call0") (result i32) (call_indirect (type $t) (i32.const 0))))
            (register "M" $M)
            (assert_trap
              (module
                (table (import "M" "tab") 1 funcref)
                (elem (i32.const 0) $f)
                (func $f (result i32) (i32.const 57005))
                (func $main (unreachable))
                (start $main))
              "unreachable")
            (assert_return (invoke $M "call0") (i32.const 57005))
            "#,
        );
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(
            results[2],
            // `wasm-wast-parser` maps `(assert_trap (module ...))` to
            // `Directive::AssertUnlinkable` (the same "outcome CATEGORY,
            // not the specific reason" bucket `assert_unlinkable` proper
            // uses -- see `grade_assert_unlinkable`'s own doc comment),
            // not `Directive::AssertTrap` (reserved for `assert_trap`
            // wrapping an ACTION, e.g. `(invoke ...)`).
            (DirectiveKind::AssertUnlinkable, DirectiveOutcome::Pass),
            "the anonymous module's own start function must genuinely trap"
        );
        assert_eq!(
            results[3],
            (DirectiveKind::AssertReturn, DirectiveOutcome::Pass),
            "expected $M's own call_indirect to observe $f's value (57005) written by the now-discarded \
             instance's own elem segment before its start function trapped -- got: {:?}",
            results[3].1
        );
    }

    /// W35 fourth slice, security-review finding (HIGH): a real,
    /// deterministic `RefCell` re-entrant-borrow panic this slice's own
    /// fixup pass made newly reachable through an entirely ORDINARY,
    /// non-circular linking pattern -- `$B` calls into `$A` (an ordinary
    /// cross-module `call`, holding `$B`'s own `Rc<RefCell<WasmInstance>>`
    /// borrowed for the call's whole duration); `$A`'s own `call_indirect`
    /// then dispatches a table entry `$B` itself earlier wrote (a
    /// `LocalFunctionRef` targeting `$B`) -- `$A`'s own `effective_local_
    /// index` can't find `$B`'s function in `$A`'s own `func_identities`
    /// (it was never imported by `$A`), so dispatch falls through to
    /// `target.callable.call(..)`, re-entering `$B`'s OWN, ALREADY mutably
    /// borrowed instance. This is NOT `CrossModuleFunction`'s own already-
    /// documented "genuinely mutual cross-instance cycle" risk (`$B`
    /// calls `$A` exactly once; `$A` never calls back into `$B` via an
    /// import of its own -- it merely dispatches a stored reference).
    /// Before the fix (`LocalFunctionRef::call`/`CrossModuleFunction::
    /// call` using `try_borrow_mut` instead of a bare `borrow_mut()`),
    /// this reproducibly PANICKED (a process abort, not a graded
    /// directive outcome) on this exact, entirely ordinary script.
    #[test]
    fn a_reentrant_dispatch_back_into_the_caller_traps_cleanly_instead_of_panicking() {
        let results = outcomes(
            r#"
            (module $A
              (type $t (func (result i32)))
              (table (export "tab") 1 funcref)
              (func (export "call0") (result i32) (call_indirect (type $t) (i32.const 0))))
            (register "A" $A)
            (module $B
              (func $callA (import "A" "call0") (result i32))
              (table (import "A" "tab") 1 funcref)
              (elem (i32.const 0) $b0)
              (func $b0 (result i32) (i32.const 222))
              (func (export "go") (result i32) (call $callA)))
            (register "B" $B)
            (assert_return (invoke $B "go") (i32.const 222))
            "#,
        );
        // The point of this test is that the process is still alive to
        // check an outcome at all -- a panic here would abort the whole
        // test binary, not merely fail this one assertion. Whether the
        // graded outcome is `Pass` (if `$A`'s own dispatch happens to
        // reach `$b0` some other way) or a clean `Fail`/`Trap` (the
        // re-entrant-borrow trap) is secondary; NEITHER may be a panic.
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::Module, DirectiveOutcome::Pass), "$B must link against $A's exports");
        assert_eq!(results[3], (DirectiveKind::Register, DirectiveOutcome::Pass));
        match &results[4].1 {
            DirectiveOutcome::Pass | DirectiveOutcome::Fail(_) | DirectiveOutcome::Trap(_) => {}
            other => panic!("expected a graded outcome (Pass/Fail/Trap), got: {other:?}"),
        }
    }

    /// W35 fourth slice, security-review finding (MEDIUM): a raw table
    /// entry a LIVE `table.init` writes (deliberately deferred to lazy,
    /// same-instance-only resolution -- see `wasm-execution`'s own
    /// `table.init` opcode handler doc comment; this slice never changed
    /// that) must NOT be misattributed to a LATER instance's own fixup
    /// pass just because that instance happens to import the same table.
    /// `$A` writes `$a0` into its OWN table via a LIVE `table.init` (never
    /// touched by any instantiate()-time fixup at all); `$B` merely
    /// IMPORTS that table, writing nothing of its own. `$B`'s own fixup
    /// pass must have NOTHING to resolve (`active_elem_writes` is empty
    /// for `$B`), so `$A`'s own later `call_indirect` through that same
    /// slot must still observe `$A`'s own value (111) -- not get silently
    /// reattributed to `$B`'s combined index space. An earlier version of
    /// `resolve_all_table_funcrefs` (a scan for `TableElement::Raw`
    /// entries in every visible table, rather than a precise, RECORDED
    /// write-list) could not tell `$A`'s own live-call write apart from
    /// something `$B` itself should resolve, and got this wrong.
    #[test]
    fn a_raw_entry_written_by_a_live_table_init_is_not_reattributed_to_a_later_importing_instance() {
        let results = outcomes(
            r#"
            (module $A
              (type $t (func (result i32)))
              (table (export "tab") 1 funcref)
              (func $a0 (result i32) (i32.const 111))
              (elem $e func $a0)
              (func (export "init_and_call") (result i32)
                (table.init 0 $e (i32.const 0) (i32.const 0) (i32.const 1))
                (call_indirect (type $t) (i32.const 0))))
            (register "A" $A)
            (module $B
              (table (import "A" "tab") 1 funcref))
            (register "B" $B)
            (assert_return (invoke $A "init_and_call") (i32.const 111))
            "#,
        );
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::Module, DirectiveOutcome::Pass), "$B must link against $A's exported table");
        assert_eq!(results[3], (DirectiveKind::Register, DirectiveOutcome::Pass));
        assert_eq!(
            results[4],
            (DirectiveKind::AssertReturn, DirectiveOutcome::Pass),
            "expected $A's own live table.init + call_indirect to observe $A's own value (111), unaffected \
             by $B merely importing the same table -- got: {:?}",
            results[4].1
        );
    }

    // ── W14: a per-module build failure degrades gracefully ─────────────

    #[test]
    fn a_module_that_fails_to_build_grades_not_yet_supported_and_does_not_abort_the_script() {
        // The real motivating case: simd_const.wast's sole i64x2.add usage
        // (an opcode this repo doesn't implement) previously aborted
        // wasm_wast_parser::parse_script for the WHOLE file. Now
        // Directive::Module(Err(_)) is just data -- everything before,
        // around, and after the broken module still parses and grades for
        // real. Two independently buildable modules bracket a broken one
        // here, proving the fix isn't order-dependent.
        let results = outcomes(
            r#"
            (module (func (export "f") (result i32) (i32.const 1)))
            (assert_return (invoke "f") (i32.const 1))
            (module (func (export "g") (result i32) (this.is.not.a.real.opcode)))
            (module (func (export "h") (result i32) (i32.const 2)))
            (assert_return (invoke "h") (i32.const 2))
            "#,
        );
        assert_eq!(results.len(), 5);
        assert_eq!(results[0], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
        assert!(matches!(results[2].1, DirectiveOutcome::NotYetSupported(_)), "{:?}", results[2]);
        assert_eq!(results[3], (DirectiveKind::Module, DirectiveOutcome::Pass));
        assert_eq!(results[4], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn invoking_the_broken_current_module_grades_not_yet_supported_not_a_stale_pass() {
        // The registry-clearing fix, isolated: a module that fails to
        // build must NOT leave the PREVIOUS module silently addressable
        // as "current" -- a bare `(invoke "f")` right after the broken
        // module must grade NotYetSupported, never re-run against
        // $good1's stale instance and produce a misleading Pass.
        let results = outcomes(
            r#"
            (module $good1 (func (export "f") (result i32) (i32.const 1)))
            (module (func (export "f") (result i32) (this.is.not.a.real.opcode)))
            (assert_return (invoke "f") (i32.const 1))
            "#,
        );
        assert_eq!(results.len(), 3);
        assert!(matches!(results[1].1, DirectiveOutcome::NotYetSupported(_)));
        assert!(matches!(results[2].1, DirectiveOutcome::NotYetSupported(_)), "{:?}", results[2]);
    }

    #[test]
    fn a_structurally_invalid_module_does_not_leave_a_stale_module_addressable_as_current() {
        // Distinct from the build-failure case above: a structural-
        // validation failure (duplicate export name) does NOT set
        // `current_module_status` (it's graded `Fail`, a real problem,
        // not a capability gap) -- so this test exercises ONLY the
        // registry-clearing fix in isolation. Without it, a bare
        // `(invoke "f")` after the broken module would silently re-run
        // against the FIRST module's still-registered instance and
        // produce a misleading Pass instead of a clean "no module
        // registered" Trap.
        let results = outcomes(
            r#"
            (module (func (export "f") (result i32) (i32.const 1)))
            (module
              (func (export "f") (result i32) i32.const 0)
              (func (export "f") (result i32) i32.const 1))
            (assert_return (invoke "f") (i32.const 1))
            "#,
        );
        assert_eq!(results.len(), 3);
        assert!(matches!(results[1].1, DirectiveOutcome::Fail(_)), "{:?}", results[1]);
        assert!(matches!(results[2].1, DirectiveOutcome::Fail(_)), "{:?}", results[2]);
    }

    #[test]
    fn registering_the_broken_current_module_grades_not_yet_supported_not_a_hard_fail() {
        // A capability gap propagates through `register` too, per the
        // spec's design: `register` right after a broken module reports
        // WHY there's no current module (a real gap), not the generic
        // "register: no current module" Fail reserved for a genuine
        // test-script-structure problem (e.g. `register` as the very
        // first directive in a file, with no module ever attempted).
        let results = outcomes(
            r#"
            (module (func (export "f") (result i32) (this.is.not.a.real.opcode)))
            (register "M")
            "#,
        );
        assert_eq!(results.len(), 2);
        assert!(matches!(results[1].1, DirectiveOutcome::NotYetSupported(_)), "{:?}", results[1]);
    }

    #[test]
    fn register_with_genuinely_no_prior_module_still_hard_fails() {
        // Contrast with the test above: no module was ever attempted at
        // all, so this really is a test-script-structure problem, not a
        // capability gap -- the pre-existing hardcoded Fail is preserved
        // for this case.
        let results = outcomes(r#"(register "M")"#);
        assert_eq!(results[0], (DirectiveKind::Register, DirectiveOutcome::Fail("register: no current module to register".to_string())));
    }

    #[test]
    fn module_instance_generative_instantiation_gives_independent_state() {
        // `instance.wast`'s own "Instantiation is generative" shape: the
        // SAME definition instantiated twice must give two INDEPENDENT
        // mutable globals -- mutating one instance's global must not be
        // observable through the other.
        let results = outcomes(
            r#"
            (module definition $M (global (export "g") (mut i32) (i32.const 0)))
            (module instance $I1 $M)
            (module instance $I2 $M)
            (register "I1" $I1)
            (register "I2" $I2)
            (module
              (import "I1" "g" (global $g1 (mut i32)))
              (import "I2" "g" (global $g2 (mut i32)))
              (func (export "run") (result i32)
                (global.set $g1 (i32.const 1))
                (global.get $g2)
              )
            )
            (assert_return (invoke "run") (i32.const 0))
            "#,
        );
        for (kind, outcome) in &results {
            assert!(outcome.is_pass(), "expected every directive to pass, got {kind:?} -> {outcome:?}");
        }
    }

    #[test]
    fn module_instance_shares_state_across_multiple_imports_of_the_same_instance() {
        // `instance.wast`'s own "Import is not generative" shape: TWO
        // imports of the SAME registered instance must resolve to the SAME
        // underlying memory, not independent copies -- exercised via
        // memory (not a mutable global) because `RegistryHost::
        // resolve_memory`'s `LinearMemory` is the one export kind this
        // crate already gives a real shared live view across import
        // boundaries (the W28 fix `resolve_memory`'s own doc comment
        // describes); `resolve_global` still copies the value at import
        // time, a separate, pre-existing, unrelated gap this PR doesn't
        // touch.
        let results = outcomes(
            r#"
            (module definition $M (memory (export "mem") 1))
            (module instance $I $M)
            (register "I" $I)
            (module
              (import "I" "mem" (memory $mem1 1))
              (import "I" "mem" (memory $mem2 1))
              (func (export "run") (result i32)
                (i32.store $mem1 (i32.const 0) (i32.const 1))
                (i32.load $mem2 (i32.const 0))
              )
            )
            (assert_return (invoke "run") (i32.const 1))
            "#,
        );
        for (kind, outcome) in &results {
            assert!(outcome.is_pass(), "expected every directive to pass, got {kind:?} -> {outcome:?}");
        }
    }

    #[test]
    fn module_instance_of_an_anonymous_definition_never_becomes_current() {
        // An anonymous `(module definition (fields...))` (no `$name`, real
        // `memory.wast`/`table.wast` shape) is validated but must NOT
        // become "the current module" -- a later unqualified action must
        // still resolve against whatever plain `(module ...)` directive
        // came after it, not this definition.
        let results = outcomes(
            r#"
            (module definition (memory 1))
            (module (func (export "f") (result i32) (i32.const 42)))
            (assert_return (invoke "f") (i32.const 42))
            "#,
        );
        assert_eq!(results[2], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn module_instance_referencing_an_unbuilt_definition_is_not_yet_supported() {
        // `type-rec.wast`'s own shape (a definition that fails to BUILD,
        // not just to instantiate): referencing it later must degrade
        // gracefully, not panic or hard-fail.
        let results = outcomes(
            r#"
            (module definition $M (func (this.is.not.a.real.opcode)))
            (module instance $I $M)
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::NotYetSupported(_)), "{:?}", results[1]);
    }

    #[test]
    fn register_of_an_id_that_never_built_for_a_capability_gap_is_not_yet_supported_not_fail() {
        // The general `Register` fix this same investigation made: a
        // `register` naming an explicit `$id` that never built for a real
        // capability-gap reason (here, `module instance` referencing a
        // definition that never built) must grade the same honest
        // `NotYetSupported` a `register` against "the current module"
        // already gets -- NOT the hard `Fail` reserved for a genuine
        // script-structure bug (an `$id` that was simply never mentioned
        // by ANY module directive at all -- see the sibling test below).
        let results = outcomes(
            r#"
            (module definition $M (func (this.is.not.a.real.opcode)))
            (module instance $I $M)
            (register "I" $I)
            "#,
        );
        assert!(matches!(results[2].1, DirectiveOutcome::NotYetSupported(_)), "{:?}", results[2]);
    }

    #[test]
    fn register_of_an_id_that_was_never_mentioned_at_all_still_hard_fails() {
        // Contrast with the capability-gap case above: `$Never` is not a
        // typo for a real gap, it's simply never been the target of ANY
        // module directive -- a genuine script-structure bug, still a
        // hard `Fail`.
        let results = outcomes(r#"(register "M" $Never)"#);
        assert!(matches!(results[0].1, DirectiveOutcome::Fail(_)), "{:?}", results[0]);
    }

    #[test]
    fn get_action_reads_a_global_export() {
        let results = outcomes(
            r#"
            (module (global (export "g") i32 (i32.const 7)))
            (assert_return (get "g") (i32.const 7))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn module_with_unresolved_import_marks_invoke_not_yet_supported() {
        // WASM05/W10: an import from a module name that is neither a real
        // `register`ed sibling NOR the built-in `spectest` stub (W07
        // addendum 2 item 4) genuinely fails to LINK (a real capability
        // gap) rather than hitting the old blanket "any import present"
        // rule -- the failure cascades (`NotYetSupported`, via
        // `current_module_status`) to the following `assert_return`.
        // Uses a deliberately unknown module name here (NOT `spectest` --
        // see `spectest_global_i32_import_resolves_to_the_real_upstream_
        // value_666` immediately below for that now-fully-supported case).
        let results = outcomes(
            r#"
            (module
              (import "totally-unknown-module" "global_i32" (global i32))
              (func (export "get_g") (result i32) global.get 0))
            (assert_return (invoke "get_g") (i32.const 666))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::NotYetSupported(_)));
    }

    // ── W07 addendum 2 item 4: the built-in `spectest` fixture module ───────

    #[test]
    fn spectest_global_i32_import_resolves_to_the_real_upstream_value_666() {
        // `global.wast`'s own `get-z1`/`get-z2` shape (real corpus): a
        // module imports `spectest.global_i32`, re-exports it via a
        // trivial getter, and the corpus expects exactly `666` --
        // verified live against the real upstream `spectest.ml` source
        // (see `SpectestModule`'s own doc comment), not guessed.
        let results = outcomes(
            r#"
            (module
              (import "spectest" "global_i32" (global i32))
              (func (export "get_g") (result i32) global.get 0))
            (assert_return (invoke "get_g") (i32.const 666))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn spectest_global_i64_and_float_globals_resolve_to_real_upstream_values() {
        let results = outcomes(
            r#"
            (module
              (import "spectest" "global_i64" (global i64))
              (import "spectest" "global_f32" (global f32))
              (import "spectest" "global_f64" (global f64))
              (func (export "get_i64") (result i64) global.get 0)
              (func (export "get_f32") (result f32) global.get 1)
              (func (export "get_f64") (result f64) global.get 2))
            (assert_return (invoke "get_i64") (i64.const 666))
            (assert_return (invoke "get_f32") (f32.const 666.6))
            (assert_return (invoke "get_f64") (f64.const 666.6))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
        assert_eq!(results[3], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn spectest_print_functions_are_callable_no_ops() {
        // No corpus directive ever asserts on printed output -- only that
        // the import resolves and the call succeeds with no trap and (for
        // `print_i32_f32`) the right arity of results (none). Exercises
        // both a zero-arg and a two-arg `print*` export.
        let results = outcomes(
            r#"
            (module
              (import "spectest" "print" (func))
              (import "spectest" "print_i32_f32" (func (param i32 f32)))
              (func (export "run")
                call 0
                (call 1 (i32.const 1) (f32.const 2.0))))
            (assert_return (invoke "run"))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn spectest_memory_and_table_resolve_with_real_upstream_limits_and_are_usable() {
        // `memory`: min 1 / max 2 pages. `table`: min 10 / max 20,
        // funcref -- both verified live against the real upstream
        // `spectest.ml` source (see `SpectestModule`'s own doc comment).
        // Exercised for real (`memory.size`/`table.size`), not just
        // resolved, to confirm the stub is backed by genuinely usable
        // `LinearMemory`/`Table` values, not a dummy placeholder.
        let results = outcomes(
            r#"
            (module
              (import "spectest" "memory" (memory 1 2))
              (import "spectest" "table" (table 10 20 funcref))
              (func (export "mem_size") (result i32) memory.size)
              (func (export "tbl_size") (result i32) table.size))
            (assert_return (invoke "mem_size") (i32.const 1))
            (assert_return (invoke "tbl_size") (i32.const 10))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
        assert_eq!(results[2], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn spectest_table64_import_resolves_as_a_real_is64_table() {
        let results = outcomes(
            r#"
            (module
              (import "spectest" "table64" (table i64 10 20 funcref))
              (func (export "tbl_size") (result i64) table.size))
            (assert_return (invoke "tbl_size") (i64.const 10))
            "#,
        );
        assert_eq!(results[1], (DirectiveKind::AssertReturn, DirectiveOutcome::Pass));
    }

    #[test]
    fn spectest_unknown_export_name_is_still_a_genuine_link_failure() {
        // The corpus's own `imports.wast`/`imports2.wast` deliberately
        // import `spectest.unknown` inside `assert_unlinkable` cases --
        // `"unknown"` is NOT a real `spectest` export (see
        // `SpectestModule`'s own doc comment for why it must stay
        // absent), so this must keep failing to link even though
        // `spectest` itself is now a real, resolvable host module.
        let results = outcomes(r#"(assert_unlinkable (module (import "spectest" "unknown" (func))) "unknown import")"#);
        assert_eq!(results[0], (DirectiveKind::AssertUnlinkable, DirectiveOutcome::Pass));
    }

    // ── W34 fourth slice: cross-module canonical type-group equivalence
    // (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`) ──────────────

    /// Two independently-declared, structurally-identical `rec` groups, at
    /// GENUINELY different flat type-section indices in each module (the
    /// importer has two unrelated padding types declared FIRST, so its own
    /// `rec` group starts at index 2, not 0 like the exporter's) -- no
    /// shared numbering between the two modules at all, exactly the
    /// cross-module comparability property the whole canonicalization
    /// algorithm exists for (MVP.md's own "no shared context... upfront").
    /// Before this slice, `wasm-runtime`'s import check only compared raw
    /// `FuncType` shape plus `(rec_group_size, rec_group_position)` plus
    /// finality -- none of which is even DECLARED equal here in a way that
    /// proves real canonical equivalence rather than coincidental shape
    /// matching, so this is a genuine, not vacuous, positive proof point
    /// (mirrors `type-subtyping.wast`'s own `M3`/`M4` "Linking" cases, and
    /// `type-equivalence.wast`'s "Semantic types (link time)" section).
    #[test]
    fn cross_module_isomorphic_rec_groups_with_no_shared_numbering_link_successfully() {
        // The MVP.md/`type-equivalence.wast` "Isomorphic recursive types"
        // headline shape (two mutually-referencing members, no `sub`
        // relation declared at all): `$a1` and `$a2` (exporter) tie to the
        // identical shape as `$b1`/`$b2` (importer) once De-Bruijn-numbered
        // relative to their OWN group, regardless of the group's absolute
        // starting index in each module.
        let results = outcomes(
            r#"
            (module
              (rec
                (type $a1 (func (param i32 (ref $a2))))
                (type $a2 (func (param i32 (ref $a1))))
              )
              (func (export "g") (type $a2))
            )
            (register "IsoExport")
            (module
              (type $pad0 (func (param i32)))
              (type $pad1 (func (param i64)))
              (rec
                (type $b1 (func (param i32 (ref $b2))))
                (type $b2 (func (param i32 (ref $b1))))
              )
              (func (import "IsoExport" "g") (type $b2))
            )
            "#,
        );
        // The importing module's own directive is index 2 (export module=0,
        // register=1, import module=2) -- `Pass` here means real linking
        // succeeded, not merely that the directive was gradeable.
        assert_eq!(results[2], (DirectiveKind::Module, DirectiveOutcome::Pass), "{:?}", results[2]);
    }

    /// The `M5`-shaped negative case (`type-subtyping.wast` lines 652-666,
    /// this crate's own vendored corpus copy): superficially similar to the
    /// positive case above (the same type NAMES, `$f1`/`$f2`/`$g1`/`$g2`,
    /// are deliberately reused across both modules, copy-paste-style), but
    /// ONE internal reference is wired to a DIFFERENT earlier group than
    /// its counterpart -- the exporter's `$g2`'s declared supertype `$f2`
    /// sits in a `rec` group whose OTHER member (an anonymous struct)
    /// references `$f1` (a group declared even EARLIER, NOT itself), while
    /// the importer's `$g1`'s declared supertype `$f1`'s own sibling struct
    /// references `$f1` REFLEXIVELY (`Rec(0)`, its own group). Canonicalized,
    /// these tie to genuinely different shapes (`Outer` vs `Rec` at that
    /// position) despite every group's own size/position/finality/`FuncType`
    /// shape matching -- exactly the class of mismatch the OLD three-part
    /// conservative guard could never see, and canonical equivalence must
    /// NOT be fooled into accepting.
    #[test]
    fn cross_module_copy_paste_shaped_type_mismatch_is_correctly_rejected() {
        let results = outcomes(
            r#"
            (module
              (rec (type $f1 (sub (func))) (type (struct (field (ref $f1)))))
              (rec (type $f2 (sub (func))) (type (struct (field (ref $f1)))))
              (rec (type $g2 (sub $f2 (func))) (type (struct)))
              (func (export "g") (type $g2))
            )
            (register "Sneaky")
            (assert_unlinkable
              (module
                (rec (type $f1 (sub (func))) (type (struct (field (ref $f1)))))
                (rec (type $g1 (sub $f1 (func))) (type (struct)))
                (func (import "Sneaky" "g") (type $g1))
              )
              "incompatible import type"
            )
            "#,
        );
        assert_eq!(results[2], (DirectiveKind::AssertUnlinkable, DirectiveOutcome::Pass), "{:?}", results[2]);
    }
}
