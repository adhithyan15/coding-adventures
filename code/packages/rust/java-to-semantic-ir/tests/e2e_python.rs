//! End-to-end execution proof: Java → SIR → Python → `python3`.
//!
//! JV02's own "Verification" section requires each milestone to prove its
//! *combination* of constructs actually lowers to a runnable program, not
//! just that lowering itself succeeds. M1 has no way to produce observable
//! output on its own terms, though — `System.out.println` is a method
//! call, out of scope until JV02 M3 — so unlike every math-language
//! frontend's own `e2e_node.rs` (which calls a native `disp`/`print`
//! builtin), there is nothing in real M1-scoped Java source to assert on
//! directly.
//!
//! Instead, `run_via_python` below takes the already-lowered `Module` and
//! redirects `main`'s trailing block value to whatever the last statement
//! computed (see its own doc comment) — the Python backend's
//! `emit_function_body` unconditionally emits `return <block.value>;`, so
//! this turns `main`'s SIR body into a callable function whose return value
//! is exactly what the test wants to observe. The Java *lowering* itself
//! (parsing, kind inference, operator selection) is entirely real and
//! untouched; only the "how do we observe the result" wiring is
//! test-harness convenience, done here rather than in the frontend.
//!
//! Backend choice: Python, not JavaScript. M1 lowers Java's `+`-based
//! string concatenation to `Expr::StrConcat`, and the JavaScript backend
//! does not accept `Feature::StringInterpolation` yet (`StrConcat` is in
//! its own "deferred, rejected at capability check" list) — the Python
//! backend already does, so one backend covers every M1 construct. M2a
//! adds `if`/`while`/`do`-`while`; the Python backend already accepts
//! `Feature::Loops` and has real codegen for `Expr::If`/`Expr::Block`
//! (the do-while desugaring's own synthetic wrapper), so no new backend
//! gap opens up for this milestone either.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use java_to_semantic_ir::compile_source;
use semantic_ir::Stmt;

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Lower `java_src`, validate it, then run it through the Python backend
/// and `python3`, returning trimmed stdout.
///
/// `java_src`'s `main` body must end with a bare expression statement
/// (legal in this frontend's own already-established simplification — see
/// `lower.rs`'s own module doc comment and M0's identically-shaped
/// `42;`-as-a-statement tests; real `javac` would reject a bare
/// non-assignment/non-call expression statement, but this frontend's
/// grammar is intentionally more permissive). That last statement is
/// popped off `main.body.stmts` and its expression becomes `main.body`'s
/// trailing value instead, so the emitted Python function ends with
/// `return <that expression>`.
fn run_via_python(name: &str, java_src: &str) -> String {
    let mut module = compile_source(java_src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );

    let main = module
        .functions
        .iter_mut()
        .find(|f| f.name == "main")
        .expect("expected a synthesized `main` function");
    match main.body.stmts.pop() {
        Some(Stmt::ExprStmt { expr, .. }) => main.body.value = expr,
        Some(other) => panic!(
            "expected `main`'s last statement to be a bare expression statement, got {other:?}"
        ),
        None => panic!("`main` has no statements to observe"),
    }
    // The Python backend special-cases a function literally named `main`:
    // it mangles it to `_sir_user_main` and auto-invokes it (discarding
    // the return value) at module scope, mirroring a real program's
    // entry point. Renaming it here sidesteps that convention entirely
    // rather than depending on its exact mangled name, so this harness
    // calls the function itself and captures what it returns.
    main.name = "probe".to_string();

    let artifact = semantic_ir_to_python::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("java_sir_e2e_{name}_{}.py", std::process::id()));
    // `create_new`, not `std::fs::write`: fails instead of following a
    // pre-existing symlink at this predictable shared-temp-dir path (see
    // `matlab-to-semantic-ir`'s identically-reasoned `e2e_node.rs`, whose
    // harness this file otherwise mirrors).
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp py (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp py");
    writeln!(file, "print(probe())").expect("write print epilogue");
    drop(file);

    // `sir-runtime-core` unconditionally imports several sibling
    // per-concern packages at its own module-load time (pairs, for its
    // display convention; exceptions, for typed `SirError`), regardless
    // of whether a given emitted program actually uses those features —
    // mirrors `semantic-ir-to-python`'s own `tests/sir22_array.rs`
    // harness, which independently discovered the same full sibling set
    // is required on `PYTHONPATH` even for programs that only exercise a
    // handful of them.
    let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    let pythonpath = std::env::join_paths([
        py_root.join("sir-runtime-core/src"),
        py_root.join("sir-runtime-pairs/src"),
        py_root.join("sir-runtime-oop/src"),
        py_root.join("sir-runtime-range/src"),
        py_root.join("sir-runtime-regex/src"),
        py_root.join("sir-runtime-exceptions/src"),
    ])
    .expect("join PYTHONPATH");

    let output = Command::new("python3")
        .arg(&path)
        .env("PYTHONPATH", &pythonpath)
        .output()
        .expect("spawn python3");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "python3 failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn wrap(body: &str) -> String {
    format!("class Main {{ public static void main(String[] args) {{ {body} }} }}")
}

