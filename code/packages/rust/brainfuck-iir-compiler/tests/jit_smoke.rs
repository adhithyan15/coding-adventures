//! JIT smoke test — prove a Brainfuck program executes correctly when
//! dispatched through the LANG VM's JIT chain (`vm-core` + `jit-core`)
//! rather than through `VMCore::execute` directly.
//!
//! ## What this actually proves
//!
//! These tests run BF programs through [`BrainfuckVM`] with `jit=true`,
//! which routes the run through
//! [`jit_core::core::JITCore::execute_with_jit`].  The wrapper supplies an
//! `InterpOnlyBackend` (see `src/vm.rs`) so no native code is generated
//! yet — Brainfuck's `load_mem` / `store_mem` opcodes have no native
//! lowering in any current backend (NullBackend, EchoBackend, future
//! WASM/x86_64).  The point of the smoke test is to lock in the
//! *wiring*: when a future backend learns BF's memory model, swapping
//! the backend in the wrapper is the only change needed for tier
//! promotion to kick in.
//!
//! Each test runs the same source twice — once with `jit=false` (pure
//! interpreter) and once with `jit=true` (JIT chain) — and asserts the
//! outputs are byte-identical.  If they ever diverge, the JIT wiring is
//! broken regardless of whether anyone reads the output.
//!
//! ## Pipeline exercised
//!
//! ```text
//! Brainfuck source
//!     │
//!     ▼ brainfuck-iir-compiler::compile_source
//! IIRModule (FullyTyped)
//!     │
//!     ▼ JITCore::execute_with_jit
//!   • Phase 1: try to compile FullyTyped fns via InterpOnlyBackend → None
//!     (no native handlers installed)
//!   • Phase 2: vm-core interprets `main`, firing the registered
//!     putchar / getchar / load_mem / store_mem / label handlers
//!   • Phase 3: promote_hot_functions (no-op — InterpOnlyBackend refuses)
//!     │
//!     ▼ collected stdout
//! Vec<u8>
//! ```

use brainfuck_iir_compiler::BrainfuckVM;

/// Run `src` through both execution paths and assert they agree.
/// Returns the (interpreter, jit) byte vectors so the caller can also
/// assert the exact expected bytes.
fn run_both(src: &str, input: &[u8], tape_size: usize, max_steps: Option<u64>)
    -> (Vec<u8>, Vec<u8>)
{
    let interp = BrainfuckVM::new(false, tape_size, max_steps).unwrap()
        .run(src, input).unwrap();
    let jit    = BrainfuckVM::new(true,  tape_size, max_steps).unwrap()
        .run(src, input).unwrap();
    assert_eq!(
        interp, jit,
        "JIT path must produce identical output to the interpreter\n\
         interpreter: {interp:?}\njit:         {jit:?}\nsource:      {src:?}",
    );
    (interp, jit)
}

// ---------------------------------------------------------------------------
// Trivial smoke: `+++.` → chr(3)
// ---------------------------------------------------------------------------

/// `+++.` increments cell[0] three times and prints it.  The smallest
/// program that exercises `store_mem`, `load_mem`, and `call_builtin
/// "putchar"` together.
#[test]
fn jit_brainfuck_three_increments_print() {
    let (_, jit) = run_both("+++.", b"", 100, None);
    assert_eq!(jit, vec![3u8],
        "expected [3] from `+++.`, got {jit:?}");
}

// ---------------------------------------------------------------------------
// Pointer arithmetic: cell[1]=1, move back, read cell[0]=0 → print 0
// ---------------------------------------------------------------------------

/// `>+<.` moves right, increments cell[1], moves back, prints cell[0].
/// Exercises the data-pointer arithmetic in `load_mem` / `store_mem`.
#[test]
fn jit_brainfuck_pointer_arithmetic() {
    let (_, jit) = run_both(">+<.", b"", 100, None);
    assert_eq!(jit, vec![0u8],
        "expected [0] from `>+<.`, got {jit:?}");
}

// ---------------------------------------------------------------------------
// Loop + input: classic `cat` until EOF
// ---------------------------------------------------------------------------

/// `,[.,]` reads a byte and echoes it until EOF.  Exercises:
/// - `call_builtin "getchar"`  (input)
/// - `call_builtin "putchar"`  (output)
/// - `jmp_if_false` / `jmp` / `label` (the canonical BF loop shape)
/// - The `label`-crossings fuel cap (must not falsely fire on a
///   short input).
#[test]
fn jit_brainfuck_cat_with_input() {
    let (_, jit) = run_both(",[.,]", b"hello", 30_000, Some(10_000));
    assert_eq!(jit, b"hello",
        "expected `hello` echoed back, got {jit:?}");
}

// ---------------------------------------------------------------------------
// Nested loop arithmetic: cell[0]=2, cell[1]=3, multiply into cell[2]=6
// ---------------------------------------------------------------------------

