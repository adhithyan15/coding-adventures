//! FLOW-MATIC through the LANG VM JIT chain — proves the emitted IIR actually
//! *executes*, not just validates.
//!
//! This slice has no I/O and no exit-code verb, so a program runs to a `STOP`
//! and `main` returns 0. The test's value is that a program whose compare/branch
//! is *miscompiled* would land on the wrong operation — here, an infinite
//! `JUMP` loop — and hang or trap instead of returning cleanly. Reaching `STOP`
//! (result 0) therefore proves the `COMPARE`/`IF`/`GO TO` control flow ran
//! correctly on the JIT.

use flow_matic_iir_compiler::compile_source;
use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use std::sync::{Arc, Mutex};
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Run a program capturing everything WRITE-ITEM emits through `putchar`.
fn run_output(source: &str) -> String {
    let mut module = compile_source(source, "fm_out").expect("FLOW-MATIC should compile");
    let mut vm = VMCore::new();
    let chars: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let chars = Arc::clone(&chars);
        vm.builtins_mut().register("putchar", move |args| {
            let b = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            chars.lock().unwrap().push(b as u8);
            Ok(Value::Null)
        });
    }
    let backend = GenericCirJit::new();
    let error_handle = backend.error_handle();
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    jit.execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution should succeed");
    if let Some(err) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit reported an error: {err}");
    }
    let bytes = chars.lock().unwrap().clone();
    String::from_utf8(bytes).expect("WRITE-ITEM output must be valid UTF-8")
}

#[test]
fn write_item_prints_a_zero_field_record() {
    // Fields start at 0; WRITE-ITEM FILE-C writes file C's record — here the one
    // field TOTAL (C) — as its digits then a newline.
    let src = "\
(0) OUTPUT REPORT FILE-C .
(1) MOVE TOTAL (C) TO TOTAL (C) ; WRITE-ITEM FILE-C ; STOP .";
    assert_eq!(run_output(src), "0\n");
}

#[test]
fn write_item_multi_field_record_is_space_separated() {
    // Two C-qualified fields → one space-separated record line. Both are 0.
    let src = "\
(0) OUTPUT REPORT FILE-C .
(1) MOVE A (C) TO A (C) ; MOVE B (C) TO B (C) ; WRITE-ITEM FILE-C ; STOP .";
    assert_eq!(run_output(src), "0 0\n");
}

fn run(source: &str) -> i64 {
    let mut module = compile_source(source, "fm_jit").expect("FLOW-MATIC should compile");
    assert_eq!(
        module.get_function("main").unwrap().type_status,
        interpreter_ir::FunctionTypeStatus::FullyTyped
    );
    let mut vm = VMCore::new();
    let backend = GenericCirJit::new();
    let error_handle = backend.error_handle();
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result = jit
        .execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution should succeed")
        .unwrap_or(Value::Null);
    if let Some(err) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit reported an error: {err}");
    }
    result.as_i64().expect("main returns an i64 exit code")
}

#[test]
fn equal_branch_reaches_stop_not_the_loop() {
    // COMPARE X (A) WITH X (A) is always EQUAL, so IF EQUAL jumps to op_3 (STOP,
    // exit 0). A miscompiled comparison would fall to OTHERWISE → op_2, an
    // infinite JUMP loop that would hang the JIT rather than return 0.
    let src = "\
(0) COMPARE X (A) WITH X (A) ;
    IF EQUAL GO TO OPERATION 3 ; OTHERWISE GO TO OPERATION 2 .
(2) JUMP TO OPERATION 2 .
(3) STOP . (END)";
    assert_eq!(run(src), 0);
}

#[test]
fn jump_chain_reaches_stop() {
    // A chain of unconditional jumps must thread through to the STOP.
    let src = "\
(0) JUMP TO OPERATION 2 .
(1) JUMP TO OPERATION 1 .
(2) JUMP TO OPERATION 3 .
(3) STOP .";
    assert_eq!(run(src), 0);
}
