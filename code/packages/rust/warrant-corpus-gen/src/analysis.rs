//! # Static rules over a lowered tree
//!
//! Two VER04r rules that need no evaluation, only the tree and the types the lowering
//! recorded on it:
//!
//! * **Pure guards (4.4).**  `expected_calls` must not depend on what the subject returned,
//!   so a subject call may sit under a guard (the condition of an `if`, the left operand
//!   guarding the right of `and`/`or`/`implies`) only if the guard itself calls no subject.
//!
//! * **The v0 divergence guard, rule 3 (3.3).**  A frozen v0 document's Rust harness
//!   evaluates `U8` as `Nat`, so a fixed-width `+ - *` or unary `-` whose value depends on a
//!   subject result could wrap in a lane and not in the harness.  No generation-time count
//!   can prove the two agree, so it is refused statically.  (Rule 2, "`wraps > 0` is an
//!   error", needs the evaluator: [`crate::arith::check_v0_wraps`] states it and S3b feeds
//!   it.)
//!
//! Both are `W0701`: the document is well typed, the corpus cannot express it honestly.

use crate::error::{code, GenError};
use crate::ir::Ir;

/// Refuse a subject call under an impure guard (VER04r 4.4).
///
/// ```text
///   if pure(x) then op(a) else true          ok   -- the guard calls no subject
///   if op(x) then op(a) else true            W0701
///   op(a) and op(b)                          W0701 -- op(a) guards op(b) and is impure
///   pure(x) and op(b)                        ok
///   op(a) and pure(b)                        ok   -- op(a) guards nothing that calls
/// ```
///
/// # Errors
///
/// `W0701` on the first violation, left to right.
pub fn check_pure_guards(ir: &Ir) -> Result<(), GenError> {
    if !ir.has_subject() {
        return Ok(());
    }
    let impure = || {
        GenError::new(
            code::W0701,
            "",
            "a subject call under an impure guard (VER04r 4.4)",
        )
    };
    match ir {
        Ir::If(c, t, e) => {
            if t.has_subject() || e.has_subject() {
                if c.has_subject() {
                    return Err(impure());
                }
                check_pure_guards(t)?;
                check_pure_guards(e)
            } else {
                check_pure_guards(c)
            }
        }
        Ir::Logic(_, l, r) => {
            if r.has_subject() {
                if l.has_subject() {
                    return Err(impure());
                }
                check_pure_guards(r)
            } else {
                check_pure_guards(l)
            }
        }
        other => other.children().into_iter().try_for_each(check_pure_guards),
    }
}

