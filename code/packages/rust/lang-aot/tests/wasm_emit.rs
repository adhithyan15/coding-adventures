//! # WebAssembly emit + run tests (LANG77 / McCarthy L3b-3a-2).
//!
//! The first *managed* `--emit` target. Emitting `.wasm` is platform-agnostic,
//! so these run on every host. Crucially, they don't just check the bytes —
//! they **run** the emitted module on the in-repo `wasm-runtime` and assert the
//! computed result (zero-external-dep verification, per the user's "extend the
//! repo's own wasm tooling" decision).
//!
//! Scope (L3b-3a-2): **scalar** McCarthy programs (no cons cells). A scalar
//! program compiles to a module whose `main` returns an `i64`, which
//! `wasm-runtime` executes today. Cons/symbol programs need the boxed-`anyref`
//! value model + WasmGC support in the engine — follow-up slices.

use lang_aot::{compile_source_to_wasm, Language};
use wasm_runtime::WasmRuntime;

/// The 8-byte WebAssembly header: magic `\0asm` + version 1.
const WASM_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

fn assert_wellformed(bytes: &[u8], what: &str) {
    assert!(bytes.len() > 16, "{what}: wasm too short ({} bytes)", bytes.len());
    assert_eq!(&bytes[..8], &WASM_HEADER, "{what}: missing wasm magic/version header");
}

/// A scalar McCarthy program emits a valid wasm module **and runs** to the
/// right value on the in-repo runtime — the end-to-end proof of the
/// McCarthy → wasm pipeline.
#[test]
fn mccarthy_scalar_emits_and_runs_on_wasm() {
    let bytes = compile_source_to_wasm(Language::McCarthyLisp, "42", "scalar")
        .expect("McCarthy `42` should emit wasm");
    assert_wellformed(&bytes, "(McCarthy 42)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("emitted wasm must load and run on the in-repo runtime");
    assert_eq!(result, vec![42], "main() should return i64 42");
}

/// Reusability: Twig is also a lisp-family frontend on the same IIR, so a Twig
/// scalar program flows through the identical wasm path with no Twig-specific
/// code — and runs to the same value.
#[test]
fn twig_scalar_emits_and_runs_on_wasm() {
    let bytes = compile_source_to_wasm(Language::Twig, "42", "twig")
        .expect("Twig scalar should emit wasm");
    assert_wellformed(&bytes, "(Twig 42)");

    let rt = WasmRuntime::new();
    let result = rt.load_and_run(&bytes, "main", &[]).expect("Twig wasm must run");
    assert_eq!(result, vec![42], "Twig main() should return 42");
}

/// ALGOL 60 now enters the same Rust IIR chain as the other LANG frontends.
#[test]
fn algol_scalar_emits_and_runs_on_wasm() {
    let source = "begin integer result; result := 42 end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol")
        .expect("ALGOL scalar should emit wasm");
    assert_wellformed(&bytes, "(ALGOL 42)");

    let rt = WasmRuntime::new();
    let result = rt.load_and_run(&bytes, "main", &[]).expect("ALGOL wasm must run");
    assert_eq!(result, vec![42], "ALGOL main() should return 42");
}

#[test]
fn algol_mod_emits_and_runs_on_wasm() {
    let source = "begin integer result; result := 17 mod 5 end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_mod")
        .expect("ALGOL mod should emit wasm");
    assert_wellformed(&bytes, "(ALGOL 17 mod 5)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL mod wasm must run");
    assert_eq!(result, vec![2], "ALGOL mod should return 2");
}

#[test]
fn algol_boolean_ops_emit_and_run_on_wasm() {
    let source = "begin boolean a, b; integer result; a := true; b := false; if (a and not b) and ((b impl a) eqv (a or b)) then result := 42 else result := 1 end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_bool_ops")
        .expect("ALGOL boolean operators should emit wasm");
    assert_wellformed(&bytes, "(ALGOL boolean operators)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL boolean-operator wasm must run");
    assert_eq!(result, vec![42], "ALGOL boolean operators should return 42");
}

#[test]
fn algol_for_loop_emits_and_runs_on_wasm() {
    let source =
        "begin integer i, result; result := 0; for i := 1 step 1 until 6 do result := result + i end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_loop")
        .expect("ALGOL loop should emit wasm");
    assert_wellformed(&bytes, "(ALGOL sum 1..6)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL loop wasm must run");
    assert_eq!(result, vec![21], "ALGOL loop should sum 1..6");
}

