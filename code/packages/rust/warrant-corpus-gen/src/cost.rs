//! # Worst-path IR cost (VER04r section 4.2 rule 6)
//!
//! The IR has no loop and no recursion, so a *static* bound on how many nodes an
//! evaluation can visit is the whole fuel: no run-time step meter is needed.  The cost of
//! an expression is the largest number of node evaluations any single evaluation of it can
//! perform:
//!
//! ```text
//!   node                                      cost
//!   ----------------------------------------  ----------------------------------------
//!   lit, var, err                             1
//!   if                                        1 + cost(cond) + MAX(cost(then), cost(else))
//!   model_call                                1 + sum(args) + cost(model body)
//!   subject_op                                1 + sum(args)    (the subject's work is not IR work)
//!   every other tag, and/or/implies too       1 + sum(operands) (a short circuit is never credited)
//! ```
//!
//! ## A worked example
//!
//! `if a >= 2u8 then xor1(a - 2u8, b) else xor1(a, b)` with `cost(xor1) = 8`:
//!
//! ```text
//!   condition  a >= 2u8            = 1 + 1 + 1                 = 3
//!   then       xor1(a - 2u8, b)    = 1 + (1+1+1) + 1 + 8       = 13
//!   else       xor1(a, b)          = 1 + 1 + 1 + 8             = 11
//!   whole      1 + 3 + max(13,11)                              = 17
//! ```
//!
//! Charging **both** branches would give 27; and for the ordinary idiom of a bit-sliced model
//! whose four branches each call the previous model once, both-branches costing is
//! exponential in model depth (gf256's `ginv` would cost 92,014,191 instead of 26,204).
//!
//! ## One computation per model, in declaration order
//!
//! A model body is costed **once**, in declaration order ([`ModelCosts::compute`]), so the
//! whole computation is linear in the size of the corpus IR.  Memoisation is never credited
//! (the cost is a bound on work, not an estimate of time).
//!
//! ## Checked arithmetic, no big numbers
//!
//! A chain `m_k(x) = m_(k-1)(m_(k-1)(x))` doubles the cost per level.  The walk therefore
//! compares every partial sum with the limit and stops with `limit` as soon as it is
//! exceeded: it never wraps and never builds a number past `2 * limit`.
//!
//! ## Where the limit comes from
//!
//! `MAX_IR_COST = 2,000` by default; a corpus that needs more declares `max_ir_cost N` in
//! `corpus.profile` with `2,000 <= N <= 65,536` ([`crate::profile`]).  The limit applies to
//! each `admit`, `body`, `expected` and model body separately.  (The independent oracle
//! also bounds a model body by the limit; VER04r names expressions, not models, so this is
//! the stricter reading, recorded as a question in the S3a report.)
//!
//! Each suite records `ir_cost = cost(admit) + cost(body) + cost(expected)` (the last only
//! for a reference suite).  VER04r 4.2 also bounds the **total** IR work of a run, which
//! [`IrWork`] implements with checked arithmetic.

use std::collections::HashMap;

use crate::error::{code, GenError};
use crate::ir::{Ir, MAX_IR_DEPTH};
use crate::lower::{LoweredModel, LoweredOp, LoweredSuite, SuiteKind};

/// The default per-expression bound.
pub const DEFAULT_MAX_IR_COST: u64 = 2_000;
/// The hard ceiling a profile may raise the bound to.
pub const MAX_IR_COST_CEILING: u64 = 65_536;
/// The longest chain of nested model calls (VER04 section 2: 64 active frames).
pub const MAX_MODEL_DEPTH: u32 = 64;
/// Total IR work of a `slim` run: `32,000,000 x 2,000`.
pub const MAX_IR_WORK_SLIM: u64 = 64_000_000_000;
/// Total IR work of a `full` run: `2^32 x 2,000`.
pub const MAX_IR_WORK_FULL: u64 = 8_589_934_592_000;

