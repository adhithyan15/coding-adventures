//! End-to-end execution tests: McCarthy source → IIR → run on
//! `mccarthy-lisp-vm`.
//!
//! These prove the headline L2a claim — that the emitted IIR actually
//! *runs* and produces the right Lisp value — using McCarthy Lisp's own
//! VM, which is built on the `lispy-runtime` value model (`vm-core` is
//! scalar-only and cannot represent symbols / cons cells; `twig-vm` is
//! Twig-specific and deliberately not used here).
//!
//! Results are inspected through `lispy-runtime`: symbols via the intern
//! table (`name_of`), cons cells via the `car` / `cdr` builtins.

use dynval_runtime::{name_of, LispyValue};
use mccarthy_lisp_iir_compiler::compile_source;
use mccarthy_lisp_vm::run;

/// Compile + run a McCarthy program, returning the result value.
fn eval(src: &str) -> LispyValue {
    let module = compile_source(src, "e2e").unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    run(&module).unwrap_or_else(|e| panic!("run {src:?}: {e}"))
}

/// The human-readable name of a symbol value.
fn sym_name(v: LispyValue) -> String {
    let id = v.as_symbol().unwrap_or_else(|| panic!("expected a symbol, got {v}"));
    name_of(id).unwrap_or_else(|| panic!("symbol id {id} has no interned name"))
}

/// car / cdr via the runtime builtins (so tests don't reach into heap unsafe).
fn car(v: LispyValue) -> LispyValue {
    dynval_runtime::builtins::car(&[v]).expect("car")
}
fn cdr(v: LispyValue) -> LispyValue {
    dynval_runtime::builtins::cdr(&[v]).expect("cdr")
}

// ============================================================
// Literals
// ============================================================

#[test]
fn integer_literal() {
    assert_eq!(eval("42").as_int(), Some(42));
    assert_eq!(eval("-7").as_int(), Some(-7));
    assert_eq!(eval("0").as_int(), Some(0));
}

#[test]
fn empty_list_is_nil() {
    assert!(eval("'()").is_nil());
}

#[test]
fn quoted_symbol() {
    assert_eq!(sym_name(eval("'FOO")), "FOO");
}

// ============================================================
// The canonical McCarthy examples
// ============================================================

#[test]
fn car_of_a_literal_list() {
    // (CAR '(A B C)) → A
    assert_eq!(sym_name(eval("(CAR '(A B C))")), "A");
}

#[test]
fn cdr_of_a_literal_list() {
    // (CDR '(A B C)) → (B C)
    let rest = eval("(CDR '(A B C))");
    assert_eq!(sym_name(car(rest)), "B");
    assert_eq!(sym_name(car(cdr(rest))), "C");
    assert!(cdr(cdr(rest)).is_nil());
}

#[test]
fn cadr_composition() {
    // (CAR (CDR '(A B C))) → B
    assert_eq!(sym_name(eval("(CAR (CDR '(A B C)))")), "B");
}

#[test]
fn cons_builds_a_pair() {
    // (CONS 'A 'B) → (A . B)
    let p = eval("(CONS 'A 'B)");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "B");
}

#[test]
fn cons_onto_a_list() {
    // (CONS 'A '(B C)) → (A B C)
    let p = eval("(CONS 'A '(B C))");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(car(cdr(p))), "B");
    assert_eq!(sym_name(car(cdr(cdr(p)))), "C");
}

// ============================================================
// Predicates
// ============================================================

#[test]
fn atom_of_a_symbol_is_true() {
    assert!(eval("(ATOM 'X)").is_true());
}

#[test]
fn atom_of_a_list_is_false() {
    assert!(eval("(ATOM '(A))").is_false());
}

#[test]
fn atom_of_nil_is_true() {
    // NIL is an atom in McCarthy's algebra.
    assert!(eval("(ATOM '())").is_true());
}

#[test]
fn eq_of_equal_symbols_is_true() {
    assert!(eval("(EQ 'A 'A)").is_true());
}