/// The L3b-3a-3c capstone: a **cons** program compiles to WasmGC and runs
/// end-to-end on the in-repo runtime. The uniform-anyref value model boxes the
/// integer atoms as `i31ref`, allocates a `$LispyPair`, and unboxes the result
/// at the return boundary — so `(CAR (CONS 7 9))` evaluates to `7`.
#[test]
fn mccarthy_cons_car_emits_and_runs_on_wasm() {
    let bytes = compile_source_to_wasm(Language::McCarthyLisp, "(CAR (CONS 7 9))", "cons")
        .expect("McCarthy (CAR (CONS 7 9)) should emit wasm");
    assert_wellformed(&bytes, "(McCarthy (CAR (CONS 7 9)))");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("emitted cons wasm must load and run on the in-repo runtime");
    assert_eq!(result, vec![7], "(CAR (CONS 7 9)) should evaluate to 7");
}

/// `CDR` reads the second field, and cons cells nest.
#[test]
fn mccarthy_cdr_and_nested_cons_run_on_wasm() {
    let rt = WasmRuntime::new();

    let cdr = compile_source_to_wasm(Language::McCarthyLisp, "(CDR (CONS 7 9))", "cdr")
        .expect("emit cdr");
    assert_eq!(rt.load_and_run(&cdr, "main", &[]).expect("run cdr"), vec![9]);

    let nested =
        compile_source_to_wasm(Language::McCarthyLisp, "(CAR (CONS (CDR (CONS 1 2)) 5))", "nested")
            .expect("emit nested");
    assert_eq!(
        rt.load_and_run(&nested, "main", &[]).expect("run nested"),
        vec![2],
        "(CAR (CONS (CDR (CONS 1 2)) 5)) should evaluate to 2"
    );
}

/// `ATOM`/`pair?` run on wasm (LANG77 L3b-3a-4b): `pair?` lowers to
/// `ref.test $LispyPair` and the lisp `not` to `i32.eqz`, so `ATOM x` =
/// `not(pair? x)` tells an atom (`1`) from a cons (`0`).
#[test]
fn mccarthy_atom_predicate_runs_on_wasm() {
    let rt = WasmRuntime::new();

    // An integer is an atom — even with no cons anywhere in the program, the
    // `$LispyPair` struct type is emitted because `pair?` needs it.
    let atom = compile_source_to_wasm(Language::McCarthyLisp, "(ATOM 5)", "atom")
        .expect("emit (ATOM 5)");
    assert_wellformed(&atom, "(ATOM 5)");
    assert_eq!(rt.load_and_run(&atom, "main", &[]).expect("run atom"), vec![1], "5 is an atom");

    // A cons cell is not an atom.
    let cons = compile_source_to_wasm(Language::McCarthyLisp, "(ATOM (CONS 1 2))", "atom_cons")
        .expect("emit (ATOM (CONS 1 2))");
    assert_eq!(
        rt.load_and_run(&cons, "main", &[]).expect("run atom-cons"),
        vec![0],
        "a cons is not an atom"
    );
}

/// `EQ`/`equal?` on atoms runs on wasm (LANG77 L3b-3a-4c): the atoms arrive
/// boxed as `i31ref`, so `equal?` unboxes both and `i32.eq`s them.
#[test]
fn mccarthy_eq_atom_equality_runs_on_wasm() {
    let rt = WasmRuntime::new();

    let eq = compile_source_to_wasm(Language::McCarthyLisp, "(EQ 5 5)", "eq")
        .expect("emit (EQ 5 5)");
    assert_wellformed(&eq, "(EQ 5 5)");
    assert_eq!(rt.load_and_run(&eq, "main", &[]).expect("run eq"), vec![1], "5 = 5");

    let neq = compile_source_to_wasm(Language::McCarthyLisp, "(EQ 5 6)", "neq")
        .expect("emit (EQ 5 6)");
    assert_eq!(rt.load_and_run(&neq, "main", &[]).expect("run neq"), vec![0], "5 != 6");

    // The compared values can be computed (a car of a cons), not just literals.
    let computed =
        compile_source_to_wasm(Language::McCarthyLisp, "(EQ (CAR (CONS 3 4)) 3)", "eq_car")
            .expect("emit eq-car");
    assert_eq!(
        rt.load_and_run(&computed, "main", &[]).expect("run eq-car"),
        vec![1],
        "(CAR (CONS 3 4)) = 3"
    );
}
