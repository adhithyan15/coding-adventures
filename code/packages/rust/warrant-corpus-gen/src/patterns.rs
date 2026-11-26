//! # Patterns of `match` (`ir.tuple` on a `Result`, `ir.data` on anything else, VER04r Appendix A.1, A.2)
//!
//! The ten patterns of a `match` (see [`Pat`]: the six of A.1, the four data patterns of A.2) and the three **corpus errors** that are decided
//! from the patterns and the scrutinee's static type alone (the fourth, a `bind` that shadows a
//! name in scope, needs the scope and is checked by the lowerer):
//!
//! ```text
//!   a match that can fall through       the arms are not exhaustive over `ok` / the error cases
//!   an arm no value can reach           every value it matches is matched by an earlier arm
//!   the same bind name twice            in one pattern (checked by the lowerer)
//! ```
//!
//! Each is refused with the generator code `ir` ([`code::IR`], the oracle's `corpus`).
//!
//! ## Exhaustiveness and reachability: the usual usefulness check
//!
//! The classic algorithm (Maranget, "Warnings for pattern matching"): a pattern vector `q` is
//! **useful** with respect to a matrix of rows `P` when some value matches `q` and no row of `P`.
//! Then
//!
//! * an arm is **unreachable** exactly when its pattern is not useful with respect to the arms
//!   before it, and
//! * the match **falls through** exactly when the all-wild vector is useful with respect to all
//!   arms.
//!
//! ```text
//!   match r with Ok(_) -> a | Err(E.A) -> b | Err(E.B) -> c | Err(_) -> d      (E = A | B | C)
//!   arm 4 (Err(_)): is [err, _] useful against [ok,_] [err,A] [err,B]?  specialise on `err`:
//!                   the column holds the cases {A, B}, not C, so the set is incomplete and the
//!                   default matrix (rows with a wildcard) is empty: useful.  Reachable.
//!   fall-through:   [_] against all four rows: `ok` and `err` are both present; under `err` the
//!                   wild arm covers every case: not useful.  Exhaustive.
//! ```
//!
//! Constructors are those of the *type*: `Result<X,E>` has `ok` (one field of type `X`) and `err`
//! (one field of the error enum `E`, whose constructors are its cases, with no fields); `Option<T>` has
//! `some` and `none`; `Bool` has `true` and `false`; a tuple has one constructor with one field per
//! component; a record has one constructor named like itself and a variant one per declared
//! constructor (S11b); every other type has none (only a wild or a bind can match it).  Errors are checked in the order of VER04r 16.10 R12: **unreachable arms
//! first, in arm order, then fall-through**.
//!
//! The check is exponential in the worst case, so it carries a work budget and refuses (`limit`)
//! beyond it, like the checker's.
//!
//! This file is written from the spec and compared with `python/warrant-oracle` (`patterns.py`),
//! which shares no code with it.

use std::collections::HashMap;

use crate::error::{code, GenError};
use crate::ir::Pat;
use crate::types::Ty;
use crate::value::Value;

/// The most nested patterns (the parser's `MAX_PATTERN_DEPTH`).
pub const MAX_PATTERN_DEPTH: usize = 8;
/// The most arms of one `match`.
pub const MAX_ARMS: usize = 256;
/// The most components of a tuple (and of a tuple pattern).
pub const MAX_TUPLE_ARITY: usize = 8;
/// Steps the usefulness check may take before it gives up with a `limit` error.
pub const WORK_BUDGET: i64 = 200_000;

/// The error type name to its cases, as the lowerer keeps them.
pub type ErrorCases = HashMap<String, Vec<String>>;

fn fail(message: &str) -> GenError {
    GenError::new(code::IR, "", message)
}

