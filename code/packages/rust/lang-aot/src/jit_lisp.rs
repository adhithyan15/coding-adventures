//! # McCarthy Lisp on the universal JIT (LANG77 / McCarthy W15)
//!
//! The eighth and final backend. `jit-core`'s [`GenericCirJit`] is a *universal
//! bytecode JIT*: any typed-IIR language plugs in by registering its builtins as
//! Rust callbacks. Unlike the AOT/LLVM tagged-word backends — which lower a
//! `call_builtin "lispy_*"` to a native `call __dyn_*` into the C runtime
//! (`dynval_runtime.c`) — the JIT dispatches the *same* `lispy_*` names to Rust
//! closures backed by the **shared [`lispy_runtime`] crate** (the C runtime's Rust
//! twin: an identical `u64` tagged-word model). So the JIT inherits the whole lisp
//! value model for free; this module is only the thin glue.
//!
//! ## Value bridging
//!
//! The JIT carries every value as a [`vm_core::value::Value`]. A `LispyValue` is a
//! single `u64`, so it rides inside a `Value::Int(i64)` as the bit pattern — the
//! JIT moves it opaquely (it never does arithmetic on a lisp word; the lowered IIR
//! boxes integer atoms as `TAG_INT` and routes all lisp ops through `call_builtin`).
//! [`to_lv`] / [`from_lv`] convert at the builtin boundary.
//!
//! ## Coercions (`unbox_int` / `truthy`)
//!
//! These program-boundary coercions live in `dynval_runtime.c` (C) but not as named
//! functions in the Rust crate — they're trivial and derived here from the crate's
//! existing public primitives (`LispyValue::as_int` / `is_truthy`), NOT duplicated.
//!
//! ## Safety
//!
//! [`to_lv`] uses [`LispyValue::from_raw_bits`] (`unsafe`): the contract is that the
//! bits are a valid `LispyValue`. They always are here — every lisp word the JIT
//! holds either came from [`from_lv`] (a real `LispyValue`) or from a lowered
//! constant the McCarthy frontend emitted (`TAG_INT`/`TAG_NIL`/`TAG_SYMBOL`
//! immediates). The frontend never fabricates a heap pointer (it performs no
//! arithmetic on lisp values), so `car`/`cdr` only ever see a real cons cell or a
//! non-heap value — for which `lispy_runtime::builtins::{car,cdr}` return `Err`,
//! which we map to `nil` rather than panicking. The input is McCarthy *source*, run
//! through the trusted frontend, so untrusted bytes cannot reach `from_raw_bits`
//! with a forged heap tag. The JIT's default step-cap (fuel) bounds execution — and
//! hence the number of `cons` allocations — so an adversarial loop cannot exhaust
//! the heap.

use crate::{compile_source_to_iir, Language, LangAotError};
use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use lispy_runtime::builtins;
use lispy_runtime::value::LispyValue;
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Reinterpret a JIT `Value` as a tagged `LispyValue`.
///
/// SAFETY: see the module-level "Safety" note — the bits are always a valid
/// `LispyValue` produced by [`from_lv`] or the McCarthy frontend's lowered
/// constants; the frontend never forges a heap pointer.
fn to_lv(v: &Value) -> LispyValue {
    unsafe { LispyValue::from_raw_bits(v.as_i64().unwrap_or(0) as u64) }
}

/// Carry a tagged `LispyValue` back into the JIT as an opaque `Value::Int`.
fn from_lv(lv: LispyValue) -> Value {
    Value::Int(lv.bits() as i64)
}

/// `nil` as a JIT value — the graceful result when a lisp builtin traps (e.g.
/// `car` of a non-pair) so a malformed program yields a value rather than a panic.
fn nil() -> Value {
    from_lv(LispyValue::NIL)
}

// ── The lisp builtins, as JIT callbacks (each `fn` is a `Copy` pointer so it can
//    be registered on both the VM interpreter and the compiled JIT path). ──