/// A recorded `max_ir_cost` above the ceiling is a corpus error (VER04r 4.2 rule 6).
fn check_limit(limit: u64) -> Result<(), GenError> {
    if limit > MAX_IR_COST_CEILING {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "max_ir_cost exceeds the 65,536 ceiling",
        ));
    }
    Ok(())
}

fn over(limit: u64) -> GenError {
    GenError::new(
        code::LIMIT,
        "",
        format!("ir-cost: an expression exceeds max_ir_cost ({limit})"),
    )
}

/// The cost and call-nesting depth of every model, in declaration order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelCosts {
    by_name: HashMap<String, (u64, u32)>,
    order: Vec<String>,
}

impl ModelCosts {
    /// Cost every model body once, in order.  A model may call only models declared before
    /// it (VER04r 4.2 rule 1); a call to a later, unknown or the same model is `W0701`.
    ///
    /// # Errors
    ///
    /// `limit` when a body (including the bodies it calls) costs more than `limit`, or
    /// nests model calls deeper than [`MAX_MODEL_DEPTH`]; `W0701` for a forward call.
    pub fn compute(models: &[LoweredModel], limit: u64) -> Result<ModelCosts, GenError> {
        check_limit(limit)?;
        let mut costs = ModelCosts::default();
        for m in models {
            let cost = ir_cost(&m.body, &costs, limit).map_err(|e| e.at(&m.name))?;
            let mut deepest = 0;
            m.body.walk(&mut |n| {
                if let Ir::Model { name, .. } = n {
                    if let Some((_, d)) = costs.by_name.get(name) {
                        deepest = deepest.max(*d);
                    }
                }
            });
            let depth = 1 + deepest;
            if depth > MAX_MODEL_DEPTH {
                return Err(GenError::new(
                    code::LIMIT,
                    &m.name,
                    "model calls nest deeper than 64 frames",
                ));
            }
            costs.by_name.insert(m.name.clone(), (cost, depth));
            costs.order.push(m.name.clone());
        }
        Ok(costs)
    }

    /// The worst-path cost of `name`'s body.
    pub fn cost(&self, name: &str) -> Option<u64> {
        self.by_name.get(name).map(|(c, _)| *c)
    }

    /// The model-call nesting depth of `name` (a model that calls none is 1).
    pub fn depth(&self, name: &str) -> Option<u32> {
        self.by_name.get(name).map(|(_, d)| *d)
    }

    /// The model names in declaration order.
    pub fn order(&self) -> &[String] {
        &self.order
    }
}

/// The worst-path cost of `ir`, with the models it calls already costed.
///
/// # Errors
///
/// `limit` if any partial sum, or the total, exceeds `limit`, or the tree is deeper than
/// [`MAX_IR_DEPTH`]; `W0701` for a call to a model that is not (yet) costed.
pub fn ir_cost(ir: &Ir, models: &ModelCosts, limit: u64) -> Result<u64, GenError> {
    check_limit(limit)?;
    if ir.depth() > MAX_IR_DEPTH {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "IR nesting exceeds 64 levels",
        ));
    }
    cost(ir, models, limit)
}

/// `acc + more`, refused as soon as it passes `limit`.  Both addends are at most `limit`
/// (a child's cost is checked when it is computed), so the sum cannot overflow a `u64`.
fn bump(acc: u64, more: u64, limit: u64) -> Result<u64, GenError> {
    match acc.checked_add(more) {
        Some(sum) if sum <= limit => Ok(sum),
        _ => Err(over(limit)),
    }
}

