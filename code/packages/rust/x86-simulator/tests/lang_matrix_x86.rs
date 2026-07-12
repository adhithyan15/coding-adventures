//! # S3 — running the LANG-FULL `NativeAot` x86_64 column *locally*
//!
//! The LANG-FULL matrix (`lang-aot/tests/lang_matrix.rs`) proves every language
//! runs on every backend *by running the backend's output*.  For the `NativeAot`
//! cell that means: compile the shared IIR with the direct machine-code backend,
//! build a real executable, and run it.  But `run_native` builds for the **host**
//! architecture — on this Apple-Silicon machine that is always the `aarch64`
//! backend.  The `x86_64-backend`'s output is verified locally only by *byte*
//! tests and is actually **executed** only on an x86 CI runner.
//!
//! This test closes that gap.  It drives the **real** language frontends through
//! the **real** AOT pipeline (`compile_source_to_iir` → `infer_types` →
//! `aot_specialise` → `x86_64-backend`), then *runs the emitted x86_64 machine
//! code* on the `x86-simulator` — so the matrix's x86_64 column is exercised
//! end-to-end **here, on aarch64**, with no Intel hardware and no CI round-trip.
//!
//! Each program below is a verbatim copy of a `lang_matrix.rs` `NativeAot` cell.
//! We assert the **same** exit code the matrix asserts, but obtained by *running
//! the x86_64 bytes* rather than the host's aarch64 bytes — a genuine retro-
//! verification of E5 native arrays and E3 native floats on the x86_64 backend.

use std::collections::HashMap;

use aot_core::infer::infer_types;
use aot_core::specialise::aot_specialise;
use jit_core::backend::FunctionContext;
use lang_aot::{compile_source_to_iir, Language};
use x86_64_backend::{compile_function_with_globals, X86_64Abi};
use x86_simulator::harness::{MachineCodeHarness, Reloc};

/// Assign each distinct module-global name (as it appears in a `global_load` /
/// `global_store`) a stable 8-byte slot index, in first-seen order. This mirrors
/// `twig-aot::collect_global_slots` — the x86_64 backend turns slot `i` into the
/// byte offset `i*8` from the `_twig_globals` base, and the harness reserves a
/// zeroed `_twig_globals` region the same `lea`/`mov` then addresses. Replicated
/// here (a few lines) rather than taking a `twig-aot` dependency just for tests.
fn collect_global_slots(module: &interpreter_ir::IIRModule) -> HashMap<String, usize> {
    let mut slots = HashMap::new();
    let mut next = 0usize;
    for f in &module.functions {
        for instr in &f.instructions {
            if instr.op == "global_load" || instr.op == "global_store" {
                if let Some(name) = instr.srcs.first().and_then(|o| o.as_str_lit()) {
                    slots.entry(name.to_string()).or_insert_with(|| {
                        let s = next;
                        next += 1;
                        s
                    });
                }
            }
        }
    }
    slots
}

/// A compiled function: its name, x86_64 machine-code bytes, and the external
/// relocations (cross-function `call`s and runtime-symbol calls) the harness
/// must resolve.  This is exactly the per-function output the AOT linker
/// concatenates — we hand it to the simulator's harness instead.
struct X86Function {
    name: String,
    bytes: Vec<u8>,
    relocs: Vec<Reloc>,
}

