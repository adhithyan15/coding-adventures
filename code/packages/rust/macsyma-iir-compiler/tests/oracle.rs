//! Oracle/golden test (macsyma-iir-vm.md §7): the SAME Macsyma source, run
//! through **two independent implementations**, and diffed:
//!
//!   (a) `macsyma-runtime` (`coding-adventures-macsyma-runtime`) — the
//!       native runtime crate, which lowers to `symbolic-ir` and evaluates
//!       via `symbolic-vm`'s shared handler table — the ground truth.
//!   (b) `macsyma_iir_compiler::compile_source` → `interpreter_ir::IIRModule`
//!       → `macsyma_vm::run` → a `LispyValue`, read back into a
//!       `symbolic_ir::IRNode` and rendered through the SAME
//!       `cas_pretty_printer::pretty` call the native runtime uses.
//!
//! Unlike `macsyma-to-semantic-ir/tests/oracle.rs` (which joins every
//! statement's own `output_text` with `"\n"`, since that lowering keeps
//! every statement as an independent `Stmt::ExprStmt`), this file only
//! diffs the LAST statement's value: `macsyma-iir-compiler::lower_file`
//! discards every earlier statement's result (a v0 program is a single
//! `main` function returning its final statement's value — see
//! `lower.rs`'s own `lower_file`), so [`ground_truth`] below reads the
//! final [`EvalResult::output_text`] only, not a joined string.
//!
//! ## Why a test-local "un-quote" reader instead of reusing a runtime
//! builtin
//!
//! `macsyma-iir-compiler::inert_apply` materialises an unevaluated
//! `Apply(head, args)` as a `cons`-chain — `(head arg0 arg1 …)` — the exact
//! shape `mccarthy-lisp-iir-compiler::lower_quote` uses for `QUOTE`. To
//! compare that against `macsyma-runtime`'s own `symbolic_ir::IRNode`
//! result with the same pretty-printer, [`read_back`] below walks the
//! returned [`dynval_runtime::LispyValue`] and rebuilds the equivalent
//! `IRNode` — the mirror image of `inert_apply`. This is test-only code
//! (never shipped in `macsyma-iir-compiler` itself, which has no reason to
//! ever convert a `LispyValue` back into `IRNode`): its input is always
//! exactly what this file's own small, hand-authored corpus produced by
//! running `macsyma-iir-compiler`'s own lowering, never untrusted source,
//! so — unlike `lower.rs`'s adversarial-input-hardened, explicitly
//! iterative tree walks (`macsyma-to-semantic-ir`'s `measure_depth_
//! iterative`/`drop_iterative` precedent) — plain recursion here is
//! correct and proportionate: the deepest nesting any corpus entry below
//! can produce is a handful of chained binary operators, not attacker-
//! controlled depth.
//!
//! ## Corpus
//!
//! v0's accepted grammar subset only (macsyma-iir-vm.md §4): literal
//! integer arithmetic (`+ - * /`, unary `-`/`+`), assignment and
//! re-assignment threading, and unevaluated symbolic `Apply` results for
//! any operand that stays free. `known_bug` is expected to stay empty for
//! every entry here — v0's accepted subset is designed so the VM-compiled
//! path and `macsyma-runtime` agree exactly, not approximately (see
//! `lower.rs`'s own doc comment for why `/`, in particular, either
//! evaluates exactly or is rejected outright, with no approximate middle
//! ground). Every construct v0 rejects (`Rational`/`Float`/`Str`,
//! `if`/`while`/`for`/`block`/`:=`, lists, comparisons, `and`/`or`/`not`,
//! `^`, function calls) already has a dedicated `Err`-asserting unit test
//! in `src/lib.rs`'s own `tests` module — this file does not re-test those
//! paths, only the ACCEPTED, diffable ones.