#[test]
fn eq_of_different_symbols_is_false() {
    assert!(eval("(EQ 'A 'B)").is_false());
}

// ============================================================
// Nesting + sequencing
// ============================================================

#[test]
fn nested_quote_builds_nested_structure() {
    // (CAR '((A B) C)) → (A B)
    let head = eval("(CAR '((A B) C))");
    assert_eq!(sym_name(car(head)), "A");
    assert_eq!(sym_name(car(cdr(head))), "B");
}

#[test]
fn last_form_is_the_program_value() {
    // A sequence of forms returns the value of the last.
    assert_eq!(sym_name(eval("'A 'B 'C")), "C");
}

#[test]
fn dotted_pair_literal() {
    // (CDR '(A . B)) → B
    assert_eq!(sym_name(eval("(CDR '(A . B))")), "B");
}

// ============================================================
// COND (L2b)
// ============================================================

#[test]
fn cond_first_true_clause_wins() {
    // 'X is an atom → first clause fires → 'A.
    assert_eq!(sym_name(eval("(COND ((ATOM 'X) 'A) ('T 'B))")), "A");
}

#[test]
fn cond_falls_through_to_later_clause() {
    // '(X) is a cons (not an atom) → first clause false → catch-all → 'B.
    assert_eq!(sym_name(eval("(COND ((ATOM '(X)) 'A) ('T 'B))")), "B");
}

#[test]
fn cond_uses_eq_predicate() {
    assert_eq!(sym_name(eval("(COND ((EQ 'A 'A) 'YES) ('T 'NO))")), "YES");
    assert_eq!(sym_name(eval("(COND ((EQ 'A 'B) 'YES) ('T 'NO))")), "NO");
}

#[test]
fn cond_with_no_matching_clause_is_nil() {
    // No clause's predicate is true → the total extension returns nil.
    assert!(eval("(COND ((EQ 'A 'B) 'YES))").is_nil());
}

#[test]
fn cond_value_feeds_an_enclosing_form() {
    // (CAR (COND ('T '(A B C)))) → A
    assert_eq!(sym_name(eval("(CAR (COND ('T '(A B C))))")), "A");
}

#[test]
fn nested_cond() {
    // Inner COND yields 'B (since '(X) is not an atom), outer matches on EQ.
    let src = "(COND ((EQ (COND ((ATOM '(X)) 'A) ('T 'B)) 'B) 'MATCHED) ('T 'NOPE))";
    assert_eq!(sym_name(eval(src)), "MATCHED");
}

#[test]
fn cond_returns_a_list_value() {
    // A clause expression can be any expression, including one that
    // builds a cons structure.
    let v = eval("(COND ('T (CONS 'A 'B)))");
    assert_eq!(sym_name(car(v)), "A");
    assert_eq!(sym_name(cdr(v)), "B");
}

// ============================================================
// LAMBDA application (L2c-1)
// ============================================================

#[test]
fn identity_lambda_on_a_symbol() {
    assert_eq!(sym_name(eval("((LAMBDA (X) X) 'A)")), "A");
}

#[test]
fn identity_lambda_on_an_int() {
    assert_eq!(eval("((LAMBDA (N) N) 42)").as_int(), Some(42));
}

#[test]
fn lambda_takes_car_of_its_argument() {
    assert_eq!(sym_name(eval("((LAMBDA (X) (CAR X)) '(A B))")), "A");
}

#[test]
fn two_parameter_lambda() {
    // ((LAMBDA (X Y) (CONS X Y)) 'A 'B) → (A . B)
    let p = eval("((LAMBDA (X Y) (CONS X Y)) 'A 'B)");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "B");
}

#[test]
fn lambda_uses_its_param_twice() {
    // ((LAMBDA (X) (CONS X X)) 'A) → (A . A)
    let p = eval("((LAMBDA (X) (CONS X X)) 'A)");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "A");
}