fn cost(ir: &Ir, models: &ModelCosts, limit: u64) -> Result<u64, GenError> {
    let mut acc = bump(0, 1, limit)?;
    match ir {
        Ir::If(c, t, e) => {
            acc = bump(acc, cost(c, models, limit)?, limit)?;
            let worst = cost(t, models, limit)?.max(cost(e, models, limit)?);
            bump(acc, worst, limit)
        }
        Ir::Model { name, args } => {
            for a in args {
                acc = bump(acc, cost(a, models, limit)?, limit)?;
            }
            let body = models.cost(name).ok_or_else(|| {
                GenError::new(
                    code::W0701,
                    "",
                    "a call to a model that is not declared before it (VER04r 4.2 rule 1)",
                )
            })?;
            bump(acc, body, limit)
        }
        other => {
            for k in other.children() {
                acc = bump(acc, cost(k, models, limit)?, limit)?;
            }
            Ok(acc)
        }
    }
}

/// The three costs of a suite and their sum, its `ir_cost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuiteCost {
    /// Cost of `admit`.
    pub admit: u64,
    /// Cost of the whole claim `body`.
    pub body: u64,
    /// Cost of `expected` (`reference` suites only).
    pub expected: Option<u64>,
    /// `admit + body + expected`: the `ir_cost` of the suite row.
    pub total: u64,
}

/// The `ir_cost` of a suite: `admit + body + (expected)`, each separately bounded by
/// `limit`.  The `subject` side of a reference suite is part of `body` and is not summed a
/// second time.
///
/// # Errors
///
/// As [`ir_cost`], attributed to the claim.
pub fn suite_cost(
    suite: &LoweredSuite,
    models: &ModelCosts,
    limit: u64,
) -> Result<SuiteCost, GenError> {
    let at = |e: GenError| e.at(&suite.claim);
    let admit = ir_cost(&suite.admit, models, limit).map_err(at)?;
    let body = ir_cost(&suite.body, models, limit).map_err(at)?;
    let expected = match &suite.kind {
        SuiteKind::Reference { expected, .. } => {
            Some(ir_cost(expected, models, limit).map_err(at)?)
        }
        SuiteKind::Predicate => None,
    };
    let total = admit + body + expected.unwrap_or(0);
    Ok(SuiteCost {
        admit,
        body,
        expected,
        total,
    })
}

/// The IR work of [D19]'s per-case check for one suite: for every `subject_op` of the body whose
/// op has an `admit`, the worst-path cost of that `admit` plus the cost of each argument that does
/// not call the subject (the arguments the check evaluates).  A static sum over the whole body
/// (a call in an untaken branch is counted), so it is an upper bound the oracle can reproduce.
/// It is charged to [`IrWork`] per product ordinal and is **not** part of the recorded `ir_cost`.
///
/// # Errors
///
/// As [`ir_cost`].
pub fn admit_check_cost(
    suite: &LoweredSuite,
    ops: &[LoweredOp],
    models: &ModelCosts,
    limit: u64,
) -> Result<u64, GenError> {
    let mut total = 0u64;
    let mut failure: Option<GenError> = None;
    suite.body.walk(&mut |n| {
        if failure.is_some() {
            return;
        }
        let Ir::Subject { op, args, .. } = n else {
            return;
        };
        let Some(admit) = ops.iter().find(|o| &o.name == op).and_then(|o| o.admit.as_ref()) else {
            return;
        };
        let mut one = |ir: &Ir| match ir_cost(ir, models, limit) {
            Ok(c) => total = total.saturating_add(c),
            Err(e) => failure = Some(e),
        };
        one(admit);
        for a in args.iter().filter(|a| !a.has_subject()) {
            one(a);
        }
    });
    failure.map_or(Ok(total), Err)
}

/// Which run the total IR work is bounded for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// A `slim` run: [`MAX_IR_WORK_SLIM`].
    Slim,
    /// A `full` run: [`MAX_IR_WORK_FULL`].
    Full,
}

/// The total IR work of a run (VER04r 4.2 rule 6, "Total IR work is bounded as well"):
/// the sum over suites of `ir_cost` times the product ordinals the run executes.  Checked
/// before the first case, in checked arithmetic, so a corpus may spend a larger cost on few
/// cases or a small cost on many, never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrWork {
    total: u128,
    limit: u64,
}