/// Drive a language frontend → IIR → x86_64 machine code, **per function**.
///
/// This mirrors `twig-aot`'s native pipeline (`compile_module_x86_64_to_text`)
/// precisely:
///
/// 1. `compile_source_to_iir` — frontend produces the shared `IIRModule`.
/// 2. for each function: `infer_types` → `aot_specialise` → `compile_function_with_relocs`.
///
/// The curated programs here have no Twig globals, so the simpler
/// `compile_function_with_relocs` (empty global-slot map) suffices — a global
/// access would surface as a `PcRel32` reloc, which these programs never emit.
///
/// Returns the per-function blobs plus the module entry point.  The backend's
/// `ExternalReloc` carries a `kind` the linker uses to pick an OS relocation
/// record; the simulator's harness only needs `(patch_offset, symbol)` (it
/// resolves internal calls itself and routes the rest to host shims), so we
/// project to the harness's lightweight `Reloc`.
fn compile_to_x86_functions(lang: Language, src: &str) -> (Vec<X86Function>, String) {
    let module = compile_source_to_iir(lang, src, "matrix_x86")
        .expect("frontend should lower the matrix program to IIR");

    // Module globals (E6/O3/AL6): each `global_load`/`global_store` name gets a
    // slot; the backend lowers an access to `lea rax,[rip+_twig_globals]` + a
    // `mov` at `[rax + slot*8]`, recorded as a `PcRel32` reloc the harness
    // resolves to its zeroed `_twig_globals` region. Empty map ⇒ no global ops
    // ⇒ no such reloc (the pre-globals programs are unchanged).
    let global_slots = collect_global_slots(&module);

    let mut funcs = Vec::with_capacity(module.functions.len());
    for fn_ in &module.functions {
        let ctx = FunctionContext {
            name: &fn_.name,
            params: &fn_.params,
            return_type: &fn_.return_type,
        };
        let inferred = infer_types(fn_);
        let cir = aot_specialise(fn_, Some(&inferred));
        let (bytes, relocs) = compile_function_with_globals(&ctx, &cir, X86_64Abi::SysV, &global_slots)
            .expect("x86_64-backend should compile the specialised CIR");
        funcs.push(X86Function {
            name: fn_.name.clone(),
            bytes,
            relocs: relocs
                .into_iter()
                .map(|r| Reloc { patch_offset: r.patch_offset, symbol: r.symbol })
                .collect(),
        });
    }

    let entry = module
        .entry_point
        .clone()
        .unwrap_or_else(|| "main".to_string());
    (funcs, entry)
}

/// Compile `src` with the x86_64 backend and **run the machine code** on the
/// simulator, returning the process exit code (`rax & 0xFF`).
fn run_on_x86_sim(lang: Language, src: &str) -> i32 {
    let (funcs, entry) = compile_to_x86_functions(lang, src);
    let mut builder = MachineCodeHarness::new();
    for f in &funcs {
        builder = builder.function(&f.name, &f.bytes, &f.relocs);
    }
    let mut sim = builder
        .build(&entry)
        .expect("harness should lay out + link the matrix program");
    sim.run().expect("x86_64 machine code should run to a clean ret")
}

/// Like [`run_on_x86_sim`] but also returns whatever the program printed via the
/// host `print_i64` / `putchar` shims — for the matrix's stdout-asserting cells
/// (BASIC `PRINT`, Oct `out`, …) whose observable result is text, not an exit
/// code.
fn run_capturing_stdout(lang: Language, src: &str) -> (i32, String) {
    let (funcs, entry) = compile_to_x86_functions(lang, src);
    let mut builder = MachineCodeHarness::new();
    for f in &funcs {
        builder = builder.function(&f.name, &f.bytes, &f.relocs);
    }
    let mut sim = builder
        .build(&entry)
        .expect("harness should lay out + link the matrix program");
    let code = sim.run().expect("x86_64 machine code should run to a clean ret");
    let out = String::from_utf8(sim.stdout.clone())
        .expect("captured stdout should be valid UTF-8");
    (code, out)
}

// ===========================================================================
// The cells — each `src` is a verbatim copy of a `lang_matrix.rs` NativeAot
// cell; each `assert_eq!` is the same exit code the matrix asserts.
// ===========================================================================

/// Twig — the canonical smoke program (`42`).  Const + ret, no relocations.
#[test]
fn twig_const_runs_on_x86_sim() {
    assert_eq!(run_on_x86_sim(Language::Twig, "42"), 42);
}

/// Twig — n-ary integer arithmetic (`(+ 10 20 12)` ⇒ 42) folded to a typed
/// binary chain (LANG-FULL TW1).  Exercises `add`/`const`/`mov`.
#[test]
fn twig_arithmetic_runs_on_x86_sim() {
    assert_eq!(run_on_x86_sim(Language::Twig, "(+ 10 20 12)"), 42);
}

