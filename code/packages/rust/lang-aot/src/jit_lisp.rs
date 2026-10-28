//! # Lisp-family languages on the universal JIT
//!
//! The eighth and final backend. `jit-core`'s `GenericCirJit` is a *universal
//! bytecode JIT*: any typed-IIR language plugs in by registering its builtins as
//! Rust callbacks. Unlike the AOT/LLVM tagged-word backends — which lower a
//! `call_builtin "dyn_*"` to a native `call __dyn_*` into the C runtime
//! (`dynval_runtime.c`) — the JIT dispatches the *same* `dyn_*` names to Rust
//! closures backed by the **shared [`dynval_runtime`] crate** (the C runtime's Rust
//! twin: an identical `u64` tagged-word model). So the JIT inherits the whole lisp
//! value model for free; this module is only the thin glue.
//!
//! ## Value bridging
//!
//! The JIT carries every value as a [`vm_core::value::Value`]. A `LispyValue` is a
//! single `u64`, so it rides inside a `Value::Int(i64)` as the bit pattern — the
//! JIT moves it opaquely (it never does arithmetic on a lisp word; the lowered IIR
//! boxes integer atoms as `TAG_INT` and routes all lisp ops through `call_builtin`).
//! `to_lv` / `from_lv` convert at the builtin boundary.
//!
//! ## Only the representation is registered
//!
//! The callbacks are the LANG83 PR7f dynamic-value primitives (`dyn_pair_new`,
//! `dyn_pair_first`, `dyn_pair_rest`, `dyn_is_pair`, `dyn_is_nil`, `dyn_tag`,
//! `dyn_unbox_int`), each derived from the crate's public primitives
//! (`LispyValue::as_int`, `bits`, `builtins::cons` …), not duplicated. Lisp's
//! truthiness, `not`, `equal?` and exit coercion are not callbacks: since LANG83
//! PR7f-2 `lower_dyn_repr` writes them as IIR in the module (truthiness and `not`
//! inline, so a function that uses them still JIT-compiles; `__lisp_equal` and
//! `__lisp_exit_code` as module functions, which run on the interpreter tier
//! because they branch and call).
//!
//! ## Safety
//!
//! `to_lv` uses `LispyValue::from_raw_bits` (`unsafe`): the contract is that the
//! bits are a valid `LispyValue`. They always are here — every lisp word the JIT
//! holds either came from `from_lv` (a real `LispyValue`) or from a lowered
//! constant the McCarthy frontend emitted (`TAG_INT`/`TAG_NIL`/`TAG_SYMBOL`
//! immediates). The frontend never fabricates a heap pointer (it performs no
//! arithmetic on lisp values), so `car`/`cdr` only ever see a real cons cell or a
//! non-heap value — for which `dynval_runtime::builtins::{car,cdr}` return `Err`.
//! That `Err` fails the run, as on every other column (#18417): the interpreter
//! tier returns a `VMError::TypeError`, and the compiled tier records the same
//! message in the JIT's error slot, which stops the run (see [`pair_reader`]).
//! It used to be mapped to `nil`, so `(CAR 5)` quietly returned nil here while
//! the VM, LLVM and native columns failed. The input is McCarthy *source*, run
//! through the trusted frontend, so untrusted bytes cannot reach `from_raw_bits`
//! with a forged heap tag. The JIT's default step-cap (fuel) bounds execution — and
//! hence the number of `cons` allocations — so an adversarial loop cannot exhaust
//! the heap.

use crate::{compile_source_to_iir, Language, LangAotError};
use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use dynval_runtime::builtins;
use dynval_runtime::value::LispyValue;
use vm_core::core::VMCore;
use vm_core::errors::VMError;
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

/// `nil` as a JIT value — the placeholder a callback returns when it cannot
/// produce a value: a malformed call (a missing argument), or a `car` of a
/// non-pair, which also records an error so the run stops (#18417).
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
/// `dyn_pair_first` / `dyn_pair_rest` of `a[0]`, or the refusal's text when
/// it is not a pair (or missing).
///
/// | argument            | result                                                        |
/// |---------------------|---------------------------------------------------------------|
/// | a cons cell         | `Ok(its car / cdr)`                                           |
/// | anything else       | `Err("dyn_pair_first: argument with tag 0 (int) is not a pair")` |
///
/// The text is `dynval-runtime`'s (`abi::not_a_pair_message`), the words the
/// C runtime prints on LLVM and native: the tag and its kind, never the word.
fn pair_read(op: &'static str, a: &[Value]) -> Result<Value, String> {
    let read = if op == "dyn_pair_first" { builtins::car } else { builtins::cdr };
    let word = a.first().map(to_lv).ok_or_else(|| format!("{op}: missing argument"))?;
    read(&[word]).map(from_lv).map_err(|_| dynval_runtime::abi::not_a_pair_message(op, word.bits()))
}
fn b_pair_p(a: &[Value]) -> Value {
    a.first().map(|x| builtins::pair_p(&[to_lv(x)]).map(from_lv).unwrap_or_else(|_| nil())).unwrap_or_else(nil)
}
/// `dyn_unbox_int` — a tagged integer → its raw machine value, the program-exit
/// coercion for an integer result. Derived from `LispyValue::as_int` (`>> 3`).
fn b_unbox_int(a: &[Value]) -> Value {
    Value::Int(a.first().and_then(|x| to_lv(x).as_int()).unwrap_or(0))
}
/// `dyn_tag` — the tag number (`bits & 7`), the one query the Lisp lowering's
/// compiler-written truthiness, `not`, `equal?` and exit coercion need (LANG83
/// PR7f-2; they were the `dyn_truthy` / `dyn_not` / `dyn_equal` /
/// `dyn_to_exit_code` callbacks here before).
fn b_tag(a: &[Value]) -> Value {
    Value::Int(a.first().map(|x| (to_lv(x).bits() & dynval_runtime::TAG_BITS) as i64).unwrap_or(0))
}
/// `dyn_is_nil` — the nil test as a tagged boolean.
fn b_is_nil(a: &[Value]) -> Value {
    a.first().map(|x| builtins::null_p(&[to_lv(x)]).map(from_lv).unwrap_or_else(|_| nil())).unwrap_or_else(nil)
}

