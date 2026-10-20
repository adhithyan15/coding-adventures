//! Fail-closed regression tests for JIT-compiled code that fails at run time.
//!
//! # The bug these tests pin down
//!
//! `Backend::run` returns a plain `Value`.  When `GenericCirJit`'s compiled
//! code hit a LANG83 `trap`, an `i64_div_floor` overflow, or a `div_i64` by
//! zero, it wrote a message into its side error slot and returned
//! `Value::Null`.  Nothing on the call path read that slot:
//!
//! ```text
//!  main (interpreted)            f (JIT-compiled)
//!  ------------------            ----------------
//!  call f MIN -1      ───────▶   i64_div_floor → Trap.Overflow
//!                     ◀─ Null ─  (slot = "trap: Trap.Overflow")
//!  call_builtin mark   ← ran!   (the caller kept going)
//!  ret 0               ← execute_with_jit returned Ok(Some(0))
//! ```
//!
//! The fix makes JIT handlers fallible (`Result<Value, VMError>`) and has
//! `JITCore` drain the backend's error slot (`Backend::take_error`) after
//! every run, so the failure propagates as `Err` and stops every caller —
//! the same outcome as if the callee had been interpreted.
//!
//! "VERIFY BY RUNNING": each test counts calls to a `mark` builtin placed
//! *after* the failing call; the count must be zero.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use interpreter_ir::function::FunctionTypeStatus;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use jit_core::core::JITCore;
use jit_core::generic_jit::GenericCirJit;
use vm_core::core::VMCore;
use vm_core::errors::VMError;
use vm_core::value::Value;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// A VM with a `mark` builtin that counts how often it runs.
fn vm_with_marks() -> (VMCore, Arc<AtomicUsize>) {
    let marks = Arc::new(AtomicUsize::new(0));
    let mut vm = VMCore::new();
    let seen = Arc::clone(&marks);
    vm.builtins_mut().register("mark", move |_args: &[Value]| {
        seen.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null)
    });
    (vm, marks)
}

/// `f(a, b) = i64_div_floor(a, b)` — fully typed, so `execute_with_jit`
/// compiles it eagerly.
fn floor_div_fn() -> IIRFunction {
    IIRFunction::new(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("i64_div_floor", Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
    )
}

/// `chk2(i) = { trap "Trap.Bounds"; ret i }` — fully typed, JIT-compiled.
fn chk2_fn() -> IIRFunction {
    IIRFunction::new(
        "chk2",
        vec![("i".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("trap", None, vec![Operand::Str("Trap.Bounds".into())], "i64"),
            IIRInstr::new("ret", None, vec![v("i")], "i64"),
        ],
    )
}

fn assert_trap(result: Result<Option<Value>, VMError>, want: &str) {
    match result {
        Err(VMError::Trap { kind }) => assert_eq!(kind, want),
        other => panic!("expected Err(Trap {{ kind: {want:?} }}), got {other:?}"),
    }
}

/// The security reviewer's repro, verbatim: an untyped (interpreted) `main`
/// calls the JIT-compiled `f(MIN, -1)`, then `mark`, then the JIT-compiled
/// trapping `chk2`, then `mark`.  It used to return `Ok(Some(Int(0)))` with
/// two marks; it must now stop at the first call with `Trap.Overflow`.
#[test]
fn reviewer_repro_traps_at_first_jit_call() {
    let main_fn = IIRFunction::new(
        "main",
        vec![],
        "any",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(i64::MIN)], "any"),
            IIRInstr::new("const", Some("y".into()), vec![Operand::Int(-1)], "any"),
            IIRInstr::new("call", Some("r".into()), vec![v("f"), v("x"), v("y")], "any"),
            IIRInstr::new("call_builtin", None, vec![v("mark")], "any"),
            IIRInstr::new("call", None, vec![v("chk2"), v("x")], "any"),
            IIRInstr::new("call_builtin", None, vec![v("mark")], "any"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "any"),
            IIRInstr::new("ret", None, vec![v("z")], "any"),
        ],
    );
    let mut module = IIRModule::new("probe", "test");
    module.add_or_replace(floor_div_fn());
    module.add_or_replace(chk2_fn());
    module.add_or_replace(main_fn);
    module.entry_point = Some("main".into());
    assert_ne!(
        module.get_function("main").unwrap().type_status,
        FunctionTypeStatus::FullyTyped,
        "main must stay on the interpreter tier for this repro"
    );

    let (mut vm, marks) = vm_with_marks();
    let backend = GenericCirJit::new();
    let slot = backend.error_handle();
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]);

    assert!(vm.is_jit_compiled("f") && vm.is_jit_compiled("chk2"), "both callees must be JIT-compiled");
    assert_trap(result, "Trap.Overflow");
    assert_eq!(marks.load(Ordering::SeqCst), 0, "the caller ran past a trapping JIT call");
    assert!(slot.lock().unwrap().is_none(), "JITCore must drain the error slot");
}

