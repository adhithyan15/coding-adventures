//! Oracle/golden test (wolfram-iir-vm.md / macsyma-iir-vm.md §7): the SAME
//! Wolfram source, run through **two independent implementations**, and
//! diffed:
//!
//!   (a) `wolfram-runtime` (`coding-adventures-wolfram-runtime`) — the
//!       native runtime crate, which lowers to `symbolic-ir` and evaluates
//!       via `symbolic-vm`'s shared handler table — the ground truth.
//!   (b) `wolfram_iir_compiler::compile_source` → `interpreter_ir::IIRModule`
//!       → `wolfram_vm::run` → a `LispyValue`, read back into a
//!       `symbolic_ir::IRNode` and rendered through the SAME
//!       `coding_adventures_wolfram_runtime::print_wolfram` function the
//!       native runtime uses.
//!
//! Mirrors `derive-iir-compiler/tests/oracle.rs`'s/`maple-iir-compiler/
//! tests/oracle.rs`'s shape exactly, only diffing the LAST statement's
//! value.
//!
//! ## Corpus
//!
//! v0's accepted grammar subset only (`wolfram-iir-vm.md`) — a
//! deliberately narrower slice than the FULL grammar
//! `wolfram-to-semantic-ir` covers: literal integer arithmetic (`+ - *
//! /`, unary `-`/`+` — Wolfram, unlike Derive/Reduce/Maple/Axiom, has a
//! real unary-plus), assignment (`=`) and re-assignment threading, and
//! unevaluated symbolic `Apply` results for any operand that stays free.
//! Every rejected construct (including Wolfram's own genuinely rich
//! surface — patterns, rules, replacement, pure functions) already has a
//! dedicated `Err`-asserting unit test in `src/lib.rs`'s own `tests`
//! module — not re-tested here.

use coding_adventures_wolfram_runtime::{print_wolfram, WolframSession};
use dynval_runtime::{builtins, name_of, LispyValue};
use symbolic_ir::{apply, int, sym, IRNode};
use wolfram_iir_compiler::compile_source;

/// One oracle corpus entry.
struct Case {
    name: &'static str,
    /// The WHOLE program, byte-for-byte identical on both the
    /// [`ground_truth`] and [`compiled`] sides.
    source: &'static str,
    expected: &'static str,
}

const CORPUS: &[Case] = &[
    // --- Literal arithmetic: precedence, chains, unary, grouping. ---
    Case {
        name: "integer_literal",
        source: "42\n",
        expected: "42",
    },
    Case {
        name: "simple_addition",
        source: "2 + 3\n",
        expected: "5",
    },
    Case {
        name: "precedence",
        source: "2 + 3 * 4\n",
        expected: "14",
    },
    Case {
        name: "left_associative_chain",
        source: "1 + 2 + 3 + 4\n",
        expected: "10",
    },
    Case {
        name: "unary_minus_leaf",
        source: "-5 + 3\n",
        expected: "-2",
    },
    Case {
        name: "unary_minus_compound",
        source: "-(5 + 3)\n",
        expected: "-8",
    },
    Case {
        name: "unary_plus_is_noop",
        source: "+5\n",
        expected: "5",
    },
    Case {
        name: "grouping_overrides_precedence",
        source: "(2 + 3) * 4\n",
        expected: "20",
    },
    Case {
        name: "exact_integer_division",
        source: "20 / 4\n",
        expected: "5",
    },
    Case {
        name: "negative_literal_exact_division",
        source: "-4 / 2\n",
        expected: "-2",
    },
    // --- Assignment: binding, reference, re-assignment threading, and
    // multiple independent variables in one program. ---
    Case {
        name: "assignment_and_reference",
        source: "x = 3\nx + 1\n",
        expected: "4",
    },
    Case {
        name: "reassignment_threading",
        source: "x = 3\nx = x + 1\nx\n",
        expected: "4",
    },
    Case {
        name: "two_independent_variables",
        source: "a = 2\nb = 3\na * b\n",
        expected: "6",
    },
    // --- Unevaluated symbolic expressions: a free symbol alone, and every
    // arithmetic head with a symbolic operand (Add/Sub/Mul/Div/Neg via
    // `inert_apply`). ---
    Case {
        name: "free_symbol_alone",
        source: "x\n",
        expected: "x",
    },
    Case {
        name: "free_symbol_addition",
        source: "x + y\n",
        expected: "x + y",
    },
    Case {
        name: "mixed_concrete_and_symbolic_multiplication",
        source: "2 * x\n",
        expected: "2*x",
    },
    Case {
        name: "symbolic_subtraction",
        source: "x - y\n",
        expected: "x - y",
    },
    Case {
        name: "symbolic_division_stays_unevaluated",
        source: "x / y\n",
        expected: "x/y",
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x\n",
        expected: "-x",
    },
    Case {
        name: "chained_symbolic_addition_nests_left_associatively",
        source: "x + y + z\n",
        expected: "x + y + z",
    },
    Case {
        name: "assigned_value_used_inside_a_symbolic_expression",
        source: "x = 2\nx + y\n",
        expected: "2 + y",
    },
];