/// Multiplication by repeated addition: `cell[0] * cell[1] → cell[2]`.
/// A classic BF idiom that exercises nested loops, multiple cells, and
/// data movement.  Setup: cell[0]=2, cell[1]=3, then the loop runs
/// `cell[0]` times, adding `cell[1]` to `cell[2]` each iteration (and
/// restoring cell[1] from cell[3]).  Finally prints `cell[2]` (= 6).
///
/// Source breakdown:
/// - `++>+++<`              cell[0]=2, cell[1]=3, return to cell[0]
/// - `[`                   outer loop while cell[0] != 0
/// -   `>[->+>+<<]`        cell[1] → cell[2]+cell[3], cell[1]=0
/// -   `>>[-<<+>>]`        cell[3] → cell[1]
/// -   `<<<-`              cell[0] -= 1
/// - `]`                   end outer loop
/// - `>>.`                  move to cell[2] and print
#[test]
fn jit_brainfuck_multiply_2_times_3() {
    let src = "++>+++<[>[->+>+<<]>>[-<<+>>]<<<-]>>.";
    let (interp, jit) = run_both(src, b"", 30_000, Some(100_000));
    assert_eq!(jit, vec![6u8],
        "expected [6] from 2*3, got jit={jit:?} interp={interp:?}");
}

// ---------------------------------------------------------------------------
// Wrap-around: 0 - 1 == 255 under u8 semantics
// ---------------------------------------------------------------------------

/// `-.` decrements cell[0] (from 0 to 255 with u8 wraparound) and prints.
/// Confirms the JIT path inherits the `VMCore::with_u8_wrap` semantics
/// configured by the wrapper.
#[test]
fn jit_brainfuck_u8_wraparound() {
    let (_, jit) = run_both("-.", b"", 100, None);
    assert_eq!(jit, vec![255u8],
        "expected [255] from `-.` with u8 wraparound, got {jit:?}");
}

// ---------------------------------------------------------------------------
// Multiple prints in sequence
// ---------------------------------------------------------------------------

/// `+.++.+++.` prints 1, 3, 6 — three separate `putchar` calls
/// interleaved with arithmetic.  Confirms the JIT chain doesn't
/// accidentally batch or reorder I/O.
#[test]
fn jit_brainfuck_multiple_outputs() {
    let (_, jit) = run_both("+.++.+++.", b"", 100, None);
    assert_eq!(jit, vec![1u8, 3u8, 6u8],
        "expected [1,3,6] from `+.++.+++.`, got {jit:?}");
}

// ---------------------------------------------------------------------------
// "The JIT is real, not a fallback" tests
// ---------------------------------------------------------------------------
//
// The tests above prove that the JIT *path* runs programs correctly.
// The tests below prove that the BF JIT bytecode compiler actually
// emits non-empty bytecode for representative programs — i.e. the JIT
// is not silently falling back to `vm-core`'s interpreter via the
// `compile()` returning `None` path.

use brainfuck_iir_compiler::BrainfuckVM as _BrainfuckVM;

/// `+++.` — the smallest BF program that exercises every basic op
/// (const_u32, load_mem, const_u8, add_u8, store_mem, call_builtin
/// putchar, ret_void).  Its bytecode should be ~30-40 bytes.
#[test]
fn jit_emits_real_bytecode_for_three_increments() {
    let vm = _BrainfuckVM::new(true, 30_000, None).unwrap();
    let len = vm.jit_bytecode_len("+++.").unwrap();
    if len.is_none() {
        // Print the CIR shape so a future test failure is debuggable.
        use brainfuck_iir_compiler::compile_source;
        use jit_core::specialise::specialise;
        use jit_core::optimizer::CIROptimizer;
        let m = compile_source("+++.", "demo").unwrap();
        let f = m.functions.iter().find(|f| f.name == "main").unwrap();
        let cir = specialise(f, 5);
        let cir = CIROptimizer::new().run(cir);
        eprintln!("CIR for `+++.` (after specialise+optimize):");
        for (i, c) in cir.iter().enumerate() {
            eprintln!("  {i}: op={:?} dest={:?} srcs={:?} ty={}",
                c.op, c.dest, c.srcs, c.ty);
        }
    }
    let len = len.expect("JIT should successfully compile `+++.` to bytecode");
    assert!(len > 0, "JIT bytecode for `+++.` should be non-empty");
    // Sanity floor — three `+` blocks (4 instrs each) + `.` (2 instrs)
    // + const_u32 prologue + ret_void.  Each instr is 3-6 bytes.  We
    // expect at least 20 bytes; we generously assert >= 15.
    assert!(len >= 15,
        "expected at least 15 bytes of bytecode, got {len}");
}

/// A program with a loop — `++[-]` zeroes a non-zero cell.  Exercises
/// `label`, `jmp_if_false`, `jmp` (the canonical BF loop shape).
/// Bytecode size should reflect the loop body, not just be a stub.
#[test]
fn jit_emits_bytecode_for_loop_program() {
    let vm = _BrainfuckVM::new(true, 30_000, Some(1000)).unwrap();
    let len = vm.jit_bytecode_len("++[-]").unwrap()
        .expect("JIT should compile `++[-]` to bytecode");
    assert!(len > 20,
        "loop program should compile to non-trivial bytecode, got {len} bytes");
}

/// Confirm the JIT path also runs the loop program correctly end-to-end
/// (parity with the interpreter).
#[test]
fn jit_runs_loop_program_correctly() {
    // `++[-].` increments cell[0] to 2, loops decrementing until 0,
    // then prints (should be 0).
    let (_, jit) = run_both("++[-].", b"", 30_000, Some(1000));
    assert_eq!(jit, vec![0u8],
        "loop should drain cell to 0, got {jit:?}");
}