/// A trap deep in a call chain — interpreted `main` → interpreted `mid` →
/// JIT-compiled `chk2` — stops *every* frame: neither `mid` nor `main` runs
/// its post-call `mark`.  (`GenericCirJit` cannot compile `call`, so a
/// JIT-compiled function is always a leaf; the chain's intermediate frames
/// are interpreted.)
#[test]
fn trap_in_jit_leaf_stops_whole_call_chain() {
    let mid = IIRFunction::new(
        "mid",
        vec![("i".into(), "any".into())],
        "any",
        vec![
            IIRInstr::new("call", Some("r".into()), vec![v("chk2"), v("i")], "any"),
            IIRInstr::new("call_builtin", None, vec![v("mark")], "any"),
            IIRInstr::new("ret", None, vec![v("r")], "any"),
        ],
    );
    let main_fn = IIRFunction::new(
        "main",
        vec![],
        "any",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(7)], "any"),
            IIRInstr::new("call", Some("r".into()), vec![v("mid"), v("x")], "any"),
            IIRInstr::new("call_builtin", None, vec![v("mark")], "any"),
            IIRInstr::new("ret", None, vec![v("r")], "any"),
        ],
    );
    let mut module = IIRModule::new("chain", "test");
    module.add_or_replace(chk2_fn());
    module.add_or_replace(mid);
    module.add_or_replace(main_fn);

    let (mut vm, marks) = vm_with_marks();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]);

    assert!(vm.is_jit_compiled("chk2"));
    assert_trap(result, "Trap.Bounds");
    assert_eq!(marks.load(Ordering::SeqCst), 0);
}

/// `div_i64` by zero in a JIT-compiled callee is `VMError::DivisionByZero`
/// for its interpreted caller — the same error the interpreter raises — and
/// the caller's post-call `mark` never runs.
#[test]
fn jit_div_by_zero_from_interpreted_caller_is_division_by_zero() {
    let div = IIRFunction::new(
        "div",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("div", Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
    );
    let main_fn = IIRFunction::new(
        "main",
        vec![],
        "any",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(10)], "any"),
            IIRInstr::new("const", Some("y".into()), vec![Operand::Int(0)], "any"),
            IIRInstr::new("call", Some("r".into()), vec![v("div"), v("x"), v("y")], "any"),
            IIRInstr::new("call_builtin", None, vec![v("mark")], "any"),
            IIRInstr::new("ret", None, vec![v("r")], "any"),
        ],
    );
    let mut module = IIRModule::new("div0", "test");
    module.add_or_replace(div);
    module.add_or_replace(main_fn);

    let (mut vm, marks) = vm_with_marks();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]);

    assert!(vm.is_jit_compiled("div"), "div must be JIT-compiled");
    assert!(matches!(result, Err(VMError::DivisionByZero)), "got {result:?}");
    assert_eq!(marks.load(Ordering::SeqCst), 0);
}

/// `JITCore::execute` on a compiled entry point surfaces the trap as `Err`,
/// and — because the slot is drained, not just read — a later successful
/// run of a different compiled function is not mistaken for a failure.
#[test]
fn direct_execute_fails_closed_and_slot_does_not_go_stale() {
    let ok = IIRFunction::new(
        "ok",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i64"),
            IIRInstr::new("ret", None, vec![v("v")], "i64"),
        ],
    );
    let mut module = IIRModule::new("direct", "test");
    module.add_or_replace(floor_div_fn());
    module.add_or_replace(ok);

    let mut vm = VMCore::new();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    assert!(jit.compile(&mut vm, &module, "f").unwrap());
    assert!(jit.compile(&mut vm, &module, "ok").unwrap());

    let trapped = jit.execute(&mut vm, &mut module, "f", &[Value::Int(1), Value::Int(0)]);
    assert_trap(trapped, "Trap.DivZero");
    let second = jit.execute(&mut vm, &mut module, "f", &[Value::Int(i64::MIN), Value::Int(-1)]);
    assert_trap(second, "Trap.Overflow"); // not the stale first message
    let fine = jit.execute(&mut vm, &mut module, "ok", &[]).expect("ok must succeed");
    assert_eq!(fine, Some(Value::Int(5)));
}