use cas_pretty_printer::{pretty, MacsymaDialect};
use coding_adventures_macsyma_runtime::MacsymaSession;
use dynval_runtime::{builtins, name_of, LispyValue};
use macsyma_iir_compiler::compile_source;
use symbolic_ir::{apply, int, sym, IRNode};

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
        source: "42$\n",
        expected: "42",
    },
    Case {
        name: "simple_addition",
        source: "2 + 3$\n",
        expected: "5",
    },
    Case {
        name: "precedence",
        source: "2 + 3 * 4$\n",
        expected: "14",
    },
    Case {
        name: "left_associative_chain",
        source: "1 + 2 + 3 + 4$\n",
        expected: "10",
    },
    Case {
        name: "unary_minus_leaf",
        source: "-5 + 3$\n",
        expected: "-2",
    },
    Case {
        name: "unary_minus_compound",
        source: "-(5 + 3)$\n",
        expected: "-8",
    },
    Case {
        name: "unary_plus_is_noop",
        source: "+5$\n",
        expected: "5",
    },
    Case {
        name: "grouping_overrides_precedence",
        source: "(2 + 3) * 4$\n",
        expected: "20",
    },
    Case {
        name: "exact_integer_division",
        source: "20 / 4$\n",
        expected: "5",
    },
    Case {
        name: "negative_literal_exact_division",
        source: "-4 / 2$\n",
        expected: "-2",
    },
    // --- Assignment: binding, reference, re-assignment threading, and
    // multiple independent variables in one program. ---
    Case {
        name: "assignment_and_reference",
        source: "x: 3$\nx + 1$\n",
        expected: "4",
    },
    Case {
        name: "reassignment_threading",
        source: "x: 3$\nx: x + 1$\nx$\n",
        expected: "4",
    },
    Case {
        name: "two_independent_variables",
        source: "a: 2$\nb: 3$\na * b$\n",
        expected: "6",
    },
    // --- Unevaluated symbolic expressions: a free symbol alone, and every
    // arithmetic head with a symbolic operand (Add/Sub/Mul/Div/Neg via
    // `inert_apply`). ---
    Case {
        name: "free_symbol_alone",
        source: "x$\n",
        expected: "x",
    },
    Case {
        name: "free_symbol_addition",
        source: "x + y$\n",
        expected: "x + y",
    },
    Case {
        name: "mixed_concrete_and_symbolic_multiplication",
        source: "2 * x$\n",
        expected: "2*x",
    },
    Case {
        name: "symbolic_subtraction",
        source: "x - y$\n",
        expected: "x - y",
    },
    Case {
        name: "symbolic_division_stays_unevaluated",
        source: "x / y$\n",
        expected: "x/y",
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x$\n",
        expected: "-x",
    },
    Case {
        name: "chained_symbolic_addition_nests_left_associatively",
        // (x + y) + z -- `combine` builds a nested inert Apply once the
        // running result is no longer concrete, exercising `read_back`'s
        // own recursion into a symbolic *argument* (not just a symbolic
        // leaf).
        source: "x + y + z$\n",
        expected: "x + y + z",
    },
    Case {
        name: "assigned_value_used_inside_a_symbolic_expression",
        // x is bound (concrete) but y is free: the whole expression stays
        // symbolic, with x's *value* substituted in, not the name "x".
        source: "x: 2$\nx + y$\n",
        expected: "2 + y",
    },
];

/// Ground truth: run `source` through `macsyma-runtime`'s own
/// [`MacsymaSession::eval_source`], taking only the LAST statement's
/// [`coding_adventures_macsyma_runtime::EvalResult::output_text`] — see
/// this file's module doc comment for why (unlike the SIR oracle file,
/// `macsyma-iir-compiler` discards every earlier statement's value).
fn ground_truth(source: &str) -> String {
    let mut session = MacsymaSession::new();
    let results = session
        .eval_source(source)
        .unwrap_or_else(|e| panic!("macsyma-runtime eval failed for {source:?}: {e}"));
    results
        .into_iter()
        .last()
        .expect("at least one statement")
        .output_text
}

/// Compiled path: `compile_source` → `macsyma_vm::run` → [`read_back`] →
/// the same `cas_pretty_printer::pretty` call the native runtime uses.
fn compiled(name: &str, source: &str) -> String {
    let module = compile_source(source, "oracle")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({source:?}): {e}"));
    let value = macsyma_vm::run(&module)
        .unwrap_or_else(|e| panic!("VM execution failed for {name} ({source:?}): {e}"));
    let node = read_back(value);
    pretty(&node, &MacsymaDialect)
}

/// Rebuild the [`symbolic_ir::IRNode`] a [`LispyValue`] represents — the
/// mirror image of `macsyma_iir_compiler`'s own `inert_apply`/`emit_int`/
/// `emit_symbol`. See this file's module doc comment for why plain
/// recursion (not an iterative work-stack) is appropriate here.
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
fn oracle_corpus_matches_native_macsyma_runtime() {
    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = ground_truth(case.source);
        if gt != case.expected {
            failures.push(format!(
                "{}: macsyma-runtime itself disagrees with this corpus entry's own `expected` \
                 (got {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the \
                 corpus rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        let got = compiled(case.name, case.source);
        if got != case.expected {
            failures.push(format!(
                "{}: macsyma-iir-compiler -> macsyma-vm disagrees with the macsyma-runtime \
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
