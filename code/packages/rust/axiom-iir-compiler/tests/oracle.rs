//! Oracle/golden test (axiom-iir-vm.md / macsyma-iir-vm.md §7): the SAME
//! Axiom source, run through **two independent implementations**, and
//! diffed:
//!
//!   (a) `axiom-runtime` (`coding-adventures-axiom-runtime`) — the native
//!       runtime crate — the ground truth.
//!   (b) `axiom_iir_compiler::compile_source` → `interpreter_ir::IIRModule`
//!       → `axiom_vm::run` → a `LispyValue`, read back into a
//!       `symbolic_ir::IRNode` and rendered through the SAME
//!       `coding_adventures_axiom_runtime::print_axiom` function the
//!       native runtime uses.
//!
//! ## Single-expression corpus — unlike every sibling oracle file
//!
//! `axiom.grammar`'s own `program = expr` (see `lower.rs`'s module doc
//! comment) means every corpus entry here is exactly ONE expression, not
//! a multi-statement program — there is no `x := 3\nx + 1`-style
//! assignment-then-reference case the sibling oracle files have, since
//! Axiom's grammar has no way to express that as one `compile_source`
//! call.
//!
//! `axiom-runtime::AxiomSession::eval_to_output` returns a single
//! [`Output`] (not `Vec<Output>`, unlike every sibling runtime), whose
//! `text` is `format_value`'s output — `print_axiom(&value.node)`, with a
//! `" : <Domain>"` suffix appended only when the evaluated value carries a
//! known domain (via a `:` declaration — out of v0's scope entirely, so
//! never triggered by anything this corpus can produce).
//!
//! ## Corpus
//!
//! v0's accepted grammar subset only (`axiom-iir-vm.md`): literal integer
//! arithmetic (`+ - * /`, unary `-` only), a bare `x := e` assignment
//! (returning `e`'s value), and unevaluated symbolic `Apply` results for
//! any operand that stays free. Every rejected construct already has a
//! dedicated `Err`-asserting unit test in `src/lib.rs`'s own `tests`
//! module — not re-tested here.
//!
//! ## A genuine finding, not assumed: `axiom-runtime` domain-tags EVERY
//! result, not just declared ones
//!
//! Confirmed empirically (this file's own first draft assumed otherwise
//! and every case failed): `format_value` (`axiom-runtime/src/lib.rs`)
//! appends `" : <Domain>"` to `print_axiom`'s own output whenever
//! `AxiomValue::domain` is `Some(_)` — and Axiom's evaluator infers a
//! domain for essentially every value, not only ones that passed through
//! an explicit `:` declaration (`42` → `"42 : PositiveInteger"`, `x + y`
//! → `"x + y : Polynomial(Integer)"`). Domain inference
//! (`axiom-runtime::domains`) is an entire fixed-table system this v0
//! slice does not implement at all (declarations/`::`/`has` are all
//! rejected outright — see `lower.rs`'s module doc comment), so the
//! compiled side's `read_back`/`print_axiom` pipeline has no domain
//! concept whatsoever and cannot reproduce this suffix. [`ground_truth`]
//! below strips it before comparing — a disclosed methodology choice
//! (compare VALUES only, not domain tags), not an accidental match.

/// Strip axiom-runtime's `" : <Domain>"` suffix (see the module doc
/// comment) from a ground-truth string, if present. Domain names never
/// contain `" : "` themselves (confirmed against `axiom-runtime::
/// domains`' fixed table — every entry is a single `NAME`-shaped
/// identifier, optionally with `(...)`-nested type arguments, never a
/// colon), so a single `rsplit_once(" : ")`-free `find`/truncate is exact.
fn strip_domain_suffix(text: &str) -> &str {
    match text.find(" : ") {
        Some(idx) => &text[..idx],
        None => text,
    }
}

use axiom_iir_compiler::compile_source;
use coding_adventures_axiom_runtime::{print_axiom, AxiomSession};
use dynval_runtime::{builtins, name_of, LispyValue};
use symbolic_ir::{apply, int, sym, IRNode};

