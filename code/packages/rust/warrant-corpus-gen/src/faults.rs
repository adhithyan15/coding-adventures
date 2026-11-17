//! # Faults, controls and the witness search (VER04r 5.6, 8)
//!
//! A *fault* is data: a deliberate bug the lanes inject around one operation to prove the
//! corpus can notice it.  There are three kinds (VER04r 8.1):
//!
//! ```text
//!   pre      rewrites parameters before the call:  rewrites = [{param, expr}, ..]
//!            every expr reads the ORIGINAL parameters; all rewrites apply simultaneously
//!   post     rewrites the result:                  expr over the parameters and `result`
//!   replace  supplies the result (the call is still made and counted): same shape as post
//! ```
//!
//! `result` is the one reserved name for the call's result.  The templates below are the
//! portable names of VER04r 8.2, instantiated **by signature**: `invert` needs a `Bool`
//! result, `swap-choice` a `Bool` parameter called `choice`, and so on.  A name the registry
//! does not know, or a template the operation's signature cannot take, is a generation error
//! (`fault`): nothing is skipped.
//!
//! ## Levels: data the generator cannot evaluate
//!
//! A fault expression belongs to a feature level.  `ignore-high-bit` and `xor-lsb` use
//! `bit_and`/`bit_xor`, which are `ir.fixed` (VER04r 8.2, [D9]).  This generator implements
//! `ir.core` only, so such a fault is emitted **as data** ([`Expr::Data`], `requires:
//! ["ir.fixed"]`) and never evaluated: no suite or chunk digest depends on it (VER04r 8.2).
//!
//! ## The three claim states
//!
//! For one fault and one *candidate* claim (a claim whose body calls the fault's op):
//!
//! ```text
//!   stand-in?   the claim is a `reference` suite whose subject side is exactly
//!               `op(b1, .., bk)` with every argument a plain binder: the reference
//!               (`expected`) can then stand in for the subject.
//!   no          -> unproven                       (a predicate, or a computed argument)
//!   yes, level not implemented -> deferred         (S10 will search)
//!   yes, level implemented     -> search every admitted case, in order:
//!                                   some case where faulted != expected -> witnessed (chunk)
//!                                   none                                -> blind
//! ```
//!
//! The search is exact: for a `pre` fault `expected` is re-evaluated at the rewritten
//! binders, for `post`/`replace` the fault expression is evaluated with `result` bound to
//! `expected`.  The witness is the **lowest** admitted case, and its chunk joins the slim set.

use crate::error::{code, GenError};
use crate::eval::{Program, Stats, Vars};
use crate::ir::{Arith, Cmp, Ir};
use crate::json::Json;
use crate::lower::{LoweredOp, LoweredSuite, SuiteKind};
use crate::suite::SuitePlan;
use crate::types::Ty;
use crate::value::Value;

/// The feature level of a fault expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// `ir.core`: the 26 tags of Appendix A.
    Core,
    /// `ir.fixed`: the fixed-width built-ins (`bit_and`, `bit_xor`, ...).
    Fixed,
}

impl Level {
    /// The name written to `requires`.
    pub fn name(self) -> &'static str {
        match self {
            Level::Core => "ir.core",
            Level::Fixed => "ir.fixed",
        }
    }

    /// Can this generator evaluate expressions of the level?
    pub fn implemented(self) -> bool {
        self == Level::Core
    }
}

/// A fault expression: IR the generator can evaluate, or data it can only write down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An `ir.core` tree.
    Core(Ir),
    /// A tree of a level this generator does not implement, as JSON.
    Data(Json),
}

impl Expr {
    fn json(&self) -> Json {
        match self {
            Expr::Core(ir) => ir.to_json(),
            Expr::Data(j) => j.clone(),
        }
    }
}

/// The three kinds of VER04r 8.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Parameter rewrites, applied simultaneously.
    Pre(Vec<(String, Expr)>),
    /// A rewrite of the result.
    Post(Expr),
    /// A replacement of the result.
    Replace(Expr),
}

/// One fault instantiated for one operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The portable fault name.
    pub name: String,
    /// The operation it perturbs.
    pub op: String,
    /// What it does.
    pub kind: Kind,
    /// The level of its expression.
    pub level: Level,
}

