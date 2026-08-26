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
//!   sibling module for real — but there's no real `spectest` host module
//!   (the official test harness's own fixture module), so an import from
//!   it still correctly grades `NotYetSupported`, not `Fail`.
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
use wasm_execution::{HostFunction, HostInterface, LinearMemory, Table, TrapError, V128Bytes, WasmValue};
use wasm_module_parser::WasmModuleParser;
use wasm_runtime::{WasmInstance, WasmRuntime};
use wasm_types::{ExternalKind, FuncType, GlobalType};
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

/// A `HostInterface` backed by the `Executor`'s own module registry
/// (WASM05/W10) -- lets a module import a function/memory/table/global
/// from a `register`ed sibling module in the same script, exactly the
/// shape the real corpus's own `assert_unlinkable`/linking cases use
/// (`register "test"` earlier in the script, then `(import "test" ...)`
/// later). No `spectest` support -- `resolve_*` simply returns `None`
/// for any module name not found in the registry, which correctly
/// surfaces as a link failure without needing a real `spectest` host.
struct RegistryHost {
    /// `Rc<RefCell<..>>`, not a borrowed reference: `HostInterface` (like
    /// any trait consumed as `Box<dyn HostInterface>`) is implicitly
    /// `'static`, so a `RegistryHost` can't hold a borrow of `Executor`'s
    /// own fields -- it needs owned, shared access to the SAME
    /// underlying registry `Executor` itself reads/writes.
    registry: ModuleRegistry,
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
        let (instance_rc, index) = self.find_export(module_name, name, ExternalKind::Function)?;
        let func_type = instance_rc.borrow().func_types.get(index as usize)?.clone();
        Some(Box::new(CrossModuleFunction { instance: instance_rc, export_name: name.to_string(), func_type }))
    }

    fn resolve_global(&self, module_name: &str, name: &str) -> Option<(GlobalType, WasmValue)> {
        let (instance_rc, index) = self.find_export(module_name, name, ExternalKind::Global)?;
        let instance = instance_rc.borrow();
        let gtype = instance.global_types.get(index as usize)?.clone();
        let gval = *instance.globals.get(index as usize)?;
        Some((gtype, gval))
    }

    fn resolve_memory(&self, module_name: &str, name: &str) -> Option<LinearMemory> {
        let (instance_rc, index) = self.find_export(module_name, name, ExternalKind::Memory)?;
        // A real clone, not a shared live view: `HostInterface::
        // resolve_memory` returns an OWNED `LinearMemory`, not a
        // reference, so a genuinely SHARED-and-mutated-across-instances
        // memory import isn't observable through this path -- link-time
        // limits compatibility is still checked for real, but a write
        // through the importing instance won't become visible to the
        // exporting one. None of the corpus vendored so far exercises
        // that (its `assert_unlinkable`/`assert_invalid` cases only
        // probe link-time acceptance/rejection, never post-link shared
        // mutation), so this is a real, named limitation, not a silently
        // wrong answer -- revisit if a future vendored file needs it.
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
        let (instance_rc, index) = self.find_export(module_name, name, ExternalKind::Table)?;
        // Same clone-not-share caveat as `resolve_memory` above.
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
/// original caller instance) will panic on a `RefCell` double-borrow --
/// a clean, safe Rust panic (borrow-checked at runtime), not a
/// memory-safety issue. None of the corpus vendored so far is circular.
struct CrossModuleFunction {
    instance: Rc<RefCell<WasmInstance>>,
    export_name: String,
    func_type: FuncType,
}

impl HostFunction for CrossModuleFunction {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }

    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let mut instance = self.instance.borrow_mut();
        WasmRuntime::new().call_typed(&mut instance, &self.export_name, args)
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
}

impl Executor {
    fn new() -> Self {
        Executor {
            runtime: WasmRuntime::new(),
            registry: Rc::new(RefCell::new(HashMap::new())),
            current_module_status: None,
        }
    }

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
                self.registry.borrow_mut().remove(&None);
                let module = match *module_result {
                    Err(e) => {
                        let reason =
                            format!("module failed to parse/build (real capability gap, not a bug): {e}");
                        self.current_module_status = Some(reason.clone());
                        return DirectiveOutcome::NotYetSupported(reason);
                    }
                    Ok(module) => module,
                };
                match self.runtime.validate(&module) {
                    Err(e) => DirectiveOutcome::Fail(format!("module failed structural validation: {e}")),
                    Ok(validated) => {
                        let host = RegistryHost { registry: Rc::clone(&self.registry) };
                        match WasmRuntime::with_host(Box::new(host)).instantiate(&validated) {
                            Ok(instance) => {
                                let instance = Rc::new(RefCell::new(instance));
                                self.registry.borrow_mut().insert(None, Rc::clone(&instance));
                                // Task #93 (linking.wast): also register
                                // under the module's own `$id`, if it has
                                // one -- the SAME live instance (`Rc::clone`,
                                // not a copy), so a LATER `(invoke $id ...)`/
                                // `(register "M" $id)` can resolve back to
                                // this specific module even after other
                                // `(module ...)` directives have since
                                // become "the current module".
                                if let Some(id) = id {
                                    self.registry.borrow_mut().insert(Some(id), instance);
                                }
                                DirectiveOutcome::Pass
                            }
                            Err(e) if is_link_error(&e) => {
                                self.current_module_status = Some(e.to_string());
                                DirectiveOutcome::NotYetSupported(format!(
                                    "module failed to link (real capability gap, not a bug): {e}"
                                ))
                            }
                            Err(e) => DirectiveOutcome::Trap(format!("instantiation trapped: {e}")),
                        }
                    }
                }
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
                    // W14: if there's no current module BECAUSE the last
                    // module directive hit a genuine capability gap
                    // (build/link failure), that gap should propagate as
                    // NotYetSupported here too, not get flattened into a
                    // hard Fail that looks like a real test-script bug.
                    // Only applies to the "current module" (`None`) case --
                    // an explicit `$id` that's simply never been defined is
                    // a real script-level bug, not a capability gap.
                    None if key.is_none() => match &self.current_module_status {
                        Some(reason) => DirectiveOutcome::NotYetSupported(reason.clone()),
                        None => DirectiveOutcome::Fail("register: no current module to register".to_string()),
                    },
                    None => DirectiveOutcome::Fail(format!("register: no module registered as {key:?}")),
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
                    let host = RegistryHost { registry: Rc::clone(&self.registry) };
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
                    .and_then(|(_, _, idx)| instance.globals.get(*idx as usize).copied())
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
        // WASM05/W10: `spectest` isn't a `register`ed sibling module, so
        // this module now genuinely fails to LINK (a real capability
        // gap, `RegistryHost` has no `spectest` support) rather than
        // hitting the old blanket "any import present" rule -- same
        // outcome (`NotYetSupported`, cascading via `current_module_status`
        // to the following `assert_return`), different, more honest
        // reason underneath.
        let results = outcomes(
            r#"
            (module
              (import "spectest" "global_i32" (global i32))
              (func (export "get_g") (result i32) global.get 0))
            (assert_return (invoke "get_g") (i32.const 666))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::NotYetSupported(_)));
    }
}
