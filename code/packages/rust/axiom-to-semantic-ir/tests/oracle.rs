//! Oracle/golden tests (HML01 §7 convention, Wave 7 close-out): the SAME
//! Axiom source, run through **two independent implementations**, and
//! diffed:
//!
//!   (a) `axiom-runtime` (`coding_adventures_axiom_runtime::AxiomSession`) —
//!       the sibling native runtime crate (MA-13d), which lowers arithmetic
//!       to `symbolic-ir`/evaluates via `symbolic-vm`'s shared handler
//!       table, plus its own `crate::domains` fixed `AxiomDomain`/
//!       `AxiomCategory` table for `:`/`::`/`has` — the ground truth.
//!   (b) `axiom_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → an actual `node` process,
//!       whose JS runtime now carries a matching port of that same fixed
//!       table (`axiomDeclareHandler`/`axiomCoerceHandler`/
//!       `axiomHasHandler`, `semantic-ir-to-javascript/src/runtime.rs`).
//!
//! Modeled directly on `macsyma-to-semantic-ir/tests/oracle.rs` (itself
//! modeled on `wolfram-to-semantic-ir`'s/`maple-to-semantic-ir`'s): same
//! overall shape (`node_available` skip-not-fail guard, a `Case`/`CORPUS`,
//! a `ground_truth`/`compiled` pair, one looping `#[test]`, a `known_bug`
//! field). Axiom is the CAS sibling that ALSO needed a real domain/category
//! layer (MA13 §2's own finding: `symbolic_ir::IRNode` has no domain/type
//! tag at all), so this file's corpus additionally covers `:`/`::`/`has`
//! end-to-end through the compiled path — the actual point of an oracle
//! test for this language, per this task's own instruction not to just
//! prove these three constructs compile inertly (`tests/e2e_node.rs`
//! already did that before this file existed, and now additionally asserts
//! real values for its own three simplest cases).
//!
//! # `program` is ONE expression — `Case::source` is one whole program,
//! `;`-blocks stand in for multi-statement sessions
//!
//! Unlike every prior SIR23 oracle file (`derive-to-semantic-ir`'s,
//! `reduce-to-semantic-ir`'s, `maple-to-semantic-ir`'s, `macsyma-to-
//! semantic-ir`'s, `wolfram-to-semantic-ir`'s own `tests/oracle.rs`, which
//! all lower a REPEATED `{ statement_line }` worksheet in one
//! `compile_source` call), `axiom.grammar`'s own `program = expr` parses
//! **exactly one** expression (`axiom-to-semantic-ir::lower`'s own module
//! doc, "program is a SINGLE expression"). A "declare, then assign" or a
//! "define, then call" test therefore has to be written as ONE Axiom
//! parenthesised, semicolon-separated block (`(s1; s2; ...)`, MA13 §4's own
//! grammar row) rather than as separate `ground_truth`/`compiled` calls —
//! this is real Axiom syntax (not a harness invention), and it is also
//! literally one of this task's own required corpus items ("Parenthesized
//! semicolon-separated blocks").
//!
//! # A harness-only "make it observable" step, adapted for Axiom's
//! single-statement `main`
//!
//! Every prior SIR23 oracle file wraps its (possibly MULTIPLE) top-level
//! statements each in `print(...)` via `wrap_top_level_in_print`, since
//! `<lang>-to-semantic-ir::compile_source` never does this itself (see
//! each of those files' own "harness-only" section). Axiom's own
//! `compile_source` always emits exactly ONE `Stmt::ExprStmt` in `main`
//! (per the note above) — but when the source is a `;`-block, that ONE
//! statement's `expr` is itself a single `SymApply(CompoundExpression,
//! [s1, s2, ..., sN])` node (`axiom-to-semantic-ir::lower`'s own
//! `COMPOUND_EXPRESSION` constant, spelled identically to
//! `reduce-to-semantic-ir`'s own `<< ... >>` head for the identical
//! reason). Naively wrapping the WHOLE block in one `print(...)` call would
//! only ever observe the OUTER `CompoundExpression(...)` term, never just
//! its last statement's value — because `CompoundExpression` has **no**
//! evaluation handler in EITHER `symbolic-vm` or this backend's JS runtime
//! (a pre-existing, already-disclosed gap: `reduce-to-semantic-ir/tests/
//! oracle.rs`'s own "finding three" confirms neither the shared engine nor
//! this JS port has ever had one — `axiom-runtime`'s own block-collapsing
//! is a completely separate, bespoke interpreter code path,
//! `eval::eval_group`, that never touches `symbolic-vm`/this JS port at
//! all).
//!
//! [`wrap_axiom_top_level_for_observation`] (below) is this file's own
//! harness-only fix, deliberately NOT a change to `semantic-ir-to-
//! javascript` itself (which would touch Reduce too, a broader change than
//! this task's own narrow three-head scope calls for): it "unrolls" a
//! top-level `CompoundExpression` into N separate top-level statements —
//! the first N-1 evaluated bare (mirroring `emit.rs`'s existing
//! `is_sym23_root_shape` per-statement `evalTerm` wrap, which already runs
//! each one's side effects in order and discards its value), and the LAST
//! wrapped in `print(...)` — exactly reproducing what `axiom-runtime`'s own
//! `eval_group` does natively. A non-`CompoundExpression` root (an ordinary
//! single statement) is wrapped in one `print(...)` unchanged, matching
//! every prior oracle file's own `wrap_top_level_in_print` for that case.
//! This means every corpus entry below — including the block-based
//! function-definition-then-call and declare-then-assign cases — gets a
//! REAL value comparison, not a `known_bug` for the display-convention gap
//! alone, unlike what a naive whole-block `print` wrap would have forced.
//!
//! # Findings (confirmed by direct inspection of `semantic-ir-to-
//! javascript`'s `runtime.rs`, and of `axiom-runtime`'s own `src/`, not
//! assumed from either crate's scope notes)
//!
//! ### Finding one — the SIR23 JS backend's existing arithmetic/comparison/
//! held-form/user-function machinery (shipped by prior PRs, not this one)
//! already agrees with `axiom-runtime` for Axiom's own surface
//!
//! `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg` fold numerically exactly like every
//! other CAS-family language sharing this engine; `Assign`/`Define`/`If`
//! are real, environment-backed held forms; a `Define`d function is a real,
//! callable, position-substituted user function on the compiled side too.
//! Axiom needs no `True`/`False` case-bridging fix of its OWN discovery
//! here (unlike Maple's, HML01 §5's own "finding four") because THIS task
//! adds `SIR_DISPLAY_AXIOM_BOOLEAN` (see `runtime.rs`) precisely so the
//! shared comparison/logic/`has`-query handlers' `True`/`False` render
//! Axiom's own lowercase `true`/`false` convention — every `known_bug: None`
//! boolean-result case below depends on that flag.
//!
//! ### Finding two — `:`/`::`/`has` now evaluate identically to
//! `axiom-runtime`, for every ATOMIC operand this corpus exercises
//!
//! `axiomDeclareHandler`/`axiomCoerceHandler`/`axiomHasHandler` (this PR,
//! `runtime.rs`) are direct, one-for-one ports of `axiom-runtime::domains`'s
//! fixed table, with every error message copied VERBATIM. Confirmed
//! end-to-end for a passing AND a failing `:` declaration, a passing and a
//! failing `::` coercion, and the book's own two confirmed `has` examples.
//!
//! ### Finding three (disclosed, NOT fixed here) — no per-language
//! infix/bracket display convention for a COMPOUND (non-atomic) Axiom
//! value, the same "finding five"-class gap every CAS-family language
//! without a full `SIR_DISPLAY_<LANG>` printer has
//!
//! `Symbolic.toDisplayString`'s generic branch renders any compound term as
//! `head(args, ...)` — no infix `+`/`-`/`*`/`/`/`^`, no `[...]` bracket
//! convention for `List`. `axiom-runtime::value::print_axiom`, by contrast,
//! is fully infix/bracket-aware. So the ONE list-literal case below needs
//! `known_bug` for this reason alone (the underlying evaluation — folding
//! each element — is completely correct on both sides; only the RENDERING
//! disagrees). This is a real, pre-existing, disclosed gap this task's own
//! narrow three-head scope does not fix (adding a full `SIR_DISPLAY_AXIOM`
//! infix printer, mirroring Derive's, would be a much larger, separate
//! change — this task adds only the minimal boolean-case flag needed to
//! make comparison/`has`-query results checkable at all).
//!
//! ## Corpus
//!
//! Covers, at minimum (per this task's own required list): arithmetic
//! precedence/associativity/right-associative `^`; every comparison (`=
//! ~= < <= > >=`); `:=` immediate assignment; `==` function definition
//! (both the declared `f(x: T, ...): T == e` and undeclared `f x == e`
//! forms) and calls (both `f(a, b)` and paren-optional `f a`);
//! `if`/`then`/`else`; a parenthesised `;`-separated block; a passing AND a
//! failing `:` declaration; a passing AND a failing `::` coercion; and the
//! book's own two confirmed `has` examples
//! (`Polynomial(Integer) has Ring` → `true`, `List(Integer) has Ring` →
//! `false`).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_axiom_runtime::AxiomSession;
use coding_adventures_axiom_to_semantic_ir::compile_source;
use semantic_ir::{EffectSet, Expr, Module, Stmt};

