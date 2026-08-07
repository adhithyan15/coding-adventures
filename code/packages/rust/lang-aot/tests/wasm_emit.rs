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

struct CapturePrintStr {
    bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl wasm_execution::HostFunction for CapturePrintStr {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::I32, wasm_types::ValueType::I32],
                results: vec![],
            });
        &FT
    }

    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let ptr = args
            .first()
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: missing ptr"))?
            .as_i32()
            .map_err(|error| wasm_execution::TrapError::new(error.message))?;
        let len = args
            .get(1)
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: missing len"))?
            .as_i32()
            .map_err(|error| wasm_execution::TrapError::new(error.message))?;
        if ptr < 0 || len < 0 {
            return Err(wasm_execution::TrapError::new("__print_str: negative ptr/len"));
        }

        let start = usize::try_from(ptr)
            .map_err(|_| wasm_execution::TrapError::new("__print_str: ptr overflow"))?;
        let len = usize::try_from(len)
            .map_err(|_| wasm_execution::TrapError::new("__print_str: len overflow"))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: range overflow"))?;
        let memory = memory
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: no linear memory"))?;
        let bytes = (start..end)
            .map(|offset| memory.load_i32_8u(offset).map(|byte| byte as u8))
            .collect::<Result<Vec<_>, _>>()?;
        self.bytes
            .lock()
            .expect("wasm print capture poisoned")
            .extend_from_slice(&bytes);
        Ok(vec![])
    }
}

struct PrintStrHost {
    bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl wasm_execution::HostInterface for PrintStrHost {
    fn resolve_function(
        &self,
        module_name: &str,
        name: &str,
    ) -> Option<Box<dyn wasm_execution::HostFunction>> {
        (module_name == "env" && name == "__print_str").then(|| {
            Box::new(CapturePrintStr {
                bytes: std::sync::Arc::clone(&self.bytes),
            }) as Box<dyn wasm_execution::HostFunction>
        })
    }

    fn resolve_global(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<(wasm_types::GlobalType, wasm_execution::WasmValue)> {
        None
    }

    fn resolve_memory(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<wasm_execution::LinearMemory> {
        None
    }

    fn resolve_table(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<wasm_execution::Table> {
        None
    }
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
fn algol_captured_and_own_strings_emit_and_run_on_wasm() {
    let source = "begin integer result; string shared; \
                  procedure setshared; shared := 'C'; \
                  integer procedure remember(n); value n; integer n; \
                     begin own string memo; if n = 1 then memo := 'A'; \
                       if memo = 'A' then remember := 1 else remember := 0 end; \
                  setshared; result := 0; \
                  if shared = 'C' then result := result + 1; \
                  result := result + remember(1) + remember(2) end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_global_string")
        .expect("ALGOL captured/own strings should emit wasm");
    assert_wellformed(&bytes, "(ALGOL captured/own strings)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL captured/own string wasm must run");
    assert_eq!(result, vec![3], "ALGOL globals should preserve string handles");
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

#[test]
fn algol_for_while_emits_and_runs_on_wasm() {
    let source = "begin integer x, result; x := 6; result := 0; for x := x - 1 while x > 0 do result := result + x end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_for_while")
        .expect("ALGOL for-while loop should emit wasm");
    assert_wellformed(&bytes, "(ALGOL for-while sum)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL for-while wasm must run");
    assert_eq!(result, vec![15], "ALGOL for-while should sum 5..1");
}

#[test]
fn algol_for_list_emits_and_runs_on_wasm() {
    let source = "begin integer i, result; i := 0; result := 0; for i := 1 step 1 until 3, 10, i + 1 while i < 13 do result := result + i end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_for_list")
        .expect("ALGOL for-list loop should emit wasm");
    assert_wellformed(&bytes, "(ALGOL for-list sum)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL for-list wasm must run");
    assert_eq!(result, vec![39], "ALGOL for-list should sum mixed elements");
}

#[test]
fn algol_dynamic_step_emits_and_runs_on_wasm() {
    let source = "begin integer i, stepvalue, result; result := 0; stepvalue := 2; for i := 1 step stepvalue until 5 do result := result + i; stepvalue := 0 - stepvalue; for i := 5 step stepvalue until 1 do result := result + i end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_dynamic_step")
        .expect("ALGOL dynamic-step loop should emit wasm");
    assert_wellformed(&bytes, "(ALGOL dynamic step)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL dynamic-step wasm must run");
    assert_eq!(result, vec![18], "ALGOL dynamic step should sum both directions");
}

#[test]
fn algol_proper_procedure_emits_and_runs_on_wasm() {
    let source = "begin integer result; procedure bump(d); value d; integer d; result := result + d; result := 40; bump(2) end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_proper_proc")
        .expect("ALGOL proper procedure should emit wasm");
    assert_wellformed(&bytes, "(ALGOL proper procedure)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL proper-procedure wasm must run");
    assert_eq!(result, vec![42], "ALGOL proper procedure should return 42");
}

#[test]
fn algol_conditional_expressions_emit_and_run_on_wasm() {
    let source = "begin boolean flag; integer i, result; flag := true; result := 0; for i := if flag then 1 else 4 step 1 until if flag then 3 else 4 do result := result + i; if if result = 6 then flag else false then result := 42 else result := result end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_cond_expr")
        .expect("ALGOL conditional expressions should emit wasm");
    assert_wellformed(&bytes, "(ALGOL conditional expressions)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL conditional-expression wasm must run");
    assert_eq!(result, vec![42], "ALGOL conditional expressions should return 42");
}