#[test]
fn lambda_body_can_use_cond() {
    assert_eq!(sym_name(eval("((LAMBDA (X) (COND ((ATOM X) 'YES) ('T 'NO))) 'Q)")), "YES");
    assert_eq!(sym_name(eval("((LAMBDA (X) (COND ((ATOM X) 'YES) ('T 'NO))) '(Q))")), "NO");
}

#[test]
fn argument_is_itself_a_lambda_application() {
    // Outer identity applied to the result of an inner lambda.
    // ((LAMBDA (X) X) ((LAMBDA (Y) (CAR Y)) '(P Q))) → P
    assert_eq!(sym_name(eval("((LAMBDA (X) X) ((LAMBDA (Y) (CAR Y)) '(P Q)))")), "P");
}

#[test]
fn lambda_result_feeds_a_primitive() {
    // (CDR ((LAMBDA (X) X) '(A B C))) → (B C); car of that → B
    assert_eq!(sym_name(eval("(CAR (CDR ((LAMBDA (X) X) '(A B C))))")), "B");
}

// ============================================================
// LABEL — named / recursive functions (L2c-2)
// ============================================================

#[test]
fn label_identity_applied() {
    // A trivial (non-recursive) labelled lambda still works.
    assert_eq!(sym_name(eval("((LABEL F (LAMBDA (X) X)) 'A)")), "A");
}

#[test]
fn label_ff_first_atom_mccarthy_canonical() {
    // McCarthy's `ff`: the first atom found by descending `car`s.
    //   ff[x] = [atom[x] → x; T → ff[car[x]]]
    // ff[((A B) C)] descends (A B) then A → A.
    let src = "((LABEL FF (LAMBDA (X) \
                  (COND ((ATOM X) X) \
                        ('T (FF (CAR X)))))) \
                '((A B) C))";
    assert_eq!(sym_name(eval(src)), "A");
}

#[test]
fn label_last_walks_the_cdr_spine() {
    // last[l] = [atom[cdr[l]] → car[l]; T → last[cdr[l]]]
    // last[(A B C)] → C (nil is an atom, so it stops at the final cell).
    let src = "((LABEL LAST (LAMBDA (L) \
                  (COND ((ATOM (CDR L)) (CAR L)) \
                        ('T (LAST (CDR L)))))) \
                '(A B C))";
    assert_eq!(sym_name(eval(src)), "C");
}

#[test]
fn label_single_element_base_case() {
    // last[(A)] → A immediately: cdr[(A)] = nil, atom[nil] = T.
    let src = "((LABEL LAST (LAMBDA (L) \
                  (COND ((ATOM (CDR L)) (CAR L)) \
                        ('T (LAST (CDR L)))))) \
                '(A))";
    assert_eq!(sym_name(eval(src)), "A");
}

#[test]
fn label_nonterminating_recursion_errors_cleanly() {
    // A self-call that never shrinks its argument must hit the VM's
    // call-depth guard (a clean error), never a native stack overflow.
    let src = "((LABEL LOOP (LAMBDA (X) (LOOP X))) 'A)";
    let module = compile_source(src, "e2e").expect("compile");
    let err = run(&module).expect_err("must not terminate");
    // CallDepthExceeded (or, defensively, the instruction-budget backstop).
    let msg = err.to_string();
    assert!(
        msg.contains("call depth") || msg.contains("instruction budget"),
        "unexpected error: {msg}"
    );
}

// ============================================================
// Closures as values + dynamic apply (L2c-3a)
// ============================================================

#[test]
fn higher_order_apply_identity() {
    // Pass identity as a value, then apply the parameter to a datum.
    //   ((LAMBDA (F) (F 'A)) (LAMBDA (X) X)) ⇒ A
    assert_eq!(sym_name(eval("((LAMBDA (F) (F 'A)) (LAMBDA (X) X))")), "A");
}

#[test]
fn higher_order_apply_a_primitive_closure() {
    // The passed closure runs a primitive in its body.
    //   ((LAMBDA (F) (F '(A B))) (LAMBDA (X) (CAR X))) ⇒ A
    assert_eq!(sym_name(eval("((LAMBDA (F) (F '(A B))) (LAMBDA (X) (CAR X)))")), "A");
}