/// The message of the refusal of an arm no value can reach.
pub const UNREACHABLE: &str = "a match arm that no value can reach";
/// The message of the refusal of a match that can fall through.
pub const FALLS_THROUGH: &str = "a match that can fall through";
/// The message of the refusal of a duplicate bind name.
pub const DUPLICATE_BIND: &str = "the same bind name twice in one pattern";
/// The message of the refusal of a bind that shadows a name in scope.
pub const SHADOWING_BIND: &str = "a bind name shadows a binder, a parameter, result or an outer bind";
/// The message of the refusal of a bind named like a model, an operation, an error type or a reserved
/// built-in (the oracle's sentence since CV2-F8; the generator said [`SHADOWING_BIND`] for it before S11b).
pub const RESERVED_BIND: &str =
    "a bind name shadows a model, an operation, an error type or a reserved built-in";

/// An abstract pattern: a wildcard (a `wild` or a `bind`), or a constructor with sub-patterns.
#[derive(Debug, Clone)]
enum Ab {
    Wild,
    Ctor(String, Vec<Ab>),
}

/// The declared records and variants, as `type name -> ((constructor, (field type, ..)), ..)`.  A
/// record has exactly one constructor, named like itself (VER04j 4.2).  Built by the lowerer.
pub type Adts = HashMap<String, Vec<(String, Vec<Ty>)>>;

/// One column of the pattern matrix: the type the patterns of that column meet.
///
/// A column is a *type*, looked at lazily (a record's field types are read when the column is
/// specialised, never expanded up front), so a deep nest of records costs what the patterns cost and
/// not the product of the declarations.
#[derive(Debug, Clone)]
enum Col {
    /// A value type.
    Ty(Ty),
    /// The field of `err`: the error enum of that name (its cases are the constructors).
    ErrOf(String),
    /// Not known (the field of a constructor the type does not have): only a wild or bind matches.
    Unknown,
}

/// What the check needs to know about the document's declared types.
struct Env<'a> {
    errors: &'a ErrorCases,
    adts: &'a Adts,
}

/// The constructors of a column with their field columns, or `None` when the set is open.
///
/// `Result<X,E>` has `ok` and `err`; `Option<T>` has `some` and `none`; `Bool` has `true` and `false`; a
/// tuple has one constructor with a field per component; a record or variant has its declared
/// constructors; the field of `err` has the cases of its error enum; every other type has none.
fn signature(col: &Col, env: &Env<'_>) -> Option<Vec<(String, Vec<Col>)>> {
    match col {
        Col::Ty(Ty::Result(ok, e)) => Some(vec![
            ("ok".to_string(), vec![Col::Ty((**ok).clone())]),
            ("err".to_string(), vec![Col::ErrOf(e.clone())]),
        ]),
        Col::Ty(Ty::Option(inner)) => Some(vec![
            ("some".to_string(), vec![Col::Ty((**inner).clone())]),
            ("none".to_string(), vec![]),
        ]),
        Col::Ty(Ty::Bool) => Some(vec![("true".to_string(), vec![]), ("false".to_string(), vec![])]),
        Col::Ty(Ty::Tuple(parts)) => Some(vec![(
            "tuple".to_string(),
            parts.iter().cloned().map(Col::Ty).collect(),
        )]),
        Col::Ty(Ty::Named(name)) => env.adts.get(name).map(|ctors| {
            ctors
                .iter()
                .map(|(c, fields)| (c.clone(), fields.iter().cloned().map(Col::Ty).collect()))
                .collect()
        }),
        Col::ErrOf(name) => env
            .errors
            .get(name)
            .map(|cases| cases.iter().map(|c| (c.clone(), vec![])).collect()),
        Col::Ty(_) | Col::Unknown => None,
    }
}

