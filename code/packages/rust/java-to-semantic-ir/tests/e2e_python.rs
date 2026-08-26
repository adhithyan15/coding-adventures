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

// No execution-proof test for JV02 M3b's lambda expressions either, for
// an analogous reason: this milestone lowers a lambda *literal* to
// `Expr::MakeClosure` (a real, structurally-verified closure value, with
// real capture-value threading -- see `tests/test_lower.rs`'s own
// `nested_lambda_captures_transitively_across_both_boundaries` test), but
// has no way to *invoke* the resulting value -- that needs `Expr::
// IndirectCall`, wiring "a bare NAME that resolves to a local holding a
// closure, not a known method name, calls through the value instead,"
// which this milestone does not add (a real, disclosed gap, not an
// oversight -- M3a's own `lower_call_expression` only recognizes callee
// names present in `method_signatures`). Without invocation, there is
// nothing a Java program using a lambda this milestone can lower could
// possibly do that produces *observably different* output than not using
// one at all, so an execution-proof harness has nothing meaningful to
// assert on. Every positive lambda test in `tests/test_lower.rs` still
// asserts the lowered `Module` passes `semantic_ir::validate()`, which is
// the honest ceiling of what's provable here.

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

// No execution-proof test for the `new int[5]`/`new int[]{...}` array-
// creation-expression forms, multi-dimensional arrays, or compound-
// assignment/increment-decrement on an indexed target -- all remain
// deferred past M4b (see the corresponding rejection tests in
// `tests/test_lower.rs`, and the follow-up tasks logged when M4b was
// scoped down to plain indexed assignment only).
