//! Oracle/golden tests (HML01 §7): the SAME IDL computation, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `idl-runtime` (`coding-adventures-idl-runtime`) -- this frontend's
//!       own sibling crate: a tree-walking interpreter over `array-runtime`
//!       (MA-12d) -- the ground truth.
//!   (b) `idl_to_semantic_ir::compile_source` -> `semantic_ir::Module` ->
//!       `semantic_ir_to_javascript::compile` -> an actual `node` process.
//!
//! Structurally this file follows `scilab-to-semantic-ir/tests/oracle.rs`'s
//! own `setup`/`final_expr`/`expected`/`known_bug` `Case` shape (the
//! nearest array-family precedent per this task's own brief), adapted to
//! `idl-runtime`'s own ground-truth mechanics.
//!
//! ## Why `setup` + `final_expr`, not one `source` string
//!
//! 1. **Assignment is silent; a bare expression auto-prints (Implied
//!    Print)** -- confirmed directly against `idl_runtime::eval::
//!    Interpreter::run`'s own doc comment and this crate's own `lower.rs`
//!    module doc: `PRINT` is the only observable-output primitive on the
//!    compiled side (there is no "implicit display" representation in
//!    `semantic_ir` at all, the identical situation every sibling oracle
//!    file documents).
//! 2. So each [`Case`] stores a `setup` (ordinary IDL statements, run
//!    identically on both sides) plus a `final_expr` (a bare expression,
//!    no terminator). [`ground_truth`] appends `final_expr` bare
//!    (`idl-runtime`'s own Implied-Print convention); [`compiled`] wraps it
//!    in `PRINT, (...)` (this frontend's only supported output path,
//!    restricted to exactly one argument -- `lower.rs`'s own documented
//!    `disp`-style scope cut).
//!
//! ## Normalization
//!
//! Identical rationale, and identical implementation, to every sibling
//! oracle file's own `normalize`: `idl-runtime`'s comparisons are
//! `array_runtime` numeric `0.0`/`1.0` values, but the JS backend's shared
//! `=`/`!=`/`<`/`<=`/`>`/`>=` builtins produce real JS booleans, formatted
//! Scheme-style (`#t`/`#f`) by `format()`. [`normalize`] maps those to
//! `"1"`/`"0"`, nothing else.
//!
//! ## `#`/`##`: acknowledged as untestable end-to-end for a NON-commutative
//! case in THIS oracle file
//!
//! This frontend has no in-scope way to construct a genuine rank-2 array
//! (no `FLTARR`/`INTARR`, and IDL's own array-literal grammar has no 2-D
//! row-separator syntax at all -- see `lower.rs`'s own "Builtins" section).
//! So every `#`/`##` case here necessarily uses single-element (rank-1)
//! operands, for which `matmul(A, B)` and `matmul(B, A)` are numerically
//! IDENTICAL (scalar multiplication commutes) -- this oracle file can
//! confirm the VALUE agrees, but (unlike `tests/test_lower.rs`'s own
//! `hash_is_matmul_with_operands_swapped`, which asserts the actual
//! `MatMul { lhs, rhs }` AST shape directly) it cannot independently prove
//! the operand-order fix through end-to-end VALUES alone. Noted here so a
//! future reader does not mistake "both sides agree" for "the swap was
//! exercised" -- the structural test is the load-bearing proof for that
//! specific decision, not this file.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_idl_runtime::eval as idl_eval;
use coding_adventures_idl_to_semantic_ir::compile_source;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. See this file's module doc comment for the
/// `setup`/`final_expr` split. `known_bug`: `None` means both sides must
/// equal `expected` exactly; `Some(reason)` means only the ground-truth
/// side is asserted (mirrors `maple-to-semantic-ir`'s/`scilab-to-semantic-ir`'s
/// own `known_bug` convention).
struct Case {
    name: &'static str,
    setup: &'static str,
    final_expr: &'static str,
    expected: &'static str,
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Plain literal arithmetic / precedence -----------------------------
    Case {
        name: "literal_arithmetic_precedence",
        setup: "x = 3 + 4 * 2 - 5\n",
        final_expr: "x",
        expected: "6",
        known_bug: None,
    },
    Case {
        name: "nested_parens_override_precedence",
        setup: "",
        final_expr: "(2 + 3) * (4 - 1)",
        expected: "15",
        known_bug: None,
    },
    Case {
        name: "unary_minus_binds_looser_than_multiplicative",
        // -a*b == -(a*b) -- IDL's own documented tier-5 unary placement
        // (confirmed against idl-parser's own precedence table), the
        // OPPOSITE of Scilab's/MATLAB's tighter-than-`*` unary.
        setup: "a = 2\nb = 3\n",
        final_expr: "-a*b",
        expected: "-6",
        known_bug: None,
    },
    Case {
        name: "power_is_left_associative",
        // 2^3^2 == (2^3)^2 == 64, NOT 2^(3^2) == 512.
        setup: "",
        final_expr: "2^3^2",
        expected: "64",
        known_bug: None,
    },
    Case {
        name: "reassignment_takes_the_later_value",
        setup: "x = 1\nx = 2\n",
        final_expr: "x",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "scalar_variable_self_addition_computes_correctly",
        setup: "x = 5\n",
        final_expr: "x + x",
        expected: "10",
        known_bug: None,
    },
    // --- Word comparisons: fold to a real JS boolean compiled-side, a
    // 0.0/1.0 numeric ground-truth-side (see module doc's Normalization). ---
    Case {
        name: "eq_comparison_is_true",
        setup: "",
        final_expr: "5 EQ 5",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "ne_comparison_is_true",
        setup: "",
        final_expr: "5 NE 3",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "lt_comparison_is_false",
        setup: "",
        final_expr: "5 LT 3",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "ge_comparison_is_true",
        setup: "",
        final_expr: "5 GE 5",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "string_equality_is_true",
        setup: "",
        final_expr: "'ab' EQ 'ab'",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "string_equality_is_false",
        setup: "",
        final_expr: "'ab' EQ 'cd'",
        expected: "0",
        known_bug: None,
    },
    // --- Array literals / 0-based subscripting ------------------------------
    Case {
        name: "plain_index_read_is_zero_based",
        // IDL is 0-based already; a[1] is the SECOND element.
        setup: "a = [10, 20, 30]\n",
        final_expr: "a[1]",
        expected: "20",
        known_bug: None,
    },
    Case {
        name: "inclusive_range_subscript_sum",
        // a[1:3] is [20, 30, 40] (inclusive of BOTH endpoints) --
        // TOTAL/summing isn't in this frontend's scope, so cross-check via
        // one element of the range instead (a[1:3] itself isn't a scalar
        // PRINT-able single value on this frontend either -- read one
        // element of the sliced-out range back through a second subscript
        // read on the ground-truth side isn't directly expressible either,
        // so this case instead confirms the FIRST element of the range).
        setup: "a = [10, 20, 30, 40, 50]\nb = a[1:3]\n",
        final_expr: "b[0]",
        expected: "20",
        known_bug: None,
    },
    Case {
        name: "wildcard_subscript_first_element",
        setup: "a = [7, 8, 9]\nb = a[*]\n",
        final_expr: "b[0]",
        expected: "7",
        known_bug: None,
    },
    Case {
        name: "indexed_assignment_then_read_back",
        setup: "a = [1, 2, 3]\na[1] = 99\n",
        final_expr: "a[1]",
        expected: "99",
        known_bug: None,
    },
    // --- `#`/`##` matrix product (rank-1, commutative -- see module doc) --
    Case {
        name: "hash_hash_ordinary_matmul",
        setup: "a = [2]\nb = [3]\nc = a ## b\n",
        final_expr: "c[0]",
        expected: "6",
        known_bug: None,
    },
    Case {
        name: "hash_reversed_matmul_agrees_for_commutative_scalars",
        setup: "a = [2]\nb = [3]\nc = a # b\n",
        final_expr: "c[0]",
        expected: "6",
        known_bug: None,
    },
    // --- Control flow: if/else, while, for, repeat --------------------------
    Case {
        name: "if_then_else_takes_the_true_branch",
        setup: "x = 5\nIF x GT 0 THEN y = 1 ELSE y = 2\n",
        final_expr: "y",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "if_then_else_takes_the_false_branch",
        setup: "x = -5\nIF x GT 0 THEN y = 1 ELSE y = 2\n",
        final_expr: "y",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "if_else_first_assigning_a_name_makes_it_visible_afterward",
        setup: "x = 0\nIF x GT 0 THEN BEGIN\n y = 1\nENDIF ELSE BEGIN\n y = 2\nENDELSE\n",
        final_expr: "y",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "for_loop_accumulator",
        setup: "total = 0\nFOR i = 1, 5 DO total = total + i\n",
        final_expr: "total",
        expected: "15",
        known_bug: None,
    },
    Case {
        name: "for_loop_with_a_literal_step",
        setup: "total = 0\nFOR i = 0, 10, 2 DO total = total + i\n",
        final_expr: "total",
        expected: "30",
        known_bug: None,
    },
    Case {
        name: "while_loop_accumulator",
        setup: "x = 0\nWHILE x LT 5 DO x = x + 1\n",
        final_expr: "x",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "repeat_until_runs_the_body_at_least_once",
        setup: "x = 0\nREPEAT x = x + 1 UNTIL x GE 3\n",
        final_expr: "x",
        expected: "3",
        known_bug: None,
    },
    // --- PRO/FUNCTION: calls, keyword arguments, two namespaces -------------
    Case {
        name: "function_with_positional_args",
        setup: "FUNCTION square, x\n RETURN, x * x\nEND\n",
        final_expr: "square(5)",
        expected: "25",
        known_bug: None,
    },
    Case {
        name: "keyword_argument_binds_to_a_differently_spelled_local",
        setup: "FUNCTION plot_it, x, COLOR=hue\n RETURN, x + hue\nEND\n",
        final_expr: "plot_it(1, COLOR=10)",
        expected: "11",
        known_bug: None,
    },
    Case {
        name: "boolean_keyword_shorthand_equals_keyword_equals_one",
        setup: "FUNCTION check, YLOG=ylog\n RETURN, ylog\nEND\n",
        final_expr: "check(/YLOG)",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "same_name_pro_reaches_the_procedure_namespace",
        setup: "PRO DOIT, x\n PRINT, x * 2\nEND\nFUNCTION DOIT, x\n RETURN, x * 3\nEND\n",
        final_expr: "DOIT(5)",
        expected: "15",
        known_bug: None,
    },
    // --- TRANSPOSE / INDGEN -------------------------------------------------
    Case {
        name: "transpose_of_a_vector_preserves_element_values",
        setup: "a = [1, 2, 3]\nb = TRANSPOSE(a)\n",
        final_expr: "b[0]",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "indgen_first_and_last_elements",
        setup: "a = INDGEN(5)\n",
        final_expr: "a[4]",
        expected: "4",
        known_bug: None,
    },
    // --- Case folding --------------------------------------------------------
    Case {
        name: "case_folded_identifier_round_trips",
        setup: "MyVar = 7\n",
        final_expr: "MYVAR",
        expected: "7",
        known_bug: None,
    },
];

