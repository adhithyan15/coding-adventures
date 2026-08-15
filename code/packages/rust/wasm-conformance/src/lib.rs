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
//! wrong. Two specific gaps in this repo's WASM stack make that claim
//! impossible to back up honestly for certain directive kinds — see each
//! one's own doc comment on [`Executor::execute`] for the reasoning, and
//! `code/specs/W05-wasm-conformance-harness.md` section 4.3 for the design
//! rationale. In short:
//! - `assert_invalid` needs an instruction-level type-checker
//!   `wasm-validator` doesn't have yet (`W02` designs it, isn't implemented).
//! - `assert_unlinkable` needs `WasmRuntime::instantiate` to actually be
//!   able to *fail* on an unresolved import — today it always silently
//!   falls back to a default value instead.
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
use wasm_execution::{TrapError, WasmValue};
use wasm_module_parser::WasmModuleParser;
use wasm_runtime::{WasmInstance, WasmRuntime};
use wasm_types::ExternalKind;
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

/// Walks a script's directives in order, maintaining the module registry
/// `invoke`/`register` need to resolve "the current module" vs. a
/// previously `register`ed one.
struct Executor {
    runtime: WasmRuntime,
    /// Keyed by `register` name, or `None` for "the current module" (the
    /// most recently processed `(module ...)` directive). A script that
    /// never uses `register` only ever touches the `None` entry.
    ///
    /// `Rc<RefCell<..>>`, not an owned `WasmInstance`, because a
    /// `register`ed module IS the same live instance as "current" -- same
    /// memory, same globals, same subsequent mutations -- not an
    /// independent copy (and `WasmInstance` isn't `Clone` anyway: it holds
    /// a `Box<dyn HostFunction>`).
    registry: HashMap<Option<String>, Rc<RefCell<WasmInstance>>>,
    /// Set when the current module has any import at all -- this repo has
    /// no host-import resolver (no `spectest`, no registry-backed linking),
    /// so an imported function/global/memory/table silently gets a default
    /// placeholder value instead of the real one (see
    /// `WasmRuntime::instantiate`'s doc comment). Any directive run against
    /// such a module is graded `NotYetSupported`, not `Fail` -- a wrong
    /// answer here would be "we didn't wire up linking," not "the
    /// interpreter is broken."
    current_has_imports: bool,
}

impl Executor {
    fn new() -> Self {
        Executor { runtime: WasmRuntime::new(), registry: HashMap::new(), current_has_imports: false }
    }