impl IrWork {
    /// An empty account for a run of `depth`.
    pub fn new(depth: Depth) -> IrWork {
        IrWork {
            total: 0,
            limit: match depth {
                Depth::Slim => MAX_IR_WORK_SLIM,
                Depth::Full => MAX_IR_WORK_FULL,
            },
        }
    }

    /// Charge `ordinals` product ordinals of a suite costing `ir_cost` each.
    ///
    /// # Errors
    ///
    /// `limit` if the running total passes the bound.
    pub fn charge(&mut self, ir_cost: u64, ordinals: u64) -> Result<(), GenError> {
        let work = u128::from(ir_cost) * u128::from(ordinals);
        self.total = self.total.saturating_add(work);
        if self.total > u128::from(self.limit) {
            return Err(GenError::new(
                code::LIMIT,
                "",
                "the total IR work of a run exceeds its bound (VER04r 4.2 rule 6)",
            ));
        }
        Ok(())
    }

    /// The work charged so far.
    pub fn total(&self) -> u128 {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Arith, Cmp, Logic};
    use crate::types::Ty;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    fn var(n: &str) -> Box<Ir> {
        Box::new(Ir::Var(n.into()))
    }
    fn lit(v: i128) -> Box<Ir> {
        Box::new(Ir::Int {
            ty: Ty::Fixed(FixedTy::U8),
            value: v,
        })
    }
    fn model(name: &str, body: Ir) -> LoweredModel {
        LoweredModel {
            name: name.into(),
            params: vec![],
            ret: Ty::Bool,
            body,
        }
    }
    fn call(name: &str, args: Vec<Ir>) -> Ir {
        Ir::Model {
            name: name.into(),
            args,
        }
    }
    fn c(ir: &Ir) -> u64 {
        ir_cost(ir, &ModelCosts::default(), 2_000).unwrap()
    }

    #[test]
    fn a_limit_above_the_ceiling_is_refused() {
        let e = ir_cost(
            &Ir::Bool(true),
            &ModelCosts::default(),
            MAX_IR_COST_CEILING + 1,
        )
        .unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert!(ModelCosts::compute(&[], u64::MAX).is_err());
        assert!(ir_cost(&Ir::Bool(true), &ModelCosts::default(), MAX_IR_COST_CEILING).is_ok());
        // checked addition: a partial sum at u64::MAX cannot wrap
        assert!(bump(u64::MAX, 1, u64::MAX).is_err());
    }

    #[test]
    fn leaves_cost_one() {
        assert_eq!(c(&Ir::Bool(true)), 1);
        assert_eq!(c(&Ir::Var("a".into())), 1);
        assert_eq!(c(&Ir::Err("X".into())), 1);
        assert_eq!(c(&Ir::Bytes("00".into())), 1);
    }

    #[test]
    fn operators_cost_one_plus_the_sum_of_their_operands() {
        // eq(a, b) = 1 + 1 + 1 = 3 ; ct-compare's largest expression is the admit
        // `len(a) == len(b)` = 1 + (1+1) + (1+1) = 5
        assert_eq!(c(&Ir::Cmp(Cmp::Eq, var("a"), var("b"))), 3);
        let admit = Ir::Cmp(
            Cmp::Eq,
            Box::new(Ir::Len(var("a"))),
            Box::new(Ir::Len(var("b"))),
        );
        assert_eq!(c(&admit), 5);
        // a - 2u8 : 1 + 1 + 1
        assert_eq!(
            c(&Ir::Arith(
                Arith::Sub,
                Ty::Fixed(FixedTy::U8),
                var("a"),
                lit(2)
            )),
            3
        );
        assert_eq!(c(&Ir::Not(var("a"))), 2);
        assert_eq!(c(&Ir::Neg(Ty::Int, var("a"))), 2);
    }