impl Fault {
    /// The `faults` row of `corpus.json` (VER04r 8.1).
    pub fn to_json(&self) -> Json {
        let mut row = vec![
            ("fault".to_string(), Json::str(self.name.as_str())),
            ("op".to_string(), Json::str(self.op.as_str())),
            ("requires".to_string(), Json::strs([self.level.name()])),
        ];
        match &self.kind {
            Kind::Pre(rewrites) => {
                row.push(("kind".into(), Json::str("pre")));
                row.push((
                    "rewrites".into(),
                    Json::arr(rewrites.iter().map(|(p, e)| {
                        Json::obj([("param", Json::str(p.as_str())), ("expr", e.json())])
                    })),
                ));
            }
            Kind::Post(e) => {
                row.push(("kind".into(), Json::str("post")));
                row.push(("expr".into(), e.json()));
            }
            Kind::Replace(e) => {
                row.push(("kind".into(), Json::str("replace")));
                row.push(("expr".into(), e.json()));
            }
        }
        Json::Obj(row)
    }
}

/// Every fault name of the registry (VER04r 8.2, the generic table).
pub const KNOWN_FAULTS: [&str; 8] = [
    "invert",
    "missing-error",
    "swap-choice",
    "off-by-one",
    "truncate-last",
    "ignore-last-byte",
    "xor-lsb",
    "ignore-high-bit",
];

/// The reserved name of the call's result in a fault expression.
pub const RESULT: &str = "result";

fn var(n: &str) -> Box<Ir> {
    Box::new(Ir::Var(n.to_string()))
}

fn nat(v: i128) -> Box<Ir> {
    Box::new(Ir::Int {
        ty: Ty::Nat,
        value: v,
    })
}

/// `len(x) - 1` (an `Int`: `Nat - Nat`).
fn len_minus_one(x: &str) -> Box<Ir> {
    Box::new(Ir::Arith(
        Arith::Sub,
        Ty::Int,
        Box::new(Ir::Len(var(x))),
        nat(1),
    ))
}

/// `len(x) > 0`.
fn nonempty(x: &str) -> Box<Ir> {
    Box::new(Ir::Cmp(Cmp::Gt, Box::new(Ir::Len(var(x))), nat(0)))
}

fn lit_data(ty: &str, value: &str) -> Json {
    Json::arr([
        Json::str("lit"),
        Json::obj([("type", Json::str(ty)), ("value", Json::str(value))]),
    ])
}

fn refuse(op: &LoweredOp, fault: &str, why: &str) -> GenError {
    GenError::new(
        code::FAULT,
        format!("{}/{fault}", op.name),
        format!("the fault cannot be instantiated on this operation: {why}"),
    )
}

