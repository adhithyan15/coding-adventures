//! # The `ir.core` evaluator (VER04r sections 2, 3 and 4.2)
//!
//! The generator needs three things from the model IR, and this module is all of them:
//!
//! | function | what it computes | used for |
//! |---|---|---|
//! | [`Program::eval`] | the value of a **pure** tree | `admit`, `expected`, guards |
//! | [`Program::trace`] | the subject calls on the pure trace, and the wraps of the pure nodes on it | `expected_calls` (4.4), `wraps` (3.3 rule 1) |
//! | [`Program::static_calls`] | the call count when it does not depend on the case | a fast path |
//!
//! It is an **own** implementation over this crate's lowered [`Ir`]: the `warrant-grammar`
//! `Evaluator` evaluates the *AST*, so a lowering bug here could never reach a digest
//! through it, treats a v0 `U8` as `Nat` (WD-013, which [D4] contradicts) and has no
//! `wraps`.  Tests use that evaluator only as a differential on sampled cases.
//!
//! ## Semantics, node by node
//!
//! ```text
//!   lit var                     the constant; the binder or model parameter (innermost first)
//!   len e                       the byte count, a Nat
//!   ok e   err "CASE"           Result construction; errors are nominal
//!   eq ne                       structural, type-directed equality (VER04 section 3)
//!   lt gt le ge                 by mathematical value (an unsigned by its non-negative value)
//!   add sub mul neg  fixed      wrap modulo 2^w, two's complement for signed (VER04r 3.2);
//!                               counts one `wraps` when the wrapped value differs from the
//!                               exact one
//!   add sub mul neg  Nat/Int    exact, inside -(2^127-1)..=2^127-1, else W0701; Nat - Nat
//!                               is an Int
//!   and or implies  if          short-circuit: the untaken operand is not evaluated, so it
//!                               neither costs nor wraps nor calls
//!   concat take drop            total: take/drop clamp, a negative count is 0; a result
//!                               over MAX_IR_VALUE is refused BEFORE it is allocated
//!   is_err e                    true for an Err
//!   tuple project               the components in order / the i-th component (`ir.tuple`)
//!   match s [[pat, arm]..]      evaluates the scrutinee, then **exactly one** arm: the first whose
//!                               pattern matches, with its binders in scope (`ir.tuple`, [D35])
//!   ir.fixed built-ins (35)     every operand left to right, then `fixed_builtin` (the bit work is
//!                               `warrant-grammar`'s `fixed`, agreed with the oracle on every row of
//!                               `fixed-eval-v1`); an Option is `Value::Opt`, consumed by `unwrap_or`
//!                               alone (both operands evaluated); `wraps` counts only add sub mul neg
//!                               [D31], never `wrap shl bit_not ...`
//!   model_call                  evaluates the arguments left to right, then the body in a
//!                               fresh frame
//!   subject_op                  cannot be evaluated: the generator has no subject
//! ```
//!
//! ## The pure trace
//!
//! `expected_calls` and `wraps` are defined on the *pure trace* of a claim body, never on
//! what a subject returned (VER04r 4.4, 3.3 rule 1):
//!
//! ```text
//!   body:   if b != 0 then sub(a, b + 1) == pure(a) else true     binders a, b
//!                │
//!                ├─ guard `b != 0` has no subject: EVALUATE it (its wraps count),
//!                │  then follow ONE branch
//!                ├─ a subject call counts 1, plus the calls in its arguments;
//!                │  its pure arguments are EVALUATED (their wraps count)
//!                └─ a node with a subject beneath it is never evaluated itself
//! ```
//!
//! An operand that only runs under a short circuit whose guard depends on a subject result
//! is skipped (the guard is not pure, which 4.4 refuses anyway for operands that call).
//!
//! A `match` arm is a **guard** whose condition is the scrutinee: the arm taken is decided by the
//! scrutinee's value and the patterns.  A subject call in an arm therefore needs a pure
//! scrutinee (else `W0701`, the impure guard), and then only the taken arm is on the trace, with
//! the pattern's binders in scope for the nodes beneath it.  A `match` with no subject call in
//! any arm is counted by its scrutinee alone.
//!
//! ## Seeded-wrong controls
//!
//! [`Sabotage`] makes the evaluator wrong on purpose (`eq` answering `ne`, `sub` adding,
//! fixed arithmetic not wrapping).  It is `#[doc(hidden)]` and used only by the tests that
//! prove a wrong evaluator changes a digest (VER04r 10.3: a corpus is only as good as the
//! evidence that its checks can fail).

use std::collections::HashMap;
use std::rc::Rc;

use coding_adventures_warrant_grammar::fixed::{self, FixedTy};

use crate::arith::{
    check_value_len, exact_neg, exact_op, fixed_neg, fixed_op, EXACT_BOUND, MAX_IR_VALUE,
};
use crate::error::{code, GenError};
use crate::ir::{Arith, Cmp, Ir, Logic, Pat};
use crate::lower::{LoweredModel, LoweredOp};
use crate::patterns::match_pattern;
use crate::types::Decls;
use crate::value::{ill_typed, Value};

/// The most model frames that may be active at once (VER04 section 2, VER04r 14).
pub const MAX_MODEL_FRAMES: usize = 64;

/// The most pending tasks the evaluator's explicit work stack may hold.  This is **not a new
/// limit**: the number of pending tasks is bounded by the cost of the worst path through the
/// expression, which is at most `max_ir_cost` (VER04r 4.2 rule 6), which is at most 65,536.
/// The constant is that ceiling, kept as a guard on the heap (the evaluator does not recurse,
/// so the call stack is not at risk) and checked on every pop so a hand-built tree cannot
/// outrun it.
pub const MAX_EVAL_TASKS: usize = 1 << 16;

/// The counters the evaluator maintains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Fixed-width node evaluations whose wrapped result differs from the exact result
    /// (VER04r 3.3 rule 1).  Every evaluation counts: there is no memoisation, so the
    /// count cannot depend on cache state or on which suite ran first.
    pub wraps: u64,
}

/// A deliberately wrong evaluator, for the negative controls.  Public only with the
/// `test-hooks` feature (or in this crate's own unit tests); otherwise it is private and the
/// only value is `None`.
macro_rules! sabotage_enum {
    ($vis:vis) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        #[allow(dead_code)]
        $vis enum Sabotage {
            /// The correct evaluator.
            #[default]
            None,
            /// `eq` answers `ne`.
            EqIsNe,
            /// `sub` adds (on every numeric type).
            SubIsAdd,
            /// Fixed-width arithmetic is exact and never wraps (the pre-[D4] oracle behaviour).
            NoWrap,
        }
    };
}
#[cfg(any(test, feature = "test-hooks"))]
sabotage_enum!(pub);
#[cfg(not(any(test, feature = "test-hooks")))]
sabotage_enum!(pub(crate));

/// The variables in scope: binder or parameter names with their values.
pub type Vars<'v> = [(&'v str, Value)];

/// The models of a document plus what the evaluator needs to know about them.
#[derive(Debug, Clone)]
pub struct Program<'a> {
    models: &'a [LoweredModel],
    /// The document's operations: the trace checks every subject call against its
    /// op's `admit` (VER04r 6, [D19]).  Empty unless [`Program::with_ops`] set it.
    ops: &'a [LoweredOp],
    /// A v0 document: a wrap while evaluating a callee's `admit` is refused ([D18], 3.3 rule 2).
    v0: bool,
    index: HashMap<&'a str, usize>,
    /// `wraps[i]`: model `i`'s call closure contains a fixed-width `add sub mul neg`.
    wraps: Vec<bool>,
    /// `fails[i]`: model `i`'s call closure has an `add sub mul neg` of any type or a
    /// `concat`, i.e. a node that can wrap or leave a bound.
    fails: Vec<bool>,
    sabotage: Sabotage,
    /// The document's records and variants (`ir.data`): what the encoders and writers of a value of a
    /// named type need.  Empty unless [`Program::with_decls`] set it.
    decls: Decls,
    /// The work-stack bound ([`MAX_EVAL_TASKS`] unless a test lowers it).
    max_tasks: usize,
}