/// M3a: a full class body with extra methods alongside `main`, unlike
/// [`wrap`] which only ever wraps `main`'s own body.
fn wrap_with_methods(extra_methods: &str, main_body: &str) -> String {
    format!(
        "class Main {{ {extra_methods} public static void main(String[] args) {{ {main_body} }} }}"
    )
}

#[test]
fn arithmetic_composition_runs_in_python() {
    if !python_available() {
        eprintln!("skipping arithmetic_composition_runs_in_python: `python3` not available");
        return;
    }
    // Multiplicative binds tighter than additive: 5 + 2 * 3 == 11, not 21.
    let out = run_via_python(
        "arithmetic_composition",
        &wrap("int x = 5; int y = 2; int z = x + y * 3; z;"),
    );
    assert_eq!(out, "11");
}

#[test]
fn integer_division_truncates_in_python() {
    if !python_available() {
        eprintln!("skipping integer_division_truncates_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "integer_division",
        &wrap("int a = 7; int b = 2; int c = a / b; c;"),
    );
    assert_eq!(out, "3");
}

#[test]
fn float_division_runs_in_python() {
    if !python_available() {
        eprintln!("skipping float_division_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "float_division",
        &wrap("double a = 7.0; double b = 2.0; double c = a / b; c;"),
    );
    assert_eq!(out, "3.5");
}

#[test]
fn string_concatenation_with_auto_stringify_runs_in_python() {
    if !python_available() {
        eprintln!("skipping string_concatenation_with_auto_stringify_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python("string_concat", &wrap(r#"String s = "n=" + 5; s;"#));
    assert_eq!(out, "n=5");
}

#[test]
fn comparison_and_logical_and_runs_in_python() {
    if !python_available() {
        eprintln!("skipping comparison_and_logical_and_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "comparison_and_logical",
        &wrap("int x = 5; int y = 3; boolean r = (x > y) && (x != y); r;"),
    );
    assert_eq!(out, "True");
}

#[test]
fn reassignment_and_unary_not_runs_in_python() {
    if !python_available() {
        eprintln!("skipping reassignment_and_unary_not_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "reassignment_not",
        &wrap("boolean flag = true; flag = !flag; flag;"),
    );
    assert_eq!(out, "False");
}

#[test]
fn var_type_inference_runs_in_python() {
    if !python_available() {
        eprintln!("skipping var_type_inference_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "var_inference",
        &wrap("var x = 10; var y = 4; var z = x - y; z;"),
    );
    assert_eq!(out, "6");
}

#[test]
fn if_else_branching_runs_in_python() {
    if !python_available() {
        eprintln!("skipping if_else_branching_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "if_else",
        &wrap("int x = 10; if (x > 5) { x = 1; } else { x = 2; } x;"),
    );
    assert_eq!(out, "1");
}

#[test]
fn if_without_else_runs_in_python() {
    if !python_available() {
        eprintln!("skipping if_without_else_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "if_no_else",
        &wrap("int x = 10; if (x < 5) { x = 999; } x;"),
    );
    assert_eq!(out, "10");
}

#[test]
fn while_loop_runs_in_python() {
    if !python_available() {
        eprintln!("skipping while_loop_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "while_loop",
        &wrap("int x = 0; while (x < 5) { x = x + 1; } x;"),
    );
    assert_eq!(out, "5");
}

#[test]
fn do_while_loop_runs_in_python() {
    if !python_available() {
        eprintln!("skipping do_while_loop_runs_in_python: `python3` not available");
        return;
    }
    // Proves the body-executes-once-unconditionally semantic specifically:
    // the condition is already false on entry, so a plain pretest `while`
    // would run zero times, but do-while must still run once.
    let out = run_via_python(
        "do_while_loop",
        &wrap("int x = 10; do { x = x + 1; } while (x < 5); x;"),
    );
    assert_eq!(out, "11");
}

#[test]
fn compound_assignment_chain_runs_in_python() {
    if !python_available() {
        eprintln!("skipping compound_assignment_chain_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "compound_assign_chain",
        &wrap("int x = 10; x += 5; x -= 2; x *= 2; x;"),
    );
    assert_eq!(out, "26");
}

#[test]
fn increment_in_a_while_loop_runs_in_python() {
    if !python_available() {
        eprintln!("skipping increment_in_a_while_loop_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "increment_while",
        &wrap("int sum = 0; int i = 0; while (i < 5) { sum = sum + i; i++; } sum;"),
    );
    assert_eq!(out, "10");
}

#[test]
fn do_while_flag_name_collision_does_not_corrupt_a_real_variable() {
    if !python_available() {
        eprintln!(
            "skipping do_while_flag_name_collision_does_not_corrupt_a_real_variable: `python3` not available"
        );
        return;
    }
    // Real execution proof for the /security-review finding fixed
    // alongside this milestone: a user variable literally named
    // `__do_while_0` (a legal Java identifier) must be mutated by the
    // loop body exactly like any other local. The pre-fix desugaring
    // generated its synthetic flag as a bare `__do_while_0` with no
    // collision check, silently shadowing this exact variable — the
    // mutation inside the loop body would apply to the *synthetic*
    // flag instead, so the real variable would come back unmodified
    // (`1`) rather than the correct `2`.
    let out = run_via_python(
        "do_while_flag_collision",
        &wrap(concat!(
            "int __do_while_0 = 1; ",
            "do { __do_while_0 = __do_while_0 + 1; } while (false); ",
            "__do_while_0;"
        )),
    );
    assert_eq!(out, "2");
}

#[test]
fn classic_for_loop_runs_in_python() {
    if !python_available() {
        eprintln!("skipping classic_for_loop_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "classic_for",
        &wrap("int sum = 0; for (int i = 0; i < 5; i++) { sum = sum + i; } sum;"),
    );
    assert_eq!(out, "10");
}

#[test]
fn classic_for_loop_with_no_declaration_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping classic_for_loop_with_no_declaration_runs_in_python: `python3` not available"
        );
        return;
    }
    // `for (i = 0; ...)` reusing an already-declared `i`, rather than
    // declaring a fresh one -- a different `for_init` grammar alternative
    // from the usual `for (int i = 0; ...)` case, exercised on its own.
    let out = run_via_python(
        "classic_for_no_decl",
        &wrap("int i = -1; int sum = 0; for (i = 0; i < 4; i++) { sum = sum + i; } sum;"),
    );
    assert_eq!(out, "6");
}

// No execution-proof test for `for (;;)` (empty clauses): without a
// `break` statement -- which has no SIR IR primitive at all (see the
// module doc comment) -- a `for (;;)` loop genuinely cannot terminate via
// any construct this milestone can lower; an execution proof would just
// hang forever. `classic_for_loop_with_all_clauses_empty_is_an_unconditional_loop`
// in `tests/test_lower.rs` covers the structural claim (the absent
// condition lowers to `BoolLit(true)`, not `false` or some other
// accidentally-terminating shape) without ever actually running it.

// No enhanced-for execution-proof test: M1/M2 have no array/collection
// construction syntax yet (JV02 M4), so there is no way to build a real
// Java expression that lowers to something Python's own `for x in xs:`
// codegen could actually iterate — `enhanced_for_loop_lowers_to_stmt_foreach`
// in `tests/test_lower.rs` covers what's honestly provable at this
// milestone (the lowering shape itself: `Stmt::ForEach`'s `var`/`iter`/
// `body` fields and scoping), not real execution.

// ── M3a: method declarations + calls ─────────────────────────────────────

#[test]
fn method_call_runs_in_python() {
    if !python_available() {
        eprintln!("skipping method_call_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "method_call",
        &wrap_with_methods(
            "static int add(int a, int b) { return a + b; }",
            "int r = add(3, 4); r;",
        ),
    );
    assert_eq!(out, "7");
}

#[test]
fn call_to_a_method_declared_after_main_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping call_to_a_method_declared_after_main_runs_in_python: `python3` not available"
        );
        return;
    }
    // `main`'s own call to `helper` resolves regardless of textual order
    // (pass 1 registers every method's signature before any body is
    // lowered) -- proven here by actually running it, not just checking
    // the lowered shape.
    let out = run_via_python(
        "forward_reference_call",
        &wrap_with_methods(
            "static int helper(int x) { return x * 2; }",
            "int r = helper(10); r;",
        ),
    );
    assert_eq!(out, "20");
}

#[test]
fn void_method_call_runs_without_crashing_in_python() {
    if !python_available() {
        eprintln!(
            "skipping void_method_call_runs_without_crashing_in_python: `python3` not available"
        );
        return;
    }
    // A void method call has no observable effect this milestone can
    // produce (no I/O, no shared mutable state between functions --
    // parameters are pass-by-value copies of primitives), so this proves
    // only what M3a can honestly prove: the call itself compiles and
    // executes cleanly (a Python function returning `None`, called and
    // discarded as a bare statement) without crashing, and the real
    // trailing value (`42`) still comes through unaffected.
    let out = run_via_python(
        "void_method_call",
        &wrap_with_methods("static void noop(int x) { }", "noop(5); 42;"),
    );
    assert_eq!(out, "42");
}

// No execution-proof test for recursion (plain or mutual): a genuinely
// *terminating* recursive call needs a base case, which needs branching
// (an `if`-guarded early `return`) -- out of scope for JV02 M3a (`return`
// is accepted only as the literal last top-level statement of a method
// body; see `lower.rs`'s own module doc comment). Without that, any
// recursive call this milestone can express would recurse forever if
// actually run. `plain_self_recursion_lowers_without_error_and_is_not_
// mutual_recursion`/`mutual_recursion_between_two_methods_sets_the_
// manifest_feature` in `tests/test_lower.rs` cover the structural lowering
// claim (the call resolves, `Feature::MutualRecursion` is set correctly)
// without ever actually running the loop.

// ── IndirectCall: invoking a lambda-valued local (task #54) ─────────────
//
// JV02 M3b lowered a lambda *literal* to `Expr::MakeClosure` (a real,
// structurally-verified closure value, with real capture-value threading
// -- see `tests/test_lower.rs`'s own `nested_lambda_captures_
// transitively_across_both_boundaries` test) but had no way to *invoke*
// the resulting value, so no execution-proof test was possible for it --
// a lambda-using program couldn't produce any observably different
// output than not using one at all. This task wires `Expr::IndirectCall`:
// a bare `NAME(args)` callee that resolves (via `resolve_name`) to a
// local/parameter holding a closure value, rather than a known top-level
// method name, now calls through that value instead -- unlocking real
// execution proofs for the first time.

#[test]
fn calling_a_lambda_valued_local_runs_in_python() {
    if !python_available() {
        eprintln!("skipping calling_a_lambda_valued_local_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "calling_a_lambda_valued_local",
        &wrap("var f = (int x) -> x + 1; f(5);"),
    );
    assert_eq!(out, "6");
}

#[test]
fn calling_a_multi_parameter_lambda_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping calling_a_multi_parameter_lambda_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "calling_a_multi_parameter_lambda",
        &wrap("var f = (int a, int b) -> a - b; f(10, 3);"),
    );
    assert_eq!(out, "7");
}

#[test]
fn calling_a_captured_lambda_from_within_a_nested_lambda_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping calling_a_captured_lambda_from_within_a_nested_lambda_runs_in_python: `python3` not available"
        );
        return;
    }
    // `g` captures `f` (itself a closure value) from its own enclosing
    // scope and invokes it -- exercises capture threading and indirect
    // invocation together, not just each in isolation.
    let out = run_via_python(
        "calling_a_captured_lambda",
        &wrap("var f = (int x) -> x + 1; var g = (int y) -> f(y) * 2; g(3);"),
    );
    assert_eq!(out, "8"); // (3 + 1) * 2
}

#[test]
fn lambda_used_as_a_repeated_callback_in_a_loop_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping lambda_used_as_a_repeated_callback_in_a_loop_runs_in_python: `python3` not available"
        );
        return;
    }
    // The realistic pattern lambda invocation exists to enable: the same
    // closure value called repeatedly across loop iterations, its
    // captured state (`n`, effectively final) read fresh each time.
    let out = run_via_python(
        "lambda_as_a_repeated_callback",
        &wrap(concat!(
            "int n = 10; ",
            "var addN = (int x) -> x + n; ",
            "int sum = 0; ",
            "for (int i = 0; i < 3; i++) { sum = sum + addN(i); } ",
            "sum;"
        )),
    );
    assert_eq!(out, "33"); // (0+10) + (1+10) + (2+10)
}

// No execution-proof test for calling a lambda-valued *method
// parameter*: this frontend has no way to declare a method parameter of
// a functional-interface type at all (`kind_of_type_node` only resolves
// primitive/`String` parameter types), so a `Kind::Closure`-typed
// parameter is not constructible in the first place -- not a gap in
// invocation itself, just a boundary of what can be expressed yet.

// ── M4a: array declarations, indexing reads, .length ─────────────────────

#[test]
fn array_literal_and_length_run_in_python() {
    if !python_available() {
        eprintln!("skipping array_literal_and_length_run_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "array_literal_and_length",
        &wrap("int[] xs = {10, 20, 30}; xs.length;"),
    );
    assert_eq!(out, "3");
}

#[test]
fn array_index_read_runs_in_python() {
    if !python_available() {
        eprintln!("skipping array_index_read_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python("array_index_read", &wrap("int[] xs = {10, 20, 30}; xs[1];"));
    assert_eq!(out, "20");
}

#[test]
fn array_sum_via_indexed_for_loop_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping array_sum_via_indexed_for_loop_runs_in_python: `python3` not available"
        );
        return;
    }
    // The realistic pattern this milestone exists to enable: `for (int i
    // = 0; i < xs.length; i++)` summing an array by index -- exercises
    // the array literal, `.length`, and indexed reads together, not just
    // each in isolation.
    let out = run_via_python(
        "array_sum_via_indexed_for_loop",
        &wrap(concat!(
            "int[] xs = {1, 2, 3, 4, 5}; ",
            "int sum = 0; ",
            "for (int i = 0; i < xs.length; i++) { sum = sum + xs[i]; } ",
            "sum;"
        )),
    );
    assert_eq!(out, "15");
}

#[test]
fn var_inferred_array_runs_in_python() {
    if !python_available() {
        eprintln!("skipping var_inferred_array_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python("var_inferred_array", &wrap("var xs = {7, 8, 9}; xs[2];"));
    assert_eq!(out, "9");
}

// ── M4b: indexed array assignment ─────────────────────────────────────

#[test]
fn indexed_assignment_runs_in_python() {
    if !python_available() {
        eprintln!("skipping indexed_assignment_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "indexed_assignment",
        &wrap("int[] xs = {1, 2, 3}; xs[1] = 99; xs[1];"),
    );
    assert_eq!(out, "99");
}

#[test]
fn indexed_assignment_with_a_variable_index_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping indexed_assignment_with_a_variable_index_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "indexed_assignment_variable_index",
        &wrap("int[] xs = {0, 0, 0}; int i = 2; xs[i] = 7; xs[2];"),
    );
    assert_eq!(out, "7");
}

#[test]
fn array_fill_via_indexed_for_loop_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping array_fill_via_indexed_for_loop_runs_in_python: `python3` not available"
        );
        return;
    }
    // Mirrors `array_sum_via_indexed_for_loop_runs_in_python`'s own
    // realistic pattern, but for writes: fill each element with its own
    // index doubled, then sum -- exercises `.length`, indexed reads, and
    // indexed *writes* together.
    let out = run_via_python(
        "array_fill_via_indexed_for_loop",
        &wrap(concat!(
            "int[] xs = {0, 0, 0, 0}; ",
            "for (int i = 0; i < xs.length; i++) { xs[i] = i * 2; } ",
            "int sum = 0; ",
            "for (int i = 0; i < xs.length; i++) { sum = sum + xs[i]; } ",
            "sum;"
        )),
    );
    assert_eq!(out, "12"); // 0 + 2 + 4 + 6
}