/// Instantiate the template `name` on `op`'s signature.
///
/// # Errors
///
/// `fault` for an unknown name or a signature the template cannot take; `W0701` for an op
/// with a parameter called `result` (the reserved name, VER04r 8.1).
pub fn instantiate(op: &LoweredOp, name: &str) -> Result<Fault, GenError> {
    check_reserved(op)?;
    let bytes_params: Vec<&str> = op
        .params
        .iter()
        .filter(|(_, t)| t.is_bytes())
        .map(|(n, _)| n.as_str())
        .collect();
    let (kind, level) = match name {
        "invert" => {
            if op.ret != Ty::Bool {
                return Err(refuse(op, name, "invert needs a Bool result"));
            }
            (Kind::Post(Expr::Core(Ir::Not(var(RESULT)))), Level::Core)
        }
        "missing-error" => {
            let payload_ok = matches!(&op.ret, Ty::Result(ok, _) if ok.is_bytes());
            let first = bytes_params.first();
            match (payload_ok, first) {
                (true, Some(p)) => (
                    Kind::Replace(Expr::Core(Ir::If(
                        Box::new(Ir::IsErr(var(RESULT))),
                        Box::new(Ir::Ok(var(p))),
                        var(RESULT),
                    ))),
                    Level::Core,
                ),
                _ => {
                    return Err(refuse(
                        op,
                        name,
                        "missing-error needs a Result<Bytes, E> result and a Bytes parameter",
                    ))
                }
            }
        }
        "swap-choice" => {
            if !op.params.iter().any(|(n, t)| n == "choice" && *t == Ty::Bool) {
                return Err(refuse(op, name, "swap-choice needs a Bool parameter `choice`"));
            }
            (
                Kind::Pre(vec![(
                    "choice".to_string(),
                    Expr::Core(Ir::Not(var("choice"))),
                )]),
                Level::Core,
            )
        }
        "off-by-one" => {
            let (arith_ty, lit_ty) = match &op.ret {
                Ty::Nat => (Ty::Int, Ty::Nat),
                t if t.is_fixed() => (t.clone(), t.clone()),
                _ => return Err(refuse(op, name, "off-by-one needs a Nat or fixed-width result")),
            };
            (
                Kind::Post(Expr::Core(Ir::Arith(
                    Arith::Add,
                    arith_ty,
                    var(RESULT),
                    Box::new(Ir::Int {
                        ty: lit_ty,
                        value: 1,
                    }),
                ))),
                Level::Core,
            )
        }
        "truncate-last" => {
            if !op.ret.is_bytes() {
                return Err(refuse(op, name, "truncate-last needs a Bytes result"));
            }
            (
                Kind::Post(Expr::Core(Ir::If(
                    nonempty(RESULT),
                    Box::new(Ir::Take(var(RESULT), len_minus_one(RESULT))),
                    var(RESULT),
                ))),
                Level::Core,
            )
        }
        "ignore-last-byte" => {
            if bytes_params.is_empty() {
                return Err(refuse(op, name, "ignore-last-byte needs a Bytes parameter"));
            }
            let rewrites = bytes_params
                .iter()
                .map(|p| {
                    (
                        (*p).to_string(),
                        Expr::Core(Ir::If(
                            nonempty(p),
                            Box::new(Ir::Concat(
                                Box::new(Ir::Take(var(p), len_minus_one(p))),
                                Box::new(Ir::Bytes("00".to_string())),
                            )),
                            var(p),
                        )),
                    )
                })
                .collect();
            (Kind::Pre(rewrites), Level::Core)
        }
        "xor-lsb" => {
            let Some(t) = op.ret.fixed() else {
                return Err(refuse(op, name, "xor-lsb needs a fixed-width result"));
            };
            (
                Kind::Post(Expr::Data(Json::arr([
                    Json::str("bit_xor"),
                    Json::arr([Json::str("var"), Json::str(RESULT)]),
                    lit_data(t.name(), "1"),
                ]))),
                Level::Fixed,
            )
        }
        "ignore-high-bit" => {
            let u64_params: Vec<&str> = op
                .params
                .iter()
                .filter(|(_, t)| t.fixed() == Some(coding_adventures_warrant_grammar::fixed::FixedTy::U64))
                .map(|(n, _)| n.as_str())
                .collect();
            if u64_params.is_empty() {
                return Err(refuse(op, name, "ignore-high-bit needs a U64 parameter"));
            }
            let rewrites = u64_params
                .iter()
                .map(|p| {
                    (
                        (*p).to_string(),
                        Expr::Data(Json::arr([
                            Json::str("bit_and"),
                            Json::arr([Json::str("var"), Json::str(*p)]),
                            lit_data("U64", "9223372036854775807"),
                        ])),
                    )
                })
                .collect();
            (Kind::Pre(rewrites), Level::Fixed)
        }
        _ => {
            return Err(GenError::new(
                code::FAULT,
                format!("{}/{name}", op.name),
                "the fault name is not in the registry of VER04r 8.2",
            ))
        }
    };
    Ok(Fault {
        name: name.to_string(),
        op: op.name.clone(),
        kind,
        level,
    })
}

/// An op with a parameter named `result` is a generation error (VER04r 8.1).
///
/// # Errors
///
/// `W0701`.
pub fn check_reserved(op: &LoweredOp) -> Result<(), GenError> {
    if op.params.iter().any(|(n, _)| n == RESULT) {
        return Err(GenError::new(
            code::W0701,
            &op.name,
            "an operation may not have a parameter named `result` (the reserved name, VER04r 8.1)",
        ));
    }
    Ok(())
}

/// The state of one fault on one candidate claim (VER04r 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimState {
    /// A case differs; the chunk holding the lowest such case.
    Witnessed(u64),
    /// A stand-in exists and no admitted case differs.
    Blind,
    /// No stand-in exists.
    Unproven,
    /// A stand-in exists but the fault's level is not implemented here.
    Deferred(Level),
}

/// How a claim's subject call maps to the op's parameters: the binder name of each argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandIn {
    /// `binders[i]` is the binder passed as the op's `i`-th parameter.
    pub binders: Vec<String>,
}

/// Does `suite` admit the reference as a stand-in for the subject of `fault`?
pub fn stand_in(suite: &LoweredSuite, fault: &Fault, op: &LoweredOp) -> Option<StandIn> {
    let SuiteKind::Reference { subject, .. } = &suite.kind else {
        return None;
    };
    let Ir::Subject { op: called, args, .. } = subject else {
        return None;
    };
    if *called != fault.op || args.len() != op.params.len() {
        return None;
    }
    let mut binders = Vec::with_capacity(args.len());
    for a in args {
        match a {
            Ir::Var(v) if suite.binders.iter().any(|(b, _)| b == v) => binders.push(v.clone()),
            _ => return None,
        }
    }
    // A pre fault rewrites by parameter, but the stand-in rewrites by binder: a binder
    // passed twice must be rewritten the same way at both positions, or there is no
    // single rewritten binder to re-evaluate `expected` at.
    // ASSUMPTION (revisit in S10): this check only compares *whether* each position is
    // rewritten, not *how*.  It is sound for today's registry, where a binder passed at two
    // positions gets the same rewrite at both (`ignore-last-byte` and `ignore-high-bit`
    // rewrite every parameter of the type uniformly; `swap-choice` has one parameter).
    // `search` debug_asserts that the rewrites landing on one binder agree.  A fault with
    // distinct per-parameter rewrites must tighten this to "same expression" or give up.
    if let Kind::Pre(rewrites) = &fault.kind {
        for (i, b) in binders.iter().enumerate() {
            let rewritten = |k: usize| rewrites.iter().any(|(p, _)| *p == op.params[k].0);
            if binders
                .iter()
                .enumerate()
                .any(|(k, other)| other == b && rewritten(k) != rewritten(i))
            {
                return None;
            }
        }
    }
    Some(StandIn { binders })
}