    fn execute(&mut self, directive: Directive) -> DirectiveOutcome {
        match directive {
            Directive::Module(module) => {
                self.current_has_imports = !module.imports.is_empty();
                match self.runtime.validate(&module) {
                    Err(e) => DirectiveOutcome::Fail(format!("module failed structural validation: {e}")),
                    Ok(validated) => match self.runtime.instantiate(&validated.module) {
                        Ok(instance) => {
                            self.registry.insert(None, Rc::new(RefCell::new(instance)));
                            DirectiveOutcome::Pass
                        }
                        Err(e) => DirectiveOutcome::Trap(format!("instantiation trapped: {e}")),
                    },
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
                match self.registry.get(&None) {
                    Some(current) => {
                        self.registry.insert(Some(name), Rc::clone(current));
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
                Ok(results) => {
                    if results.len() != expected.len() {
                        DirectiveOutcome::Fail(format!(
                            "expected {} result(s), got {}",
                            expected.len(),
                            results.len()
                        ))
                    } else {
                        match results.iter().zip(expected.iter()).find(|(r, e)| !value_matches_expected(r, e)) {
                            None => DirectiveOutcome::Pass,
                            Some((r, e)) => DirectiveOutcome::Fail(format!("expected {e:?}, got {r:?}")),
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

            // `wasm-runtime`'s `instantiate` never fails on an unresolved
            // import -- it always falls back to a default value (see this
            // crate's module-level doc comment) -- so there is currently no
            // path through which linking can be observed to fail.
            Directive::AssertUnlinkable { .. } => DirectiveOutcome::NotYetSupported(
                "WasmRuntime::instantiate never fails on an unresolved import, so linking failure can't be observed yet".to_string(),
            ),
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

    fn run_action(&mut self, action: &Action) -> Result<Vec<WasmValue>, ActionError> {
        match action {
            Action::Invoke { module, name, args } => {
                let key = module.clone();
                if self.registry_module_has_imports(&key) {
                    return Err(ActionError::NotYetSupported(
                        "module has unresolved imports (no host/linking support yet)".to_string(),
                    ));
                }
                let instance_rc = self
                    .registry
                    .get(&key)
                    .ok_or_else(|| ActionError::Trap(format!("no module registered as {key:?}")))?;
                let mut instance = instance_rc.borrow_mut();
                let wasm_args: Vec<WasmValue> = args.iter().map(const_value_to_wasm_value).collect();
                self.runtime
                    .call_typed(&mut instance, name, &wasm_args)
                    .map_err(|e: TrapError| ActionError::Trap(e.to_string()))
            }
            Action::Get { module, name } => {
                let key = module.clone();
                let instance_rc = self
                    .registry
                    .get(&key)
                    .ok_or_else(|| ActionError::Trap(format!("no module registered as {key:?}")))?;
                let instance = instance_rc.borrow();
                instance
                    .exports
                    .iter()
                    .find(|(n, kind, _)| n == name && *kind == ExternalKind::Global)
                    .and_then(|(_, _, idx)| instance.globals.get(*idx as usize).copied())
                    .map(|v| vec![v])
                    .ok_or_else(|| ActionError::Trap(format!("no global export named {name:?}")))
            }
        }
    }

    /// `Action::Invoke`/`Action::Get` target either "the current module"
    /// (`module: None`) or a `register`ed one (`module: Some(name)`) --
    /// this only tracks the CURRENT module's import status (`register`
    /// only ever registers the current module in this crate's simplified
    /// model), which is enough for every file this phase vendors.
    fn registry_module_has_imports(&self, key: &Option<String>) -> bool {
        key.is_none() && self.current_has_imports
    }
}

enum ActionError {
    Trap(String),
    NotYetSupported(String),
}

fn const_value_to_wasm_value(c: &ConstValue) -> WasmValue {
    match *c {
        ConstValue::I32(v) => WasmValue::I32(v),
        ConstValue::I64(v) => WasmValue::I64(v),
        ConstValue::F32Bits(bits) => WasmValue::F32(f32::from_bits(bits)),
        ConstValue::F64Bits(bits) => WasmValue::F64(f64::from_bits(bits)),
        // WASM17: `(ref.null func/extern)` -> Ref(None); `(ref.extern n)`
        // -> Ref(Some(n)). Falls out for free since `WasmValue::Ref` already
        // wraps the identical `Option<u32>` shape `ConstValue::Ref` does.
        ConstValue::Ref(v) => WasmValue::Ref(v),
    }
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

fn value_matches_expected(actual: &WasmValue, expected: &Expected) -> bool {
    match expected {
        Expected::Value(ConstValue::I32(v)) => matches!(actual, WasmValue::I32(a) if a == v),
        Expected::Value(ConstValue::I64(v)) => matches!(actual, WasmValue::I64(a) if a == v),
        Expected::Value(ConstValue::F32Bits(bits)) => matches!(actual, WasmValue::F32(a) if a.to_bits() == *bits),
        Expected::Value(ConstValue::F64Bits(bits)) => matches!(actual, WasmValue::F64(a) if a.to_bits() == *bits),
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

    #[test]
    fn assert_return_nan_canonical_accepts_either_sign_exact_payload() {
        assert!(value_matches_expected(&WasmValue::F32(f32::from_bits(0x7FC0_0000)), &Expected::NanCanonicalF32));
        assert!(value_matches_expected(&WasmValue::F32(f32::from_bits(0xFFC0_0000)), &Expected::NanCanonicalF32));
        assert!(!value_matches_expected(&WasmValue::F32(f32::from_bits(0x7FC0_0001)), &Expected::NanCanonicalF32));
    }

    #[test]
    fn assert_return_nan_arithmetic_accepts_any_payload_with_quiet_bit() {
        assert!(value_matches_expected(&WasmValue::F64(f64::from_bits(0x7FF8_0000_0000_002A)), &Expected::NanArithmeticF64));
        assert!(!value_matches_expected(
            &WasmValue::F64(f64::from_bits(0x7FF0_0000_0000_002A)), // quiet bit clear
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
    fn assert_unlinkable_is_always_not_yet_supported() {
        let results = outcomes(r#"(assert_unlinkable (module (import "m" "f" (func))) "unknown import")"#);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].1, DirectiveOutcome::NotYetSupported(_)));
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