#[test]
fn closure_returned_then_applied() {
    // A lambda returns another lambda (no free vars), which is then applied:
    //   (((LAMBDA (X) (LAMBDA (Y) Y)) 'Z) 'W) ⇒ W
    // The head of the outer call is itself an application returning a
    // closure, so it lowers to a dynamic `apply`.
    assert_eq!(sym_name(eval("(((LAMBDA (X) (LAMBDA (Y) Y)) 'Z) 'W)")), "W");
}

#[test]
fn bare_lambda_value_is_a_closure_pair() {
    // A LAMBDA standing alone is now a value: a (*CLOSURE* …) pair, whose
    // car is the un-forgeable tag symbol.
    let v = eval("(LAMBDA (X) X)");
    assert_eq!(sym_name(car(v)), "*CLOSURE*");
}

#[test]
fn applying_a_non_closure_is_a_clean_runtime_error() {
    // `('FOO 'A)` — the head evaluates to a symbol, not a closure.  It must
    // be a clean NotAClosure error, never a panic.
    let module = compile_source("('FOO 'A)", "e2e").expect("compile");
    let err = run(&module).expect_err("applying a symbol must fail");
    assert!(err.to_string().contains("non-closure"), "unexpected: {err}");
}

#[test]
fn omega_combinator_terminates_with_a_depth_error() {
    // ((LAMBDA (X) (X X)) (LAMBDA (X) (X X))) — the Ω combinator: it
    // type-checks in untyped Lisp and self-applies forever.  Lowered with
    // closures + `apply`, it must hit the call-depth guard (clean error),
    // never overflow the native stack.
    let module = compile_source("((LAMBDA (X) (X X)) (LAMBDA (X) (X X)))", "e2e").expect("compile");
    let err = run(&module).expect_err("Ω must not terminate");
    let msg = err.to_string();
    assert!(
        msg.contains("call depth") || msg.contains("instruction budget"),
        "unexpected error: {msg}"
    );
}

// ============================================================
// Free-variable capture (L2c-3b)
// ============================================================

#[test]
fn closure_captures_then_applied_later() {
    // Curry: the inner lambda captures X, is returned as a closure, and is
    // applied later — the captured X must survive in the closure's env.
    //   (((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B) ⇒ (A . B)
    let p = eval("(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "B");
}

#[test]
fn direct_application_captures_enclosing_param() {
    // The inner lambda is directly applied (not returned), still capturing X.
    //   ((LAMBDA (X) ((LAMBDA (Y) (CONS X Y)) 'B)) 'A) ⇒ (A . B)
    let p = eval("((LAMBDA (X) ((LAMBDA (Y) (CONS X Y)) 'B)) 'A)");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "B");
}

#[test]
fn transitive_capture_through_two_levels() {
    // The innermost lambda references X (outermost) and Z (middle) — both
    // must thread through the lifting chain.
    //   ((LAMBDA (X) ((LAMBDA (Z) ((LAMBDA (Y) (CONS X (CONS Z Y))) '()))
    //                 'B)) 'A) ⇒ (A B)
    let p = eval(
        "((LAMBDA (X) ((LAMBDA (Z) ((LAMBDA (Y) (CONS X (CONS Z Y))) '())) 'B)) 'A)",
    );
    assert_eq!(sym_name(car(p)), "A"); // X
    assert_eq!(sym_name(car(cdr(p))), "B"); // Z
    assert!(cdr(cdr(p)).is_nil());
}

#[test]
fn captured_closure_passed_to_a_higher_order_function() {
    // Build an adder-ish closure that captures X, pass it to a function
    // that applies it.
    //   ((LAMBDA (G) (G 'B)) ((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A))
    //     ⇒ (A . B)
    let p = eval("((LAMBDA (G) (G 'B)) ((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A))");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "B");
}