#[test]
fn algol_nested_blocks_emit_and_run_on_wasm() {
    let source = "begin integer x, result; boolean flag; x := 1; flag := true; result := 0; begin integer x; boolean flag; x := 10; flag := false; begin integer x; x := 31; if not flag then result := x else result := 1 end; result := result + x end; if flag then result := result + x else result := 0 end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_nested_blocks")
        .expect("ALGOL nested blocks should emit wasm");
    assert_wellformed(&bytes, "(ALGOL nested blocks)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL nested-block wasm must run");
    assert_eq!(result, vec![42], "ALGOL nested blocks should return 42");
}

#[test]
fn algol_runtime_string_local_emits_and_runs_on_wasm() {
    let source = "begin string s; integer result; \
                  string procedure pick(n); value n; integer n; \
                    if n > 0 then pick := 'HI' else pick := 'LO'; \
                  s := pick(1); \
                  if s = 'HI' then result := 42 else result := 0; \
                  print(s) end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_runtime_string")
        .expect("ALGOL runtime string local should emit wasm");
    assert_wellformed(&bytes, "(ALGOL runtime string local)");

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let result = WasmRuntime::with_host(Box::new(PrintStrHost {
        bytes: std::sync::Arc::clone(&output),
    }))
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL runtime string wasm must run");
    assert_eq!(result, vec![42]);
    assert_eq!(&*output.lock().expect("wasm print capture poisoned"), b"HI");
}

#[test]
fn algol_runtime_string_ordering_emits_and_runs_on_wasm() {
    let source = "begin string s; integer result; \
                  string procedure pick(n); value n; integer n; \
                    if n > 0 then pick := 'HI' else pick := 'LO'; \
                  s := pick(1); \
                  if s < 'LO' then result := 42 else result := 0; \
                  print(s) end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_runtime_string_ordering")
        .expect("ALGOL runtime string ordering should emit wasm");
    assert_wellformed(&bytes, "(ALGOL runtime string ordering)");

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let result = WasmRuntime::with_host(Box::new(PrintStrHost {
        bytes: std::sync::Arc::clone(&output),
    }))
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL runtime string ordering wasm must run");
    assert_eq!(result, vec![42]);
    assert_eq!(&*output.lock().expect("wasm print capture poisoned"), b"HI");
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

/// `COND` runs on wasm (LANG77 L3b-3a-4d): the clause guards branch with lisp
/// truthiness — a predicate result tests directly, while a lisp value is wrapped
/// `not(is_null(...))`, so an integer atom (even `0`) is true and only `nil` is
/// false. The control flow (`jmp_if_false`/`label`/`jmp`/`mov`) already lowered.
#[test]
fn mccarthy_cond_runs_on_wasm() {
    let rt = WasmRuntime::new();
    let go = |src: &str| {
        let bytes = compile_source_to_wasm(Language::McCarthyLisp, src, "cond").expect("emit COND");
        rt.load_and_run(&bytes, "main", &[]).expect("run COND")
    };

    // First clause true (a predicate guard): 5 is an atom → 7.
    assert_eq!(go("(COND ((ATOM 5) 7) (5 9))"), vec![7]);
    // First clause false → second clause's atom guard `5` is truthy → 9.
    assert_eq!(go("(COND ((ATOM (CONS 1 2)) 7) (5 9))"), vec![9]);
    // The atom `0` is TRUE in lisp (only `nil` is false) → first clause fires.
    assert_eq!(go("(COND (0 7) (5 9))"), vec![7], "0 is truthy in lisp");
    // No clause matches (the only guard is false) → nil result (exit 0).
    assert_eq!(go("(COND ((ATOM (CONS 1 2)) 7))"), vec![0], "no clause → nil");
    // An `EQ` guard: true and false branches.
    assert_eq!(go("(COND ((EQ 1 1) 7) (5 9))"), vec![7]);
    assert_eq!(go("(COND ((EQ 1 2) 7) (5 9))"), vec![9]);
}

