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
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Run a program with a stdin record stream (`input`), capturing stdout. The
/// `input_more` builtin peeks whether a record remains (the EOF-aware read from
/// PL09 D4); `input_i64` pops the next field.
fn run_pipe(source: &str, input: &[i64]) -> String {
    let mut module = compile_source(source, "fm_pipe").expect("FLOW-MATIC should compile");
    let mut vm = VMCore::new();
    let queue: Arc<Mutex<VecDeque<i64>>> = Arc::new(Mutex::new(input.iter().copied().collect()));
    let chars: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let queue = Arc::clone(&queue);
        vm.builtins_mut().register("input_more", move |_args| {
            Ok(Value::Int(if queue.lock().unwrap().is_empty() { 0 } else { 1 }))
        });
    }
    {
        let queue = Arc::clone(&queue);
        vm.builtins_mut().register("input_i64", move |_args| {
            Ok(Value::Int(queue.lock().unwrap().pop_front().unwrap_or(0)))
        });
    }
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
    String::from_utf8(bytes).expect("output must be valid UTF-8")
}

#[test]
fn read_process_write_loop_echoes_records_until_end_of_data() {
    // The canonical record-processing shape: READ a record, IF END OF DATA stop,
    // else MOVE a field to the output record, WRITE it, and JUMP back. Feeding
    // two records [5, 3] must echo "5\n3\n" then halt at end-of-data.
    let src = "\
(0) INPUT SRC FILE-A ; OUTPUT OUT FILE-C .
(1) READ-ITEM FILE-A ; IF END OF DATA GO TO OPERATION 4 .
(2) MOVE N (A) TO N (C) ; WRITE-ITEM FILE-C .
(3) JUMP TO OPERATION 1 .
(4) STOP .";
    assert_eq!(run_pipe(src, &[5, 3]), "5\n3\n");
}

#[test]
fn read_loop_over_empty_input_writes_nothing() {
    // No records → the first READ hits end-of-data and the loop stops before any
    // WRITE.
    let src = "\
(0) INPUT SRC FILE-A ; OUTPUT OUT FILE-C .
(1) READ-ITEM FILE-A ; IF END OF DATA GO TO OPERATION 4 .
(2) MOVE N (A) TO N (C) ; WRITE-ITEM FILE-C .
(3) JUMP TO OPERATION 1 .
(4) STOP .";
    assert_eq!(run_pipe(src, &[]), "");
}

#[test]
fn read_multi_field_record_reads_fields_in_order() {
    // A two-field input record: READ pulls Q (A) then UP (A) (sorted field
    // order), and WRITE emits both, space-separated.
    // The MOVEs reference Q (A) and UP (A) so they are modelled as fields of
    // file A (fields are discovered from `field` nodes; READ-ITEM/WRITE-ITEM name
    // only the handle).
    let src = "\
(0) INPUT SRC FILE-A ; OUTPUT OUT FILE-A .
(1) READ-ITEM FILE-A ; IF END OF DATA GO TO OPERATION 4 .
(2) MOVE Q (A) TO Q (A) ; MOVE UP (A) TO UP (A) ; WRITE-ITEM FILE-A .
(3) JUMP TO OPERATION 1 .
(4) STOP .";
    // Fields Q(A), UP(A) sort as [Q, UP]; input [3, 100] → "3 100\n".
    assert_eq!(run_pipe(src, &[3, 100]), "3 100\n");
}

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