#[test]
fn shadowing_param_is_not_captured() {
    // The inner lambda's own param X shadows the outer X — the body sees the
    // inner X, not the captured one.
    //   ((LAMBDA (X) ((LAMBDA (X) X) 'INNER)) 'OUTER) ⇒ INNER
    assert_eq!(sym_name(eval("((LAMBDA (X) ((LAMBDA (X) X) 'INNER)) 'OUTER)")), "INNER");
}

// ============================================================
// LABEL capture + LABEL-as-value: recursive closures (L2c-3c)
// ============================================================

#[test]
fn label_body_captures_enclosing_variable() {
    // The recursive `F` returns the captured `N` at its base case.
    //   ((LAMBDA (N) ((LABEL F (LAMBDA (X)
    //       (COND ((ATOM X) N) ('T (F (CAR X)))))) '((A) B))) 'Z) ⇒ Z
    let src = "((LAMBDA (N) ((LABEL F (LAMBDA (X) \
                 (COND ((ATOM X) N) ('T (F (CAR X)))))) '((A) B))) 'Z)";
    assert_eq!(sym_name(eval(src)), "Z");
}

#[test]
fn recursive_label_passed_as_a_value_and_applied() {
    // Pass a recursive LABEL (last) as a value to a higher-order function,
    // which applies it.
    //   ((LAMBDA (G) (G '(A B C)))
    //    (LABEL LAST (LAMBDA (L)
    //        (COND ((ATOM (CDR L)) (CAR L)) ('T (LAST (CDR L))))))) ⇒ C
    let src = "((LAMBDA (G) (G '(A B C))) \
                (LABEL LAST (LAMBDA (L) \
                    (COND ((ATOM (CDR L)) (CAR L)) ('T (LAST (CDR L)))))))";
    assert_eq!(sym_name(eval(src)), "C");
}

#[test]
fn recursive_label_value_with_capture() {
    // A recursive LABEL that *captures* an enclosing var, used as a value:
    // descends to the first atom and returns the captured STOP.
    //   ((LAMBDA (STOP) ((LAMBDA (G) (G '(A B)))
    //       (LABEL F (LAMBDA (X)
    //           (COND ((ATOM X) STOP) ('T (F (CAR X)))))))) 'DONE) ⇒ DONE
    let src = "((LAMBDA (STOP) ((LAMBDA (G) (G '(A B))) \
                 (LABEL F (LAMBDA (X) \
                     (COND ((ATOM X) STOP) ('T (F (CAR X)))))))) 'DONE)";
    assert_eq!(sym_name(eval(src)), "DONE");
}

#[test]
fn nonterminating_label_value_hits_depth_guard() {
    // A non-terminating recursive LABEL used as a value must still hit the
    // call-depth guard when applied — never a native stack overflow.
    //   ((LAMBDA (G) (G 'A)) (LABEL LOOP (LAMBDA (X) (LOOP X))))
    let src = "((LAMBDA (G) (G 'A)) (LABEL LOOP (LAMBDA (X) (LOOP X))))";
    let module = compile_source(src, "e2e").expect("compile");
    let err = run(&module).expect_err("must not terminate");
    let msg = err.to_string();
    assert!(
        msg.contains("call depth") || msg.contains("instruction budget"),
        "unexpected error: {msg}"
    );
}

#[test]
fn nested_label_calls_outer_captured_label() {
    // Regression: an inner LABEL `G` calls an outer LABEL `F` that captured
    // an enclosing `N`, but `G`'s own body doesn't mention `N`.  `F`'s
    // recursive-call forwarding needs `N` live inside `G`, so `G` must
    // transitively capture it.  Descends ((A) B) → atom → returns N=Z.
    let src = "((LAMBDA (N) \
                  ((LABEL F (LAMBDA (X) \
                     (COND ((ATOM X) N) \
                           ('T ((LABEL G (LAMBDA (Y) (F Y))) (CAR X)))))) \
                   '((A) B))) \
                'Z)";
    assert_eq!(sym_name(eval(src)), "Z");
}