/// Is a `node` binary on `PATH`? Mirrors every sibling oracle file's
/// identical `node_available`: the test below skips (logs, does not fail)
/// when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// What a corpus entry expects on BOTH the native and the compiled side.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Expected {
    /// Both sides must succeed and print exactly this text.
    Value(&'static str),
    /// Both sides must FAIL, each error message containing this substring.
    /// (A substring, not byte-for-byte equality: `axiom-runtime`'s Rust
    /// `EvalError` and this file's thrown JS `Error` are independently
    /// constructed strings that happen to match verbatim for every ATOMIC
    /// operand this corpus uses — see the module doc's "Finding two" — but
    /// requiring only a shared substring keeps this assertion honest about
    /// what is actually being checked, rather than baking in an unstated
    /// assumption that the two error types are byte-identical in general.)
    Error(&'static str),
}

/// One oracle corpus entry. `source` is the WHOLE program, byte-for-byte
/// identical on both the [`ground_truth`] and [`compiled`] sides.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: Expected,
    /// `None`: both `ground_truth` and `compiled` must match `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely (not even invoked),
    /// with `reason` naming which documented finding (module doc) applies.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Arithmetic: precedence, right-associative `^`, unary minus
    // binding LOOSER than `^`, exact-integer vs. genuine-rational
    // division (MA13 §4's own precedence table; grammar confirmed
    // directly against `code/grammars/axiom/axiom.grammar`). ---
    Case {
        name: "literal_arithmetic_precedence",
        source: "1 + 2*3",
        expected: Expected::Value("7"),
        known_bug: None,
    },
    Case {
        name: "parens_override_precedence",
        source: "(1 + 2)*3",
        expected: Expected::Value("9"),
        known_bug: None,
    },
    Case {
        name: "power_is_right_associative",
        // 2^3^2 == 2^(3^2) == 2^9 == 512, NOT (2^3)^2 == 64 -- confirmed
        // directly against `axiom.grammar`'s own `power = postfix [ (CARET
        // | POW) unary ]` (the RHS `unary` recurses back into `power`).
        source: "2^3^2",
        expected: Expected::Value("512"),
        known_bug: None,
    },
    Case {
        name: "both_power_spellings_agree",
        source: "2**3**2",
        expected: Expected::Value("512"),
        known_bug: None,
    },
    Case {
        name: "unary_minus_binds_looser_than_power",
        // -2^2 -> Neg(Pow(2, 2)) = Neg(4) = -4, NOT (-2)^2 = 4.
        source: "-2^2",
        expected: Expected::Value("-4"),
        known_bug: None,
    },
    Case {
        name: "exact_integer_division_folds_to_an_integer",
        source: "10 / 2",
        expected: Expected::Value("5"),
        known_bug: None,
    },
    Case {
        name: "inexact_division_folds_to_a_rational",
        source: "1 / 3",
        expected: Expected::Value("1/3"),
        known_bug: None,
    },
    // --- Comparisons: Axiom's OWN spelling (`~=` not-equal, confirmed
    // directly -- NOT Maple's `<>`, NOT Wolfram's `!=`). Every comparison
    // folds to the shared True/False SYMBOL, rendered lowercase on BOTH
    // sides thanks to `SIR_DISPLAY_AXIOM_BOOLEAN` (this PR). ---
    Case {
        name: "equality_is_true",
        source: "1 = 1",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    Case {
        name: "not_equal_is_true",
        source: "1 ~= 2",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    Case {
        name: "less_than_is_true",
        source: "1 < 2",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    Case {
        name: "greater_than_is_false",
        source: "3 > 5",
        expected: Expected::Value("false"),
        known_bug: None,
    },
    Case {
        name: "less_equal_boundary_is_true",
        source: "3 <= 3",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    Case {
        name: "greater_equal_is_false",
        source: "3 >= 4",
        expected: Expected::Value("false"),
        known_bug: None,
    },
    // --- `:=` immediate assignment (bare -- a single top-level statement,
    // no block needed). ---
    Case {
        name: "bare_assignment",
        source: "x := 5",
        expected: Expected::Value("5"),
        known_bug: None,
    },
    // --- `==` function definition: the DECLARED form alone (no call) --
    // register_function's own disclosed presentation convention (echoes
    // the bare name). ---
    Case {
        name: "declared_function_definition_alone",
        source: "power(x: Integer, n: NonNegativeInteger): Integer == x ** n",
        expected: Expected::Value("power"),
        known_bug: None,
    },
    // --- `==` function definition (DECLARED form) + call, `f(a, b)`
    // two-argument explicit-parens form -- a `;`-block, exercising
    // `wrap_axiom_top_level_for_observation`'s CompoundExpression unroll. ---
    Case {
        name: "declared_function_definition_and_call",
        source: "(power(x: Integer, n: NonNegativeInteger): Integer == x ** n; power(2, 3))",
        expected: Expected::Value("8"),
        known_bug: None,
    },
    // --- `==` function definition (UNDECLARED, duck-typed form) + call,
    // paren-optional single-argument `f a` form -- also a `;`-block. ---
    Case {
        name: "undeclared_function_definition_and_paren_optional_call",
        source: "(f x == x * x; f 6)",
        expected: Expected::Value("36"),
        known_bug: None,
    },
    // --- if/then/else (both branches; `else` mandatory in this cut). ---
    Case {
        name: "if_then_else_true_branch",
        source: "if 1 > 0 then 1 else -1",
        expected: Expected::Value("1"),
        known_bug: None,
    },
    Case {
        name: "if_then_else_false_branch",
        source: "if 1 < 0 then 1 else -1",
        expected: Expected::Value("-1"),
        known_bug: None,
    },
    // --- Parenthesised, semicolon-separated block: "value is the last
    // expression's value" (MA13 §4) -- proven end-to-end via the unroll
    // harness, not just "runs without throwing" (already covered by
    // `tests/e2e_node.rs`'s own `a_multi_statement_block_runs_in_node`). ---
    Case {
        name: "parenthesized_semicolon_block",
        source: "(x := 1; x + 1)",
        expected: Expected::Value("2"),
        known_bug: None,
    },
    // --- `a : T` declaration -- PASSING (structural resolve succeeds;
    // `axiom-runtime::eval_declaration`'s own disclosed presentation
    // convention: echoes `true`, MA13 §3/§4). ---
    Case {
        name: "declaration_of_a_valid_domain_succeeds",
        source: "a : PositiveInteger",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    // --- `a : T` declaration -- FAILING: `T` is not one of the fixed
    // built-in domains (a structural `resolve_domain` rejection, the SAME
    // failure mode on both sides -- no cross-statement state needed). ---
    Case {
        name: "declaration_of_an_unknown_domain_fails_identically",
        source: "a : Matrix",
        expected: Expected::Error("is not one of this cut's fixed built-in domains"),
        known_bug: None,
    },
    // --- `a : T` declaration THEN a mismatched `a := v` -- FAILING, via
    // the book's own confirmed error shape (MA13 §3, quoted verbatim in
    // both `axiom-runtime::eval_assignment` and `assignHandler`'s own
    // disclosed cross-head addition, `runtime.rs`). A `;`-block: the
    // declaration is evaluated bare (populates `axiomDeclaredDomains`),
    // then the mismatched assignment throws BEFORE the unroll harness's
    // own `print(...)` wrapper ever runs -- so this case needed no
    // CompoundExpression handling fix at all to fail correctly (the
    // exception propagates during argument evaluation, never reaching the
    // "no handler for CompoundExpression" fallback path either way). ---
    Case {
        name: "declaration_then_mismatched_assignment_fails_identically",
        source: "(a : PositiveInteger; a := -1)",
        expected: Expected::Error(
            "Cannot convert right-hand side of assignment -1 to an object of the type \
             PositiveInteger of the left-hand side.",
        ),
        known_bug: None,
    },
    // --- `e :: T` coercion -- PASSING (the book's own confirmed example,
    // paren-optional shorthand spelling). ---
    Case {
        name: "coercion_of_an_integer_to_fraction_integer_succeeds",
        source: "3 :: Fraction Integer",
        expected: Expected::Value("3"),
        known_bug: None,
    },
    // --- `e :: T` coercion -- FAILING (the `PositiveInteger` subdomain
    // predicate rejects a negative literal, the book's own confirmed error
    // shape adapted for the standalone `::` case). ---
    Case {
        name: "coercion_of_a_negative_integer_to_positive_integer_fails_identically",
        source: "-1 :: PositiveInteger",
        expected: Expected::Error("Cannot convert -1 to an object of the type PositiveInteger."),
        known_bug: None,
    },
    // --- `D has C` category-membership query -- the book's own TWO
    // confirmed worked examples (MA13 §3/§4), run literally end-to-end
    // through the compiled JS path. ---
    Case {
        name: "polynomial_integer_has_ring_the_books_own_confirmed_true_example",
        source: "Polynomial(Integer) has Ring",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    Case {
        name: "list_integer_has_ring_the_books_own_confirmed_false_example",
        source: "List(Integer) has Ring",
        expected: Expected::Value("false"),
        known_bug: None,
    },
    // --- A couple of extra `has` cases beyond the two book-confirmed ones,
    // spot-checking the rest of the fixed membership table end-to-end. ---
    Case {
        name: "boolean_does_not_have_ring",
        source: "Boolean has Ring",
        expected: Expected::Value("false"),
        known_bug: None,
    },
    Case {
        name: "float_has_ordered_set",
        source: "Float has OrderedSet",
        expected: Expected::Value("true"),
        known_bug: None,
    },
    // --- List literal: elementwise evaluation is correct on both sides,
    // but the bracket display convention is missing on the compiled side
    // (finding three) -- known_bug for DISPLAY only, not evaluation. ---
    Case {
        name: "list_literal_evaluates_elementwise",
        source: "[1 + 1, 2*3]",
        expected: Expected::Value("[2, 6]"),
        known_bug: Some(
            "Finding three (module doc): List(Add(1,1), Mul(2,3)) folds its elements correctly \
             on the compiled side (List has no HANDLERS entry, but evalApply's applicative-order \
             argument evaluation folds each element for free) -- this is NOT an evaluation gap. \
             But the compiled side's generic Symbolic.toDisplayString prints \"List(2, 6)\", never \
             Axiom's own bracket surface \"[2, 6]\" -- no SIR_DISPLAY_AXIOM infix/bracket \
             convention exists (only the minimal boolean-case flag this task adds).",
        ),
    },
];

/// Ground truth: run the WHOLE program through `axiom-runtime`'s own
/// [`AxiomSession::feed`] (a single call, since `source` is always exactly
/// one top-level Axiom statement/block per `axiom.grammar`'s own `program =
/// expr` design), then strip the `"(n) "` prompt-index prefix AND any
/// trailing `" : Domain"` suffix -- `symbolic_ir::IRNode` (and therefore
/// the whole compiled SIR/JS path) has no domain concept at all (MA13 §2's
/// own central finding), so the domain suffix has no compiled-side
/// counterpart to compare against; comparing only the bare VALUE text is
/// the honest, apples-to-apples comparison. Safe for every value this
/// corpus's OWN chosen cases can produce: none of them contains the
/// literal substring `" : "` in its own printed VALUE (only the domain
/// suffix ever does), so splitting on the FIRST occurrence cannot
/// mis-truncate a legitimate value.
fn ground_truth(source: &str) -> Result<String, String> {
    let mut session = AxiomSession::new();
    let raw = session.feed(source)?;
    let trimmed = raw.trim_end_matches('\n');
    let after_prompt = trimmed.split_once(") ").map_or(trimmed, |(_, rest)| rest);
    let value_only = after_prompt.split(" : ").next().unwrap_or(after_prompt);
    Ok(value_only.to_string())
}

/// Harness-only fix (see the module doc's own "A harness-only 'make it
/// observable' step" section): `main`'s ONE `Stmt::ExprStmt` is either an
/// ordinary expression (wrapped in a single `print(...)`, mirroring every
/// sibling oracle file's own `wrap_top_level_in_print`) or a top-level
/// `SymApply(CompoundExpression, [s1, ..., sN])` block, which is instead
/// "unrolled" into N separate statements -- the first N-1 evaluated bare
/// (side effects only, value discarded, exactly like `emit.rs`'s own
/// per-statement `evalTerm` wrap for a bare SIR23 root), the LAST wrapped
/// in `print(...)`. Never touches `semantic-ir-to-javascript` itself.
fn wrap_axiom_top_level_for_observation(module: &mut Module) {
    for f in &mut module.functions {
        if f.name != "main" {
            continue;
        }
        assert_eq!(
            f.body.stmts.len(),
            1,
            "axiom-to-semantic-ir::compile_source always lowers `program` to exactly one \
             top-level Stmt::ExprStmt (its own module doc: \"program is a SINGLE expression\")"
        );
        let Stmt::ExprStmt { expr, span } = f.body.stmts.remove(0) else {
            panic!("main's sole statement must be an ExprStmt");
        };
        let is_compound_expression = matches!(
            &expr,
            Expr::SymApply { head, .. }
                if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "CompoundExpression")
        );
        if is_compound_expression {
            let Expr::SymApply { args, .. } = expr else {
                unreachable!("just matched SymApply above");
            };
            let last_index = args.len().saturating_sub(1);
            let mut new_stmts = Vec::with_capacity(args.len());
            for (i, a) in args.into_iter().enumerate() {
                if i == last_index {
                    new_stmts.push(Stmt::ExprStmt {
                        expr: Expr::BuiltinCall {
                            name: "print".to_string(),
                            args: vec![a],
                            effects: EffectSet::PURE,
                            span: span.clone(),
                        },
                        span: span.clone(),
                    });
                } else {
                    new_stmts.push(Stmt::ExprStmt {
                        expr: a,
                        span: span.clone(),
                    });
                }
            }
            f.body.stmts = new_stmts;
        } else {
            f.body.stmts = vec![Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "print".to_string(),
                    args: vec![expr],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                },
                span,
            }];
        }
    }
}

/// Compiled path: run `source` (unchanged) through
/// `axiom_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// [`wrap_axiom_top_level_for_observation`], `semantic_ir_to_javascript::
/// compile`, and an actual `node` process. Returns `Ok(stdout)` on a clean
/// exit, `Err(stderr)` otherwise -- both a passing-case AND a failing-case
/// corpus entry route through this same function; the caller decides which
/// outcome to expect. Mirrors `macsyma-to-semantic-ir/tests/oracle.rs`'s
/// own `compiled` helper, down to the `OpenOptions::create_new(true)` temp
/// file handling (fails instead of silently following an existing symlink
/// planted at the shared, predictable system temp path).
fn compiled(name: &str, source: &str) -> Result<String, String> {
    let mut module = compile_source(source, "prog")
        .map_err(|e| format!("lowering failed for {name} ({source:?}): {e:?}"))?;
    let report = semantic_ir::validate(&module);
    if !report.is_ok() {
        return Err(format!(
            "SIR validation failed for {name}: {:?}",
            report.issues
        ));
    }
    wrap_axiom_top_level_for_observation(&mut module);
    let artifact = semantic_ir_to_javascript::compile(&module)
        .map_err(|e| format!("backend emit failed for {name}: {e:?}"))?;

    let mut path = std::env::temp_dir();
    path.push(format!(
        "axiom_sir_oracle_{name}_{}.js",
        std::process::id()
    ));
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

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\n', '\r'])
            .to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn matches_expected(result: &Result<String, String>, expected: &Expected) -> bool {
    match (result, expected) {
        (Ok(v), Expected::Value(exp)) => v == exp,
        (Err(e), Expected::Error(sub)) => e.contains(sub),
        _ => false,
    }
}

#[test]
fn oracle_corpus_matches_native_axiom_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_axiom_runtime: `node` not available");
        return;
    }

    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = ground_truth(case.source);
        if !matches_expected(&gt, &case.expected) {
            failures.push(format!(
                "{}: axiom-runtime itself disagrees with this corpus entry's own `expected` \
                 (got {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the \
                 corpus rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        match case.known_bug {
            None => {
                let got = compiled(case.name, case.source);
                if !matches_expected(&got, &case.expected) {
                    failures.push(format!(
                        "{}: axiom-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees \
                         with the axiom-runtime ground truth (got {got:?}, expected {:?}) -- see \
                         this file's module doc for the documented findings before assuming this \
                         is a new one",
                        case.name, case.expected
                    ));
                }
            }
            Some(reason) => {
                // KNOWN BUG: the compiled-side assertion is deliberately
                // skipped (not even invoked) for this entry -- see this
                // file's module doc comment for why, and `reason` for
                // exactly which documented finding applies here.
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