    #[test]
    fn a_short_circuit_is_never_credited() {
        // and(a, or(b, c)) = 1 + 1 + (1+1+1) = 5 even though `b` may skip `c`
        let t = Ir::Logic(
            Logic::And,
            var("a"),
            Box::new(Ir::Logic(Logic::Or, var("b"), var("c"))),
        );
        assert_eq!(c(&t), 5);
        assert_eq!(c(&Ir::Logic(Logic::Implies, var("a"), var("b"))), 3);
    }

    #[test]
    fn if_charges_the_condition_and_only_the_worse_branch() {
        // if c then t else e  = 1 + 1 + max(1, 1) = 3
        assert_eq!(c(&Ir::If(var("c"), var("t"), var("e"))), 3);
        // then-branch heavier: 1 + 1 + max(3, 1) = 5; else-branch heavier is symmetric
        let heavy = Box::new(Ir::Cmp(Cmp::Eq, var("a"), var("b")));
        assert_eq!(c(&Ir::If(var("c"), heavy.clone(), var("e"))), 5);
        assert_eq!(c(&Ir::If(var("c"), var("e"), heavy)), 5);
    }

    #[test]
    fn the_documented_worked_example_costs_17() {
        // model xor1 has cost 8 (a stand-in body of 8 nodes)
        let body = (0..7).fold(Ir::Bool(true), |acc, _| Ir::Not(Box::new(acc)));
        assert_eq!(c(&body), 8);
        let models = ModelCosts::compute(&[model("xor1", body)], 2_000).unwrap();
        assert_eq!(models.cost("xor1"), Some(8));
        let cond = Ir::Cmp(Cmp::Ge, var("a"), lit(2));
        let then = call(
            "xor1",
            vec![
                Ir::Arith(Arith::Sub, Ty::Fixed(FixedTy::U8), var("a"), lit(2)),
                *var("b"),
            ],
        );
        let other = call("xor1", vec![*var("a"), *var("b")]);
        assert_eq!(ir_cost(&cond, &models, 2_000).unwrap(), 3);
        assert_eq!(ir_cost(&then, &models, 2_000).unwrap(), 13);
        assert_eq!(ir_cost(&other, &models, 2_000).unwrap(), 11);
        let whole = Ir::If(Box::new(cond), Box::new(then), Box::new(other));
        assert_eq!(ir_cost(&whole, &models, 2_000).unwrap(), 17);
    }

    #[test]
    fn subject_op_costs_one_plus_its_arguments_only() {
        let call = Ir::Subject {
            op: "f".into(),
            args: vec![*var("a"), Ir::Len(var("b"))],
            widths: vec![4],
        };
        assert_eq!(c(&call), 1 + 1 + 2);
    }

    #[test]
    fn model_costs_are_computed_once_in_declaration_order_with_depths() {
        let m0 = model("m0", Ir::Cmp(Cmp::Eq, var("a"), var("b"))); // 3
        let m1 = model("m1", call("m0", vec![*var("a"), *var("b")])); // 1 + 2 + 3 = 6
        let m2 = model(
            "m2",
            call("m1", vec![call("m0", vec![*var("a"), *var("b")])]),
        ); // 1+(1+2+3)+6 = 13
        let costs = ModelCosts::compute(&[m0, m1, m2], 2_000).unwrap();
        assert_eq!(costs.cost("m0"), Some(3));
        assert_eq!(costs.cost("m1"), Some(6));
        assert_eq!(costs.cost("m2"), Some(13));
        assert_eq!(costs.depth("m0"), Some(1));
        assert_eq!(costs.depth("m1"), Some(2));
        assert_eq!(costs.depth("m2"), Some(3));
        assert_eq!(costs.cost("nope"), None);
        assert_eq!(costs.depth("nope"), None);
        assert_eq!(costs.order(), ["m0", "m1", "m2"]);
    }