// ── task #59: compound-assignment/increment-decrement on an indexed
//    target ──────────────────────────────────────────────────────────
//
// These prove the emitted Python actually computes the right *value* --
// the finer "seq/index evaluated exactly once, not twice" property these
// statements exist to fix is proven structurally instead, in
// `tests/test_lower.rs` (asserting the lowered IR contains exactly one
// `LetStarBinding` per temp, both the read and the write referencing the
// same `VarRef`). An execution proof can't add much there: this
// frontend has no fields, no mutable lambda-capture (effectively-final
// is enforced), and no incdec-as-a-value, so there is currently no way
// to write real Java source whose index expression has an observable
// side effect distinguishing "evaluated once" from "evaluated twice" --
// and for a *pure* index expression, a buggy double-evaluating lowering
// would still often compute the same final numeric answer, making a
// value-based proof weaker here than the structural one anyway.

#[test]
fn compound_assignment_on_an_indexed_target_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping compound_assignment_on_an_indexed_target_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "indexed_compound_assignment",
        &wrap("int[] xs = {1, 2, 3}; xs[1] += 10; xs[1];"),
    );
    assert_eq!(out, "12");
}

#[test]
fn increment_of_an_indexed_target_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping increment_of_an_indexed_target_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "indexed_increment",
        &wrap("int[] xs = {1, 2, 3}; xs[0]++; xs[0]++; xs[0];"),
    );
    assert_eq!(out, "3");
}