fn lookup<'v>(vars: &'v Vars<'_>, name: &str) -> Option<&'v Value> {
    vars.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
}

/// One fault being searched on one suite.
pub struct Probe<'a> {
    /// The fault.
    pub fault: &'a Fault,
    /// Its op.
    pub op: &'a LoweredOp,
    /// The stand-in mapping.
    pub stand_in: StandIn,
}

/// Search every admitted case of `plan` for the lowest one where each probe's faulted value
/// differs from `expected`.  Returns, per probe, the product ordinal of the witness (or
/// `None` for a blind probe).  One pass serves all probes of the suite and stops as soon as
/// every probe has a witness.
///
/// # Errors
///
/// Evaluation errors, attributed to the claim and the fault.
pub fn search(
    program: &Program<'_>,
    suite: &LoweredSuite,
    plan: &SuitePlan<'_>,
    probes: &[Probe<'_>],
) -> Result<Vec<Option<u64>>, GenError> {
    let mut found: Vec<Option<u64>> = vec![None; probes.len()];
    if probes.is_empty() {
        return Ok(found);
    }
    let SuiteKind::Reference { expected, .. } = &suite.kind else {
        return Err(GenError::internal("a witness search needs a reference suite"));
    };
    let mut left = probes.len();
    let mut st = Stats::default();
    plan.scan_admitted(&mut |ordinal, vars| {
        let want = program.eval(expected, vars, &mut st)?;
        for (i, probe) in probes.iter().enumerate() {
            if found[i].is_some() {
                continue;
            }
            let at = |e: GenError| e.at(&format!("{}: {}/{}", suite.claim, probe.op.name, probe.fault.name));
            // the op's parameters, as the subject call would receive them
            let mut env: Vec<(&str, Value)> = Vec::with_capacity(probe.op.params.len() + 1);
            for ((pn, _), b) in probe.op.params.iter().zip(&probe.stand_in.binders) {
                let v = lookup(vars, b)
                    .ok_or_else(|| GenError::internal("a stand-in binder is not in scope"))?;
                env.push((pn.as_str(), v.clone()));
            }
            let faulted = match &probe.fault.kind {
                Kind::Pre(rewrites) => {
                    let mut moved: Vec<(&str, Value)> = vars.to_vec();
                    let mut rewritten_to: Vec<(String, Value)> = Vec::new();
                    for (param, expr) in rewrites {
                        let Expr::Core(ir) = expr else {
                            return Err(at(GenError::internal("a data fault was searched")));
                        };
                        let nv = program.eval(ir, &env, &mut st).map_err(at)?;
                        let pos = probe
                            .op
                            .params
                            .iter()
                            .position(|(n, _)| n == param)
                            .ok_or_else(|| at(GenError::internal("a rewrite names no parameter")))?;
                        let binder = &probe.stand_in.binders[pos];
                        for slot in moved.iter_mut().filter(|(n, _)| n == binder) {
                            // uniform rewrites per binder (see `stand_in`): a second rewrite
                            // landing on the same binder must agree with the first
                            debug_assert!(
                                !rewritten_to.iter().any(|(b, v)| b == binder && *v != nv),
                                "two rewrites of one binder disagree"
                            );
                            slot.1 = nv.clone();
                        }
                        rewritten_to.push((binder.clone(), nv.clone()));
                    }
                    program.eval(expected, &moved, &mut st).map_err(at)?
                }
                Kind::Post(e) | Kind::Replace(e) => {
                    let Expr::Core(ir) = e else {
                        return Err(at(GenError::internal("a data fault was searched")));
                    };
                    env.push((RESULT, want.clone()));
                    program.eval(ir, &env, &mut st).map_err(at)?
                }
            };
            if faulted != want {
                found[i] = Some(ordinal);
                left -= 1;
            }
        }
        Ok(left > 0)
    })?;
    Ok(found)
}