/// One oracle corpus entry.
struct Case {
    name: &'static str,
    /// The WHOLE program — exactly one expression, byte-for-byte
    /// identical on both the [`ground_truth`] and [`compiled`] sides.
    source: &'static str,
    expected: &'static str,
}

const CORPUS: &[Case] = &[
    // --- Literal arithmetic: precedence, chains, unary, grouping. ---
    Case {
        name: "integer_literal",
        source: "42",
        expected: "42",
    },
    Case {
        name: "simple_addition",
        source: "2 + 3",
        expected: "5",
    },
    Case {
        name: "precedence",
        source: "2 + 3 * 4",
        expected: "14",
    },
    Case {
        name: "left_associative_chain",
        source: "1 + 2 + 3 + 4",
        expected: "10",
    },
    Case {
        name: "unary_minus_leaf",
        source: "-5 + 3",
        expected: "-2",
    },
    Case {
        name: "unary_minus_compound",
        source: "-(5 + 3)",
        expected: "-8",
    },
    Case {
        name: "grouping_overrides_precedence",
        source: "(2 + 3) * 4",
        expected: "20",
    },
    Case {
        name: "exact_integer_division",
        source: "20 / 4",
        expected: "5",
    },
    Case {
        name: "negative_literal_exact_division",
        source: "-4 / 2",
        expected: "-2",
    },
    // --- Assignment: returns the assigned value (no reference case is
    // expressible -- see the module doc comment). ---
    Case {
        name: "assignment_returns_the_assigned_value",
        source: "x := 3",
        expected: "3",
    },
    Case {
        name: "assignment_with_an_arithmetic_rhs",
        source: "x := 2 + 3",
        expected: "5",
    },
    // --- Unevaluated symbolic expressions: a free symbol alone, and every
    // arithmetic head with a symbolic operand (Add/Sub/Mul/Div/Neg via
    // `inert_apply`). ---
    Case {
        name: "free_symbol_alone",
        source: "x",
        expected: "x",
    },
    Case {
        name: "free_symbol_addition",
        source: "x + y",
        expected: "x + y",
    },
    Case {
        name: "mixed_concrete_and_symbolic_multiplication",
        source: "2 * x",
        expected: "2*x",
    },
    Case {
        name: "symbolic_subtraction",
        source: "x - y",
        expected: "x - y",
    },
    Case {
        name: "symbolic_division_stays_unevaluated",
        source: "x / y",
        expected: "x/y",
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x",
        expected: "-x",
    },
    Case {
        name: "chained_symbolic_addition_nests_left_associatively",
        source: "x + y + z",
        expected: "x + y + z",
    },
];

/// Ground truth: run `source` through `axiom-runtime`'s own
/// [`AxiomSession::eval_to_output`] (a single [`Output`], not `Vec` —
/// see the module doc comment).
fn ground_truth(source: &str) -> String {
    let mut session = AxiomSession::new();
    let text = session
        .eval_to_output(source)
        .unwrap_or_else(|e| panic!("axiom-runtime eval failed for {source:?}: {e}"))
        .text;
    strip_domain_suffix(&text).to_string()
}

/// Compiled path: `compile_source` → `axiom_vm::run` → [`read_back`] →
/// the same `print_axiom` function the native runtime uses.
fn compiled(name: &str, source: &str) -> String {
    let module = compile_source(source, "oracle")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({source:?}): {e}"));
    let value = axiom_vm::run(&module)
        .unwrap_or_else(|e| panic!("VM execution failed for {name} ({source:?}): {e}"));
    let node = read_back(value);
    print_axiom(&node)
}

/// Rebuild the [`symbolic_ir::IRNode`] a [`LispyValue`] represents — the
/// mirror image of `axiom_iir_compiler`'s own `inert_apply`/`emit_int`/
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
fn oracle_corpus_matches_native_axiom_runtime() {
    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = ground_truth(case.source);
        if gt != case.expected {
            failures.push(format!(
                "{}: axiom-runtime itself disagrees with this corpus entry's own `expected` \
                 (got {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the \
                 corpus rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        let got = compiled(case.name, case.source);
        if got != case.expected {
            failures.push(format!(
                "{}: axiom-iir-compiler -> axiom-vm disagrees with the axiom-runtime ground \
                 truth (got {got:?}, expected {:?})",
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
