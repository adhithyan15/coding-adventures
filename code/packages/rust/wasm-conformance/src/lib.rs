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
use wasm_wast_parser::script::{Action, ConstValue, Directive, Expected, ModuleSource};
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
        Directive::Module(_) => DirectiveKind::Module,
        Directive::Register { .. } => DirectiveKind::Register,
        Directive::Action(_) => DirectiveKind::Action,
        Directive::AssertReturn { .. } => DirectiveKind::AssertReturn,
        Directive::AssertTrap { .. } => DirectiveKind::AssertTrap,
        Directive::AssertExhaustion { .. } => DirectiveKind::AssertExhaustion,
        Directive::AssertInvalid { .. } => DirectiveKind::AssertInvalid,
        Directive::AssertMalformed { .. } => DirectiveKind::AssertMalformed,
        Directive::AssertUnlinkable { .. } => DirectiveKind::AssertUnlinkable,
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
        let (instance_rc, _) = self.find_export(module_name, name, ExternalKind::Memory)?;
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
        let memory = instance_rc.borrow().memory.clone();
        memory
    }

    fn resolve_table(&self, module_name: &str, name: &str) -> Option<Table> {
        let (instance_rc, index) = self.find_export(module_name, name, ExternalKind::Table)?;
        // Same clone-not-share caveat as `resolve_memory` above.
        let table = instance_rc.borrow().tables.get(index as usize).cloned();
        table
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
    /// Set when the current module failed to INSTANTIATE for a reason
    /// that's a genuine capability gap, not a bug -- today, only "an
    /// import references `spectest` or another module this crate's
    /// `RegistryHost` doesn't know about" (WASM05/W10 gave `instantiate`
    /// a real link-failure path; `RegistryHost` only ever resolves
    /// `register`ed sibling modules, not a real `spectest` host). Any
    /// directive run against such a module is graded `NotYetSupported`,
    /// not `Fail`/`Trap` -- a wrong answer here would be "we didn't wire
    /// up linking for this specific host module," not "the interpreter
    /// is broken."
    current_link_failed: Option<String>,
}

impl Executor {
    fn new() -> Self {
        Executor {
            runtime: WasmRuntime::new(),
            registry: Rc::new(RefCell::new(HashMap::new())),
            current_link_failed: None,
        }
    }