    #[test]
    fn forward_self_and_unknown_model_calls_are_w0701() {
        let later = model("a", call("b", vec![]));
        let b = model("b", Ir::Bool(true));
        let e = ModelCosts::compute(&[later, b], 2_000).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.at, "a");
        let selfcall = model("s", call("s", vec![]));
        assert_eq!(
            ModelCosts::compute(&[selfcall], 2_000).unwrap_err().code,
            code::W0701
        );
        assert_eq!(
            ir_cost(&call("ghost", vec![]), &ModelCosts::default(), 2_000)
                .unwrap_err()
                .code,
            code::W0701
        );
    }

    #[test]
    fn a_doubling_chain_stops_at_the_ceiling_without_overflow() {
        // m_k(x) = m_(k-1)(m_(k-1)(x)): the cost doubles per level.  With 63 levels the
        // true cost is ~2^63; the walk must stop at the limit instead.
        let mut models = vec![model("m0", Ir::Bool(true))];
        for k in 1..=63 {
            let prev = format!("m{}", k - 1);
            models.push(model(
                &format!("m{k}"),
                call(&prev, vec![call(&prev, vec![])]),
            ));
        }
        let e = ModelCosts::compute(&models, MAX_IR_COST_CEILING).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert!(e.message.contains("max_ir_cost"));
        // a short prefix is fine and exact: m_k costs 3 * 2^k - 2 ... check m3
        let short = ModelCosts::compute(&models[..4], 2_000).unwrap();
        // m0=1; m1 = 1+(1+1)+1 = 4; m2 = 1+(1+4)+4 = 10; m3 = 1+(1+10)+10 = 22
        assert_eq!(short.cost("m0"), Some(1));
        assert_eq!(short.cost("m1"), Some(4));
        assert_eq!(short.cost("m2"), Some(10));
        assert_eq!(short.cost("m3"), Some(22));
    }

    #[test]
    fn the_limit_is_inclusive_and_a_declared_budget_raises_it() {
        // A balanced tree of `and`s keeps the depth small while the cost grows.
        fn tree(leaves: usize) -> Ir {
            if leaves == 1 {
                Ir::Bool(true)
            } else {
                let l = leaves / 2;
                Ir::Logic(Logic::And, Box::new(tree(l)), Box::new(tree(leaves - l)))
            }
        }
        // n leaves cost 2n - 1
        assert_eq!(
            ir_cost(&tree(1000), &ModelCosts::default(), 2_000).unwrap(),
            1999
        );
        assert_eq!(
            ir_cost(&tree(1000), &ModelCosts::default(), 1_999).unwrap(),
            1999
        );
        let e = ir_cost(&tree(1000), &ModelCosts::default(), 1_998).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert_eq!(
            ir_cost(&tree(1001), &ModelCosts::default(), 2_000)
                .unwrap_err()
                .code,
            code::LIMIT
        );
        assert_eq!(
            ir_cost(&tree(1001), &ModelCosts::default(), 4_000).unwrap(),
            2001
        );
    }

    #[test]
    fn a_model_that_exceeds_the_limit_alone_is_refused_with_its_name() {
        fn tree(leaves: usize) -> Ir {
            if leaves == 1 {
                Ir::Bool(true)
            } else {
                let l = leaves / 2;
                Ir::Logic(Logic::And, Box::new(tree(l)), Box::new(tree(leaves - l)))
            }
        }
        let e = ModelCosts::compute(&[model("fat", tree(1500))], 2_000).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert_eq!(e.at, "fat");
    }

    #[test]
    fn ir_deeper_than_64_is_refused_before_costing() {
        let mut ir = Ir::Bool(true);
        for _ in 0..64 {
            ir = Ir::Not(Box::new(ir));
        }
        // 65 levels
        assert_eq!(ir.depth(), 65);
        assert_eq!(
            ir_cost(&ir, &ModelCosts::default(), 2_000)
                .unwrap_err()
                .code,
            code::LIMIT
        );
        let ok = match ir {
            Ir::Not(inner) => *inner,
            _ => unreachable!(),
        };
        assert_eq!(ok.depth(), 64);
        assert_eq!(ir_cost(&ok, &ModelCosts::default(), 2_000).unwrap(), 64);
    }

    #[test]
    fn model_call_depth_is_capped_at_64() {
        let mut models = vec![model("m0", Ir::Bool(true))];
        for k in 1..64 {
            models.push(model(
                &format!("m{k}"),
                call(&format!("m{}", k - 1), vec![]),
            ));
        }
        // m63 has depth 64: allowed
        let ok = ModelCosts::compute(&models, 2_000).unwrap();
        assert_eq!(ok.depth("m63"), Some(64));
        models.push(model("m64", call("m63", vec![])));
        let e = ModelCosts::compute(&models, 2_000).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert_eq!(e.at, "m64");
    }

    #[test]
    fn suite_cost_sums_admit_body_and_expected_only() {
        let body = Ir::Cmp(
            Cmp::Eq,
            Box::new(Ir::Subject {
                op: "f".into(),
                args: vec![*var("a")],
                widths: vec![],
            }),
            Box::new(Ir::Model {
                name: "same".into(),
                args: vec![*var("a")],
            }),
        );
        let same = model("same", Ir::Var("a".into()));
        let models = ModelCosts::compute(&[same], 2_000).unwrap();
        let reference = LoweredSuite {
            id: "warrant/x/exhaustive/c".into(),
            claim: "c".into(),
            binders: vec![],
            admit: Ir::Bool(true),
            body: body.clone(),
            kind: SuiteKind::Reference {
                subject: Ir::Subject {
                    op: "f".into(),
                    args: vec![*var("a")],
                    widths: vec![],
                },
                expected: Ir::Model {
                    name: "same".into(),
                    args: vec![*var("a")],
                },
                result: Ty::Bool,
            },
            ops: vec!["f".into()],
        };
        // admit 1 ; body = 1 + (1+1) + (1+1+1) = 6 ; expected = 1 + 1 + 1 = 3
        let sc = suite_cost(&reference, &models, 2_000).unwrap();
        assert_eq!(
            sc,
            SuiteCost {
                admit: 1,
                body: 6,
                expected: Some(3),
                total: 10
            }
        );
        let predicate = LoweredSuite {
            kind: SuiteKind::Predicate,
            ..reference.clone()
        };
        let sp = suite_cost(&predicate, &models, 2_000).unwrap();
        assert_eq!(
            sp,
            SuiteCost {
                admit: 1,
                body: 6,
                expected: None,
                total: 7
            }
        );
        // each part is separately bounded and the error names the claim
        let e = suite_cost(&reference, &models, 5).unwrap_err();
        assert_eq!(e.at, "c");
    }

    #[test]
    fn total_ir_work_is_bounded_with_checked_arithmetic() {
        let mut slim = IrWork::new(Depth::Slim);
        // 2,000 * 32,000,000 = the slim bound exactly
        slim.charge(2_000, 32_000_000).unwrap();
        assert_eq!(slim.total(), u128::from(MAX_IR_WORK_SLIM));
        assert_eq!(slim.charge(1, 1).unwrap_err().code, code::LIMIT);
        let mut full = IrWork::new(Depth::Full);
        full.charge(2_000, 1 << 32).unwrap();
        assert_eq!(full.total(), u128::from(MAX_IR_WORK_FULL));
        assert_eq!(full.charge(1, 1).unwrap_err().code, code::LIMIT);
        // a large cost on few cases, or a small cost on many, never both
        let mut a = IrWork::new(Depth::Slim);
        a.charge(65_536, 900_000).unwrap();
        assert!(a.charge(65_536, 100_000).is_err());
        // u64::MAX * u64::MAX does not overflow the u128 account
        let mut huge = IrWork::new(Depth::Full);
        assert_eq!(
            huge.charge(u64::MAX, u64::MAX).unwrap_err().code,
            code::LIMIT
        );
        assert_eq!(MAX_IR_WORK_SLIM, 32_000_000 * 2_000);
        assert_eq!(MAX_IR_WORK_FULL, (1u64 << 32) * 2_000);
    }
}