/// Symbols run on wasm (LANG77 W1 / F6): `QUOTE`/symbol literals are interned to
/// distinct integers in a reserved range (boxed as `i31ref`), so `EQ` tells
/// symbols apart — `(EQ 'A 'A)` → T, `(EQ 'A 'B)` → nil — disjoint from integers.
#[test]
fn mccarthy_symbols_run_on_wasm() {
    let rt = WasmRuntime::new();
    let go = |src: &str| {
        let bytes = compile_source_to_wasm(Language::McCarthyLisp, src, "sym").expect("emit symbol");
        rt.load_and_run(&bytes, "main", &[]).expect("run symbol")
    };

    // Same symbol is EQ; different symbols are not.
    assert_eq!(go("(EQ 'A 'A)"), vec![1], "'A = 'A");
    assert_eq!(go("(EQ 'A 'B)"), vec![0], "'A != 'B");
    assert_eq!(go("(EQ 'FOO 'FOO)"), vec![1], "multi-char symbol");
    // A symbol is an atom (not a cons).
    assert_eq!(go("(ATOM 'A)"), vec![1], "'A is an atom");
    // Symbols flow through cons cells.
    assert_eq!(go("(EQ (CAR (CONS 'A 'B)) 'A)"), vec![1], "car of a cons of symbols");
    // A symbol guard in COND.
    assert_eq!(go("(COND ((EQ 'A 'B) 7) ((EQ 'X 'X) 9) (5 1))"), vec![9]);
    // Symbol ids are disjoint from integer atoms.
    assert_eq!(go("(EQ 'A 5)"), vec![0], "a symbol never equals an integer");
}

/// `LAMBDA`/`LABEL` + user calls + recursion run on wasm (LANG77 W2 / F7). The
/// frontend lifts each `LAMBDA`/`LABEL` to its own function; the structural pass
/// makes the call boundary uniform-anyref (params anyref, args boxed, returns
/// anyref), so a lambda can be applied and a `LABEL` can recurse.
#[test]
fn mccarthy_lambda_and_recursion_run_on_wasm() {
    let rt = WasmRuntime::new();
    let go = |src: &str| {
        let bytes = compile_source_to_wasm(Language::McCarthyLisp, src, "lam").expect("emit lambda");
        rt.load_and_run(&bytes, "main", &[]).expect("run lambda")
    };

    // Identity lambda returns its argument.
    assert_eq!(go("((LAMBDA (X) X) 5)"), vec![5], "id lambda");
    // A lambda that conses its argument; CAR reads it back.
    assert_eq!(go("(CAR ((LAMBDA (X) (CONS X X)) 7))"), vec![7], "lambda body builds a cons");
    // Two parameters, bound positionally.
    assert_eq!(go("(CDR ((LAMBDA (X Y) (CONS X Y)) 3 4))"), vec![4], "two-arg lambda");
    // A predicate inside a lambda.
    assert_eq!(go("((LAMBDA (X) (EQ X X)) 5)"), vec![1], "EQ inside a lambda");
    // A lambda returning a bare atom.
    assert_eq!(go("((LAMBDA (X) 9) 5)"), vec![9], "lambda returns an atom");

    // A recursive LABEL: walk down a list to its atom tail (no arithmetic in
    // McCarthy 1.0). f(L) = if (ATOM L) then 99 else f(CDR L).
    let rec = "((LABEL F (LAMBDA (L) (COND ((ATOM L) 99) ((EQ 1 1) (F (CDR L)))))) (CONS 1 (CONS 2 3)))";
    assert_eq!(go(rec), vec![99], "recursive LABEL walks to the atom");
}