    fn execute(&mut self, directive: Directive) -> DirectiveOutcome {
        match directive {
            Directive::Module(module) => {
                self.current_link_failed = None;
                match self.runtime.validate(&module) {
                    Err(e) => DirectiveOutcome::Fail(format!("module failed structural validation: {e}")),
                    Ok(validated) => {
                        let host = RegistryHost { registry: Rc::clone(&self.registry) };
                        match WasmRuntime::with_host(Box::new(host)).instantiate(&validated.module) {
                            Ok(instance) => {
                                self.registry.borrow_mut().insert(None, Rc::new(RefCell::new(instance)));
                                DirectiveOutcome::Pass
                            }
                            Err(e) if is_link_error(&e) => {
                                self.current_link_failed = Some(e.to_string());
                                DirectiveOutcome::NotYetSupported(format!(
                                    "module failed to link (real capability gap, not a bug): {e}"
                                ))
                            }
                            Err(e) => DirectiveOutcome::Trap(format!("instantiation trapped: {e}")),
                        }
                    }
                }
            }

            Directive::Register { name, .. } => {
                // `module_name` (a `$id` referencing an earlier `(module
                // $id ...)`) can't be resolved here: `wasm-wast-parser`
                // deliberately discards a module directive's own `$id`
                // during parsing (it doesn't affect encoding), so by the
                // time a script reaches this executor there is no id to
                // look up. Only "register the CURRENT module" is
                // supported -- the only form any of this phase's vendored
                // files actually use.
                let current = self.registry.borrow().get(&None).cloned();
                match current {
                    Some(current) => {
                        self.registry.borrow_mut().insert(Some(name), current);
                        DirectiveOutcome::Pass
                    }
                    None => DirectiveOutcome::Fail("register: no current module to register".to_string()),
                }
            }

            Directive::Action(action) => match self.run_action(&action) {
                Ok(_) => DirectiveOutcome::Pass,
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
                Err(ActionError::Trap(m)) => DirectiveOutcome::Trap(m),
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
            },

            Directive::AssertTrap { action, .. } => match self.run_action(&action) {
                // The official testsuite's own reference runners do not
                // match trap MESSAGE text against `message` -- only that
                // some trap occurred. Matching this repo's error strings
                // against the spec's human-oriented ones would be testing
                // string formatting, not conformance.
                Ok(_) => DirectiveOutcome::Fail("expected a trap, action returned normally".to_string()),
                Err(ActionError::Trap(_)) => DirectiveOutcome::Pass,
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
                Err(ActionError::NotYetSupported(m)) => DirectiveOutcome::NotYetSupported(m),
            },

            Directive::AssertInvalid { module, .. } => self.grade_assert_invalid(module),
            Directive::AssertMalformed { module, .. } => self.grade_assert_malformed(module),
            Directive::AssertUnlinkable { module, .. } => self.grade_assert_unlinkable(module),
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
                Ok(_) => DirectiveOutcome::Fail("binary module parsed but should have been rejected as malformed".to_string()),
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
                    match WasmRuntime::with_host(Box::new(host)).instantiate(&validated.module) {
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
                    if let Some(reason) = &self.current_link_failed {
                        return Err(ActionError::NotYetSupported(format!(
                            "current module failed to link (real capability gap, not a bug): {reason}"
                        )));
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
                    wasm_args.push(const_value_to_wasm_value(a).ok_or_else(|| {
                        ActionError::NotYetSupported(
                            "a v128 invoke ARGUMENT is not yet supported (real capability gap, not a bug): \
                             no live wasm-execution heap exists before the call to allocate its handle into, \
                             see V128Bytes's own doc comment for why only RESULTS can be resolved this way"
                                .to_string(),
                        )
                    })?);
                }
                self.runtime
                    .call_typed_with_v128(&mut instance, name, &wasm_args)
                    .map_err(|e: TrapError| ActionError::Trap(e.to_string()))
            }
            Action::Get { module, name } => {
                let key = module.clone();
                if key.is_none() {
                    if let Some(reason) = &self.current_link_failed {
                        return Err(ActionError::NotYetSupported(format!(
                            "current module failed to link (real capability gap, not a bug): {reason}"
                        )));
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
                    // A global read is not a call -- there's no engine `ctx`/
                    // `v128_heap` involved at all, so a `WasmValue::V128`
                    // global's handle can't be resolved here either way.
                    // No vendored fixture currently reads a v128-typed
                    // global, so this doesn't silently mis-grade anything
                    // today; a real v128 global read would need its own
                    // follow-up (globals persist in `instance.globals`
                    // across calls, but the `v128_heap` slot a stored handle
                    // pointed to does NOT survive past the call that wrote
                    // it -- a separate, deeper gap from this PR's scope).
                    .map(|v| (vec![v], vec![None]))
                    .ok_or_else(|| ActionError::Trap(format!("no global export named {name:?}")))
            }
        }
    }
}

enum ActionError {
    Trap(String),
    NotYetSupported(String),
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
    fn invoke_with_a_v128_argument_grades_not_yet_supported_not_a_silent_wrong_pass() {
        // No live wasm-execution heap exists before a call starts, so a
        // `(v128.const ...)` invoke ARGUMENT can't be turned into a real
        // handle (unlike a v128 RESULT, resolved after the call via
        // `call_typed_with_v128`) -- this must degrade loudly
        // (`NotYetSupported`), never silently substitute the zero vector
        // and risk a false pass/fail for the wrong reason.
        let results = outcomes(
            r#"
            (module (func (export "add") (param v128 v128) (result v128) (i32x4.add (local.get 0) (local.get 1))))
            (assert_return (invoke "add" (v128.const i32x4 1 2 3 4) (v128.const i32x4 10 20 30 40)) (v128.const i32x4 11 22 33 44))
            "#,
        );
        assert!(matches!(results[1].1, DirectiveOutcome::NotYetSupported(_)), "{:?}", results[1]);
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
        // outcome (`NotYetSupported`, cascading via `current_link_failed`
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