#[test]
fn array_accumulation_via_indexed_compound_assignment_runs_in_python() {
    // Mirrors `array_fill_via_indexed_for_loop_runs_in_python`'s own
    // realistic pattern, but for `+=`: accumulate each element's own
    // index into a running histogram-style array, then sum -- the
    // pattern this task exists to make idiomatic (`xs[i] += v;` instead
    // of the M4b-only `xs[i] = xs[i] + v;` workaround).
    if !python_available() {
        eprintln!(
            "skipping array_accumulation_via_indexed_compound_assignment_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "array_accumulation_via_indexed_compound_assignment",
        &wrap(concat!(
            "int[] xs = {0, 0, 0, 0}; ",
            "for (int i = 0; i < xs.length; i++) { xs[i] += i + 1; } ",
            "int sum = 0; ",
            "for (int i = 0; i < xs.length; i++) { sum += xs[i]; } ",
            "sum;"
        )),
    );
    assert_eq!(out, "10"); // (1) + (2) + (3) + (4)
}

// ── M4c: new-based array-creation expressions ─────────────────────────

#[test]
fn new_sized_array_allocate_then_fill_by_index_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping new_sized_array_allocate_then_fill_by_index_runs_in_python: `python3` not available"
        );
        return;
    }
    // The realistic pattern M4b (indexed assignment) and M4c (sized
    // array creation) together exist to enable: allocate a zero-filled
    // array, then fill it by index, then sum -- exercising `new int[N]`,
    // indexed writes, indexed reads, and `.length` all together.
    let out = run_via_python(
        "new_sized_array_fill_by_index",
        &wrap(concat!(
            "int[] xs = new int[5]; ",
            "for (int i = 0; i < xs.length; i++) { xs[i] = i + 1; } ",
            "int sum = 0; ",
            "for (int i = 0; i < xs.length; i++) { sum = sum + xs[i]; } ",
            "sum;"
        )),
    );
    assert_eq!(out, "15"); // 1 + 2 + 3 + 4 + 5
}