/// Ground truth: run `source` through `wolfram-runtime`'s own
/// [`WolframSession::eval_to_outputs`], taking only the LAST statement's
/// `Output::text`.
fn ground_truth(source: &str) -> String {
    let mut session = WolframSession::new();
    let outputs = session
        .eval_to_outputs(source)
        .unwrap_or_else(|e| panic!("wolfram-runtime eval failed for {source:?}: {e}"));
    outputs
        .into_iter()
        .last()
        .expect("at least one statement")
        .text
}

/// Compiled path: `compile_source` → `wolfram_vm::run` → [`read_back`] →
/// the same `print_wolfram` function the native runtime uses.
fn compiled(name: &str, source: &str) -> String {
    let module = compile_source(source, "oracle")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({source:?}): {e}"));
    let value = wolfram_vm::run(&module)
        .unwrap_or_else(|e| panic!("VM execution failed for {name} ({source:?}): {e}"));
    let node = read_back(value);
    print_wolfram(&node)
}

/// Rebuild the [`symbolic_ir::IRNode`] a [`LispyValue`] represents — the
/// mirror image of `wolfram_iir_compiler`'s own `inert_apply`/`emit_int`/
/// `emit_symbol`. Plain recursion is appropriate here for the same
/// reason `macsyma-iir-compiler/tests/oracle.rs`'s own `read_back`
/// module doc gives: test-only code over a small, hand-authored corpus.
fn read_back(v: LispyValue) -> IRNode {
    if let Some(n) = v.as_int() {
        return int(n);
    }
    if let Some(s) = v.as_symbol() {
        let name = name_of(s).expect("interned symbol has a name");
        return sym(name);
    }
    // Otherwise `v` must be the inert-Apply cons shape: (head . arglist).
    let head_value = builtins::car(&[v]).unwrap_or_else(|e| panic!("car of apply pair: {e}"));
    let head_name = match read_back(head_value) {
        IRNode::Symbol(name) => name,
        other => panic!("expected a symbol head, got {other:?}"),
    };
    let mut rest = builtins::cdr(&[v]).unwrap_or_else(|e| panic!("cdr of apply pair: {e}"));
    let mut args = Vec::new();
    while !rest.is_nil() {
        let arg = builtins::car(&[rest]).unwrap_or_else(|e| panic!("car of arg list: {e}"));
        args.push(read_back(arg));
        rest = builtins::cdr(&[rest]).unwrap_or_else(|e| panic!("cdr of arg list: {e}"));
    }
    apply(sym(head_name), args)
}

#[test]
fn oracle_corpus_matches_native_wolfram_runtime() {
    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = ground_truth(case.source);
        if gt != case.expected {
            failures.push(format!(
                "{}: wolfram-runtime itself disagrees with this corpus entry's own `expected` \
                 (got {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the \
                 corpus rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        let got = compiled(case.name, case.source);
        if got != case.expected {
            failures.push(format!(
                "{}: wolfram-iir-compiler -> wolfram-vm disagrees with the wolfram-runtime \
                 ground truth (got {got:?}, expected {:?})",
                case.name, case.expected
            ));
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