/// Register every McCarthy `dyn_*` builtin on a VM + JIT pair, backed by the
/// shared `dynval_runtime` crate. Each is registered on both the VM (the
/// interpreter fallback for cold/untyped frames) and the `GenericCirJit` (the
/// compiled path) so the two agree.
///
/// `dyn_pair_first` / `dyn_pair_rest` are the two that can fail, and they fail
/// the run on both tiers (#18417):
///
/// | tier                          | how a non-pair fails                                          |
/// |-------------------------------|---------------------------------------------------------------|
/// | VM (interpreter fallback)     | the builtin returns `Err(VMError::TypeError { … })`            |
/// | `GenericCirJit` (compiled)    | the callback records the message in the JIT's error slot (it cannot return an `Err`); the JIT stops after that call and `JITCore` reports it |
fn register_lispy_builtins(vm: &mut VMCore, backend: &GenericCirJit) {
    for op in ["dyn_pair_first", "dyn_pair_rest"] {
        vm.builtins_mut().register(op, move |args: &[Value]| {
            pair_read(op, args).map_err(|context| VMError::TypeError {
                expected: "a pair".into(),
                actual: args.first().map_or_else(|| "no argument".into(), |x| {
                    format!("a value with tag {}", to_lv(x).bits() & dynval_runtime::TAG_BITS)
                }),
                context,
            })
        });
        let error_slot = backend.error_handle();
        backend.register_builtin(op, move |args: &[Value]| {
            pair_read(op, args).unwrap_or_else(|message| {
                // First error wins, as in the JIT's own `set_error`.
                error_slot
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_or_insert_with(|| format!("TypeError: {message}"));
                nil() // never read: the JIT stops right after this call
            })
        });
    }
    for (name, f) in [
        ("dyn_pair_new", b_cons as fn(&[Value]) -> Value),
        ("dyn_is_pair", b_pair_p),
        ("dyn_is_nil", b_is_nil),
        ("dyn_tag", b_tag),
        ("dyn_unbox_int", b_unbox_int),
    ] {
        vm.builtins_mut().register(name, move |args: &[Value]| Ok(f(args)));
        backend.register_builtin(name, move |args: &[Value]| f(args));
    }
}

/// Compile McCarthy `source` and **run it on the universal JIT**, returning the
/// program's integer result (the entry point's return value).
///
/// The full tagged-word pipeline (`lower_heap_builtins_runtime` → `intern_symbols`
/// → `lower_dyn_repr`) is applied — the same lowering the native AOT / LLVM
/// backends use — then the `dyn_*` builtins are wired to `dynval_runtime` and the
/// module is driven through [`JITCore::execute_with_jit`].
///
/// Returns `Ok(None)` if the entry point returns no value (a `void` program).
/// Covers all of McCarthy F1–F7 (scalar / cons / ATOM / EQ / COND / symbols /
/// `LAMBDA` / `LABEL` / recursion) — the JIT is the eighth and final backend.
pub fn run_mccarthy_on_jit(source: &str) -> Result<Option<i64>, LangAotError> {
    let mut module = compile_source_to_iir(Language::McCarthyLisp, source, "jit")?;
    iir_builtin_lowering::lower_heap_builtins_runtime(&mut module).map_err(|e| LangAotError::JitBackendError(e.to_string()))?;
    iir_builtin_lowering::intern_symbols(&mut module);
    iir_builtin_lowering::lower_dyn_repr(&mut module).map_err(|e| LangAotError::JitBackendError(e.to_string()))?;

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

/// Compile Macsyma `source` and run its scalar integer subset on the universal
/// JIT, returning the entry point's integer result.
///
/// Macsyma v0 emits dynamic arithmetic as `call_builtin "+"/"-"/"*"/"/"`
/// even when both operands are concrete integers. The shared
/// [`iir_builtin_lowering::lower_dynamic_arith`] pass rewrites those calls to
/// typed arithmetic plus `box`/`unbox`; those two representation operations are
/// identities on the generic VM/JIT because [`Value::Int`] already carries the
/// dynamic integer value. Unlike McCarthy Lisp, this scalar slice therefore
/// needs no tagged-word runtime callbacks.
///
/// Returns `Ok(None)` if the entry point returns no value.
pub fn run_macsyma_on_jit(source: &str) -> Result<Option<i64>, LangAotError> {
    let mut module = compile_source_to_iir(Language::Macsyma, source, "jit")?;
    iir_builtin_lowering::lower_dynamic_arith(&mut module);

    let mut vm = VMCore::new();
    let backend = GenericCirJit::new();
    let error_handle = backend.error_handle();

    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result = jit
        .execute_with_jit(&mut vm, &mut module, "main", &[])
        .map_err(|e| LangAotError::JitBackendError(format!("{e:?}")))?;

    if let Some(msg) = error_handle
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        return Err(LangAotError::JitBackendError(msg));
    }
    Ok(result.and_then(|v| v.as_i64()))
}