fn normalise(p: &Pat) -> Ab {
    match p {
        Pat::Wild | Pat::Bind(_) => Ab::Wild,
        Pat::Ok(inner) => Ab::Ctor("ok".to_string(), vec![normalise(inner)]),
        Pat::Err(case) => Ab::Ctor("err".to_string(), vec![Ab::Ctor(case.clone(), vec![])]),
        Pat::ErrAny => Ab::Ctor("err".to_string(), vec![Ab::Wild]),
        Pat::Tuple(parts) => Ab::Ctor("tuple".to_string(), parts.iter().map(normalise).collect()),
        Pat::Some(inner) => Ab::Ctor("some".to_string(), vec![normalise(inner)]),
        Pat::None => Ab::Ctor("none".to_string(), vec![]),
        Pat::Bool(v) => Ab::Ctor(if *v { "true" } else { "false" }.to_string(), vec![]),
        Pat::Ctor(name, parts) => Ab::Ctor(name.clone(), parts.iter().map(normalise).collect()),
    }
}

struct Budget(i64);

impl Budget {
    fn spend(&mut self, n: usize) -> Result<(), GenError> {
        self.0 -= i64::try_from(n).unwrap_or(i64::MAX);
        if self.0 < 0 {
            return Err(GenError::new(
                code::LIMIT,
                "",
                "the exhaustiveness check exceeds its work budget",
            ));
        }
        Ok(())
    }
}

type Row = Vec<Ab>;

fn specialise(rows: &[Row], ctor: &str, arity: usize) -> Vec<Row> {
    let mut out = Vec::new();
    for row in rows {
        match &row[0] {
            Ab::Wild => {
                let mut r = vec![Ab::Wild; arity];
                r.extend(row[1..].iter().cloned());
                out.push(r);
            }
            Ab::Ctor(c, args) if c == ctor => {
                let mut r = args.clone();
                r.extend(row[1..].iter().cloned());
                out.push(r);
            }
            Ab::Ctor(..) => {}
        }
    }
    out
}