fn b_cons(a: &[Value]) -> Value {
    match (a.first(), a.get(1)) {
        (Some(x), Some(y)) => builtins::cons(&[to_lv(x), to_lv(y)]).map(from_lv).unwrap_or_else(|_| nil()),
        _ => nil(),
    }
}
fn b_car(a: &[Value]) -> Value {
    a.first().map(|x| builtins::car(&[to_lv(x)]).map(from_lv).unwrap_or_else(|_| nil())).unwrap_or_else(nil)
}
fn b_cdr(a: &[Value]) -> Value {
    a.first().map(|x| builtins::cdr(&[to_lv(x)]).map(from_lv).unwrap_or_else(|_| nil())).unwrap_or_else(nil)
}
fn b_pair_p(a: &[Value]) -> Value {
    a.first().map(|x| builtins::pair_p(&[to_lv(x)]).map(from_lv).unwrap_or_else(|_| nil())).unwrap_or_else(nil)
}
fn b_not(a: &[Value]) -> Value {
    a.first().map(|x| builtins::not(&[to_lv(x)]).map(from_lv).unwrap_or_else(|_| nil())).unwrap_or_else(nil)
}
fn b_equal(a: &[Value]) -> Value {
    match (a.first(), a.get(1)) {
        (Some(x), Some(y)) => builtins::equal_p(&[to_lv(x), to_lv(y)]).map(from_lv).unwrap_or_else(|_| nil()),
        _ => nil(),
    }
}
/// `lispy_truthy` — a tagged value → a raw machine `0`/`1` (false iff `#f`/nil),
/// for the backend's `jmp_if_false` in a `COND`. Derived from `LispyValue::is_truthy`.
fn b_truthy(a: &[Value]) -> Value {
    Value::Int(a.first().map(|x| to_lv(x).is_truthy() as i64).unwrap_or(0))
}
/// `lispy_unbox_int` — a tagged integer → its raw machine value, the program-exit
/// coercion for an integer result. Derived from `LispyValue::as_int` (`>> 3`).
fn b_unbox_int(a: &[Value]) -> Value {
    Value::Int(a.first().and_then(|x| to_lv(x).as_int()).unwrap_or(0))
}
/// `lispy_to_exit_code` — the program-exit coercion for a **polymorphic** result
/// (a `LAMBDA` whose return type is `any`): dispatch on the runtime tag, exactly as
/// `__dyn_to_exit_code` does in `dynval_runtime.c`. Integer → its raw value;
/// `#t`/`#f`/nil → `1`/`0`/`0`; a symbol or pair → its tagged word verbatim. Built
/// from `LispyValue`'s existing predicates — not duplicated.
fn b_to_exit_code(a: &[Value]) -> Value {
    Value::Int(
        a.first()
            .map(|x| {
                let lv = to_lv(x);
                if let Some(n) = lv.as_int() {
                    n
                } else if lv.is_true() {
                    1
                } else if lv.is_false() || lv.is_nil() {
                    0
                } else {
                    lv.bits() as i64 // symbol / pair: the tagged word, verbatim
                }
            })
            .unwrap_or(0),
    )
}

/// Register every McCarthy `lispy_*` builtin on a VM + JIT pair, backed by the
/// shared `lispy_runtime` crate. Each is registered on both the VM (the
/// interpreter fallback for cold/untyped frames) and the `GenericCirJit` (the
/// compiled path) so the two agree.
fn register_lispy_builtins(vm: &mut VMCore, backend: &GenericCirJit) {
    for (name, f) in [
        ("lispy_cons", b_cons as fn(&[Value]) -> Value),
        ("lispy_car", b_car),
        ("lispy_cdr", b_cdr),
        ("lispy_pair_p", b_pair_p),
        ("lispy_not", b_not),
        ("lispy_equal", b_equal),
        ("lispy_truthy", b_truthy),
        ("lispy_unbox_int", b_unbox_int),
        ("lispy_to_exit_code", b_to_exit_code),
    ] {
        vm.builtins_mut().register(name, move |args: &[Value]| Ok(f(args)));
        backend.register_builtin(name, move |args: &[Value]| f(args));
    }
}

/// Compile McCarthy `source` and **run it on the universal JIT**, returning the
/// program's integer result (the entry point's return value).
///
/// The full tagged-word pipeline (`lower_heap_builtins_runtime` → `intern_symbols`
/// → `lower_lisp_repr`) is applied — the same lowering the native AOT / LLVM
/// backends use — then the `lispy_*` builtins are wired to `lispy_runtime` and the
/// module is driven through [`JITCore::execute_with_jit`].
///
/// Returns `Ok(None)` if the entry point returns no value (a `void` program).
/// Covers all of McCarthy F1–F7 (scalar / cons / ATOM / EQ / COND / symbols /
/// `LAMBDA` / `LABEL` / recursion) — the JIT is the eighth and final backend.
pub fn run_mccarthy_on_jit(source: &str) -> Result<Option<i64>, LangAotError> {
    let mut module = compile_source_to_iir(Language::McCarthyLisp, source, "jit")?;
    iir_builtin_lowering::lower_heap_builtins_runtime(&mut module);
    iir_builtin_lowering::intern_symbols(&mut module);
    iir_builtin_lowering::lower_lisp_repr(&mut module);

    let mut vm = VMCore::new();
    let backend = GenericCirJit::new(); // default step-cap (fuel) bounds execution
    register_lispy_builtins(&mut vm, &backend);
    let error_handle = backend.error_handle();

    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result = jit
        .execute_with_jit(&mut vm, &mut module, "main", &[])
        .map_err(|e| LangAotError::JitBackendError(format!("{e:?}")))?;

    if let Some(msg) = error_handle.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Err(LangAotError::JitBackendError(msg));
    }
    Ok(result.and_then(|v| v.as_i64()))
}
