//! Bounded type-checker diagnostics (#18474 security review).
//!
//! Each non-exhaustive `match` on a union-typed scrutinee used to list
//! *every* unmatched variant, so ~60,000 variants × ~50,000 one-arm
//! matches asked for ~33 GB of messages. The list is now cut to
//! `MAX_LISTED_NAMES` names plus "… and K more", and a program keeps at
//! most `MAX_DIAGNOSTICS` diagnostics (then one note) and
//! `MAX_DIAGNOSTIC_BYTES` of messages.
//!
//! Both checkers are covered: the `Program` checker (`check_program`) for
//! exhaustiveness and the count cap, and the grammar checker behind
//! `type_check_source` for the count cap (its exhaustiveness check does not
//! fire on Twig sources today; `grammar-type-checker`'s own tests cover its
//! list truncation).

use twig_parser::{parse, TypedMode};
use twig_type_checker::{check_program, type_check_source};
use type_checker_protocol::bounded::{
    DIAGNOSTICS_TRUNCATED, MAX_DIAGNOSTICS, MAX_DIAGNOSTIC_BYTES, MAX_DIAGNOSTIC_MESSAGE_BYTES,
    MAX_LISTED_NAMES,
};
use type_checker_protocol::TypeErrorDiagnostic;

/// A union of `variants` empty variants, and a function whose `U`-typed
/// parameter is matched `matches` times by `(match c ((V0) 0))`, each
/// missing every other variant. (The annotation is what gives the
/// scrutinee a union kind.)
fn program(variants: usize, matches: usize) -> String {
    let variants: String = (0..variants).map(|i| format!(" (V{i})")).collect();
    format!(
        "(module m (typed strict))\n(union U{variants})\n(define (f (c : U))\n(begin\n{}))\n",
        "(match c ((V0) 0))\n".repeat(matches)
    )
}

fn program_errors(src: &str) -> Vec<TypeErrorDiagnostic> {
    check_program(&parse(src).unwrap(), Some(TypedMode::Strict)).errors
}

fn grammar_errors(src: &str) -> Vec<TypeErrorDiagnostic> {
    type_check_source(src).unwrap().errors
}

#[test]
fn a_missing_variant_list_at_the_limit_is_whole_and_one_more_is_summarised() {
    // MAX_LISTED_NAMES missing (V1..=V8): all named.
    let at = &program_errors(&program(MAX_LISTED_NAMES + 1, 1))[0].message;
    assert!(at.ends_with("`V8`"), "{at}");
    assert!(!at.contains("more"), "{at}");
    // One more: the first MAX_LISTED_NAMES, then a count.
    let over = &program_errors(&program(MAX_LISTED_NAMES + 2, 1))[0].message;
    assert!(over.ends_with("`V8`, … and 1 more"), "{over}");

}

/// `n` unresolved names in a strict module: one diagnostic each.
fn unresolved(n: usize) -> String {
    let names: String = (0..n).map(|i| format!("x{i}\n")).collect();
    format!("(module m (typed strict))\n{names}")
}

#[test]
fn exactly_max_diagnostics_are_kept_and_one_more_adds_a_single_note() {
    for errors in [
        program_errors(&program(3, MAX_DIAGNOSTICS)),
        program_errors(&unresolved(MAX_DIAGNOSTICS)),
        grammar_errors(&unresolved(MAX_DIAGNOSTICS)),
    ] {
        assert_eq!(errors.len(), MAX_DIAGNOSTICS);
        assert!(errors.iter().all(|e| e.message != DIAGNOSTICS_TRUNCATED));
    }
    for errors in [
        program_errors(&program(3, MAX_DIAGNOSTICS + 1)),
        program_errors(&unresolved(MAX_DIAGNOSTICS + 1)),
        grammar_errors(&unresolved(MAX_DIAGNOSTICS + 1)),
    ] {
        assert_eq!(errors.len(), MAX_DIAGNOSTICS + 1);
        assert_eq!(errors.last().unwrap().message, DIAGNOSTICS_TRUNCATED);
    }
}

/// The review's shape, scaled down: 20,000 variants and 5,000 one-arm
/// matches. Before the fix each message listed 19,999 variants (~180 KB),
/// ~900 MB in all. Now the checker keeps 256 short messages and one note,
/// and the grammar checker keeps 256 of 20,000 unresolved names.
#[test]
fn many_errors_stay_within_the_byte_budget() {
    let src = program(20_000, 5_000);
    for errors in [program_errors(&src), grammar_errors(&unresolved(20_000))] {
        assert_eq!(errors.len(), MAX_DIAGNOSTICS + 1);
        let bytes: usize = errors.iter().map(|e| e.message.len()).sum();
        assert!(bytes <= MAX_DIAGNOSTIC_BYTES, "{bytes}");
        assert!(errors.iter().all(|e| e.message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES));
    }
    let first = &program_errors(&src)[0].message;
    assert!(first.ends_with("… and 19991 more"), "{first}");
}
