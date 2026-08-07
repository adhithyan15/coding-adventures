//! Oracle/golden tests (HML01 §7): the SAME Q source, run through **two
//! independent implementations**, and diffed:
//!
//!   (a) `q-runtime` (`coding-adventures-q-runtime`) — this frontend's own
//!       sibling crate, a tree-walking interpreter over `array-runtime` —
//!       the ground truth.
//!   (b) `q_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → an actual `node` process.
//!
//! This is the direct Q sibling of
//! [`j-to-semantic-ir`'s own `tests/oracle.rs`](../../j-to-semantic-ir/tests/oracle.rs)
//! (itself sibling to `apl-to-semantic-ir`'s/`matlab-to-semantic-ir`'s) —
//! same overall shape (`node_available` skip-not-fail guard, a
//! `Case`/`CORPUS`, a `ground_truth`/`compiled` pair, one looping `#[test]`).
//!
//! ## No `setup`/`final_expr` split, no `normalize()`
//!
//! `q-runtime::eval`'s own module doc comment states plainly: "Assignment
//! is silent; a bare (non-assignment) statement auto-prints its result" —
//! and this crate's own lowering wraps a bare top-level `noun_expr` in the
//! shared `"print"` builtin unconditionally, exactly like J/APL. So [`Case`]
//! is just `name` + `source` (one full program, byte-identical on both
//! sides) + `expected`, plus `known_bug` (see below).
//!
//! ## A pre-existing shared-crate display gap (matches `j-to-semantic-ir/
//! tests/oracle.rs`'s own "Bug A"), FIXED as of task #109
//!
//! `semantic-ir-to-javascript` originally had exactly two per-source-
//! language display flags (`SIR_DISPLAY_APL_HIGH_MINUS`,
//! `SIR_DISPLAY_J_UNDERSCORE`) and no third one for Q. Empirically
//! confirmed directly (a throwaway probe that ran each shape through
//! `node` before this file was finalized, mirroring `j-to-semantic-ir/
//! tests/oracle.rs`'s own verification discipline):
//!
//! - A **bare/boxed scalar** negative result (`formatSeen`'s `typeof v ===
//!   "number"` branch) renders via plain `String(v)` when no display flag is
//!   set — which happens to be Q's OWN convention already (`-5`, ASCII, no
//!   high-minus) — so this path had **no bug for Q at all**, unlike J
//!   (which needs a leading underscore no pre-existing flag produced).
//! - A **genuine NDArray** result (any [`semantic_ir::Expr::ElementwiseOp`]/
//!   `Reduce`/`Scan`/`Ravel`/`Catenate`, or `mapNDArrayRank1Plus`'s rank≥1
//!   branch inside `neg`/`q_reverse`/etc.) reached `ArrayRt.display` →
//!   `ArrayRt.fmtNum`, which rendered APL's own high-minus `¯` for ANY
//!   negative value whenever `SIR_DISPLAY_J_UNDERSCORE` was unset —
//!   *unconditionally*, with no flag gating it for Q either. Confirmed
//!   directly: `3-4` (a dyadic `ElementwiseOp`) printed `¯1`, not Q's own
//!   `-1`; `-/1 2 10` (`Reduce`) printed `¯11`.
//!
//! This was the exact same shared-crate gap `j-to-semantic-ir`'s own oracle
//! file already found and left unfixed (per this repo's "found, NOT fixed
//! here" discipline for a bug in a crate consumed by many other frontends)
//! — not a Q-specific bug, and not something this crate's own lowering could
//! route around. Task #109 closed it directly in `semantic-ir-to-javascript`
//! (a third, mutually-exclusive `SIR_DISPLAY_Q_ASCII_MINUS` display flag,
//! following the same pattern `SIR_DISPLAY_J_UNDERSCORE` established for
//! J — see that crate's own `src/emit.rs`/`src/runtime.rs` and
//! `CHANGELOG.md` for the full writeup), so every `CORPUS` entry below now
//! has `known_bug: None` and is checked end-to-end on both sides. The
//! `known_bug` field itself is kept on [`Case`] (unused by any entry today)
//! so a future frontier bug can be recorded the same way without a struct
//! change.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_q_runtime::eval as q_eval;
use q_to_semantic_ir::compile_source;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. `source` is the WHOLE program, byte-for-byte
/// identical on both the `ground_truth` and `compiled` sides.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    /// `None`: both `ground_truth` and `compiled` must equal `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely -- see this file's
    /// module doc comment's "shared-crate display gap" section. No
    /// `CORPUS` entry uses `Some` today (task #109 fixed the one
    /// pre-existing gap directly in `semantic-ir-to-javascript`) -- the
    /// field stays available for a future genuine bug.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Base breadth: right-to-left, no precedence, grouping ---
    Case {
        name: "right_to_left_no_operator_precedence",
        source: "2*3+4\n",
        expected: "14",
        known_bug: None,
    },
    Case {
        name: "parenthesised_grouping_overrides_order",
        source: "(2*3)+4\n",
        expected: "10",
        known_bug: None,
    },
    Case {
        name: "stranded_vector_literal",
        source: "1 2 3\n",
        expected: "1 2 3",
        known_bug: None,
    },
    Case {
        name: "whitespace_sensitive_strand_vs_subtraction_strand",
        source: "2 -1\n",
        expected: "2 -1",
        known_bug: None,
    },
    Case {
        name: "whitespace_sensitive_strand_vs_subtraction_subtract",
        source: "2 - 1\n",
        expected: "1",
        known_bug: None,
    },
    // Third spelling of the same disambiguation: NO space at all on either
    // side (`2-1`, fully glued). MA11 §3 bullet 2's rule only folds a `-`
    // into a signed-numeric-literal token when it is "at a position where a
    // new list-stranding element may start" -- i.e. preceded by whitespace
    // after a completed noun. With no preceding space, `2-1` never reaches
    // that check at all (confirmed directly: `q-lexer`'s own
    // `no_space_at_all_is_also_subtraction`/`no_space_at_all_stays_
    // subtraction` tests and `q-runtime`'s own `scalar("2-1\n") == 1.0`
    // assertion, both tokenizing/evaluating this as ordinary subtraction,
    // never a two-element strand) -- so this lowers to the exact same
    // `ElementwiseOp` shape as `2 - 1` above, just reached via a different
    // tokenization path. Genuinely new coverage here: those two upstream
    // tests only confirm the *lexer*/*runtime* get this right in isolation;
    // this case confirms the SAME resolved CST shape also lowers correctly
    // through `q-to-semantic-ir` -> `semantic-ir-to-javascript` -> `node`,
    // not just through `q-runtime`'s own evaluator.
    Case {
        name: "whitespace_sensitive_strand_vs_subtraction_no_space_at_all",
        source: "2-1\n",
        expected: "1",
        known_bug: None,
    },

    // --- All 6 comparisons (always 0/1, never negative -- no display gap) ---
    Case { name: "comparison_eq_true", source: "3=3\n", expected: "1", known_bug: None },
    Case { name: "comparison_ne_true_q_spelling", source: "3<>4\n", expected: "1", known_bug: None },
    Case { name: "comparison_lt_true", source: "3<4\n", expected: "1", known_bug: None },
    Case { name: "comparison_le_true", source: "3<=3\n", expected: "1", known_bug: None },
    Case { name: "comparison_ge_false", source: "3>=4\n", expected: "0", known_bug: None },
    Case { name: "comparison_gt_true", source: "4>3\n", expected: "1", known_bug: None },

    // --- Dyadic arithmetic: positive results, clean both-sides-pass ---
    Case { name: "dyadic_add", source: "3+4\n", expected: "7", known_bug: None },
    Case {
        name: "dyadic_percent_true_division",
        source: "6%4\n",
        expected: "1.5",
        known_bug: None,
    },
    Case {
        name: "dyadic_min_amp_max_pipe",
        source: "(3&7),(3|7)\n",
        expected: "3 7",
        known_bug: None,
    },
    // Dyadic subtraction with a NEGATIVE result -- used to hit the display
    // gap (fixed by task #109; see this file's module doc comment).
    Case {
        name: "dyadic_sub_negative_result",
        source: "3-4\n",
        expected: "-1",
        known_bug: None,
    },

    // --- Monadic primitives ---
    Case { name: "monadic_plus_flip_is_identity", source: "+5\n", expected: "5", known_bug: None },
    // Monadic `-` on a PARENTHESISED (non-glued) scalar operand -- `neg`'s
    // own rank-0 fallback unwraps to a bare number, so this is a CLEAN
    // negative-result pass, no display gap (confirmed empirically; see
    // this file's module doc comment).
    Case {
        name: "monadic_minus_negates_a_scalar_expression",
        source: "-(3+4)\n",
        expected: "-7",
        known_bug: None,
    },
    // Monadic `-` on a genuine VECTOR operand used to hit the display gap
    // (`mapNDArrayRank1Plus`'s rank>=1 branch always re-wraps in an
    // NDArray) -- fixed by task #109; see this file's module doc comment.
    Case {
        name: "monadic_minus_negates_a_vector",
        source: "-(1 2 3)\n",
        expected: "-1 -2 -3",
        known_bug: None,
    },
    Case { name: "monadic_star_is_first_not_sign", source: "*1 2 3\n", expected: "1", known_bug: None },
    Case { name: "monadic_percent_is_reciprocal", source: "%4\n", expected: "0.25", known_bug: None },
    Case {
        name: "monadic_bang_is_zero_based_til",
        source: "!5\n",
        expected: "0 1 2 3 4",
        known_bug: None,
    },
    Case { name: "monadic_comma_is_enlist", source: ",5\n", expected: "5", known_bug: None },
    Case { name: "monadic_hash_is_count", source: "#1 2 3\n", expected: "3", known_bug: None },
    Case {
        name: "monadic_underscore_is_floor_of_a_negative_scalar",
        source: "_ -3.2\n",
        expected: "-4",
        known_bug: None,
    },
    Case {
        name: "monadic_amp_is_where",
        source: "&0 1 1 0 1\n",
        expected: "1 2 4",
        known_bug: None,
    },
    Case { name: "monadic_pipe_is_reverse", source: "|1 2 3\n", expected: "3 2 1", known_bug: None },
    Case { name: "monadic_tilde_is_not", source: "~0 1 5\n", expected: "1 0 0", known_bug: None },

    // --- Dyadic bespoke primitives ---
    Case { name: "dyadic_comma_joins", source: "1,2\n", expected: "1 2", known_bug: None },
    Case {
        name: "dyadic_hash_takes_with_cycling",
        source: "5#1 2 3\n",
        expected: "1 2 3 1 2",
        known_bug: None,
    },
    Case {
        name: "dyadic_underscore_drops_from_front",
        source: "2_1 2 3 4\n",
        expected: "3 4",
        known_bug: None,
    },
    Case {
        name: "dyadic_tilde_is_deep_match_true",
        source: "(1 2 3)~1 2 3\n",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "dyadic_tilde_is_deep_match_false",
        source: "(1 2 3)~1 2 4\n",
        expected: "0",
        known_bug: None,
    },

    // --- Adverbs: reduce, scan ---
    Case { name: "reduce_sums_a_vector", source: "+/1 2 3 4\n", expected: "10", known_bug: None },
    Case {
        name: "scan_keeps_every_running_fold",
        source: "+\\1 2 3 4\n",
        expected: "1 3 6 10",
        known_bug: None,
    },
    Case {
        name: "each_on_an_elementwise_primitive_matches_direct_application",
        source: "-'1 2 3\n",
        expected: "-1 -2 -3",
        known_bug: None,
    },
    // Reduce-of-scan with a negative result -- Reduce's own NDArray output
    // used to hit the display gap (fixed by task #109; see this file's
    // module doc comment).
    Case {
        name: "scan_then_reduce_negative",
        source: "-/+\\1 2 3\n",
        expected: "-8",
        known_bug: None,
    },

    // --- Assignment: silent, chained, later reference ---
    Case {
        name: "variable_assignment_and_later_reference",
        source: "n:3\nn+4\n",
        expected: "7",
        known_bug: None,
    },
    Case {
        name: "chained_assignment_sets_both_names",
        source: "a:b:3\na\nb\n",
        expected: "3\n3",
        known_bug: None,
    },

    // --- List literals: dual syntax, same value ---
    Case {
        name: "explicit_list_literal_matches_stranding",
        source: "(1;2;3)\n",
        expected: "1 2 3",
        known_bug: None,
    },

    // --- Function literals: the headline novelty ---
    Case {
        name: "function_literal_implicit_params_dyadic_call",
        source: "f:{x+y}\n2 f 3\n",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "function_literal_called_monadically_binds_only_x",
        source: "f:{x+1}\nf 5\n",
        expected: "6",
        known_bug: None,
    },
    Case {
        name: "function_literal_explicit_param_list",
        source: "f:{[a;b] a*b}\n3 f 4\n",
        expected: "12",
        known_bug: None,
    },
    Case {
        name: "multi_statement_function_body_returns_last_value",
        source: "f:{[x] a:x+1; a*2}\nf 5\n",
        expected: "12",
        known_bug: None,
    },
    Case {
        name: "function_literal_is_assignable_without_being_called",
        source: "f:{x+y}\n",
        expected: "",
        known_bug: None,
    },
    Case {
        name: "calling_an_inline_function_literal_monadically",
        source: "{x*2} 5\n",
        expected: "10",
        known_bug: None,
    },
    Case {
        name: "calling_an_inline_function_literal_dyadically",
        source: "2 {x+y} 3\n",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "a_function_body_calling_another_already_defined_function",
        source: "double:{x*2}\nadd1:{x+1}\ndouble(add1 5)\n",
        expected: "12",
        known_bug: None,
    },
    Case {
        name: "passing_a_function_value_as_an_argument_to_another_function",
        source: "apply:{[g] g 5}\ninc:{x+1}\napply inc\n",
        expected: "6",
        known_bug: None,
    },
    Case {
        name: "a_function_body_reads_a_top_level_global_array_variable",
        source: "n:10\nf:{x+n}\nf 5\n",
        expected: "15",
        known_bug: None,
    },
];

