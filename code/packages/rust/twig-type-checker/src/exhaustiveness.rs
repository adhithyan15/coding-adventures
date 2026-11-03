//! Match-arm exhaustiveness checking for `(match scrutinee arm+)` expressions.
//!
//! ## What is exhaustiveness?
//!
//! A `match` expression is *exhaustive* when every possible value of the
//! scrutinee is covered by at least one arm.  In Twig, when the scrutinee is
//! a union type like `(union Expr (IntLit …) (NameRef …))`, an exhaustive
//! match must either:
//!
//! 1. Have a `Variant` arm for every variant: `(IntLit …)` and `(NameRef …)`.
//! 2. Have a `_` wildcard arm (matches anything).
//! 3. Have a bare-name binding arm like `(x body)` (also matches anything).
//!
//! If some variants are not covered and there's no wildcard/binding arm, the
//! program has a logic error: calling `match` with an uncovered value at
//! runtime would fall through to `nil` silently.
//!
//! ## Example of non-exhaustive match
//!
//! ```scheme
//! (union Shape (Circle (r : Int)) (Rect (w : Int) (h : Int)) (Triangle (b : Int) (h : Int)))
//!
//! ;; Missing Triangle arm — TW05-B reports this:
//! (match shape
//!   ((Circle r) (* 3 (* r r)))
//!   ((Rect w h) (* w h)))
//! ;; error: non-exhaustive match on union `Shape`: unmatched variants: `Triangle`
//! ```
//!
//! ## When does exhaustiveness NOT fire?
//!
//! - The scrutinee's kind is `Any` — the checker can't determine the union
//!   type, so it stays silent.
//! - The union name doesn't appear in `env.unions` — unusual but possible if
//!   the scrutinee is passed as a parameter; again, silent.
//! - There is a `Wildcard` or `Binding` arm — by definition exhaustive.
//!
//! ## Complexity
//!
//! O(|arms| × |variants|) — both sets are tiny in practice.

use std::collections::HashSet;

use twig_parser::{MatchArm, MatchPat};
use type_checker_protocol::bounded::{diagnostics_closed, name_list, push_bounded, MAX_LISTED_NAMES};
use type_checker_protocol::TypeErrorDiagnostic;

use crate::env::TypeEnv;

/// Check that every variant of `union_name` is covered by `arms`.
///
/// If the match is exhaustive, nothing is appended to `errors`.
/// If some variants are uncovered and no wildcard/binding arm exists,
/// one `TypeErrorDiagnostic` is appended listing all missing variants.
///
/// `line` and `column` come from the `Match` node so the error underlines
/// the `(match …)` form, not an individual arm.
pub fn check_exhaustiveness(
    union_name: &str,
    arms: &[MatchArm],
    env: &TypeEnv,
    line: usize,
    column: usize,
    errors: &mut Vec<TypeErrorDiagnostic>,
) {
    // A diagnostic nobody will keep is not worth building (#18474 review:
    // the diagnostics are capped, see `type_checker_protocol::bounded`).
    if diagnostics_closed(errors) {
        return;
    }

    // Look up the complete variant set for this union.
    let all_variants = match env.unions.get(union_name) {
        Some(v) => v,
        // Union isn't registered — the checker silently skips rather than
        // cascading a spurious "non-exhaustive" error on top of an unresolved
        // union-name error elsewhere.
        None => return,
    };

    // Walk through each arm and decide whether the match is exhaustive.
    //
    // We use an early-return as soon as we see a wildcard or binding arm,
    // because those catch-all patterns make the match trivially exhaustive
    // regardless of which variants were listed before them.
    let mut covered: HashSet<&str> = HashSet::new();

    for arm in arms {
        match &arm.pat {
            MatchPat::Wildcard | MatchPat::Binding(_) => {
                // Catch-all arm → exhaustive.
                return;
            }
            MatchPat::Variant { name, .. } => {
                covered.insert(name.as_str());
            }
        }
    }

    // Find variants that were never covered, keeping only the few the
    // message will name (#18474 security review).  The message used to list
    // *every* missing variant, so each zero-arm `(match x)` on a 60,000-
    // variant union formatted ~600 KB, and 50,000 of them ~33 GB.  Now one
    // pass counts the missing variants and keeps the first
    // `MAX_LISTED_NAMES`; the message says "… and K more".  The pass is
    // O(variants) time and O(1) memory, and runs at most
    // `MAX_DIAGNOSTICS` + 1 times per program (the `diagnostics_closed` check above).
    let mut missing = all_variants.iter().filter(|v| !covered.contains(v.as_str()));
    let listed: Vec<&str> = missing.by_ref().take(MAX_LISTED_NAMES).map(String::as_str).collect();
    if listed.is_empty() {
        // Every variant was covered — the match is exhaustive.
        return;
    }
    let total = listed.len() + missing.count();
    let names = name_list(&listed, total);

    push_bounded(
        errors,
        TypeErrorDiagnostic {
            message: format!(
                "non-exhaustive match on union `{union_name}`: unmatched variants: {names}"
            ),
            line,
            column,
        },
    );
}
