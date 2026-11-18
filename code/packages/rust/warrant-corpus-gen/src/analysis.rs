//! # Static rules over a lowered tree
//!
//! Two VER04r rules that need no evaluation, only the tree and the types the lowering
//! recorded on it:
//!
//! * **Pure guards (4.4).**  `expected_calls` must not depend on what the subject returned,
//!   so a subject call may sit under a guard (the condition of an `if`, the left operand
//!   guarding the right of `and`/`or`/`implies`, the scrutinee of a `match` whose arm calls)
//!   only if the guard itself calls no subject.
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
    if ir.depth() > crate::ir::MAX_IR_DEPTH {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "IR nesting exceeds 64 levels",
        ));
    }
    guards(ir)
}

fn guards(ir: &Ir) -> Result<(), GenError> {
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
                guards(t)?;
                guards(e)
            } else {
                guards(c)
            }
        }
        Ir::Logic(_, l, r) => {
            if r.has_subject() {
                if l.has_subject() {
                    return Err(impure());
                }
                guards(r)
            } else {
                guards(l)
            }
        }
        // A `match` arm is a guard whose condition is the scrutinee (VER04r 4.4): a subject call in
        // an arm needs a pure scrutinee.  With no call in any arm only the scrutinee is of interest.
        Ir::Match { scrutinee, arms } => {
            if arms.iter().any(|(_, body)| body.has_subject()) {
                if scrutinee.has_subject() {
                    return Err(impure());
                }
                arms.iter().try_for_each(|(_, body)| guards(body))
            } else {
                guards(scrutinee)
            }
        }
        other => other.children().into_iter().try_for_each(guards),
    }
}

/// Refuse, in a **v0** document, a fixed-width `+ - *` or unary `-` with a `subject_op`
/// beneath it (VER04r 3.3 rule 3), or a `model_call` with a `subject_op` beneath it whose
/// callee's call closure contains such a node (`model_wraps(name)` says so).  The caller
/// applies this only to v0 documents; a v1 document wraps everywhere (rule 5).
///
/// # Errors
///
/// `W0701` naming the type or the model.
pub fn check_v0_static_arith(ir: &Ir, model_wraps: &dyn Fn(&str) -> bool) -> Result<(), GenError> {
    let mut bad: Option<String> = None;
    ir.walk(&mut |n| {
        if bad.is_some() {
            return;
        }
        match n {
            Ir::Arith(_, ty, ..) | Ir::Neg(ty, _) if ty.is_fixed() && n.has_subject() => {
                bad = Some(format!("{} arithmetic", ty.name()));
            }
            Ir::Model { name, args } if args.iter().any(Ir::has_subject) && model_wraps(name) => {
                bad = Some(format!(
                    "model `{name}` (fixed-width arithmetic in its call closure)"
                ));
            }
            _ => {}
        }
    });
    match bad {
        None => Ok(()),
        Some(what) => Err(GenError::new(
            code::W0701,
            "",
            format!(
                "{what} over a subject result in a v0 document: the v0 harness \
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
    fn model_calls_over_a_subject_result_follow_the_callee_flag() {
        let call = Ir::Model {
            name: "inc".into(),
            args: vec![*subj("f")],
        };
        let e = check_v0_static_arith(&call, &|n| n == "inc").unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("inc"));
        assert!(check_v0_static_arith(&call, &|_| false).is_ok());
        // pure arguments are the evaluator's `wraps` business, not rule 3
        let pure = Ir::Model {
            name: "inc".into(),
            args: vec![*var("a")],
        };
        assert!(check_v0_static_arith(&pure, &|_| true).is_ok());
    }

    #[test]
    fn a_hand_built_tree_deeper_than_64_is_refused_not_recursed() {
        let mut ir = Ir::Bool(true);
        for _ in 0..200 {
            ir = Ir::Not(Box::new(ir));
        }
        assert_eq!(check_pure_guards(&ir).unwrap_err().code, code::LIMIT);
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
        let e = check_v0_static_arith(&over_subject, &|_| false).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("U8"));
        // nested above the arithmetic node: the node still has the subject beneath it
        let wrapped = Ir::Cmp(Cmp::Eq, Box::new(over_subject), var("c"));
        assert!(check_v0_static_arith(&wrapped, &|_| false).is_err());
        // negation counts too
        let neg = Ir::Neg(Ty::Fixed(FixedTy::I8), subj("f"));
        assert!(check_v0_static_arith(&neg, &|_| false).is_err());
        // exact arithmetic is not wrapping arithmetic
        let exact = Ir::Arith(Arith::Add, Ty::Int, subj("f"), var("b"));
        assert!(check_v0_static_arith(&exact, &|_| false).is_ok());
        // pure fixed-width arithmetic (a subject-independent node) is counted by `wraps`
        // at S3b, not refused here, even when the subject is elsewhere in the claim
        let pure = Ir::Cmp(
            Cmp::Eq,
            subj("f"),
            Box::new(Ir::Arith(Arith::Add, u8t, var("a"), var("b"))),
        );
        assert!(check_v0_static_arith(&pure, &|_| false).is_ok());
    }

    #[test]
    fn a_match_arm_is_a_guard_and_its_scrutinee_must_be_pure_when_an_arm_calls() {
        use crate::ir::Pat;
        let arms = |body: Box<Ir>| {
            vec![
                (Pat::Ok(Box::new(Pat::Wild)), *body),
                (Pat::ErrAny, Ir::Bool(true)),
            ]
        };
        let m = |scrutinee: Box<Ir>, body: Box<Ir>| Ir::Match {
            scrutinee,
            arms: arms(body),
        };
        // a pure scrutinee over an arm that calls: fine
        assert!(check_pure_guards(&m(var("r"), subj("f"))).is_ok());
        // a scrutinee that calls over an arm that calls: the impure guard, W0701
        let e = check_pure_guards(&m(subj("g"), subj("f"))).unwrap_err();
        assert_eq!(e.code, code::W0701);
        // the second arm counts too
        let second = Ir::Match {
            scrutinee: subj("g"),
            arms: vec![
                (Pat::Ok(Box::new(Pat::Wild)), Ir::Bool(true)),
                (Pat::ErrAny, *subj("f")),
            ],
        };
        assert!(check_pure_guards(&second).is_err());
        // no arm calls: descend into the scrutinee as usual (a call there guards nothing)
        assert!(check_pure_guards(&m(subj("g"), Box::new(Ir::Bool(true)))).is_ok());
        // ... and an impure guard nested in the scrutinee is still found
        let nested = m(
            Box::new(Ir::If(subj("g"), subj("f"), Box::new(Ir::Bool(true)))),
            Box::new(Ir::Bool(true)),
        );
        assert!(check_pure_guards(&nested).is_err());
        // an impure guard inside an arm is found
        let inner = m(
            var("r"),
            Box::new(Ir::If(subj("g"), subj("f"), Box::new(Ir::Bool(true)))),
        );
        assert!(check_pure_guards(&inner).is_err());
    }
}