#[test]
fn new_array_with_initializer_runs_in_python() {
    if !python_available() {
        eprintln!("skipping new_array_with_initializer_runs_in_python: `python3` not available");
        return;
    }
    let out = run_via_python(
        "new_array_with_initializer",
        &wrap("int[] xs = new int[]{10, 20, 30}; xs[1];"),
    );
    assert_eq!(out, "20");
}

// ── M4d: multi-dimensional arrays ──────────────────────────────────────

#[test]
fn two_dimensional_array_literal_and_chained_index_read_run_in_python() {
    if !python_available() {
        eprintln!(
            "skipping two_dimensional_array_literal_and_chained_index_read_run_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "two_dimensional_chained_index",
        &wrap("int[][] grid = {{1, 2, 3}, {4, 5, 6}}; grid[1][2];"),
    );
    assert_eq!(out, "6");
}

#[test]
fn nested_indexed_for_loop_sums_a_two_dimensional_array_in_python() {
    if !python_available() {
        eprintln!(
            "skipping nested_indexed_for_loop_sums_a_two_dimensional_array_in_python: `python3` not available"
        );
        return;
    }
    // The realistic pattern M4d exists to enable: a nested `for` loop
    // walking both dimensions by index, exercising `.length` on both the
    // outer array and each inner row (via an intermediate `row` local --
    // `grid[i].length` itself, a mixed index-then-dot suffix chain, was
    // deferred at M4d's own time, later resolved as task #60 -- see the
    // dedicated section below), plus chained indexed reads.
    let out = run_via_python(
        "two_dimensional_array_nested_sum",
        &wrap(concat!(
            "int[][] grid = {{1, 2}, {3, 4}, {5, 6}}; ",
            "int sum = 0; ",
            "for (int i = 0; i < grid.length; i++) { ",
            "  int[] row = grid[i]; ",
            "  for (int j = 0; j < row.length; j++) { ",
            "    sum = sum + row[j]; ",
            "  } ",
            "} ",
            "sum;"
        )),
    );
    assert_eq!(out, "21"); // 1+2+3+4+5+6
}

#[test]
fn ragged_two_dimensional_array_runs_in_python() {
    if !python_available() {
        eprintln!("skipping ragged_two_dimensional_array_runs_in_python: `python3` not available");
        return;
    }
    // Java arrays are genuinely ragged -- each row is its own
    // independent array, so rows of differing length are legal and must
    // actually run correctly, not just structurally lower. Uses
    // intermediate `row0`/`row1` locals for the same reason the nested-
    // loop test above does (`grid[0].length` itself is deferred).
    let out = run_via_python(
        "ragged_two_dimensional_array",
        &wrap(concat!(
            "int[][] grid = {{1, 2, 3}, {4}}; ",
            "int[] row0 = grid[0]; ",
            "int[] row1 = grid[1]; ",
            "row0.length + row1.length;"
        )),
    );
    assert_eq!(out, "4"); // 3 + 1
}

// No execution-proof test for a `var`-inferred multi-dimensional array
// literal or indexed assignment on a chained (multi-dimensional) target
// -- both remain deferred past M4d (see the corresponding rejection
// tests in `tests/test_lower.rs`, and the follow-up tasks logged when
// M4c/M4d were scoped down from their own original bundling with those
// items). Compound-assignment/increment-decrement on an indexed target
// (task #59) and a mixed index-then-`.length` chain (task #60), also
// scoped down from that same M4c/M4d bundling, are resolved -- see the
// dedicated sections above/below.

// ── task #60: mixed index/dot primary-suffix chains ───────────────────

#[test]
fn mixed_index_then_dot_length_chain_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping mixed_index_then_dot_length_chain_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "mixed_index_then_dot_length",
        &wrap("int[][] grid = {{1, 2, 3}, {4, 5}}; grid[0].length;"),
    );
    assert_eq!(out, "3");
}