impl<'a> Program<'a> {
    /// The program of a lowered document's models (declaration order, which is also the
    /// only order a model may call in: VER04r 4.2 rule 1).
    pub fn new(models: &'a [LoweredModel]) -> Program<'a> {
        let index: HashMap<&str, usize> = models
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.as_str(), i))
            .collect();
        let mut wraps: Vec<bool> = Vec::with_capacity(models.len());
        let mut fails: Vec<bool> = Vec::with_capacity(models.len());
        for m in models {
            let f = may_fail_with(&m.body, &|name| {
                index
                    .get(name)
                    .and_then(|i| fails.get(*i))
                    .copied()
                    .unwrap_or(false)
            });
            fails.push(f);
            let w = may_wrap_with(&m.body, &|name| {
                index
                    .get(name)
                    .and_then(|i| wraps.get(*i))
                    .copied()
                    .unwrap_or(false)
            });
            wraps.push(w);
        }
        Program {
            models,
            ops: &[],
            v0: false,
            index,
            wraps,
            fails,
            sabotage: Sabotage::None,
            decls: Decls::default(),
            max_tasks: MAX_EVAL_TASKS,
        }
    }

    /// The same program that also knows the document's records and variants (`ir.data`).
    pub fn with_decls(mut self, decls: Decls) -> Program<'a> {
        self.decls = decls;
        self
    }

    /// The records and variants this program knows.
    pub fn decls(&self) -> &Decls {
        &self.decls
    }

    /// The same program that also knows the document's operations, so that [`Program::trace`]
    /// refuses a subject call whose arguments are outside its op's `admit` (VER04r 6, [D19]).
    /// `v0` says the document is v0: then a wrap while evaluating an `admit` is refused
    /// (3.3 rule 2 applies to every evaluation the generator performs), though it is never
    /// added to the recorded `wraps`.
    pub fn with_ops(mut self, ops: &'a [LoweredOp], v0: bool) -> Program<'a> {
        self.ops = ops;
        self.v0 = v0;
        self
    }

    /// The `admit` of operation `op`, with the op, if it has one.
    fn op_admit(&self, op: &str) -> Option<(&'a LoweredOp, &'a Ir)> {
        let lop = self.ops.iter().find(|o| o.name == op)?;
        lop.admit.as_ref().map(|a| (lop, a))
    }

    /// Does `ir` contain a subject call of an operation that has an `admit`?  Only such a
    /// body needs the per-case admit check (and so the trace).
    pub fn calls_admitted_op(&self, ir: &Ir) -> bool {
        let mut found = false;
        ir.walk(&mut |n| {
            if let Ir::Subject { op, .. } = n {
                found |= self.op_admit(op).is_some();
            }
        });
        found
    }

    /// The static half of [D19]: a call of an operation with an `admit` whose `admit` reads a
    /// parameter whose argument contains a subject call cannot be checked at generation (the
    /// generator never learns what the inner call returns), so it is refused, never skipped.
    /// A subject-dependent argument for a parameter the `admit` does not read is fine
    /// (`power(add(a, b), 2u64)` with `admit` `exp <= 4294967295`): it is simply not an input
    /// of the check.
    ///
    /// # Errors
    ///
    /// `W0701`.
    pub fn check_admit_calls(&self, ir: &Ir) -> Result<(), GenError> {
        let mut bad: Option<String> = None;
        ir.walk(&mut |n| {
            if let Ir::Subject { op, args, .. } = n {
                if bad.is_some() {
                    return;
                }
                if let Some((lop, admit)) = self.op_admit(op) {
                    let mut reads: Vec<String> = Vec::new();
                    admit.walk(&mut |m| {
                        if let Ir::Var(v) = m {
                            reads.push(v.clone());
                        }
                    });
                    let dependent = lop
                        .params
                        .iter()
                        .zip(args)
                        .any(|((p, _), a)| a.has_subject() && reads.contains(p));
                    if dependent {
                        bad = Some(op.clone());
                    }
                }
            }
        });
        match bad {
            None => Ok(()),
            Some(op) => Err(GenError::new(
                code::W0701,
                "",
                format!(
                    "the admit of op `{op}` reads a parameter whose argument depends on a \
                     subject result, so it cannot be checked at generation (VER04r 6)"
                ),
            )),
        }
    }

    /// Are the argument values `args` inside the `admit` of operation `op`?  `None` is an
    /// argument the generator cannot know (it depends on a subject result): the plan has
    /// refused every call whose `admit` reads one, so the `admit` never looks at it.  An
    /// operation without an `admit` admits everything.  Wraps counted here are not the
    /// document's (VER04r 3.3 rule 1 counts the nodes of the claim, not this check).
    ///
    /// # Errors
    ///
    /// Every [`Program::eval`] error.
    pub fn admits_call(&self, op: &str, args: &[Option<Value>]) -> Result<bool, GenError> {
        let Some((lop, admit)) = self.op_admit(op) else {
            return Ok(true);
        };
        let pvars: Vec<(&str, Value)> = lop
            .params
            .iter()
            .zip(args)
            .filter_map(|((n, _), v)| v.clone().map(|v| (n.as_str(), v)))
            .collect();
        let mut st = Stats::default();
        // A size error while evaluating a callee's `admit` is a plain refusal (VER04r 8.4 step 3):
        // it is never what an `omit_example` line excuses as `value-over-limit`, so the flag
        // does not leave this function.
        let admitted = self
            .eval(admit, &pvars, &mut st)
            .map_err(|mut e| {
                e.size = false;
                e
            })?
            .as_bool()?;
        if self.v0 && st.wraps > 0 {
            // [D18]: not recorded in `wraps`, but refused: the v0 harness would evaluate the
            // same `admit` as `Nat`.
            return Err(GenError::new(
                code::W0701,
                "",
                format!(
                    "U8 arithmetic wraps while evaluating the admit of op `{op}`; v0 harness \
                     evaluates U8 as Nat (VER04r 3.3 rule 2)"
                ),
            )
            .in_case());
        }
        Ok(admitted)
    }

    /// The same program with a seeded-wrong evaluator.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn with_sabotage(mut self, s: Sabotage) -> Program<'a> {
        self.sabotage = s;
        self
    }

    /// The same program with a lower work-stack bound, to test the guard.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn with_max_tasks(mut self, tasks: usize) -> Program<'a> {
        self.max_tasks = tasks.min(MAX_EVAL_TASKS);
        self
    }

    /// Could evaluating `ir` perform any `add sub mul neg` (of any type: a fixed-width one
    /// can wrap, an exact one can leave the 2^127 window) or `concat` (which can pass
    /// `MAX_IR_VALUE`), directly or inside a model it calls?  Such a pure node on the trace
    /// is evaluated so that a failure surfaces at generation (VER04r 3.2, 3.3).
    pub fn may_fail_or_wrap(&self, ir: &Ir) -> bool {
        may_fail_with(ir, &|name| {
            self.index
                .get(name)
                .and_then(|i| self.fails.get(*i))
                .copied()
                .unwrap_or(false)
        })
    }

    /// Could evaluating `ir` perform a fixed-width `+ - *` or unary `-` (directly or
    /// inside a model it calls)?
    pub fn may_wrap(&self, ir: &Ir) -> bool {
        may_wrap_with(ir, &|name| {
            self.index
                .get(name)
                .and_then(|i| self.wraps.get(*i))
                .copied()
                .unwrap_or(false)
        })
    }

    /// Does model `name` (or a model it calls) perform fixed-width arithmetic?
    pub fn model_wraps(&self, name: &str) -> bool {
        self.index
            .get(name)
            .and_then(|i| self.wraps.get(*i))
            .copied()
            .unwrap_or(false)
    }

    /// Evaluate a pure tree.  `st.wraps` is incremented for every fixed-width node whose
    /// wrapped result differs from the exact one.
    ///
    /// The evaluator is **iterative**: an explicit heap-allocated work stack replaces the
    /// call stack, so the documented bounds (IR depth 64, 64 model frames: VER04r 14) run on
    /// a default 2 MiB thread in a debug build.
    ///
    /// # Errors
    ///
    /// `W0701` for a subject call, an unbound variable, an ill-typed hand-built tree, an
    /// exact `Nat`/`Int` outside the 2^127 window or a `Bytes` result over
    /// [`MAX_IR_VALUE`]; `limit` for more than [`MAX_MODEL_FRAMES`] active model frames or a
    /// work stack past [`MAX_EVAL_TASKS`].
    pub fn eval<'t>(
        &'t self,
        ir: &'t Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
    ) -> Result<Value, GenError> {
        let mut tasks: Vec<Task<'t>> = vec![Task::Eval(ir)];
        let mut vals: Vec<Value> = Vec::new();
        let mut frames: Vec<Vec<(&'t str, Value)>> = Vec::new();
        // The binders of the `match` arms being evaluated, as `(frame depth, name, value)`: the
        // entries of the current frame are contiguous at the top (an arm finishes before the frame
        // that holds it is popped), so a lookup stops at the first entry of another depth.
        let mut binds: Vec<(usize, &'t str, Value)> = Vec::new();
        while let Some(task) = tasks.pop() {
            if tasks.len() > self.max_tasks {
                return Err(GenError::new(
                    code::LIMIT,
                    "",
                    "evaluation holds more pending work than the IR bounds allow",
                ));
            }
            match task {
                Task::Eval(node) => {
                    let env: &Vars<'_> = frames.last().map_or(vars, |f| f.as_slice());
                    if let Ir::Var(name) = node {
                        let depth = frames.len();
                        let bound = binds
                            .iter()
                            .rev()
                            .take_while(|(d, _, _)| *d == depth)
                            .find(|(_, n, _)| *n == name.as_str())
                            .map(|(_, _, v)| v.clone());
                        if let Some(v) = bound {
                            vals.push(v);
                            continue;
                        }
                    }
                    self.start(node, env, &mut tasks, &mut vals)?;
                }
                Task::Arms(arms) => {
                    let value = pop(&mut vals)?;
                    let depth = frames.len();
                    let mut picked = None;
                    for (pattern, body) in arms {
                        let mut found: Vec<(&'t str, Value)> = Vec::new();
                        if match_pattern(pattern, &value, &mut found) {
                            picked = Some((body, found));
                            break;
                        }
                    }
                    let Some((body, found)) = picked else {
                        return Err(GenError::internal("a match fell through"));
                    };
                    tasks.push(Task::PopBinds(found.len()));
                    tasks.push(Task::Eval(body));
                    binds.extend(found.into_iter().map(|(n, v)| (depth, n, v)));
                }
                Task::PopBinds(n) => {
                    binds.truncate(binds.len().saturating_sub(n));
                }
                Task::Apply(node) => self.apply(node, &mut vals, st)?,
                Task::CallModel(i) => {
                    let m = &self.models[i];
                    let at = vals
                        .len()
                        .checked_sub(m.params.len())
                        .ok_or_else(|| GenError::internal("a model call lost its arguments"))?;
                    let frame: Vec<(&str, Value)> = m
                        .params
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .zip(vals.drain(at..))
                        .collect();
                    frames.push(frame);
                    if frames.len() > MAX_MODEL_FRAMES {
                        return Err(GenError::new(
                            code::LIMIT,
                            "",
                            "model calls nest deeper than 64 frames",
                        ));
                    }
                    tasks.push(Task::PopFrame);
                    tasks.push(Task::Eval(&m.body));
                }
                Task::PopFrame => {
                    frames.pop();
                }
                Task::Logic(op, right) => {
                    let left = pop(&mut vals)?.as_bool()?;
                    // Short circuit: `and`/`implies` evaluate the right operand when the
                    // left is true; `or` when it is false.  `implies` is `not left or right`.
                    let (decided, value) = match op {
                        Logic::And => (!left, false),
                        Logic::Or => (left, true),
                        Logic::Implies => (!left, true),
                    };
                    if decided {
                        vals.push(Value::Bool(value));
                    } else {
                        tasks.push(Task::CheckBool);
                        tasks.push(Task::Eval(right));
                    }
                }
                Task::CheckBool => {
                    let b = pop(&mut vals)?.as_bool()?;
                    vals.push(Value::Bool(b));
                }
                Task::Branch(t, e) => {
                    let c = pop(&mut vals)?.as_bool()?;
                    tasks.push(Task::Eval(if c { t } else { e }));
                }
            }
        }
        pop(&mut vals)
    }

    /// Begin evaluating `node`: push a leaf's value, or schedule its operands and then its
    /// application (operands are evaluated left to right).
    fn start<'t>(
        &'t self,
        node: &'t Ir,
        env: &Vars<'_>,
        tasks: &mut Vec<Task<'t>>,
        vals: &mut Vec<Value>,
    ) -> Result<(), GenError> {
        match node {
            Ir::Bool(b) => vals.push(Value::Bool(*b)),
            Ir::Int { value, .. } => vals.push(Value::Int(*value)),
            Ir::Bytes(hex) => vals.push(Value::bytes(&decode_hex(hex)?)),
            Ir::Var(name) => vals.push(lookup(env, name)?.clone()),
            Ir::Err(case) => vals.push(Value::Err(Rc::from(case.as_str()))),
            Ir::Subject { .. } => {
                return Err(GenError::new(
                    code::W0701,
                    "",
                    "a subject call cannot be evaluated by the generator (VER04r 4.2 rule 3)",
                ))
            }
            Ir::Model { name, args } => {
                let i = *self
                    .index
                    .get(name.as_str())
                    .ok_or_else(|| GenError::new(code::W0701, "", "a call to an unknown model"))?;
                if self.models[i].params.len() != args.len() {
                    return Err(GenError::new(
                        code::W0701,
                        "",
                        "a model call has the wrong arity",
                    ));
                }
                tasks.push(Task::CallModel(i));
                tasks.extend(args.iter().rev().map(Task::Eval));
            }
            Ir::Len(a) | Ir::Ok(a) | Ir::Not(a) | Ir::IsErr(a) | Ir::Neg(_, a) => {
                tasks.push(Task::Apply(node));
                tasks.push(Task::Eval(a));
            }
            Ir::Cmp(_, a, b)
            | Ir::Arith(_, _, a, b)
            | Ir::Concat(a, b)
            | Ir::Take(a, b)
            | Ir::Drop(a, b) => {
                tasks.push(Task::Apply(node));
                tasks.push(Task::Eval(b));
                tasks.push(Task::Eval(a));
            }
            Ir::Logic(op, a, b) => {
                tasks.push(Task::Logic(*op, b));
                tasks.push(Task::Eval(a));
            }
            Ir::If(c, t, e) => {
                tasks.push(Task::Branch(t, e));
                tasks.push(Task::Eval(c));
            }
            Ir::Tuple(parts) => {
                tasks.push(Task::Apply(node));
                tasks.extend(parts.iter().rev().map(Task::Eval));
            }
            Ir::Project(base, _) => {
                tasks.push(Task::Apply(node));
                tasks.push(Task::Eval(base));
            }
            Ir::Match {
                scrutinee, arms, ..
            } => {
                tasks.push(Task::Arms(arms));
                tasks.push(Task::Eval(scrutinee));
            }
            // `ir.data` (S11c-2): `some`, `ctor` and `field` evaluate their operands, then apply;
            // `none` is a leaf.
            Ir::None(_) => vals.push(Value::Opt(None)),
            Ir::Some(a) | Ir::Field(a, _) => {
                tasks.push(Task::Apply(node));
                tasks.push(Task::Eval(a));
            }
            Ir::Ctor { args, .. } => {
                tasks.push(Task::Apply(node));
                tasks.extend(args.iter().rev().map(Task::Eval));
            }
            // The 35 built-ins of Appendix A.1 (S10c-2): every operand left to right, then the
            // node.  `unwrap_or` evaluates both of its operands (no short circuit).
            Ir::Fixed { args, .. } => {
                tasks.push(Task::Apply(node));
                tasks.extend(args.iter().rev().map(Task::Eval));
            }
        }
        Ok(())
    }

    /// Apply a node to its already-evaluated operands (on top of `vals`).
    fn apply(&self, node: &Ir, vals: &mut Vec<Value>, st: &mut Stats) -> Result<(), GenError> {
        let v = match node {
            Ir::Len(_) => Value::Int(pop(vals)?.as_bytes()?.len() as i128),
            Ir::Ok(_) => Value::Ok(Box::new(pop(vals)?)),
            Ir::Not(_) => Value::Bool(!pop(vals)?.as_bool()?),
            Ir::IsErr(_) => match pop(vals)? {
                Value::Err(_) => Value::Bool(true),
                Value::Ok(_) => Value::Bool(false),
                _ => return Err(ill_typed("a Result operand")),
            },
            Ir::Neg(ty, _) => {
                let x = pop(vals)?.as_int()?;
                Value::Int(match ty.fixed() {
                    Some(t) if self.sabotage != Sabotage::NoWrap => {
                        let (v, wrapped) = fixed_neg(t, x);
                        st.wraps += u64::from(wrapped);
                        v
                    }
                    _ => exact_neg(x)?,
                })
            }
            Ir::Cmp(op, ..) => {
                let y = pop(vals)?;
                let x = pop(vals)?;
                Value::Bool(self.compare(*op, &x, &y)?)
            }
            Ir::Arith(op, ty, ..) => {
                let y = pop(vals)?.as_int()?;
                let x = pop(vals)?.as_int()?;
                Value::Int(self.arith(*op, ty.fixed(), x, y, st)?)
            }
            Ir::Concat(..) => {
                let y = pop(vals)?;
                let x = pop(vals)?;
                let (x, y) = (x.as_bytes()?, y.as_bytes()?);
                // Checked BEFORE the allocation (VER04r 4.2 rule 6).
                check_value_len((x.len() as u64) + (y.len() as u64))?;
                let mut out = Vec::with_capacity(x.len() + y.len());
                out.extend_from_slice(x);
                out.extend_from_slice(y);
                Value::Bytes(Rc::from(out))
            }
            Ir::Tuple(parts) => {
                let at = vals
                    .len()
                    .checked_sub(parts.len())
                    .ok_or_else(|| GenError::internal("a tuple lost its components"))?;
                Value::Tuple(vals.drain(at..).collect())
            }
            Ir::Some(_) => Value::Opt(Some(Box::new(pop(vals)?))),
            Ir::Ctor { name, args } => {
                let at = vals
                    .len()
                    .checked_sub(args.len())
                    .ok_or_else(|| GenError::internal("a constructor lost its operands"))?;
                Value::Con(Rc::from(name.as_str()), vals.drain(at..).collect())
            }
            Ir::Field(_, index) => match pop(vals)? {
                Value::Con(_, items) => items
                    .into_iter()
                    .nth(*index)
                    .ok_or_else(|| ill_typed("a record with that field"))?,
                _ => return Err(ill_typed("a record operand")),
            },
            Ir::Project(_, index) => match pop(vals)? {
                Value::Tuple(items) => items
                    .into_iter()
                    .nth(*index)
                    .ok_or_else(|| ill_typed("a tuple with that component"))?,
                _ => return Err(ill_typed("a tuple operand")),
            },
            Ir::Take(..) | Ir::Drop(..) => {
                let n = pop(vals)?.as_int()?;
                let x = pop(vals)?;
                let x = x.as_bytes()?;
                let at = clamp_count(n, x.len());
                Value::bytes(if matches!(node, Ir::Take(..)) {
                    &x[..at]
                } else {
                    &x[at..]
                })
            }
            Ir::Fixed {
                tag,
                ty,
                at: width,
                args,
            } => {
                let from = vals
                    .len()
                    .checked_sub(args.len())
                    .ok_or_else(|| GenError::internal("a built-in lost its operands"))?;
                let operands: Vec<Value> = vals.drain(from..).collect();
                fixed_builtin(tag, ty.or(*width), &operands)?
            }
            _ => return Err(GenError::internal("a node was applied without operands")),
        };
        vals.push(v);
        Ok(())
    }

    fn compare(&self, op: Cmp, x: &Value, y: &Value) -> Result<bool, GenError> {
        Ok(match op {
            Cmp::Eq => (x == y) != (self.sabotage == Sabotage::EqIsNe),
            Cmp::Ne => x != y,
            Cmp::Lt => x.as_int()? < y.as_int()?,
            Cmp::Gt => x.as_int()? > y.as_int()?,
            Cmp::Le => x.as_int()? <= y.as_int()?,
            Cmp::Ge => x.as_int()? >= y.as_int()?,
        })
    }

    fn arith(
        &self,
        op: Arith,
        fixed: Option<coding_adventures_warrant_grammar::fixed::FixedTy>,
        x: i128,
        y: i128,
        st: &mut Stats,
    ) -> Result<i128, GenError> {
        let op = if self.sabotage == Sabotage::SubIsAdd && op == Arith::Sub {
            Arith::Add
        } else {
            op
        };
        match fixed {
            Some(t) if self.sabotage != Sabotage::NoWrap => {
                let (v, wrapped) = fixed_op(op, t, x, y);
                st.wraps += u64::from(wrapped);
                Ok(v)
            }
            // `NoWrap` sabotage, or an exact type: exact arithmetic inside the window.
            _ => exact_op(op, x, y),
        }
    }

    /// The subject calls on the pure trace of `ir` (VER04r 4.4), evaluating the pure nodes
    /// that sit on it so their wraps are counted (3.3 rule 1).  See the module
    /// documentation for what is on the trace.
    ///
    /// # Errors
    ///
    /// `W0701` for a subject call under an impure guard (the lowering already refuses it)
    /// and every [`Program::eval`] error.
    pub fn trace<'c>(
        &self,
        ir: &'c Ir,
        vars: &'c Vars<'c>,
        st: &mut Stats,
    ) -> Result<u64, GenError> {
        let impure = || {
            GenError::new(
                code::W0701,
                "",
                "a subject call under an impure guard (VER04r 4.4)",
            )
        };
        // The scopes of the `match` arms taken so far: the case's variables plus the pattern's
        // binders.  `None` is the case's own variables.
        let mut scopes: Vec<Vec<(&'c str, Value)>> = Vec::new();
        let mut stack: Vec<(&'c Ir, Option<usize>)> = vec![(ir, None)];
        let mut calls = 0u64;
        while let Some((node, scope)) = stack.pop() {
            if !node.has_subject() {
                // Every pure node on the trace that can wrap or leave a bound is evaluated,
                // so a failure surfaces here and not in every lane (VER04r 3.2, 3.3).
                if self.may_fail_or_wrap(node) {
                    let env: &Vars<'_> = scope.map_or(vars, |i| scopes[i].as_slice());
                    self.eval(node, env, st)?;
                }
                continue;
            }
            let env: &Vars<'_> = scope.map_or(vars, |i| scopes[i].as_slice());
            match node {
                Ir::Subject { op, args, .. } => {
                    calls += 1;
                    if self.op_admit(op).is_some() {
                        // [D19]: this call is on the trace, so its arguments (pure: the
                        // plan refuses the rest) must be inside the callee's `admit`.
                        let mut vals = Vec::with_capacity(args.len());
                        for a in args {
                            vals.push(if a.has_subject() {
                                None
                            } else {
                                Some(self.eval(a, env, &mut Stats::default())?)
                            });
                        }
                        if !self.admits_call(op, &vals)? {
                            return Err(outside_admit(op));
                        }
                    }
                    // Left to right, as the oracle walks: the first offending call of a case
                    // is the same one in both (a stack pops the last pushed first).
                    stack.extend(args.iter().rev().map(|a| (a, scope)));
                }
                Ir::If(c, t, e) => {
                    if t.has_subject() || e.has_subject() {
                        if c.has_subject() {
                            return Err(impure());
                        }
                        let taken = self.eval(c, env, st)?.as_bool()?;
                        stack.push((if taken { t } else { e }, scope));
                    } else {
                        stack.push((c, scope));
                    }
                }
                Ir::Logic(op, a, b) => {
                    if b.has_subject() {
                        if a.has_subject() {
                            return Err(impure());
                        }
                        let left = self.eval(a, env, st)?.as_bool()?;
                        let runs = match op {
                            Logic::And | Logic::Implies => left,
                            Logic::Or => !left,
                        };
                        if runs {
                            stack.push((b, scope));
                        }
                    } else {
                        stack.push((a, scope));
                    }
                }
                Ir::Match {
                    scrutinee, arms, ..
                } => {
                    // A `match` arm is a guard whose condition is the scrutinee (VER04r 4.4).
                    if arms.iter().any(|(_, body)| body.has_subject()) {
                        if scrutinee.has_subject() {
                            return Err(impure());
                        }
                        let value = self.eval(scrutinee, env, st)?;
                        let mut picked = None;
                        for (pattern, body) in arms {
                            let mut found: Vec<(&'c str, Value)> = Vec::new();
                            if match_pattern(pattern, &value, &mut found) {
                                picked = Some((body, found));
                                break;
                            }
                        }
                        let Some((body, found)) = picked else {
                            return Err(GenError::internal("a match fell through"));
                        };
                        let mut inner: Vec<(&'c str, Value)> = env.to_vec();
                        inner.extend(found);
                        scopes.push(inner);
                        stack.push((body, Some(scopes.len() - 1)));
                    } else {
                        // Only the scrutinee calls: the arms are not on the trace.
                        stack.push((scrutinee, scope));
                    }
                }
                other => stack.extend(other.children().into_iter().rev().map(|c| (c, scope))),
            }
        }
        Ok(calls)
    }

    /// The call count of `ir` when it does not depend on the case: no `if`, short
    /// circuit or `match` arm has a subject in a branch.  `None` means "trace each case".
    pub fn static_calls(&self, ir: &Ir) -> Option<u64> {
        let mut stack = vec![ir];
        let mut calls = 0u64;
        while let Some(node) = stack.pop() {
            if !node.has_subject() {
                continue;
            }
            match node {
                Ir::Subject { args, .. } => {
                    calls += 1;
                    stack.extend(args.iter());
                }
                Ir::If(_, t, e) if t.has_subject() || e.has_subject() => return None,
                Ir::If(c, ..) => stack.push(c),
                Ir::Logic(_, _, b) if b.has_subject() => return None,
                Ir::Logic(_, a, _) => stack.push(a),
                Ir::Match { arms, .. } if arms.iter().any(|(_, b)| b.has_subject()) => {
                    return None
                }
                Ir::Match { scrutinee, .. } => stack.push(scrutinee),
                other => stack.extend(other.children()),
            }
        }
        Some(calls)
    }
}

/// The refusal of [D19] for a subject call outside its op's `admit` (the caller adds the
/// claim and the admitted-case ordinal).
pub fn outside_admit(op: &str) -> GenError {
    GenError::new(
        code::W0701,
        "",
        format!("a call of op `{op}` has arguments outside the op's admit (VER04r 6)"),
    )
    .in_case()
}

/// The work items of the iterative evaluator.
enum Task<'t> {
    /// Evaluate a node (push its value, or schedule its operands).
    Eval(&'t Ir),
    /// Apply a node to the operand values on top of the value stack.
    Apply(&'t Ir),
    /// Open the frame of model `i` over the argument values on the value stack.
    CallModel(usize),
    /// Close the innermost model frame.
    PopFrame,
    /// `and`/`or`/`implies`: the left value is on the stack; the right operand follows.
    Logic(Logic, &'t Ir),
    /// The right operand of a connective was evaluated: it must be a Bool.
    CheckBool,
    /// `if`: the condition is on the stack; take one branch.
    Branch(&'t Ir, &'t Ir),
    /// `match`: the scrutinee's value is on the stack; evaluate the first arm that matches.
    Arms(&'t [(Pat, Ir)]),
    /// Close the binders of the arm just evaluated.
    PopBinds(usize),
}

fn pop(vals: &mut Vec<Value>) -> Result<Value, GenError> {
    vals.pop()
        .ok_or_else(|| GenError::internal("the value stack ran dry"))
}

/// One `ir.fixed` built-in of VER04r Appendix A.1 on its evaluated operands.  `t` is the width it
/// works at (the static type argument if it has one, else the first operand's type: [`Ir::Fixed`]).
///
/// The bit-level work is `warrant-grammar`'s `fixed` module, which the oracle's independent
/// evaluator (`ir_eval._fixed_fn`) agrees with on every operand of `tests/fixed_eval_parity.rs`.  What
/// this function adds is the *dispatch* and the three conventions the module does not own:
///
/// ```text
///   Option       try checked_* div rem div_trunc rem_trunc at   give `Value::Opt`; `unwrap_or` is
///                                                               the one consumer, and evaluates
///                                                               both operands (no short circuit)
///   shift count  an exact Nat (never reduced by the host); n >= w is decided by `fixed::*`
///   wraps        none of these counts a wrap [D31]: only add sub mul neg do (the `Arith` and
///                `Neg` nodes), so `wrap`, `shl`, `bit_not` ... never touch `Stats`
/// ```
fn fixed_builtin(tag: &str, t: Option<FixedTy>, ops: &[Value]) -> Result<Value, GenError> {
    let ty = || t.ok_or_else(|| ill_typed("a width for the built-in"));
    let int = |i: usize| -> Result<i128, GenError> {
        ops.get(i)
            .ok_or_else(|| GenError::internal("a built-in lost an operand"))?
            .as_int()
    };
    // A shift count or bit index is a Nat: never negative.
    let count = |i: usize| -> Result<u128, GenError> {
        u128::try_from(int(i)?).map_err(|_| ill_typed("a Nat count"))
    };
    let some = |v: Option<i128>| Value::Opt(v.map(|x| Box::new(Value::Int(x))));
    Ok(match tag {
        "max_value" => Value::Int(ty()?.max()),
        "min_value" => Value::Int(ty()?.min()),
        "bit_and" => Value::Int(fixed::bit_and(ty()?, int(0)?, int(1)?)),
        "bit_or" => Value::Int(fixed::bit_or(ty()?, int(0)?, int(1)?)),
        "bit_xor" => Value::Int(fixed::bit_xor(ty()?, int(0)?, int(1)?)),
        "bit_not" => Value::Int(fixed::bit_not(ty()?, int(0)?)),
        "shl" => Value::Int(fixed::shl(ty()?, int(0)?, count(1)?)),
        "shr" => Value::Int(fixed::shr(ty()?, int(0)?, count(1)?)),
        "sar" => Value::Int(fixed::sar(ty()?, int(0)?, count(1)?)),
        "rotl" => Value::Int(fixed::rotl(ty()?, int(0)?, count(1)?)),
        "rotr" => Value::Int(fixed::rotr(ty()?, int(0)?, count(1)?)),
        "bit" => Value::Bool(fixed::bit(ty()?, int(0)?, count(1)?)),
        "popcount" => Value::Int(i128::try_from(fixed::popcount(ty()?, int(0)?)).unwrap_or(0)),
        "clz" => Value::Int(i128::try_from(fixed::clz(ty()?, int(0)?)).unwrap_or(0)),
        "ctz" => Value::Int(i128::try_from(fixed::ctz(ty()?, int(0)?)).unwrap_or(0)),
        // `widen` and the two views of the mathematical value change the type, not the integer.
        "widen" | "to_int" | "to_nat" => Value::Int(int(0)?),
        "wrap" => Value::Int(ty()?.wrap(int(0)?)),
        "try" => some(ty()?.try_from_int(int(0)?)),
        "checked_add" => some(fixed::checked('+', ty()?, int(0)?, int(1)?)),
        "checked_sub" => some(fixed::checked('-', ty()?, int(0)?, int(1)?)),
        "checked_mul" => some(fixed::checked('*', ty()?, int(0)?, int(1)?)),
        // Floor division and the divisor-signed remainder: on `Int`/`Nat` operands, and, for two
        // unsigned fixed operands (the only fixed pair the typer admits), plain division.
        "div" => some(fixed::floor_div(int(0)?, int(1)?)),
        "rem" => some(fixed::floor_mod(int(0)?, int(1)?)),
        "div_trunc" => some(fixed::div_trunc(ty()?, int(0)?, int(1)?)),
        "rem_trunc" => some(fixed::rem_trunc(ty()?, int(0)?, int(1)?)),
        "imin" => Value::Int(int(0)?.min(int(1)?)),
        "imax" => Value::Int(int(0)?.max(int(1)?)),
        "be_bytes" => Value::bytes(&fixed::be_bytes(ty()?, int(0)?)),
        "le_bytes" => Value::bytes(&fixed::le_bytes(ty()?, int(0)?)),
        "from_be" | "from_le" => {
            let data = ops
                .first()
                .ok_or_else(|| GenError::internal("a built-in lost an operand"))?
                .as_bytes()?;
            let read = if tag == "from_be" {
                fixed::from_be(ty()?, data)
            } else {
                fixed::from_le(ty()?, data)
            };
            // A runtime width other than the target's is a corpus error, never a truncation or a
            // zero extension (Appendix A.1).
            Value::Int(read.ok_or_else(|| {
                GenError::new(
                    code::IR,
                    "",
                    "from_be/from_le need exactly the target's width in bytes",
                )
            })?)
        }
        "at" => {
            let data = ops
                .first()
                .ok_or_else(|| GenError::internal("a built-in lost an operand"))?
                .as_bytes()?;
            let i = int(1)?;
            some(usize::try_from(i).ok().and_then(|i| data.get(i)).map(|b| i128::from(*b)))
        }
        "unwrap_or" => match ops {
            [Value::Opt(Some(v)), _] => (**v).clone(),
            [Value::Opt(None), default] => default.clone(),
            _ => return Err(ill_typed("an Option and a default")),
        },
        _ => return Err(GenError::internal("an unknown ir.fixed tag")),
    })
}

/// The largest exact intermediate of VER04r 3.2, re-exported for the tests.
pub const EXACT_LIMIT: i128 = EXACT_BOUND;
/// `MAX_IR_VALUE` re-exported next to the evaluator that enforces it.
pub const VALUE_LIMIT: u64 = MAX_IR_VALUE;

fn lookup<'v>(vars: &'v Vars<'_>, name: &str) -> Result<&'v Value, GenError> {
    vars.iter()
        .rev()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v)
        .ok_or_else(|| GenError::new(code::W0701, "", "an unbound variable in an IR tree"))
}

/// `take`/`drop` clamp: a negative count is 0, a count past the length is the length.
fn clamp_count(n: i128, len: usize) -> usize {
    if n <= 0 {
        0
    } else {
        usize::try_from(n).map_or(len, |n| n.min(len))
    }
}

/// Decode a validated lowercase even-length hex literal.
fn decode_hex(hex: &str) -> Result<Vec<u8>, GenError> {
    let bad = || GenError::new(code::W0701, "", "a Bytes literal is even lowercase hex");
    let b = hex.as_bytes();
    // The shape is checked first, then the size, as the oracle does: only a well-formed
    // literal over MAX_IR_VALUE is a *size* error (VER04r 8.4, [D34]).
    if !b.len().is_multiple_of(2) || !b.iter().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(bad());
    }
    if (b.len() / 2) as u64 > MAX_IR_VALUE {
        return Err(GenError::size_error(
            "a Bytes literal exceeds MAX_IR_VALUE (VER04r 4.2 rule 6)",
        ));
    }
    let nibble = |c: u8| match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(bad()),
    };
    b.chunks(2)
        .map(|p| Ok(nibble(p[0])? << 4 | nibble(p[1])?))
        .collect()
}

/// Does `ir` contain a fixed-width `add sub mul neg`, or a call of a model for which
/// `model_wraps` says yes?
fn may_fail_with(ir: &Ir, model_fails: &dyn Fn(&str) -> bool) -> bool {
    let mut found = false;
    ir.walk(&mut |n| match n {
        Ir::Arith(..) | Ir::Neg(..) | Ir::Concat(..) => found = true,
        Ir::Model { name, .. } if model_fails(name) => found = true,
        _ => {}
    });
    found
}

fn may_wrap_with(ir: &Ir, model_wraps: &dyn Fn(&str) -> bool) -> bool {
    let mut found = false;
    ir.walk(&mut |n| match n {
        Ir::Arith(_, ty, ..) | Ir::Neg(ty, _) if ty.is_fixed() => found = true,
        Ir::Model { name, .. } if model_wraps(name) => found = true,
        _ => {}
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Ty;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    fn var(n: &str) -> Box<Ir> {
        Box::new(Ir::Var(n.into()))
    }
    fn fx(t: FixedTy, v: i128) -> Box<Ir> {
        Box::new(Ir::Int {
            ty: Ty::Fixed(t),
            value: v,
        })
    }
    fn nat(v: i128) -> Box<Ir> {
        Box::new(Ir::Int {
            ty: Ty::Nat,
            value: v,
        })
    }
    fn int(v: i128) -> Box<Ir> {
        Box::new(Ir::Int {
            ty: Ty::Int,
            value: v,
        })
    }
    fn bytes(h: &str) -> Box<Ir> {
        Box::new(Ir::Bytes(h.into()))
    }
    fn model(name: &str, params: &[(&str, Ty)], ret: Ty, body: Ir) -> LoweredModel {
        LoweredModel {
            name: name.into(),
            params: params
                .iter()
                .map(|(n, t)| (n.to_string(), t.clone()))
                .collect(),
            ret,
            body,
            requires: vec!["ir.core"],
        }
    }
    fn run(ir: &Ir) -> Result<Value, GenError> {
        Program::new(&[]).eval(ir, &[], &mut Stats::default())
    }
    fn wraps_of(ir: &Ir) -> u64 {
        let mut st = Stats::default();
        Program::new(&[]).eval(ir, &[], &mut st).unwrap();
        st.wraps
    }
    fn subject(op: &str, args: Vec<Ir>) -> Ir {
        Ir::Subject {
            op: op.into(),
            args,
            widths: vec![],
        }
    }

    // Each expectation is computed by hand from VER04r 3.2 and Appendix A.

    #[test]
    fn literals_and_hex_decoding() {
        assert_eq!(run(&Ir::Bool(true)).unwrap(), Value::Bool(true));
        assert_eq!(run(&nat(5)).unwrap(), Value::Int(5));
        assert_eq!(run(&bytes("00ff")).unwrap(), Value::bytes(&[0, 255]));
        assert_eq!(run(&bytes("")).unwrap(), Value::bytes(&[]));
        for bad in ["0", "zz", "00FF"] {
            assert_eq!(
                run(&Ir::Bytes(bad.into())).unwrap_err().code,
                code::W0701,
                "{bad}"
            );
        }
        // a literal over MAX_IR_VALUE is refused before it is decoded, and it is a *size* error
        let huge = "00".repeat(65_537);
        assert!(run(&Ir::Bytes(huge)).unwrap_err().size);
        assert!(run(&Ir::Bytes("00".repeat(65_536))).is_ok());
        // a malformed literal is never a size error, however long (the shape is checked first)
        for bad in ["zz".repeat(65_537), format!("{}0", "00".repeat(65_537))] {
            let e = run(&Ir::Bytes(bad)).unwrap_err();
            assert!(!e.size, "{e}");
        }
    }

    /// An `ir.fixed` node as the lowering builds it.
    fn fixed(tag: &'static str, ty: Option<FixedTy>, at: Option<FixedTy>, args: Vec<Ir>) -> Ir {
        Ir::Fixed { tag, ty, at, args }
    }

    fn lit(t: FixedTy, v: i128) -> Ir {
        *fx(t, v)
    }

    #[test]
    fn the_ir_fixed_built_ins_evaluate_the_cases_a_host_gets_wrong() {
        let (u8t, i8t) = (FixedTy::U8, FixedTy::I8);
        let nat = |n: i128| Ir::Int {
            ty: Ty::Nat,
            value: n,
        };
        // a shift count of the width or more is decided before any host shift
        let shl = fixed("shl", None, Some(u8t), vec![lit(u8t, 1), nat(8)]);
        assert_eq!(run(&shl).unwrap(), Value::Int(0));
        let shl7 = fixed("shl", None, Some(u8t), vec![lit(u8t, 1), nat(7)]);
        assert_eq!(run(&shl7).unwrap(), Value::Int(128));
        let sar = fixed("sar", None, Some(i8t), vec![lit(i8t, -128), nat(300)]);
        assert_eq!(run(&sar).unwrap(), Value::Int(-1));
        let bit = fixed("bit", None, Some(u8t), vec![lit(u8t, 255), nat(8)]);
        assert_eq!(run(&bit).unwrap(), Value::Bool(false));
        // rotations reduce the count modulo the width
        let rotl = fixed("rotl", None, Some(u8t), vec![lit(u8t, 0x81), nat(9)]);
        assert_eq!(run(&rotl).unwrap(), Value::Int(3));
        // a division that cannot happen is `none`; `unwrap_or` evaluates both operands
        let q = fixed("div_trunc", None, Some(i8t), vec![lit(i8t, -128), lit(i8t, -1)]);
        assert_eq!(run(&q).unwrap(), Value::Opt(None));
        let q = fixed("div_trunc", None, Some(i8t), vec![lit(i8t, -7), lit(i8t, 2)]);
        assert_eq!(run(&q).unwrap(), Value::Opt(Some(Box::new(Value::Int(-3)))));
        let un = fixed("unwrap_or", None, None, vec![q, lit(i8t, 99)]);
        assert_eq!(run(&un).unwrap(), Value::Int(-3));
        // the explicit truncations never count a wrap [D31]; the arithmetic nodes do
        let wrap = fixed("wrap", Some(u8t), None, vec![lit(i8t, -1)]);
        assert_eq!(run(&wrap).unwrap(), Value::Int(255));
        assert_eq!(wraps_of(&wrap), 0);
        let not = fixed("bit_not", None, Some(u8t), vec![lit(u8t, 0)]);
        assert_eq!(wraps_of(&not), 0);
        let add_inside = fixed(
            "bit_not",
            None,
            Some(u8t),
            vec![Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1))],
        );
        assert_eq!(run(&add_inside).unwrap(), Value::Int(255));
        assert_eq!(wraps_of(&add_inside), 1);
        // the endian bytes and their inverse; a width that is not the target's is a corpus error
        let be = fixed("be_bytes", None, Some(FixedTy::U32), vec![lit(FixedTy::U32, 0x0102_0304)]);
        assert_eq!(run(&be).unwrap(), Value::bytes(&[1, 2, 3, 4]));
        let back = fixed("from_le", Some(FixedTy::U32), None, vec![Ir::Bytes("01020304".into())]);
        assert_eq!(run(&back).unwrap(), Value::Int(0x0403_0201));
        let short = fixed("from_be", Some(FixedTy::U32), None, vec![Ir::Bytes("0102".into())]);
        assert_eq!(run(&short).unwrap_err().code, code::IR);
        // `at` is `none` out of range, for a negative index too
        let at = |i: i128| {
            fixed(
                "at",
                None,
                None,
                vec![
                    Ir::Bytes("0a14".into()),
                    Ir::Int {
                        ty: Ty::Int,
                        value: i,
                    },
                ],
            )
        };
        assert_eq!(run(&at(1)).unwrap(), Value::Opt(Some(Box::new(Value::Int(20)))));
        assert_eq!(run(&at(2)).unwrap(), Value::Opt(None));
        assert_eq!(run(&at(-1)).unwrap(), Value::Opt(None));
    }

    #[test]
    fn a_hand_built_ir_fixed_node_that_is_not_well_typed_is_refused_never_guessed() {
        // no width for a built-in that needs one
        let no_width = fixed("bit_not", None, None, vec![lit(FixedTy::U8, 1)]);
        assert_eq!(run(&no_width).unwrap_err().code, code::W0701);
        // a negative shift count is not a Nat
        let negative = fixed(
            "shl",
            None,
            Some(FixedTy::U8),
            vec![
                lit(FixedTy::U8, 1),
                Ir::Int {
                    ty: Ty::Int,
                    value: -1,
                },
            ],
        );
        assert_eq!(run(&negative).unwrap_err().code, code::W0701);
        // `unwrap_or` of something that is not an Option; an unknown tag
        let bad = fixed("unwrap_or", None, None, vec![lit(FixedTy::U8, 1), lit(FixedTy::U8, 2)]);
        assert_eq!(run(&bad).unwrap_err().code, code::W0701);
        let unknown = fixed("nonsense", None, None, vec![]);
        assert!(run(&unknown).is_err());
        // a byte operand where an integer is needed
        let wrong = fixed("popcount", None, Some(FixedTy::U8), vec![Ir::Bytes("00".into())]);
        assert!(run(&wrong).is_err());
    }

    #[test]
    fn fixed_width_arithmetic_wraps_and_counts_one_wrap_per_differing_node() {
        let u8t = FixedTy::U8;
        let add = Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1));
        let mut st = Stats::default();
        let p = Program::new(&[]);
        assert_eq!(p.eval(&add, &[], &mut st).unwrap(), Value::Int(0));
        assert_eq!(st.wraps, 1);
        // in range: no wrap counted
        let ok = Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 100), fx(u8t, 55));
        assert_eq!(wraps_of(&ok), 0);
        let sub = Ir::Arith(Arith::Sub, Ty::Fixed(u8t), fx(u8t, 0), fx(u8t, 1));
        assert_eq!(run(&sub).unwrap(), Value::Int(255));
        assert_eq!(wraps_of(&sub), 1);
        let mul = Ir::Arith(Arith::Mul, Ty::Fixed(u8t), fx(u8t, 16), fx(u8t, 16));
        assert_eq!(run(&mul).unwrap(), Value::Int(0));
        // I8: 127 + 1 = -128, and -(-128) = -128
        let i8t = FixedTy::I8;
        let add = Ir::Arith(Arith::Add, Ty::Fixed(i8t), fx(i8t, 127), fx(i8t, 1));
        assert_eq!(run(&add).unwrap(), Value::Int(-128));
        let neg = Ir::Neg(Ty::Fixed(i8t), fx(i8t, -128));
        assert_eq!(run(&neg).unwrap(), Value::Int(-128));
        assert_eq!(wraps_of(&neg), 1);
        let neg5 = Ir::Neg(Ty::Fixed(i8t), fx(i8t, 5));
        assert_eq!(run(&neg5).unwrap(), Value::Int(-5));
        assert_eq!(wraps_of(&neg5), 0);
        // U64 MAX * MAX = 1 (needs more than 128 bits when exact)
        let u64t = FixedTy::U64;
        let max = i128::from(u64::MAX);
        let mm = Ir::Arith(Arith::Mul, Ty::Fixed(u64t), fx(u64t, max), fx(u64t, max));
        assert_eq!(run(&mm).unwrap(), Value::Int(1));
    }

    #[test]
    fn a_node_below_a_wrap_counts_every_wrapping_node() {
        // (255u8 + 1u8) + 255u8 : the inner wraps (256 -> 0), the outer does not (0+255).
        let u8t = FixedTy::U8;
        let inner = Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1));
        let outer = Ir::Arith(Arith::Add, Ty::Fixed(u8t), Box::new(inner), fx(u8t, 255));
        assert_eq!(run(&outer).unwrap(), Value::Int(255));
        assert_eq!(wraps_of(&outer), 1);
        // (255 + 1) + 1 = 1: the inner wraps, the outer (0 + 1) does not
        let inner = Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1));
        let o = Ir::Arith(Arith::Add, Ty::Fixed(u8t), Box::new(inner), fx(u8t, 1));
        assert_eq!(wraps_of(&o), 1);
    }

    #[test]
    fn nat_and_int_are_exact_and_nat_minus_nat_goes_negative() {
        // 3 - 5 = -2 as an Int; no wrap is counted
        let sub = Ir::Arith(Arith::Sub, Ty::Int, nat(3), nat(5));
        assert_eq!(run(&sub).unwrap(), Value::Int(-2));
        assert_eq!(wraps_of(&sub), 0);
        let neg = Ir::Neg(Ty::Int, nat(7));
        assert_eq!(run(&neg).unwrap(), Value::Int(-7));
        // 2^100 * 2^40 leaves the 2^127 window: W0701, never a wrapped value
        let big = Ir::Arith(Arith::Mul, Ty::Int, int(1 << 100), int(1 << 40));
        assert_eq!(run(&big).unwrap_err().code, code::W0701);
        // the largest legal value is accepted
        let max = Ir::Arith(Arith::Add, Ty::Int, int(i128::MAX - 1), int(1));
        assert_eq!(run(&max).unwrap(), Value::Int(i128::MAX));
        let over = Ir::Arith(Arith::Add, Ty::Int, int(i128::MAX), int(1));
        assert!(run(&over).is_err());
        assert_eq!(EXACT_LIMIT, i128::MAX);
    }

    #[test]
    fn comparisons_order_by_mathematical_value_and_equality_is_structural() {
        let lt = Ir::Cmp(Cmp::Lt, nat(2), int(3));
        assert_eq!(run(&lt).unwrap(), Value::Bool(true));
        for (c, a, b, want) in [
            (Cmp::Le, 3, 3, true),
            (Cmp::Gt, 3, 3, false),
            (Cmp::Ge, 3, 3, true),
            (Cmp::Ne, 3, 4, true),
            (Cmp::Eq, 3, 4, false),
        ] {
            let ir = Ir::Cmp(c, nat(a), nat(b));
            assert_eq!(run(&ir).unwrap(), Value::Bool(want), "{c:?} {a} {b}");
        }
        // a negative Int against a Nat
        assert_eq!(
            run(&Ir::Cmp(Cmp::Lt, int(-1), nat(0))).unwrap(),
            Value::Bool(true)
        );
        // Bytes equality compares contents; Ok/Err are nominal
        let b = Ir::Cmp(Cmp::Eq, bytes("0102"), bytes("0102"));
        assert_eq!(run(&b).unwrap(), Value::Bool(true));
        let r = Ir::Cmp(
            Cmp::Eq,
            Box::new(Ir::Err("A".into())),
            Box::new(Ir::Err("B".into())),
        );
        assert_eq!(run(&r).unwrap(), Value::Bool(false));
        let r = Ir::Cmp(
            Cmp::Eq,
            Box::new(Ir::Ok(bytes("00"))),
            Box::new(Ir::Ok(bytes("00"))),
        );
        assert_eq!(run(&r).unwrap(), Value::Bool(true));
        // ordering needs numbers
        assert!(run(&Ir::Cmp(Cmp::Lt, bytes("00"), bytes("01"))).is_err());
    }

    #[test]
    fn logic_short_circuits_and_skips_the_untaken_operand() {
        // the right operand would divide... here: be unbound; it must not be evaluated
        let boom = || var("unbound");
        let t = || Box::new(Ir::Bool(true));
        let f = || Box::new(Ir::Bool(false));
        assert_eq!(
            run(&Ir::Logic(Logic::And, f(), boom())).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            run(&Ir::Logic(Logic::Or, t(), boom())).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            run(&Ir::Logic(Logic::Implies, f(), boom())).unwrap(),
            Value::Bool(true)
        );
        // and when the left does not decide, the right is evaluated (and fails here)
        assert!(run(&Ir::Logic(Logic::And, t(), boom())).is_err());
        assert!(run(&Ir::Logic(Logic::Or, f(), boom())).is_err());
        assert!(run(&Ir::Logic(Logic::Implies, t(), boom())).is_err());
        // truth table of the three connectives
        for (a, b) in [(false, false), (false, true), (true, false), (true, true)] {
            let (x, y) = (Box::new(Ir::Bool(a)), Box::new(Ir::Bool(b)));
            assert_eq!(
                run(&Ir::Logic(Logic::And, x.clone(), y.clone())).unwrap(),
                Value::Bool(a && b)
            );
            assert_eq!(
                run(&Ir::Logic(Logic::Or, x.clone(), y.clone())).unwrap(),
                Value::Bool(a || b)
            );
            assert_eq!(
                run(&Ir::Logic(Logic::Implies, x, y)).unwrap(),
                Value::Bool(!a || b)
            );
        }
        assert_eq!(run(&Ir::Not(t())).unwrap(), Value::Bool(false));
        assert!(run(&Ir::Not(nat(1))).is_err());
    }

    #[test]
    fn if_evaluates_exactly_one_branch_and_counts_only_its_wraps() {
        let u8t = FixedTy::U8;
        let wrapping = || Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1));
        let calm = || Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 1), fx(u8t, 1));
        let take_wrap = Ir::If(
            Box::new(Ir::Bool(true)),
            Box::new(wrapping()),
            Box::new(calm()),
        );
        assert_eq!(wraps_of(&take_wrap), 1);
        let skip_wrap = Ir::If(
            Box::new(Ir::Bool(false)),
            Box::new(wrapping()),
            Box::new(calm()),
        );
        assert_eq!(wraps_of(&skip_wrap), 0);
        assert_eq!(run(&skip_wrap).unwrap(), Value::Int(2));
        // an untaken branch with an unbound variable is never touched
        let guarded = Ir::If(Box::new(Ir::Bool(true)), nat(1), var("nope"));
        assert_eq!(run(&guarded).unwrap(), Value::Int(1));
    }

    #[test]
    fn bytes_surgery_is_total_and_clamps() {
        let v = |ir: Ir| run(&ir).unwrap();
        assert_eq!(
            v(Ir::Concat(bytes("01"), bytes("0203"))),
            Value::bytes(&[1, 2, 3])
        );
        assert_eq!(v(Ir::Take(bytes("010203"), nat(2))), Value::bytes(&[1, 2]));
        assert_eq!(
            v(Ir::Take(bytes("010203"), nat(9))),
            Value::bytes(&[1, 2, 3])
        );
        assert_eq!(v(Ir::Take(bytes("010203"), int(-4))), Value::bytes(&[]));
        assert_eq!(v(Ir::Drop(bytes("010203"), nat(1))), Value::bytes(&[2, 3]));
        assert_eq!(v(Ir::Drop(bytes("010203"), nat(9))), Value::bytes(&[]));
        assert_eq!(
            v(Ir::Drop(bytes("010203"), int(-1))),
            Value::bytes(&[1, 2, 3])
        );
        // a count past usize (a huge exact Int) clamps rather than panicking
        assert_eq!(
            v(Ir::Take(bytes("0102"), int(i128::MAX))),
            Value::bytes(&[1, 2])
        );
        assert_eq!(v(Ir::Len(bytes("010203"))), Value::Int(3));
        assert_eq!(v(Ir::Len(bytes(""))), Value::Int(0));
        // take/drop/len need bytes
        assert!(run(&Ir::Len(nat(1))).is_err());
        assert!(run(&Ir::Take(nat(1), nat(1))).is_err());
        assert!(run(&Ir::Concat(nat(1), bytes("00"))).is_err());
    }

    #[test]
    fn concat_over_max_ir_value_is_refused_before_it_is_built() {
        // 40,000 + 40,000 > 65,536.  The operands are model parameters so no literal is
        // large; the product is never allocated.
        let big = Value::Bytes(Rc::from(vec![0u8; 40_000]));
        let ms = [model(
            "cat",
            &[("x", Ty::BYTES), ("y", Ty::BYTES)],
            Ty::BYTES,
            Ir::Concat(var("x"), var("y")),
        )];
        let p = Program::new(&ms);
        let call = Ir::Model {
            name: "cat".into(),
            args: vec![Ir::Var("a".into()), Ir::Var("b".into())],
        };
        let vars = [("a", big.clone()), ("b", big)];
        let e = p.eval(&call, &vars, &mut Stats::default()).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("MAX_IR_VALUE"));
        assert!(e.size, "a concat past MAX_IR_VALUE is a size error");
        // exactly 65,536 is fine
        let half = Value::Bytes(Rc::from(vec![0u8; 32_768]));
        let vars = [("a", half.clone()), ("b", half)];
        let v = p.eval(&call, &vars, &mut Stats::default()).unwrap();
        assert_eq!(v.as_bytes().unwrap().len(), 65_536);
        assert_eq!(VALUE_LIMIT, 65_536);
    }

    #[test]
    fn results_ok_err_and_is_err() {
        let ok = Ir::Ok(bytes("aa"));
        assert_eq!(
            run(&ok).unwrap(),
            Value::Ok(Box::new(Value::bytes(&[0xaa])))
        );
        assert_eq!(run(&Ir::IsErr(Box::new(ok))).unwrap(), Value::Bool(false));
        let err = Ir::Err("BAD".into());
        assert_eq!(run(&err).unwrap(), Value::Err(Rc::from("BAD")));
        assert_eq!(run(&Ir::IsErr(Box::new(err))).unwrap(), Value::Bool(true));
        assert!(run(&Ir::IsErr(nat(1))).is_err());
    }

    #[test]
    fn models_get_a_fresh_frame_and_call_earlier_models() {
        // inc(x: U8) = x + 1u8 ; twice(x: U8) = inc(inc(x))
        let u8t = FixedTy::U8;
        let ms = [
            model(
                "inc",
                &[("x", Ty::Fixed(u8t))],
                Ty::Fixed(u8t),
                Ir::Arith(Arith::Add, Ty::Fixed(u8t), var("x"), fx(u8t, 1)),
            ),
            model(
                "twice",
                &[("x", Ty::Fixed(u8t))],
                Ty::Fixed(u8t),
                Ir::Model {
                    name: "inc".into(),
                    args: vec![Ir::Model {
                        name: "inc".into(),
                        args: vec![Ir::Var("x".into())],
                    }],
                },
            ),
        ];
        let p = Program::new(&ms);
        assert!(p.model_wraps("inc") && p.model_wraps("twice"));
        assert!(!p.model_wraps("nope"));
        let call = Ir::Model {
            name: "twice".into(),
            args: vec![Ir::Var("a".into())],
        };
        let mut st = Stats::default();
        // 254 + 1 = 255 (no wrap), + 1 = 0 (wrap): one wrap
        let v = p.eval(&call, &[("a", Value::Int(254))], &mut st).unwrap();
        assert_eq!(v, Value::Int(0));
        assert_eq!(st.wraps, 1);
        // 255: both increments wrap? 255+1=0 (wrap), 0+1=1 (no): one wrap
        let mut st = Stats::default();
        p.eval(&call, &[("a", Value::Int(255))], &mut st).unwrap();
        assert_eq!(st.wraps, 1);
        // the caller's `x`-named variable does not leak into the model's frame: the
        // model's own parameter shadows it
        let mut st = Stats::default();
        let v = p
            .eval(
                &Ir::Model {
                    name: "inc".into(),
                    args: vec![Ir::Int {
                        ty: Ty::Fixed(u8t),
                        value: 7,
                    }],
                },
                &[("x", Value::Int(100))],
                &mut st,
            )
            .unwrap();
        assert_eq!(v, Value::Int(8));
        // wrong arity and unknown models are refused
        let bad = Ir::Model {
            name: "inc".into(),
            args: vec![],
        };
        assert!(p.eval(&bad, &[], &mut Stats::default()).is_err());
        let unknown = Ir::Model {
            name: "ghost".into(),
            args: vec![],
        };
        assert!(p.eval(&unknown, &[], &mut Stats::default()).is_err());
    }

    #[test]
    fn a_subject_call_and_an_unbound_variable_cannot_be_evaluated() {
        let e = run(&subject("f", vec![])).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("subject"));
        assert!(run(&Ir::Var("x".into())).is_err());
    }

    #[test]
    fn the_work_stack_and_the_model_frames_are_bounded_without_recursion() {
        // a tree deeper than lowering allows (it caps at 64) is refused, not recursed into;
        // the bound is lowered here to keep the test small
        let mut ir = Ir::Bool(true);
        for _ in 0..40 {
            ir = Ir::Not(Box::new(ir));
        }
        let p = Program::new(&[]).with_max_tasks(32);
        assert_eq!(
            p.eval(&ir, &[], &mut Stats::default()).unwrap_err().code,
            code::LIMIT
        );
        assert!(Program::new(&[])
            .eval(&ir, &[], &mut Stats::default())
            .is_ok());
        let q = Program::new(&[]).with_max_tasks(usize::MAX);
        assert!(q.eval(&ir, &[], &mut Stats::default()).is_ok());
        assert_eq!(MAX_EVAL_TASKS, 1 << 16);
        // 64 active model frames are allowed, 65 are a limit error (VER04 section 2)
        let chain = |n: usize| -> Vec<LoweredModel> {
            let mut ms = vec![model("m0", &[("x", Ty::Bool)], Ty::Bool, *var("x").clone())];
            for i in 1..n {
                ms.push(model(
                    &format!("m{i}"),
                    &[("x", Ty::Bool)],
                    Ty::Bool,
                    Ir::Model {
                        name: format!("m{}", i - 1),
                        args: vec![Ir::Var("x".into())],
                    },
                ));
            }
            ms
        };
        let call = |n: usize| Ir::Model {
            name: format!("m{}", n - 1),
            args: vec![Ir::Bool(true)],
        };
        let ok = chain(64);
        assert_eq!(
            Program::new(&ok)
                .eval(&call(64), &[], &mut Stats::default())
                .unwrap(),
            Value::Bool(true)
        );
        let deep = chain(65);
        let e = Program::new(&deep)
            .eval(&call(65), &[], &mut Stats::default())
            .unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert!(e.message.contains("64 frames"));
    }

    // -- trace, static_calls ------------------------------------------------------------

    fn trace_of(ir: &Ir, vars: &Vars<'_>) -> (u64, u64) {
        let mut st = Stats::default();
        let n = Program::new(&[]).trace(ir, vars, &mut st).unwrap();
        (n, st.wraps)
    }

    #[test]
    fn nested_subject_calls_each_count_once() {
        // add(add(a, b), c) == add(a, add(b, c)) has four calls per case (VER04r 4.4)
        let add = |x: Ir, y: Ir| subject("add", vec![x, y]);
        let v = |n: &str| Ir::Var(n.into());
        let body = Ir::Cmp(
            Cmp::Eq,
            Box::new(add(add(v("a"), v("b")), v("c"))),
            Box::new(add(v("a"), add(v("b"), v("c")))),
        );
        let p = Program::new(&[]);
        assert_eq!(p.static_calls(&body), Some(4));
        assert_eq!(trace_of(&body, &[]), (4, 0));
    }

    #[test]
    fn a_pure_guard_selects_the_branch_whose_calls_count() {
        // if b != 0 then f(a) == pure else true : one call when b != 0, none otherwise
        let body = Ir::If(
            Box::new(Ir::Cmp(Cmp::Ne, var("b"), nat(0))),
            Box::new(Ir::Cmp(
                Cmp::Eq,
                Box::new(subject("f", vec![Ir::Var("a".into())])),
                nat(1),
            )),
            Box::new(Ir::Bool(true)),
        );
        let p = Program::new(&[]);
        assert_eq!(p.static_calls(&body), None);
        let vars = |b: i128| [("a", Value::Int(1)), ("b", Value::Int(b))];
        assert_eq!(trace_of(&body, &vars(0)).0, 0);
        assert_eq!(trace_of(&body, &vars(7)).0, 1);
        // and/or/implies: the right operand's calls count only when it runs
        let g = Ir::Cmp(Cmp::Ne, var("b"), nat(0));
        let call = || subject("f", vec![]);
        for (op, runs_when_b_nonzero) in [
            (Logic::And, true),
            (Logic::Implies, true),
            (Logic::Or, false),
        ] {
            let ir = Ir::Logic(op, Box::new(g.clone()), Box::new(call()));
            assert_eq!(
                trace_of(&ir, &vars(7)).0,
                u64::from(runs_when_b_nonzero),
                "{op:?}"
            );
            assert_eq!(
                trace_of(&ir, &vars(0)).0,
                u64::from(!runs_when_b_nonzero),
                "{op:?}"
            );
        }
        // a subject on the left only: it counts, the pure right operand is not a call
        let left = Ir::Logic(Logic::And, Box::new(call()), Box::new(Ir::Bool(true)));
        assert_eq!(trace_of(&left, &[]).0, 1);
        assert_eq!(p.static_calls(&left), Some(1));
        // a subject only in the condition of an `if` counts once
        let cond = Ir::If(
            Box::new(call()),
            Box::new(Ir::Bool(true)),
            Box::new(Ir::Bool(false)),
        );
        assert_eq!(trace_of(&cond, &[]).0, 1);
    }

    #[test]
    fn an_impure_guard_is_w0701_in_the_trace_too() {
        let call = || subject("f", vec![]);
        let bad_if = Ir::If(Box::new(call()), Box::new(call()), Box::new(Ir::Bool(true)));
        let e = Program::new(&[])
            .trace(&bad_if, &[], &mut Stats::default())
            .unwrap_err();
        assert_eq!(e.code, code::W0701);
        let bad_and = Ir::Logic(Logic::And, Box::new(call()), Box::new(call()));
        assert!(Program::new(&[])
            .trace(&bad_and, &[], &mut Stats::default())
            .is_err());
    }

    #[test]
    fn pure_nodes_on_the_trace_are_evaluated_for_wraps_and_subject_dependent_ones_are_not() {
        let u8t = FixedTy::U8;
        let wrapping = |a: i128| Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, a), fx(u8t, 1));
        // f(255u8 + 1u8) == (255u8 + 1u8): the subject argument and the expected side each
        // wrap once; the comparison and the call are not evaluated
        let body = Ir::Cmp(
            Cmp::Eq,
            Box::new(subject("f", vec![wrapping(255)])),
            Box::new(wrapping(255)),
        );
        assert_eq!(trace_of(&body, &[]), (1, 2));
        // a wrapping node WITH a subject beneath it is subject-dependent: never evaluated
        let dep = Ir::Cmp(
            Cmp::Eq,
            Box::new(Ir::Arith(
                Arith::Add,
                Ty::Fixed(u8t),
                Box::new(subject("f", vec![])),
                fx(u8t, 1),
            )),
            fx(u8t, 0),
        );
        assert_eq!(trace_of(&dep, &[]), (1, 0));
        // a guard's wraps count, and only the taken branch's
        let guarded = Ir::If(
            Box::new(Ir::Cmp(Cmp::Eq, Box::new(wrapping(255)), fx(u8t, 0))),
            Box::new(subject("f", vec![wrapping(255)])),
            Box::new(Ir::Bool(true)),
        );
        // the guard wraps (1); it is true, so the then branch runs: its argument wraps (2)
        assert_eq!(trace_of(&guarded, &[]), (1, 2));
        // under a pure left operand that decides, the right operand's wraps do not count
        let sc = Ir::Logic(
            Logic::And,
            Box::new(Ir::Bool(false)),
            Box::new(subject("f", vec![wrapping(255)])),
        );
        assert_eq!(trace_of(&sc, &[]), (0, 0));
    }

    #[test]
    fn a_model_calls_wraps_count_through_the_trace() {
        let u8t = FixedTy::U8;
        let ms = [model(
            "inc",
            &[("x", Ty::Fixed(u8t))],
            Ty::Fixed(u8t),
            Ir::Arith(Arith::Add, Ty::Fixed(u8t), var("x"), fx(u8t, 1)),
        )];
        let p = Program::new(&ms);
        let body = Ir::Cmp(
            Cmp::Eq,
            Box::new(subject("f", vec![Ir::Var("a".into())])),
            Box::new(Ir::Model {
                name: "inc".into(),
                args: vec![Ir::Var("a".into())],
            }),
        );
        assert!(p.may_wrap(&body));
        let mut st = Stats::default();
        let n = p.trace(&body, &[("a", Value::Int(255))], &mut st).unwrap();
        assert_eq!((n, st.wraps), (1, 1));
        let mut st = Stats::default();
        p.trace(&body, &[("a", Value::Int(3))], &mut st).unwrap();
        assert_eq!(st.wraps, 0);
        // a body with nothing that can wrap needs no trace
        let calm = Ir::Cmp(Cmp::Eq, Box::new(subject("f", vec![])), nat(1));
        assert!(!p.may_wrap(&calm));
    }

    // -- ir.tuple: tuple, project, match (S10t-2) -------------------------------------------------

    fn tup(parts: Vec<Ir>) -> Ir {
        Ir::Tuple(parts)
    }
    fn proj(base: Ir, i: usize) -> Ir {
        Ir::Project(Box::new(base), i)
    }
    fn mt(scrutinee: Ir, arms: Vec<(Pat, Ir)>) -> Ir {
        Ir::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            on_result: true,
        }
    }
    fn okp(p: Pat) -> Pat {
        Pat::Ok(Box::new(p))
    }
    fn bind(n: &str) -> Pat {
        Pat::Bind(n.into())
    }

    #[test]
    fn a_tuple_evaluates_its_components_in_order_and_project_picks_one() {
        let t = tup(vec![Ir::Bool(true), *nat(3), tup(vec![*nat(1), *bytes("ab")])]);
        assert_eq!(
            run(&t).unwrap(),
            Value::Tuple(vec![
                Value::Bool(true),
                Value::Int(3),
                Value::Tuple(vec![Value::Int(1), Value::bytes(&[0xab])])
            ])
        );
        assert_eq!(run(&proj(t.clone(), 1)).unwrap(), Value::Int(3));
        assert_eq!(run(&proj(proj(t.clone(), 2), 1)).unwrap(), Value::bytes(&[0xab]));
        // equality is structural, component by component
        let eq = |a: Ir, b: Ir| run(&Ir::Cmp(Cmp::Eq, Box::new(a), Box::new(b))).unwrap();
        assert_eq!(eq(t.clone(), t.clone()), Value::Bool(true));
        assert_eq!(
            eq(tup(vec![*nat(1), *nat(2)]), tup(vec![*nat(2), *nat(1)])),
            Value::Bool(false)
        );
        // a tuple operand that wraps counts its wrap
        let u8t = FixedTy::U8;
        let wrapping = tup(vec![
            Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1)),
            *nat(0),
        ]);
        assert_eq!(wraps_of(&wrapping), 1);
    }

    #[test]
    fn a_project_outside_the_tuple_or_of_a_non_tuple_is_an_ill_typed_tree() {
        assert_eq!(run(&proj(tup(vec![*nat(1), *nat(2)]), 2)).unwrap_err().code, code::W0701);
        assert_eq!(run(&proj(*nat(1), 0)).unwrap_err().code, code::W0701);
    }

    #[test]
    fn a_match_evaluates_exactly_one_arm_the_first_that_matches() {
        let r = |v: Value| vec![("r", v)];
        let m = mt(
            *var("r"),
            vec![
                (okp(Pat::Tuple(vec![bind("a"), Pat::Wild])), *var("a")),
                (Pat::Err("X".into()), *nat(10)),
                (Pat::ErrAny, *nat(20)),
            ],
        );
        let p = Program::new(&[]);
        let go = |v: Value| p.eval(&m, &r(v), &mut Stats::default()).unwrap();
        let pair = Value::Tuple(vec![Value::Int(7), Value::Int(8)]);
        assert_eq!(go(Value::Ok(Box::new(pair))), Value::Int(7));
        assert_eq!(go(Value::Err("X".into())), Value::Int(10));
        assert_eq!(go(Value::Err("Y".into())), Value::Int(20));
        // the arm that is not taken is not evaluated: a wrap in it is not counted
        let u8t = FixedTy::U8;
        let wrap = Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1));
        let m = mt(
            *var("r"),
            vec![(okp(Pat::Wild), Ir::Bool(true)), (Pat::ErrAny, Ir::Cmp(Cmp::Eq, Box::new(wrap), fx(u8t, 0)))],
        );
        let mut st = Stats::default();
        p.eval(&m, &r(Value::Ok(Box::new(Value::Int(0)))), &mut st).unwrap();
        assert_eq!(st.wraps, 0);
        p.eval(&m, &r(Value::Err("E".into())), &mut st).unwrap();
        assert_eq!(st.wraps, 1);
    }

    #[test]
    fn ir_data_is_evaluated_some_none_ctor_field_and_the_data_patterns() {
        // `ctor` builds a constructor value, `field` reads it back, `some`/`none` build an Option
        let point = Ir::Ctor {
            name: "P".into(),
            args: vec![*nat(1), *nat(2)],
        };
        assert_eq!(
            run(&point).unwrap(),
            Value::Con(Rc::from("P"), vec![Value::Int(1), Value::Int(2)])
        );
        assert_eq!(run(&Ir::Field(Box::new(point), 1)).unwrap(), Value::Int(2));
        assert_eq!(run(&Ir::None(Ty::Nat)).unwrap(), Value::Opt(None));
        assert_eq!(
            run(&Ir::Some(nat(1))).unwrap(),
            Value::Opt(Some(Box::new(Value::Int(1))))
        );
        // match on an Option: Some(x) -> x | None -> 0, in the order written
        let arms = || {
            vec![
                (Pat::Some(Box::new(bind("x"))), *var("x")),
                (Pat::None, *nat(0)),
            ]
        };
        let on = |s: Ir| Ir::Match {
            scrutinee: Box::new(s),
            arms: arms(),
            on_result: false,
        };
        assert_eq!(run(&on(Ir::Some(nat(7)))).unwrap(), Value::Int(7));
        assert_eq!(run(&on(Ir::None(Ty::Nat))).unwrap(), Value::Int(0));
        // a Bool pattern, and a data pattern on a Result: `Ok(true)`
        let ok_true = Ir::Match {
            scrutinee: Box::new(Ir::Ok(Box::new(Ir::Bool(true)))),
            arms: vec![
                (okp(Pat::Bool(true)), Ir::Bool(true)),
                (okp(Pat::Bool(false)), Ir::Bool(false)),
                (Pat::ErrAny, Ir::Bool(false)),
            ],
            on_result: true,
        };
        assert_eq!(run(&ok_true).unwrap(), Value::Bool(true));
        // equality of constructor values is structural: the name and every field
        let con = |n: &str, v: i128| Ir::Ctor {
            name: n.into(),
            args: vec![*nat(v)],
        };
        let eq = |a: Ir, b: Ir| Ir::Cmp(Cmp::Eq, Box::new(a), Box::new(b));
        assert_eq!(run(&eq(con("A", 1), con("A", 1))).unwrap(), Value::Bool(true));
        assert_eq!(run(&eq(con("A", 1), con("B", 1))).unwrap(), Value::Bool(false));
        assert_eq!(run(&eq(con("A", 1), con("A", 2))).unwrap(), Value::Bool(false));
    }

    #[test]
    fn a_bind_is_visible_to_its_arm_and_to_a_nested_match_but_not_after() {
        // match r with Ok(x) -> (match s with Ok(y) -> (x, y) | Err(_) -> (x, x)) | Err(_) -> (0, 0)
        let inner = mt(
            *var("s"),
            vec![
                (okp(bind("y")), tup(vec![*var("x"), *var("y")])),
                (Pat::ErrAny, tup(vec![*var("x"), *var("x")])),
            ],
        );
        let m = mt(
            *var("r"),
            vec![
                (okp(bind("x")), inner),
                (Pat::ErrAny, tup(vec![*nat(0), *nat(0)])),
            ],
        );
        let ok = |n: i128| Value::Ok(Box::new(Value::Int(n)));
        let vars = vec![("r", ok(1)), ("s", ok(2))];
        let p = Program::new(&[]);
        assert_eq!(
            p.eval(&m, &vars, &mut Stats::default()).unwrap(),
            Value::Tuple(vec![Value::Int(1), Value::Int(2)])
        );
        let vars = vec![("r", ok(1)), ("s", Value::Err("E".into()))];
        assert_eq!(
            p.eval(&m, &vars, &mut Stats::default()).unwrap(),
            Value::Tuple(vec![Value::Int(1), Value::Int(1)])
        );
        // `x` is not bound outside the arm: a variable read after the match is unbound
        let after = Ir::Logic(
            Logic::And,
            Box::new(Ir::Cmp(Cmp::Eq, Box::new(m.clone()), Box::new(m))),
            Box::new(Ir::Cmp(Cmp::Eq, var("x"), nat(1))),
        );
        assert_eq!(
            p.eval(&after, &vars, &mut Stats::default()).unwrap_err().code,
            code::W0701
        );
    }

    #[test]
    fn a_callee_parameter_named_like_a_callers_bind_is_the_callees_own() {
        // model g(x: Nat) -> Nat = x ;  match r with Ok(x) -> g(5) | Err(_) -> 0
        // the callee's `x` is its parameter (5), not the caller's bind (1)
        let g = model("g", &[("x", Ty::Nat)], Ty::Nat, *var("x"));
        let call = Ir::Model {
            name: "g".into(),
            args: vec![*nat(5)],
        };
        let m = mt(
            *var("r"),
            vec![(okp(bind("x")), call), (Pat::ErrAny, *nat(0))],
        );
        let models = [g];
        let vars = vec![("r", Value::Ok(Box::new(Value::Int(1))))];
        assert_eq!(
            Program::new(&models).eval(&m, &vars, &mut Stats::default()).unwrap(),
            Value::Int(5)
        );
        // and a match inside the callee binds in the callee's frame, leaving the caller's alone
        let inner = mt(
            Ir::Ok(var("x")),
            vec![(okp(bind("y")), *var("y")), (Pat::ErrAny, *nat(0))],
        );
        let h = model("h", &[("x", Ty::Nat)], Ty::Nat, inner);
        let call = Ir::Model {
            name: "h".into(),
            args: vec![*nat(9)],
        };
        let outer = mt(
            *var("r"),
            vec![
                (okp(bind("y")), Ir::Arith(Arith::Add, Ty::Int, Box::new(call), var("y"))),
                (Pat::ErrAny, *nat(0)),
            ],
        );
        let models = [h];
        assert_eq!(
            Program::new(&models).eval(&outer, &vars, &mut Stats::default()).unwrap(),
            Value::Int(10),
            "h(9) = 9 with its own y; the caller's y is 1"
        );
    }

    #[test]
    fn a_match_that_falls_through_is_an_internal_refusal() {
        let m = mt(*var("r"), vec![(Pat::Err("X".into()), *nat(1))]);
        let e = Program::new(&[])
            .eval(&m, &[("r", Value::Ok(Box::new(Value::Int(0))))], &mut Stats::default())
            .unwrap_err();
        assert!(e.message.contains("fell through"), "{e}");
    }

    #[test]
    fn a_match_arm_is_a_guard_for_call_counting() {
        // match r with Ok(x) -> f(x) == x | Err(_) -> true : a call exactly when r is an Ok
        let call = |a: Ir| Ir::Cmp(Cmp::Eq, Box::new(subject("f", vec![a])), var("x"));
        let m = mt(
            *var("r"),
            vec![(okp(bind("x")), call(*var("x"))), (Pat::ErrAny, Ir::Bool(true))],
        );
        let p = Program::new(&[]);
        assert_eq!(p.static_calls(&m), None, "the count depends on the case");
        let ok = vec![("r", Value::Ok(Box::new(Value::Int(3))))];
        let err = vec![("r", Value::Err("E".into()))];
        assert_eq!(trace_of(&m, &ok), (1, 0));
        assert_eq!(trace_of(&m, &err), (0, 0));
        // a call in the scrutinee counts whatever the arms are, and the arms are not traced
        let scrut = mt(
            subject("f", vec![*var("a")]),
            vec![(okp(Pat::Wild), Ir::Bool(true)), (Pat::ErrAny, Ir::Bool(false))],
        );
        assert_eq!(p.static_calls(&scrut), Some(1));
        assert_eq!(trace_of(&scrut, &[("a", Value::Int(0))]), (1, 0));
        // a call in an arm under a scrutinee that calls: the impure guard (VER04r 4.4)
        let impure = mt(
            subject("f", vec![*var("a")]),
            vec![(okp(bind("x")), subject("g", vec![*var("x")])), (Pat::ErrAny, Ir::Bool(true))],
        );
        let e = p
            .trace(&impure, &[("a", Value::Int(0))], &mut Stats::default())
            .unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("impure guard"));
    }

    #[test]
    fn the_wraps_of_the_scrutinee_and_the_taken_arm_are_counted_and_the_other_arm_is_not() {
        // The scrutinee wraps once when it is evaluated to pick the arm; the taken arm calls
        // f(255u8 + 1u8) and the argument's wrap counts once; the untaken arm's wrap does not.
        let u8t = FixedTy::U8;
        let wrap = |a: i128| Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, a), fx(u8t, 1));
        let scrut = Ir::If(
            Box::new(Ir::Cmp(Cmp::Eq, Box::new(wrap(255)), fx(u8t, 0))),
            Box::new(Ir::Ok(nat(1))),
            Box::new(Ir::Err("E".into())),
        );
        let called = |op: &str| {
            Ir::Cmp(
                Cmp::Eq,
                Box::new(subject(op, vec![wrap(255)])),
                Box::new(Ir::Bool(true)),
            )
        };
        let m = mt(
            scrut,
            vec![(okp(Pat::Wild), called("f")), (Pat::ErrAny, called("g"))],
        );
        let mut st = Stats::default();
        assert_eq!(Program::new(&[]).trace(&m, &[], &mut st).unwrap(), 1);
        assert_eq!(st.wraps, 2, "the scrutinee's wrap and the taken arm's argument's wrap");
    }

    // -- the seeded-wrong evaluators ---------------------------------------------------------

    #[test]
    fn each_sabotage_changes_an_answer_that_the_correct_evaluator_gets_right() {
        let u8t = FixedTy::U8;
        let eq = Ir::Cmp(Cmp::Eq, nat(1), nat(1));
        let sub = Ir::Arith(Arith::Sub, Ty::Int, nat(5), nat(3));
        let wrap = Ir::Arith(Arith::Add, Ty::Fixed(u8t), fx(u8t, 255), fx(u8t, 1));
        let eval = |s: Sabotage, ir: &Ir| {
            let mut st = Stats::default();
            let v = Program::new(&[])
                .with_sabotage(s)
                .eval(ir, &[], &mut st)
                .unwrap();
            (v, st.wraps)
        };
        assert_eq!(eval(Sabotage::None, &eq).0, Value::Bool(true));
        assert_eq!(eval(Sabotage::EqIsNe, &eq).0, Value::Bool(false));
        assert_eq!(eval(Sabotage::None, &sub).0, Value::Int(2));
        assert_eq!(eval(Sabotage::SubIsAdd, &sub).0, Value::Int(8));
        assert_eq!(eval(Sabotage::None, &wrap), (Value::Int(0), 1));
        assert_eq!(eval(Sabotage::NoWrap, &wrap), (Value::Int(256), 0));
        // NoWrap also disables the wrapping negation
        let neg = Ir::Neg(Ty::Fixed(FixedTy::I8), fx(FixedTy::I8, -128));
        assert_eq!(eval(Sabotage::NoWrap, &neg).0, Value::Int(128));
        // `ne` is untouched by the EqIsNe sabotage
        let ne = Ir::Cmp(Cmp::Ne, nat(1), nat(2));
        assert_eq!(eval(Sabotage::EqIsNe, &ne).0, Value::Bool(true));
    }
}