/// Ground truth: run `setup` followed by a bare `final_expr` (Implied
/// Print) through `idl-runtime`, and take the LAST accumulated output line
/// -- `setup` itself never prints anything as long as it consists purely
/// of assignments (this crate's own oracle corpus discipline), so the
/// accumulated output is exactly `final_expr`'s own printed value.
fn ground_truth(setup: &str, final_expr: &str) -> String {
    let src = format!("{setup}{final_expr}\n");
    let out = idl_eval(&src).unwrap_or_else(|e| panic!("idl-runtime eval failed for {src:?}: {e}"));
    out.trim().to_string()
}

/// Compiled path: run `setup` followed by `PRINT, final_expr` -- this
/// frontend's only supported output path -- through
/// `idl_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// `semantic_ir_to_javascript::compile`, and an actual `node` process.
fn compiled(name: &str, setup: &str, final_expr: &str) -> String {
    let src = format!("{setup}PRINT, {final_expr}\n");
    let module = compile_source(&src, "prog")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({src:?}): {e:?}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact = semantic_ir_to_javascript::compile(&module)
        .unwrap_or_else(|e| panic!("backend emit failed for {name}: {e:?}"));

    let mut path = std::env::temp_dir();
    path.push(format!("idl_sir_oracle_{name}_{}.js", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    drop(file);

    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Normalize display-*spelling*-only differences -- see the module doc's
/// "Normalization" section. Never used to paper over a genuine value
/// mismatch.
fn normalize(s: &str) -> String {
    match s {
        "#t" | "true" => "1".to_string(),
        "#f" | "false" => "0".to_string(),
        other => other.to_string(),
    }
}

#[test]
fn oracle_corpus_matches_native_idl_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_idl_runtime: `node` not available");
        return;
    }

    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = normalize(&ground_truth(case.setup, case.final_expr));
        if gt != case.expected {
            failures.push(format!(
                "{}: idl-runtime itself disagrees with this corpus entry's own `expected` (got \
                 {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the corpus \
                 rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        match case.known_bug {
            None => {
                let got = normalize(&compiled(case.name, case.setup, case.final_expr));
                if got != case.expected {
                    failures.push(format!(
                        "{}: idl-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees \
                         with the idl-runtime ground truth (got {got:?}, expected {:?})",
                        case.name, case.expected
                    ));
                }
            }
            Some(reason) => {
                eprintln!(
                    "{}: skipping compiled-side assertion (KNOWN BUG, not fixed in this PR): {reason}",
                    case.name
                );
            }
        }
    }

    assert!(
        failures.is_empty(),
        "oracle corpus mismatches ({} of {}):\n{}",
        failures.len(),
        CORPUS.len(),
        failures.join("\n")
    );
}