#[test]
fn nested_indexed_for_loop_using_mixed_index_dot_length_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping nested_indexed_for_loop_using_mixed_index_dot_length_runs_in_python: `python3` not available"
        );
        return;
    }
    // The realistic pattern task #60 exists to make idiomatic: the same
    // sum `nested_indexed_for_loop_sums_a_two_dimensional_array_in_
    // python` computes, but reading each row's own length directly off
    // the indexed chain (`grid[i].length`) instead of through an
    // intermediate `row` local.
    let out = run_via_python(
        "two_dimensional_array_nested_sum_via_mixed_chain",
        &wrap(concat!(
            "int[][] grid = {{1, 2}, {3, 4}, {5, 6}}; ",
            "int sum = 0; ",
            "for (int i = 0; i < grid.length; i++) { ",
            "  for (int j = 0; j < grid[i].length; j++) { ",
            "    sum = sum + grid[i][j]; ",
            "  } ",
            "} ",
            "sum;"
        )),
    );
    assert_eq!(out, "21"); // 1+2+3+4+5+6
}

#[test]
fn chained_index_then_dot_length_on_a_three_dimensional_array_runs_in_python() {
    if !python_available() {
        eprintln!(
            "skipping chained_index_then_dot_length_on_a_three_dimensional_array_runs_in_python: `python3` not available"
        );
        return;
    }
    let out = run_via_python(
        "three_dimensional_mixed_index_then_dot_length",
        &wrap("int[][][] cube = {{{1, 2}}, {{3}}}; cube[0][0].length;"),
    );
    assert_eq!(out, "2");
}
