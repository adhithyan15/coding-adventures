//! JIT smoke test: prove a Dartmouth BASIC program can be executed
//! through the LANG VM's JIT chain (`vm-core` + `jit-core`) just like
//! it can through the AOT chain (`lang-aot`).
//!
//! The whole point of the IIR-as-lingua-franca design is that *any*
//! frontend that produces an `IIRModule` plugs into both pipelines.
//! `dartmouth-basic-iir-compiler` lives on the same lingua franca, so
//! we should get JIT execution for free.
//!
//! ## Pipeline exercised
//!
//! ```text
//! BASIC source
//!     │
//!     ▼ dartmouth-basic-iir-compiler
//! IIRModule
//!     │
//!     ▼ JITCore::execute_with_jit
//!   (vm-core interprets; jit-core compiles hot fns into native;
//!    builtins like `print_i64` resolve through a custom registry)
//! captured stdout
//! ```
//!
//! ## What we capture
//!
//! Since the JIT path runs in-process, we register a custom `print_i64`
//! handler on the VM's `BuiltinRegistry` that pushes each printed
//! integer to a shared `Vec<i64>`.  After execution we assert the
//! captured numbers match what the BASIC program wrote.

use std::sync::{Arc, Mutex};

use dartmouth_basic_iir_compiler::compile_source;
use jit_core::backend::NullBackend;
use jit_core::core::JITCore;
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Run a BASIC source through the JIT chain and return everything that
/// `PRINT` wrote.  Each `PRINT n` becomes one entry in the returned vec.
fn jit_execute_and_capture_prints(source: &str) -> String {
    let mut module = compile_source(source, "jit_demo")
        .expect("BASIC source must compile");

    // VMCore wires the IIR dispatch loop; JITCore tier-promotes hot
    // functions (none in our V1 BASIC since the iir-compiler doesn't
    // mark them FullyTyped — that's a separate optimisation).  The
    // interpreted path still runs the whole program.
    let mut vm = VMCore::new();

    // BA2: BASIC `PRINT` renders output one character at a time through the
    // universal `putchar` builtin (digits via the synthetic recursive
    // `__basic_print_int` helper, plus separator spaces and the line-ending
    // newline) — *not* the old line-buffered `print_i64`.  Capture the raw
    // byte stream and decode it to a string for comparison.
    let printed: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let printed = Arc::clone(&printed);
        vm.builtins_mut().register("putchar", move |args| {
            let b = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            printed.lock().unwrap().push(b as u8);
            Ok(Value::Null)
        });
    }

    let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));
    jit.execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution must succeed");

    // Bind the cloned bytes to a local first so the `MutexGuard` temporary is
    // dropped before `printed` goes out of scope (a tail
    // `printed.lock().unwrap().clone()` expression holds the guard until the
    // block ends, which older rustc rejects — E0597).
    let bytes = printed.lock().unwrap().clone();
    String::from_utf8(bytes).expect("BASIC PRINT output must be valid UTF-8")
}

/// Smallest possible PRINT test: `10 PRINT 42 / 20 END` must push the
/// single integer 42 through the `print_i64` builtin.
#[test]
fn jit_basic_print_42() {
    let got = jit_execute_and_capture_prints("10 PRINT 42\n20 END\n");
    assert_eq!(got, "42\n",
        "expected printed `42` + newline, got {got:?}");
}

/// LET + arithmetic + PRINT: confirms that mov / add / `call_builtin`
/// `print_i64` all work in the JIT chain.
///
/// `10 LET A = 30 / 20 LET B = 12 / 30 PRINT A + B / 40 END`
/// should print 42.
#[test]
fn jit_basic_let_arithmetic_print() {
    let src = "10 LET A = 30\n\
               20 LET B = 12\n\
               30 PRINT A + B\n\
               40 END\n";
    let got = jit_execute_and_capture_prints(src);
    assert_eq!(got, "42\n",
        "expected `42` + newline from 30 + 12, got {got:?}");
}

/// BA6 — `READ` / `DATA`: a single value flows from the DATA pool into a
/// variable.  `10 DATA 42 / 20 READ X / 30 PRINT X / 40 END` ⇒ [42].
#[test]
fn jit_basic_read_data_single() {
    let src = "10 DATA 42\n\
               20 READ X\n\
               30 PRINT X\n\
               40 END\n";
    let got = jit_execute_and_capture_prints(src);
    assert_eq!(got, "42\n", "READ X from DATA 42 should print 42, got {got:?}");
}

/// BA6 — multi-`READ` advances the pointer, and `RESTORE` rewinds it.  The
/// pool is `10, 20, 30`; `READ A, B` takes 10 and 20; after `RESTORE`, `READ C`
/// takes 10 again.  ⇒ prints 10, 20, 10 — proving sequential consumption *and*
/// the rewind.
#[test]
fn jit_basic_read_restore_rewinds() {
    let src = "10 DATA 10, 20, 30\n\
               20 READ A, B\n\
               30 PRINT A\n\
               40 PRINT B\n\
               50 RESTORE\n\
               60 READ C\n\
               70 PRINT C\n\
               80 END\n";
    let got = jit_execute_and_capture_prints(src);
    assert_eq!(got, "10\n20\n10\n",
        "READ A,B then RESTORE then READ C should print 10,20,10, got {got:?}");
}

/// FOR / NEXT loop: `10 FOR I = 1 TO 3 / 20 PRINT I / 30 NEXT I / 40 END`
/// should print 1, 2, 3 in order.  Exercises label / jmp_if_false /
/// add / jmp under the JIT interpreter.
#[test]
fn jit_basic_for_loop_prints_1_2_3() {
    let src = "10 FOR I = 1 TO 3\n\
               20 PRINT I\n\
               30 NEXT I\n\
               40 END\n";
    let got = jit_execute_and_capture_prints(src);
    assert_eq!(got, "1\n2\n3\n",
        "expected 1,2,3 (each on its own line) from FOR I = 1 TO 3, got {got:?}");
}

/// IF / GOTO branch: `10 LET A = 7 / 20 IF A > 5 THEN 100 / 30 PRINT 0 /
/// 40 END / 100 PRINT A / 110 END` should print 7 (the THEN branch
/// hits line 100, skips the `PRINT 0`).
#[test]
fn jit_basic_if_then_goto() {
    let src = "10 LET A = 7\n\
               20 IF A > 5 THEN 100\n\
               30 PRINT 0\n\
               40 END\n\
               100 PRINT A\n\
               110 END\n";
    let got = jit_execute_and_capture_prints(src);
    assert_eq!(got, "7\n",
        "expected `7` + newline from IF A > 5 THEN 100, got {got:?}");
}