/// Is a `node` binary on `PATH`? The test below skips (logs, does not
/// fail) when it is not.
fn ground_truth(source: &str) -> String {
    q_eval(source)
        .unwrap_or_else(|e| panic!("q-runtime eval failed for {source:?}: {e}"))
        .trim_end_matches('\n')
        .to_string()
}

fn compiled(name: &str, source: &str) -> String {
    let module = compile_source(source, "prog")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({source:?}): {e:?}"));
    let report = semantic_ir::validate(&module);
    assert!(report.is_ok(), "SIR validation failed for {name}: {:?}", report.issues);
    let artifact =
        semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("q_sir_oracle_{name}_{}.js", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes()).expect("write temp js");
    drop(file);

    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim_end_matches('\n').to_string()
}

#[test]
fn oracle_corpus_matches_native_q_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_q_runtime: `node` not available");
        return;
    }
    let mut failures = Vec::new();
    for case in CORPUS {
        let gt = ground_truth(case.source);
        if gt != case.expected {
            failures.push(format!(
                "{}: ground_truth mismatch -- got {gt:?}, expected {:?}",
                case.name, case.expected
            ));
            continue;
        }
        if let Some(reason) = case.known_bug {
            eprintln!("{}: skipping compiled-side check (known_bug: {reason})", case.name);
            continue;
        }
        let got = compiled(case.name, case.source);
        if got != case.expected {
            failures.push(format!(
                "{}: compiled mismatch -- got {got:?}, expected {:?} (source: {:?})",
                case.name, case.expected, case.source
            ));
        }
    }
    assert!(failures.is_empty(), "oracle mismatches:\n{}", failures.join("\n"));
}