/// Is the pattern vector `vec` useful with respect to `rows` (all of the column types `tys`)?
fn useful(
    rows: &[Row],
    vec: &[Ab],
    tys: &[Col],
    env: &Env<'_>,
    budget: &mut Budget,
) -> Result<bool, GenError> {
    budget.spend(1 + rows.len())?;
    let Some(head) = vec.first() else {
        return Ok(rows.is_empty());
    };
    let sig = signature(&tys[0], env);
    match head {
        Ab::Ctor(ctor, args) => {
            let fields = sig
                .and_then(|s| s.into_iter().find(|(c, _)| c == ctor))
                .map_or_else(|| vec![Col::Unknown; args.len()], |(_, f)| f);
            let mut v: Vec<Ab> = args.clone();
            v.extend(vec[1..].iter().cloned());
            let mut t = fields;
            t.extend(tys[1..].iter().cloned());
            useful(&specialise(rows, ctor, args.len()), &v, &t, env, budget)
        }
        Ab::Wild => {
            let present: Vec<&str> = rows
                .iter()
                .filter_map(|r| match &r[0] {
                    Ab::Ctor(c, _) => Some(c.as_str()),
                    Ab::Wild => None,
                })
                .collect();
            if let Some(sig) = sig
                .as_ref()
                .filter(|s| s.iter().all(|(c, _)| present.contains(&c.as_str())))
            {
                for (ctor, fields) in sig {
                    let mut v: Vec<Ab> = vec![Ab::Wild; fields.len()];
                    v.extend(vec[1..].iter().cloned());
                    let mut t = fields.clone();
                    t.extend(tys[1..].iter().cloned());
                    if useful(&specialise(rows, ctor, fields.len()), &v, &t, env, budget)? {
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
            let default: Vec<Row> = rows
                .iter()
                .filter(|r| matches!(r[0], Ab::Wild))
                .map(|r| r[1..].to_vec())
                .collect();
            useful(&default, &vec[1..], &tys[1..], env, budget)
        }
    }
}

/// Refuse a match that can fall through or has an arm no value can reach (generator code `ir`), over a
/// scrutinee whose type holds no record or variant (see [`check_arms_with`]).
///
/// # Errors
///
/// As [`check_arms_with`].
pub fn check_arms(patterns: &[&Pat], ty: &Ty, errors: &ErrorCases) -> Result<(), GenError> {
    check_arms_with(patterns, ty, errors, &Adts::new())
}

/// Refuse a match that can fall through or has an arm no value can reach (generator code `ir`).
///
/// `patterns` are the IR patterns of the arms in order, already typed against `ty` (the type of
/// the scrutinee: a `Result`, an `Option`, a `Bool`, a tuple, a record or a variant); `errors` maps an
/// error type to its cases and `adts` a record or variant to its constructors.  Unreachable arms are
/// found first, in arm order, then fall-through (VER04r 16.10 R12, now over every scrutinee type, R35 (4)).
///
/// # Errors
///
/// `limit` for 0 or more than [`MAX_ARMS`] arms or past the work budget; `ir` for the two
/// corpus errors.
pub fn check_arms_with(
    patterns: &[&Pat],
    ty: &Ty,
    errors: &ErrorCases,
    adts: &Adts,
) -> Result<(), GenError> {
    if patterns.is_empty() || patterns.len() > MAX_ARMS {
        return Err(GenError::new(code::LIMIT, "", "a match has 1 to 256 arms"));
    }
    let env = Env { errors, adts };
    let column = [Col::Ty(ty.clone())];
    let mut budget = Budget(WORK_BUDGET);
    let mut rows: Vec<Row> = Vec::new();
    for p in patterns {
        let abstract_pattern = normalise(p);
        if !useful(&rows, std::slice::from_ref(&abstract_pattern), &column, &env, &mut budget)? {
            return Err(fail(UNREACHABLE));
        }
        rows.push(vec![abstract_pattern]);
    }
    if useful(&rows, &[Ab::Wild], &column, &env, &mut budget)? {
        return Err(fail(FALLS_THROUGH));
    }
    Ok(())
}

/// Match `value` against `pattern`; on success the `bind`s are appended to `binds` (on failure
/// the caller discards whatever was appended).  Values are the evaluator's own: a `Result` is
/// [`Value::Ok`] or [`Value::Err`], a tuple is [`Value::Tuple`].  A value of the wrong shape
/// never matches (the tree was typed at lowering).
pub fn match_pattern<'p>(
    pattern: &'p Pat,
    value: &Value,
    binds: &mut Vec<(&'p str, Value)>,
) -> bool {
    match pattern {
        Pat::Wild => true,
        Pat::Bind(name) => {
            binds.push((name.as_str(), value.clone()));
            true
        }
        Pat::Ok(inner) => match value {
            Value::Ok(payload) => match_pattern(inner, payload, binds),
            _ => false,
        },
        Pat::ErrAny => matches!(value, Value::Err(_)),
        Pat::Err(case) => matches!(value, Value::Err(c) if **c == **case),
        Pat::Tuple(parts) => match value {
            Value::Tuple(items) if items.len() == parts.len() => parts
                .iter()
                .zip(items)
                .all(|(part, item)| match_pattern(part, item, binds)),
            _ => false,
        },
        // The four data patterns (`ir.data`, S11c-2): a value of the scrutinee's type has exactly the form
        // the pattern names, so a mismatch of shape is simply "does not match".
        Pat::Some(inner) => match value {
            Value::Opt(Some(payload)) => match_pattern(inner, payload, binds),
            _ => false,
        },
        Pat::None => matches!(value, Value::Opt(None)),
        Pat::Ctor(name, parts) => match value {
            Value::Con(ctor, items) if **ctor == **name && items.len() == parts.len() => parts
                .iter()
                .zip(items)
                .all(|(part, item)| match_pattern(part, item, binds)),
            _ => false,
        },
        Pat::Bool(b) => matches!(value, Value::Bool(v) if v == b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn errs() -> ErrorCases {
        [("E".to_string(), vec!["A".into(), "B".into(), "C".into()])].into()
    }

    fn res(ok: Ty) -> Ty {
        Ty::Result(Box::new(ok), "E".into())
    }

    fn ok(p: Pat) -> Pat {
        Pat::Ok(Box::new(p))
    }

    fn check(pats: &[Pat], ty: &Ty) -> Result<(), String> {
        let refs: Vec<&Pat> = pats.iter().collect();
        check_arms(&refs, ty, &errs()).map_err(|e| format!("{}: {}", e.code, e.message))
    }

    #[test]
    fn exhaustive_matches_pass() {
        let t = res(Ty::Bool);
        assert!(check(&[ok(Pat::Wild), Pat::ErrAny], &t).is_ok());
        assert!(check(&[Pat::Wild], &t).is_ok());
        assert!(check(&[Pat::Bind("whole".into())], &t).is_ok());
        assert!(check(
            &[
                ok(Pat::Wild),
                Pat::Err("A".into()),
                Pat::Err("B".into()),
                Pat::Err("C".into())
            ],
            &t
        )
        .is_ok());
        // a case then err_any then ok is fine in any order
        assert!(check(&[Pat::Err("A".into()), Pat::ErrAny, ok(Pat::Wild)], &t).is_ok());
    }

    #[test]
    fn a_match_that_falls_through_is_ir() {
        let t = res(Ty::Bool);
        for pats in [
            vec![ok(Pat::Wild)],
            vec![Pat::ErrAny],
            vec![ok(Pat::Wild), Pat::Err("A".into()), Pat::Err("B".into())],
        ] {
            let e = check(&pats, &t).unwrap_err();
            assert_eq!(e, format!("ir: {FALLS_THROUGH}"));
        }
    }

    #[test]
    fn an_arm_no_value_reaches_is_ir_and_is_found_before_fall_through() {
        let t = res(Ty::Bool);
        for pats in [
            vec![Pat::Wild, ok(Pat::Wild)],
            vec![ok(Pat::Wild), Pat::ErrAny, Pat::Err("A".into())],
            vec![ok(Pat::Wild), Pat::Err("A".into()), Pat::Err("A".into()), Pat::ErrAny],
            vec![
                ok(Pat::Wild),
                Pat::Err("A".into()),
                Pat::Err("B".into()),
                Pat::Err("C".into()),
                Pat::ErrAny,
            ],
        ] {
            assert_eq!(check(&pats, &t).unwrap_err(), format!("ir: {UNREACHABLE}"));
        }
        // R12: this match both falls through (no Err arm) and has an unreachable arm; the
        // unreachable arm is reported
        let both = [ok(Pat::Wild), ok(Pat::Bind("x".into()))];
        assert_eq!(check(&both, &t).unwrap_err(), format!("ir: {UNREACHABLE}"));
    }

    #[test]
    fn tuple_patterns_are_checked_component_by_component() {
        let pair = res(Ty::Tuple(vec![Ty::Bool, Ty::Bool]));
        let tup = |a: Pat, b: Pat| ok(Pat::Tuple(vec![a, b]));
        assert!(check(&[tup(Pat::Bind("a".into()), Pat::Wild), Pat::ErrAny], &pair).is_ok());
        // Ok((a, b)) then Err(_) then Ok(_): the last arm is unreachable (a tuple pattern of two
        // wilds covers every Ok)
        let e = check(
            &[tup(Pat::Bind("a".into()), Pat::Bind("b".into())), Pat::ErrAny, ok(Pat::Wild)],
            &pair,
        )
        .unwrap_err();
        assert_eq!(e, format!("ir: {UNREACHABLE}"));
        // Ok(t) then Ok((a, b)): the bind covers the tuple
        let e = check(
            &[ok(Pat::Bind("t".into())), tup(Pat::Wild, Pat::Wild), Pat::ErrAny],
            &pair,
        )
        .unwrap_err();
        assert_eq!(e, format!("ir: {UNREACHABLE}"));
    }

    #[test]
    fn nested_results_complete_independently() {
        let t = res(res(Ty::Bool));
        assert!(check(
            &[ok(ok(Pat::Wild)), ok(Pat::ErrAny), Pat::ErrAny],
            &t
        )
        .is_ok());
        assert_eq!(
            check(&[ok(ok(Pat::Wild)), Pat::ErrAny], &t).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
    }

    #[test]
    fn an_unknown_error_type_is_an_open_set() {
        // No error type `Z` is known: the cases never complete, so only err_any (or a wild)
        // closes the match.
        let t = Ty::Result(Box::new(Ty::Bool), "Z".into());
        let (ok_wild, case_a, any) = (ok(Pat::Wild), Pat::Err("A".into()), Pat::ErrAny);
        assert!(check_arms(&[&ok_wild, &case_a], &t, &errs()).is_err());
        assert!(check_arms(&[&ok_wild, &case_a, &any], &t, &errs()).is_ok());
    }

    #[test]
    fn the_arm_count_and_the_work_budget_are_limits() {
        let t = res(Ty::Bool);
        let none: Vec<&Pat> = vec![];
        assert_eq!(check_arms(&none, &t, &errs()).unwrap_err().code, code::LIMIT);
        let wild = Pat::Wild;
        let many: Vec<&Pat> = (0..257).map(|_| &wild).collect();
        assert_eq!(check_arms(&many, &t, &errs()).unwrap_err().code, code::LIMIT);
        // a wide tuple of deeply nested Results: the check either completes or gives up with a
        // `limit` refusal, but never panics or loops
        let deep = (0..8).fold(Ty::Bool, |t, _| res(t));
        let wide = Ty::Tuple(vec![deep; 8]);
        let nested_ok = |n: usize| (0..n).fold(Pat::Wild, |p, _| ok(p));
        let mut pats: Vec<Pat> = Vec::new();
        for i in 0..8 {
            let mut parts = vec![Pat::Wild; 8];
            parts[i] = nested_ok(8);
            pats.push(Pat::Tuple(parts));
            let mut parts = vec![Pat::Wild; 8];
            parts[i] = Pat::ErrAny;
            pats.push(Pat::Tuple(parts));
        }
        pats.push(Pat::Wild);
        let refs: Vec<&Pat> = pats.iter().collect();
        match check_arms(&refs, &wide, &errs()) {
            Ok(()) => {}
            Err(e) => assert_eq!(e.code, code::LIMIT),
        }
    }

    #[test]
    fn matching_at_run_time_binds_and_picks_the_first_arm() {
        let v = Value::Ok(Box::new(Value::Tuple(vec![Value::Bool(true), Value::Int(3)])));
        let p = ok(Pat::Tuple(vec![Pat::Bind("a".into()), Pat::Bind("n".into())]));
        let mut binds = Vec::new();
        assert!(match_pattern(&p, &v, &mut binds));
        assert_eq!(
            binds,
            vec![("a", Value::Bool(true)), ("n", Value::Int(3))]
        );
        assert!(!match_pattern(&Pat::ErrAny, &v, &mut Vec::new()));
        assert!(!match_pattern(&ok(Pat::Tuple(vec![Pat::Wild])), &v, &mut Vec::new()));
        let err = Value::Err(Rc::from("B"));
        assert!(match_pattern(&Pat::Err("B".into()), &err, &mut Vec::new()));
        assert!(!match_pattern(&Pat::Err("A".into()), &err, &mut Vec::new()));
        assert!(match_pattern(&Pat::ErrAny, &err, &mut Vec::new()));
        assert!(!match_pattern(&ok(Pat::Wild), &err, &mut Vec::new()));
        assert!(match_pattern(&Pat::Wild, &err, &mut Vec::new()));
        let mut b = Vec::new();
        let bind_w = Pat::Bind("w".into());
        assert!(match_pattern(&bind_w, &err, &mut b));
        assert_eq!(b, vec![("w", err)]);
    }

    // -- ir.data (slice S11b): Bool, Option, records and variants ---------------------------------

    fn opt(t: Ty) -> Ty {
        Ty::Option(Box::new(t))
    }

    fn named(n: &str) -> Ty {
        Ty::Named(n.into())
    }

    /// `record P(x: Bool, y: Bool)`, `variant V = Circle(Bool) | Rect(Bool, Bool) | Dot`,
    /// `variant K = Red | Green | Blue`.
    fn adts() -> Adts {
        let mut a = Adts::new();
        a.insert("P".into(), vec![("P".into(), vec![Ty::Bool, Ty::Bool])]);
        a.insert(
            "V".into(),
            vec![
                ("Circle".into(), vec![Ty::Bool]),
                ("Rect".into(), vec![Ty::Bool, Ty::Bool]),
                ("Dot".into(), vec![]),
            ],
        );
        a.insert(
            "K".into(),
            vec![
                ("Red".into(), vec![]),
                ("Green".into(), vec![]),
                ("Blue".into(), vec![]),
            ],
        );
        a
    }

    fn check_data(pats: &[Pat], ty: &Ty) -> Result<(), String> {
        let refs: Vec<&Pat> = pats.iter().collect();
        check_arms_with(&refs, ty, &errs(), &adts()).map_err(|e| format!("{}: {}", e.code, e.message))
    }

    fn ctor(n: &str, parts: Vec<Pat>) -> Pat {
        Pat::Ctor(n.into(), parts)
    }

    fn some(p: Pat) -> Pat {
        Pat::Some(Box::new(p))
    }

    #[test]
    fn bool_and_option_are_closed_sets_of_two() {
        assert!(check_data(&[Pat::Bool(true), Pat::Bool(false)], &Ty::Bool).is_ok());
        assert!(check_data(&[Pat::Bool(true), Pat::Wild], &Ty::Bool).is_ok());
        assert_eq!(
            check_data(&[Pat::Bool(true)], &Ty::Bool).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        assert_eq!(
            check_data(&[Pat::Bool(true), Pat::Bool(true), Pat::Bool(false)], &Ty::Bool).unwrap_err(),
            format!("ir: {UNREACHABLE}")
        );
        let o = opt(Ty::Bool);
        assert!(check_data(&[some(Pat::Wild), Pat::None], &o).is_ok());
        assert!(check_data(&[some(Pat::Bind("x".into())), Pat::Wild], &o).is_ok());
        assert_eq!(check_data(&[some(Pat::Wild)], &o).unwrap_err(), format!("ir: {FALLS_THROUGH}"));
        assert_eq!(check_data(&[Pat::None], &o).unwrap_err(), format!("ir: {FALLS_THROUGH}"));
        // an arm after a wild, and a repeated `Some`, are unreachable
        assert_eq!(
            check_data(&[Pat::Wild, some(Pat::Wild)], &o).unwrap_err(),
            format!("ir: {UNREACHABLE}")
        );
        assert_eq!(
            check_data(&[some(Pat::Wild), some(Pat::Wild), Pat::None], &o).unwrap_err(),
            format!("ir: {UNREACHABLE}")
        );
        // nesting: `Some(Some(_))` and `None` leave `Some(None)`
        let oo = opt(opt(Ty::Bool));
        assert_eq!(
            check_data(&[some(some(Pat::Wild)), Pat::None], &oo).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        assert!(check_data(&[some(some(Pat::Wild)), some(Pat::None), Pat::None], &oo).is_ok());
        // a refutable payload: `Some(true)` leaves `Some(false)`
        assert_eq!(
            check_data(&[some(Pat::Bool(true)), Pat::None], &o).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
    }

    #[test]
    fn variants_and_records_have_the_constructors_they_declare() {
        let k = named("K");
        assert!(check_data(&[ctor("Red", vec![]), ctor("Green", vec![]), ctor("Blue", vec![])], &k).is_ok());
        assert_eq!(
            check_data(&[ctor("Red", vec![]), ctor("Green", vec![])], &k).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        assert_eq!(
            check_data(&[ctor("Red", vec![]), ctor("Red", vec![]), Pat::Wild], &k).unwrap_err(),
            format!("ir: {UNREACHABLE}")
        );
        // a wild after every constructor is unreachable
        assert_eq!(
            check_data(
                &[ctor("Red", vec![]), ctor("Green", vec![]), ctor("Blue", vec![]), Pat::Wild],
                &k
            )
            .unwrap_err(),
            format!("ir: {UNREACHABLE}")
        );
        // constructors with fields
        let v = named("V");
        let full = [
            ctor("Circle", vec![Pat::Wild]),
            ctor("Rect", vec![Pat::Wild, Pat::Wild]),
            ctor("Dot", vec![]),
        ];
        assert!(check_data(&full, &v).is_ok());
        assert_eq!(
            check_data(&full[..2], &v).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        // a record is one constructor named like it; a refutable field leaves a hole
        let p = named("P");
        assert!(check_data(&[ctor("P", vec![Pat::Wild, Pat::Wild])], &p).is_ok());
        assert_eq!(
            check_data(&[ctor("P", vec![Pat::Wild, Pat::Bool(true)])], &p).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        assert!(check_data(
            &[ctor("P", vec![Pat::Wild, Pat::Bool(true)]), ctor("P", vec![Pat::Wild, Pat::Bool(false)])],
            &p
        )
        .is_ok());
        assert_eq!(
            check_data(
                &[ctor("P", vec![Pat::Wild, Pat::Wild]), ctor("P", vec![Pat::Wild, Pat::Bool(true)])],
                &p
            )
            .unwrap_err(),
            format!("ir: {UNREACHABLE}")
        );
        // a data pattern inside a Result and inside a tuple
        let r = res(Ty::Bool);
        assert!(check_data(&[ok(Pat::Bool(true)), ok(Pat::Bool(false)), Pat::ErrAny], &r).is_ok());
        assert_eq!(
            check_data(&[ok(Pat::Bool(true)), Pat::ErrAny], &r).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        let t = Ty::Tuple(vec![Ty::Bool, opt(Ty::Bool)]);
        assert!(check_data(
            &[Pat::Tuple(vec![Pat::Bool(true), Pat::Wild]), Pat::Tuple(vec![Pat::Bool(false), Pat::Wild])],
            &t
        )
        .is_ok());
        // a type the declarations do not know has no constructor: only a wild or a bind closes it
        let unknown = named("Q");
        assert_eq!(
            check_data(&[ctor("Red", vec![])], &unknown).unwrap_err(),
            format!("ir: {FALLS_THROUGH}")
        );
        assert!(check_data(&[ctor("Red", vec![]), Pat::Wild], &unknown).is_ok());
    }

    #[test]
    fn data_patterns_meet_the_values_of_their_type_and_nothing_else() {
        use std::rc::Rc;
        let some = Value::Opt(Some(Box::new(Value::Bool(true))));
        let mut b = Vec::new();
        let bind_some = Pat::Some(Box::new(Pat::Bind("x".into())));
        assert!(match_pattern(&bind_some, &some, &mut b));
        assert_eq!(b, vec![("x", Value::Bool(true))]);
        assert!(!match_pattern(&Pat::None, &some, &mut Vec::new()));
        assert!(match_pattern(&Pat::None, &Value::Opt(None), &mut Vec::new()));
        let con = Value::Con(Rc::from("P"), vec![Value::Int(1)]);
        assert!(match_pattern(&Pat::Ctor("P".into(), vec![Pat::Wild]), &con, &mut Vec::new()));
        assert!(!match_pattern(&Pat::Ctor("Q".into(), vec![Pat::Wild]), &con, &mut Vec::new()));
        assert!(!match_pattern(&Pat::Ctor("P".into(), vec![]), &con, &mut Vec::new()));
        assert!(match_pattern(&Pat::Bool(true), &Value::Bool(true), &mut Vec::new()));
        assert!(!match_pattern(&Pat::Bool(true), &Value::Bool(false), &mut Vec::new()));
        // a pattern of the wrong shape never matches
        assert!(!match_pattern(&Pat::Some(Box::new(Pat::Wild)), &Value::Bool(true), &mut Vec::new()));
    }
}