/// ALGOL 60 — integer `mod` arithmetic in a begin/end block (`17 mod 5` ⇒ 2).
#[test]
fn algol_integer_arithmetic_runs_on_x86_sim() {
    let src = "begin integer result; result := 17 - 5 - 5 - 5 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 2);
}

/// ALGOL 60 — a typed procedure (`integer procedure sq(x) … sq := x*x;
/// result := sq(7)` ⇒ 49).  The first **multi-function** program here, so it
/// exercises the harness's *internal* `call` relocation patching (`main`→`sq`).
#[test]
fn algol_procedure_call_runs_on_x86_sim() {
    let src = "begin integer result; integer procedure sq(x); value x; integer x; \
               sq := x * x; result := sq(7) end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 49);
}

/// ALGOL 60 — a **module global** shared between a procedure and its enclosing
/// block (LANG-FULL enabler **E6**).  `counter` is read+written by `incr` and
/// seeded by the block, so the frontend materialises it as a `_twig_globals`
/// slot: `incr` does `lea rax,[rip+_twig_globals]` + `mov`s at `[rax+0]`, the
/// block likewise.  This is the FIRST x86-sim cell to exercise a `PcRel32`
/// relocation against the globals data symbol — the harness resolves it to its
/// zeroed `_twig_globals` region.  `counter := 40; result := incr(2)` ⇒ 42,
/// run on the real x86_64 bytes locally (the same exit code the matrix's
/// NativeAot column asserts for this E6 program).
#[test]
fn algol_module_global_runs_on_x86_sim() {
    let src = "begin integer counter, result; \
               integer procedure incr(x); value x; integer x; \
                  incr := counter := counter + x; \
               counter := 40; \
               result := incr(2) end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}
// (An Oct `static` (O3) and ALGOL `own` (AL6) cell can be added once their
// frontend PRs land on main; both reuse the exact same `_twig_globals` path
// this E6 cell exercises, so the harness support added here already covers them.)

/// ALGOL 60 — **E3 real arithmetic + equality** (`r := 2.5 * 2.0; if r = 5.0`
/// ⇒ 42).  Runs the x86_64 backend's **SSE2** output (`movabs`/`movsd`/`mulsd`/
/// `ucomisd`/`setcc`) locally — the E3-native float column the matrix could only
/// execute on the x86 CI runner.
#[test]
fn algol_real_equality_runs_on_x86_sim() {
    let src = "begin real r; integer result; r := 2.5 * 2.0; \
               if r = 5.0 then result := 42 else result := 0 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}

/// ALGOL 60 — **E3 real division + ordered comparison** (`r := 7.0 / 2.0;
/// if r < 4.0` ⇒ 1).  Runs `divsd` + the ordered-`ucomisd` flag reading.
#[test]
fn algol_real_division_runs_on_x86_sim() {
    let src = "begin real r; integer result; r := 7.0 / 2.0; \
               if r < 4.0 then result := 1 else result := 0 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 1);
}

/// ALGOL 60 — **E5 straight-line integer array** (`A[1] := 40; A[3] := 2;
/// result := A[1] + A[3]` ⇒ 42).  Runs the x86_64 backend's *native* static
/// array model locally: `alloc_array` → a `__twig_alloc_bytes` call (routed to
/// the simulator's bump-heap host shim), each `array_set`/`array_get` an
/// explicit unsigned bounds `cmp` + `jb` over a `ud2` trap, then a base+idx*8
/// load/store at offset 8.  This **retro-verifies E5 native arrays on x86_64**,
/// on aarch64.
#[test]
fn algol_static_array_runs_on_x86_sim() {
    let src = "begin integer array A[1:3]; integer result; \
               A[1] := 40; A[3] := 2; result := A[1] + A[3] end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}

// ===========================================================================
// S4 — broader coverage: more matrix programs run on the x86_64 column locally.
// (Exploratory batch — confirm which the current opcode set already handles.)
// ===========================================================================

/// Twig — top-level value `define`s summed (`(define x 40)(define y 2)(+ x y)`
/// ⇒ 42).  Exercises multiple typed registers in `main`.
#[test]
fn twig_define_runs_on_x86_sim() {
    assert_eq!(run_on_x86_sim(Language::Twig, "(define x 40) (define y 2) (+ x y)"), 42);
}

/// Nib — `u8` saturating-add wrap guard (`200 +? 100` clamps to 255 ⇒ exit 1).
/// Exercises a narrow add + a clamp branch (`cmp`/`jcc`) at u8 width.
#[test]
fn nib_u8_wrap_runs_on_x86_sim() {
    let src = "fn main() -> u8 { let x: u8 = 200 +? 100; if x == 255 { return 1; } return 0; }";
    assert_eq!(run_on_x86_sim(Language::Nib, src), 1);
}

/// Nib — unary `~` complement at u8 width (`~0` ⇒ 255 ⇒ exit 1).  Exercises the
/// `not` op + the u8 value mask.
#[test]
fn nib_complement_runs_on_x86_sim() {
    let src = "fn main() -> u8 { let x: u8 = ~0; if x == 255 { return 1; } return 0; }";
    assert_eq!(run_on_x86_sim(Language::Nib, src), 1);
}

/// ALGOL — a switch / computed goto (`goto s[3]` ⇒ exit 49).  Exercises the
/// 1-based index compare chain + multiple `jmp`/`jcc`/`label`s.
#[test]
fn algol_switch_runs_on_x86_sim() {
    let src = "begin integer result; switch s := a1, a2, a3; integer i; i := 3; \
               goto s[i]; a1: result := 1; goto done; a2: result := 2; goto done; \
               a3: result := 49; done: end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 49);
}

/// ALGOL — `for`-loop sum of squares into an array (`1+4+9+16+25` ⇒ exit 55).
/// Exercises a counted loop + `array_set`/`array_get` inside it.
#[test]
fn algol_for_loop_array_runs_on_x86_sim() {
    let src = "begin integer array A[1:5]; integer i, result; \
               for i := 1 step 1 until 5 do A[i] := i * i; \
               result := 0; \
               for i := 1 step 1 until 5 do result := result + A[i] end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 55);
}

/// Dartmouth BASIC — `PRINT 42` ⇒ stdout `42` via the `print_i64` host shim.
#[test]
fn basic_print_runs_on_x86_sim() {
    let (_code, out) = run_capturing_stdout(Language::DartmouthBasic, "10 PRINT 42\n20 END\n");
    // BASIC `PRINT` terminates the line, so the shim emits a trailing newline.
    assert_eq!(out, "42\n");
}

/// Dartmouth BASIC — `FOR`/`NEXT` accumulator summing 1..5 ⇒ stdout `15`.
#[test]
fn basic_for_loop_runs_on_x86_sim() {
    let src = "10 LET S = 0\n20 FOR I = 1 TO 5\n30 LET S = S + I\n40 NEXT I\n50 PRINT S\n60 END\n";
    let (_code, out) = run_capturing_stdout(Language::DartmouthBasic, src);
    assert_eq!(out, "15\n");
}

/// Dartmouth BASIC — a **`DIM` array** (LANG-FULL BA3) on the x86_64 backend.
/// `DIM A(3)` reserves a 4-element (0..3 inclusive) integer array; each
/// `LET A(i) = e` is an `array_set` and each `A(i)` read is a bounds-checked
/// `array_get` — the *same* shared array ops `algol_static_array_…` exercises,
/// but reached through the **BASIC** frontend's lowering rather than ALGOL's.
/// `A(1)+A(2)+A(3)` = 10 + 20 + 12 ⇒ stdout `42`, proving the BASIC array path
/// produces correct native machine code (not just the ALGOL one already
/// covered by `algol_static_array` / `algol_for_loop_array`).
#[test]
fn basic_array_runs_on_x86_sim() {
    let src = "10 DIM A(3)\n20 LET A(1) = 10\n30 LET A(2) = 20\n\
               40 LET A(3) = 12\n50 PRINT A(1) + A(2) + A(3)\n60 END\n";
    let (_code, out) = run_capturing_stdout(Language::DartmouthBasic, src);
    assert_eq!(out, "42\n");
}

/// Oct — bitwise complement (the `not` op → x86 `0xF7 /2`) printed via `out`
/// (`fn main() { out(1, ~0); }` ⇒ stdout `255`, the u8-masked `-1`).  This is
/// the cell that surfaced the missing group-3 `0xF7` opcode in the simulator.
#[test]
fn oct_complement_runs_on_x86_sim() {
    let (_code, out) = run_capturing_stdout(Language::Oct, "fn main() { out(1, ~0); }");
    assert_eq!(out, "255");
}

/// Oct — a **`static` module global** (LANG-FULL O3) on the x86_64 backend.
/// `counter` is a top-level `static` shared across functions: it lives in the
/// `_twig_globals` data region (S8), `bump()` — a *separate* function —
/// increments it twice, and `main` prints it (`out` → stdout) ⇒ `42`. This is
/// the Oct counterpart to `algol_module_global_runs_on_x86_sim`: the same
/// `lea [rip+_twig_globals]` + `mov [rax+slot*8]` lowering, exercised from a
/// real Oct frontend's output on the simulator. A per-function register would
/// print `40`; `42` proves the global persisted across the two `bump` calls.
#[test]
fn oct_static_global_runs_on_x86_sim() {
    let src = "static counter: u8 = 40; \
               fn bump() { counter = counter + 1; } \
               fn main() { bump(); bump(); out(1, counter); }";
    let (_code, out) = run_capturing_stdout(Language::Oct, src);
    assert_eq!(out, "42");
}

/// ALGOL 60 — an **`own` static-lifetime variable** (LANG-FULL AL6) on the
/// x86_64 backend.  `bump`'s `own integer n` is a module global (the same
/// `_twig_globals` slot mechanism, S8) that persists across calls:
/// `bump(1) + bump(1) + bump(1)` accumulates 1 + 2 + 3 = 6 on the one cell.  A
/// non-`own` local (a register) would give 3 — so 6 proves the global survived
/// the three real x86_64 `call`s, executed on the simulator.  Together with
/// `algol_module_global_runs_on_x86_sim` (E6) and `oct_static_global_…` (O3)
/// this exercises all three module-global frontends locally.
#[test]
fn algol_own_variable_runs_on_x86_sim() {
    let src = "begin integer result; \
               integer procedure bump(d); value d; integer d; \
                  begin own integer n; n := n + d; bump := n end; \
               result := bump(1) + bump(1) + bump(1) end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 6);
}

/// Nib — **unsigned division** (`84 / 2` ⇒ exit 42).  The `div` op lowers to the
/// x86 unsigned-division sequence `xor rdx,rdx; div rcx` — exercising the
/// group-3 `0xF7 /6` end-to-end (S4's unit tests cover `div` in isolation; this
/// runs it from real backend output).
#[test]
fn nib_unsigned_division_runs_on_x86_sim() {
    assert_eq!(run_on_x86_sim(Language::Nib, "fn main() -> u8 { return 84 / 2; }"), 42);
}

/// ALGOL — **signed integer division** (`85 div 2` ⇒ exit 42).  ALGOL's `div`
/// lowers to the signed sequence `cqo; idiv rcx` — exercising `0xF7 /7` + `cqo`
/// end-to-end (the `idiv`/`cqo` path S4 otherwise only unit-tests).
#[test]
fn algol_signed_division_runs_on_x86_sim() {
    let src = "begin integer result; result := 85 div 2 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}

/// ALGOL — the **`abs` standard function** (LANG-FULL AL8) on the x86_64 backend.
/// `abs` lowers to `if E < 0 then -E else E` — a `cmp_lt` against zero, a
/// `jmp_if_false`, and a negated vs pass-through `mov` into one slot.  This is
/// the first matrix cell to run that **compare-and-branch-into-a-merged-result**
/// shape from real x86_64 machine code on the simulator (the existing ALGOL
/// cells branch for control flow — `for`/`switch` — but don't merge a value back
/// out of two arms).  `abs(0 - 42)` = 42 ⇒ exit 42 proves the negated arm and
/// the join produce correct native code.
#[test]
fn algol_abs_runs_on_x86_sim() {
    let src = "begin integer result; result := abs(0 - 42) end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}

/// ALGOL — the **`sign` standard function** (LANG-FULL AL8) on the x86_64
/// backend.  `sign` lowers to the *nested* conditional `if E > 0 then 1 else
/// if E < 0 then -1 else 0` — two compares (`cmp_gt`, `cmp_lt`) and three
/// `i64` constants moved into one slot across three branch arms.  Where the
/// `algol_abs` cell merges a value out of **two** arms, this runs the **three**-
/// way merge from real x86_64 machine code.  `43 + sign(0 - 1)` = 43 + (-1) =
/// 42 ⇒ exit 42 proves the negative arm (-1) and the join produce correct
/// native code.  Together with `algol_abs_runs_on_x86_sim` this exercises both
/// AL8 standard functions locally on the x86_64 backend.
#[test]
fn algol_sign_runs_on_x86_sim() {
    let src = "begin integer result; result := 43 + sign(0 - 1) end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}

/// Brainfuck — build 65 on the tape and `putchar` it (`++++++++[>++++++++<-]>+.`
/// ⇒ stdout `A`).  Exercises the **byte-tape** opcode surface the arithmetic
/// programs never touch: `__twig_alloc_bytes` for the tape, 8-bit load/store
/// (`load_byte`/`store_byte`), a `[...]` loop, and the `putchar` host shim.
#[test]
fn brainfuck_putchar_runs_on_x86_sim() {
    let (_code, out) = run_capturing_stdout(Language::Brainfuck, "++++++++[>++++++++<-]>+.");
    assert_eq!(out, "A");
}

/// Like [`run_capturing_stdout`] but feeds `input` to the program's `getchar`
/// (the harness stdin buffer) — for the Brainfuck `,` programs.
fn run_with_stdin(lang: Language, src: &str, input: &[u8]) -> String {
    let (funcs, entry) = compile_to_x86_functions(lang, src);
    let mut builder = MachineCodeHarness::new().stdin(input);
    for f in &funcs {
        builder = builder.function(&f.name, &f.bytes, &f.relocs);
    }
    let mut sim = builder
        .build(&entry)
        .expect("harness should lay out + link the matrix program");
    sim.run().expect("x86_64 machine code should run to a clean ret");
    String::from_utf8(sim.stdout.clone()).expect("stdout should be valid UTF-8")
}

/// Brainfuck — **read a byte from stdin**, `+`, print (`,+.` with input `A` ⇒
/// `B`).  Exercises the `getchar` host shim consuming a real input byte.
#[test]
fn brainfuck_stdin_increment_runs_on_x86_sim() {
    assert_eq!(run_with_stdin(Language::Brainfuck, ",+.", b"A"), "B");
}

/// Brainfuck — **echo two bytes** (`,.,.` with input `Hi` ⇒ `Hi`).
#[test]
fn brainfuck_stdin_echo_runs_on_x86_sim() {
    assert_eq!(run_with_stdin(Language::Brainfuck, ",.,.", b"Hi"), "Hi");
}

/// Brainfuck — **cat until EOF** (`,[.,]` with input `Hi` ⇒ `Hi`).  The `[...]`
/// loop reads+prints until `getchar` returns EOF, which the Brainfuck IIR clamps
/// to a 0 cell so the loop halts — so this also checks the simulator's EOF (`-1`)
/// convention threads correctly through the backend's clamp.
#[test]
fn brainfuck_cat_runs_on_x86_sim() {
    assert_eq!(run_with_stdin(Language::Brainfuck, ",[.,]", b"Hi"), "Hi");
}