/// Refuse, in a **v0** document, a fixed-width `+ - *` or unary `-` with a `subject_op`
/// beneath it (VER04r 3.3 rule 3).  The caller applies this only to v0 documents; a v1
/// document wraps everywhere and the rule is moot (rule 5).
///
/// # Errors
///
/// `W0701` naming the type.
pub fn check_v0_static_arith(ir: &Ir) -> Result<(), GenError> {
    let mut bad: Option<String> = None;
    ir.walk(&mut |n| {
        let ty = match n {
            Ir::Arith(_, ty, ..) | Ir::Neg(ty, _) => ty,
            _ => return,
        };
        if bad.is_none() && ty.is_fixed() && n.has_subject() {
            bad = Some(ty.name());
        }
    });
    match bad {
        None => Ok(()),
        Some(ty) => Err(GenError::new(
            code::W0701,
            "",
            format!(
                "{ty} arithmetic over a subject result in a v0 document: the v0 harness \
                 evaluates U8 as Nat, so the wrapping cannot be proven to agree (VER04r 3.3 rule 3)"
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Arith, Cmp, Logic};
    use crate::types::Ty;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    fn subj(op: &str) -> Box<Ir> {
        Box::new(Ir::Subject {
            op: op.into(),
            args: vec![],
            widths: vec![],
        })
    }
    fn var(n: &str) -> Box<Ir> {
        Box::new(Ir::Var(n.into()))
    }

    #[test]
    fn a_pure_guard_over_a_subject_call_is_fine() {
        let ok = Ir::If(
            Box::new(Ir::Cmp(Cmp::Ne, var("b"), var("c"))),
            subj("f"),
            Box::new(Ir::Bool(true)),
        );
        assert!(check_pure_guards(&ok).is_ok());
        let and = Ir::Logic(
            Logic::And,
            Box::new(Ir::Cmp(Cmp::Ne, var("b"), var("c"))),
            subj("f"),
        );
        assert!(check_pure_guards(&and).is_ok());
    }

    #[test]
    fn an_impure_if_condition_is_w0701() {
        let bad = Ir::If(subj("g"), subj("f"), Box::new(Ir::Bool(true)));
        let e = check_pure_guards(&bad).unwrap_err();
        assert_eq!(e.code, code::W0701);
        // the else branch counts as guarded too
        let bad_else = Ir::If(subj("g"), Box::new(Ir::Bool(true)), subj("f"));
        assert_eq!(check_pure_guards(&bad_else).unwrap_err().code, code::W0701);
    }

    #[test]
    fn an_impure_left_operand_guarding_a_subject_right_is_w0701_for_all_three_connectives() {
        for l in [Logic::And, Logic::Or, Logic::Implies] {
            let bad = Ir::Logic(l, subj("g"), subj("f"));
            assert_eq!(
                check_pure_guards(&bad).unwrap_err().code,
                code::W0701,
                "{l:?}"
            );
        }
    }

    #[test]
    fn an_impure_operand_that_guards_nothing_is_fine() {
        // the subject is on the left only: nothing it guards calls the subject
        let ok = Ir::Logic(Logic::And, subj("g"), Box::new(Ir::Bool(true)));
        assert!(check_pure_guards(&ok).is_ok());
        // `if` whose branches are pure: the (impure) condition is just an operand
        let ok_if = Ir::If(
            subj("g"),
            Box::new(Ir::Bool(true)),
            Box::new(Ir::Bool(false)),
        );
        assert!(check_pure_guards(&ok_if).is_ok());
    }

    #[test]
    fn the_rule_applies_inside_arguments_and_nested_nodes() {
        let inner = Ir::If(subj("g"), subj("f"), Box::new(Ir::Bool(true)));
        let outer = Ir::Subject {
            op: "h".into(),
            args: vec![inner.clone()],
            widths: vec![],
        };
        assert_eq!(check_pure_guards(&outer).unwrap_err().code, code::W0701);
        let in_cmp = Ir::Cmp(Cmp::Eq, Box::new(inner), var("x"));
        assert_eq!(check_pure_guards(&in_cmp).unwrap_err().code, code::W0701);
        // no subject anywhere: trivially fine
        assert!(check_pure_guards(&Ir::Not(var("x"))).is_ok());
    }

    #[test]
    fn v0_rule_3_refuses_fixed_arithmetic_over_a_subject_result_only() {
        let u8t = Ty::Fixed(FixedTy::U8);
        let over_subject = Ir::Arith(Arith::Add, u8t.clone(), subj("f"), var("b"));
        let e = check_v0_static_arith(&over_subject).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("U8"));
        // nested above the arithmetic node: the node still has the subject beneath it
        let wrapped = Ir::Cmp(Cmp::Eq, Box::new(over_subject), var("c"));
        assert!(check_v0_static_arith(&wrapped).is_err());
        // negation counts too
        let neg = Ir::Neg(Ty::Fixed(FixedTy::I8), subj("f"));
        assert!(check_v0_static_arith(&neg).is_err());
        // exact arithmetic is not wrapping arithmetic
        let exact = Ir::Arith(Arith::Add, Ty::Int, subj("f"), var("b"));
        assert!(check_v0_static_arith(&exact).is_ok());
        // pure fixed-width arithmetic (a subject-independent node) is counted by `wraps`
        // at S3b, not refused here, even when the subject is elsewhere in the claim
        let pure = Ir::Cmp(
            Cmp::Eq,
            subj("f"),
            Box::new(Ir::Arith(Arith::Add, u8t, var("a"), var("b"))),
        );
        assert!(check_v0_static_arith(&pure).is_ok());
    }
}
