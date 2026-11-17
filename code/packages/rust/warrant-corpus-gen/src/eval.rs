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
//! ## Seeded-wrong controls
//!
//! [`Sabotage`] makes the evaluator wrong on purpose (`eq` answering `ne`, `sub` adding,
//! fixed arithmetic not wrapping).  It is `#[doc(hidden)]` and used only by the tests that
//! prove a wrong evaluator changes a digest (VER04r 10.3: a corpus is only as good as the
//! evidence that its checks can fail).

use std::collections::HashMap;
use std::rc::Rc;

use crate::arith::{
    check_value_len, exact_neg, exact_op, fixed_neg, fixed_op, EXACT_BOUND, MAX_IR_VALUE,
};
use crate::error::{code, GenError};
use crate::ir::{Arith, Cmp, Ir, Logic, MAX_IR_DEPTH};
use crate::lower::LoweredModel;
use crate::value::{ill_typed, Value};

/// The deepest evaluation recursion allowed: 64 model frames of up to 64 IR levels, plus
/// the caller's own tree (VER04r 14: IR depth 64, model-call nesting 64).
pub const MAX_EVAL_DEPTH: usize = (MAX_IR_DEPTH + 1) * 65;

/// The counters the evaluator maintains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Fixed-width node evaluations whose wrapped result differs from the exact result
    /// (VER04r 3.3 rule 1).  Every evaluation counts: there is no memoisation, so the
    /// count cannot depend on cache state or on which suite ran first.
    pub wraps: u64,
}

/// A deliberately wrong evaluator, for the negative controls.  Not part of the API.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sabotage {
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

/// The variables in scope: binder or parameter names with their values.
pub type Vars<'v> = [(&'v str, Value)];

/// The models of a document plus what the evaluator needs to know about them.
#[derive(Debug, Clone)]
pub struct Program<'a> {
    models: &'a [LoweredModel],
    index: HashMap<&'a str, usize>,
    /// `wraps[i]`: model `i`'s call closure contains a fixed-width `add sub mul neg`.
    wraps: Vec<bool>,
    sabotage: Sabotage,
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
        for m in models {
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
            index,
            wraps,
            sabotage: Sabotage::None,
        }
    }

    /// The same program with a seeded-wrong evaluator.
    #[doc(hidden)]
    pub fn with_sabotage(mut self, s: Sabotage) -> Program<'a> {
        self.sabotage = s;
        self
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
    /// # Errors
    ///
    /// `W0701` for a subject call, an unbound variable, an ill-typed hand-built tree, an
    /// exact `Nat`/`Int` outside the 2^127 window or a `Bytes` result over
    /// [`MAX_IR_VALUE`]; `limit` for a recursion deeper than [`MAX_EVAL_DEPTH`].
    pub fn eval(&self, ir: &Ir, vars: &Vars<'_>, st: &mut Stats) -> Result<Value, GenError> {
        self.go(ir, vars, st, 0)
    }

    /// The recursion.  Debug builds give every local of every `match` arm its own stack
    /// slot, so the arms that need real work live in `#[inline(never)]` helpers: the frame
    /// that is on the stack once per nesting level stays small.
    fn go(
        &self,
        ir: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        depth: usize,
    ) -> Result<Value, GenError> {
        if depth > MAX_EVAL_DEPTH {
            return Err(GenError::new(
                code::LIMIT,
                "",
                "evaluation nests deeper than the IR and model-call limits allow",
            ));
        }
        let d = depth + 1;
        match ir {
            Ir::Bool(b) => Ok(Value::Bool(*b)),
            Ir::Int { value, .. } => Ok(Value::Int(*value)),
            Ir::Bytes(hex) => Ok(Value::bytes(&decode_hex(hex)?)),
            Ir::Var(name) => Ok(lookup(vars, name)?.clone()),
            Ir::Err(case) => Ok(Value::Err(Rc::from(case.as_str()))),
            Ir::Subject { .. } => Err(GenError::new(
                code::W0701,
                "",
                "a subject call cannot be evaluated by the generator (VER04r 4.2 rule 3)",
            )),
            Ir::Model { name, args } => self.model_call(name, args, vars, st, d),
            Ir::Len(a) => self.len_of(a, vars, st, d),
            Ir::Ok(a) => Ok(Value::Ok(Box::new(self.go(a, vars, st, d)?))),
            Ir::Cmp(op, a, b) => self.cmp_node(*op, a, b, vars, st, d),
            Ir::Arith(op, ty, a, b) => self.arith_node(*op, ty, a, b, vars, st, d),
            Ir::Neg(ty, a) => self.neg_node(ty, a, vars, st, d),
            Ir::Logic(op, a, b) => self.logic_node(*op, a, b, vars, st, d),
            Ir::Not(a) => Ok(Value::Bool(!self.go(a, vars, st, d)?.as_bool()?)),
            Ir::If(c, t, e) => self.if_node(c, t, e, vars, st, d),
            Ir::Concat(a, b) => self.concat_node(a, b, vars, st, d),
            Ir::Take(a, n) => self.cut_node(true, a, n, vars, st, d),
            Ir::Drop(a, n) => self.cut_node(false, a, n, vars, st, d),
            Ir::IsErr(a) => self.is_err_node(a, vars, st, d),
        }
    }

    #[inline(never)]
    fn model_call(
        &self,
        name: &str,
        args: &[Ir],
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let i = *self
            .index
            .get(name)
            .ok_or_else(|| GenError::new(code::W0701, "", "a call to an unknown model"))?;
        let m = &self.models[i];
        if m.params.len() != args.len() {
            return Err(GenError::new(
                code::W0701,
                "",
                "a model call has the wrong arity",
            ));
        }
        let mut frame: Vec<(&str, Value)> = Vec::with_capacity(args.len());
        for ((pname, _), a) in m.params.iter().zip(args) {
            frame.push((pname.as_str(), self.go(a, vars, st, d)?));
        }
        self.go(&m.body, &frame, st, d)
    }

    #[inline(never)]
    fn len_of(&self, a: &Ir, vars: &Vars<'_>, st: &mut Stats, d: usize) -> Result<Value, GenError> {
        let v = self.go(a, vars, st, d)?;
        Ok(Value::Int(v.as_bytes()?.len() as i128))
    }

    #[inline(never)]
    fn cmp_node(
        &self,
        op: Cmp,
        a: &Ir,
        b: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let x = self.go(a, vars, st, d)?;
        let y = self.go(b, vars, st, d)?;
        Ok(Value::Bool(self.compare(op, &x, &y)?))
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn arith_node(
        &self,
        op: Arith,
        ty: &crate::types::Ty,
        a: &Ir,
        b: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let x = self.go(a, vars, st, d)?.as_int()?;
        let y = self.go(b, vars, st, d)?.as_int()?;
        Ok(Value::Int(self.arith(op, ty.fixed(), x, y, st)?))
    }

    #[inline(never)]
    fn neg_node(
        &self,
        ty: &crate::types::Ty,
        a: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let x = self.go(a, vars, st, d)?.as_int()?;
        Ok(Value::Int(match ty.fixed() {
            Some(t) if self.sabotage != Sabotage::NoWrap => {
                let (v, wrapped) = fixed_neg(t, x);
                st.wraps += u64::from(wrapped);
                v
            }
            _ => exact_neg(x)?,
        }))
    }

    #[inline(never)]
    fn logic_node(
        &self,
        op: Logic,
        a: &Ir,
        b: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let left = self.go(a, vars, st, d)?.as_bool()?;
        // Short circuit: `and`/`implies` evaluate the right operand when the left is true;
        // `or` when it is false.  `implies` is `not left or right`.
        let (decided, value) = match op {
            Logic::And => (!left, false),
            Logic::Or => (left, true),
            Logic::Implies => (!left, true),
        };
        if decided {
            Ok(Value::Bool(value))
        } else {
            Ok(Value::Bool(self.go(b, vars, st, d)?.as_bool()?))
        }
    }

    #[inline(never)]
    fn if_node(
        &self,
        c: &Ir,
        t: &Ir,
        e: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        if self.go(c, vars, st, d)?.as_bool()? {
            self.go(t, vars, st, d)
        } else {
            self.go(e, vars, st, d)
        }
    }

    #[inline(never)]
    fn concat_node(
        &self,
        a: &Ir,
        b: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let x = self.go(a, vars, st, d)?;
        let y = self.go(b, vars, st, d)?;
        let (x, y) = (x.as_bytes()?, y.as_bytes()?);
        // Checked BEFORE the allocation (VER04r 4.2 rule 6).
        check_value_len((x.len() as u64) + (y.len() as u64))?;
        let mut out = Vec::with_capacity(x.len() + y.len());
        out.extend_from_slice(x);
        out.extend_from_slice(y);
        Ok(Value::Bytes(Rc::from(out)))
    }

    /// `take` (`keep_head`) or `drop`.
    #[inline(never)]
    fn cut_node(
        &self,
        keep_head: bool,
        a: &Ir,
        n: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        let x = self.go(a, vars, st, d)?;
        let n = self.go(n, vars, st, d)?.as_int()?;
        let x = x.as_bytes()?;
        let at = clamp_count(n, x.len());
        Ok(Value::bytes(if keep_head { &x[..at] } else { &x[at..] }))
    }

    #[inline(never)]
    fn is_err_node(
        &self,
        a: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        d: usize,
    ) -> Result<Value, GenError> {
        match self.go(a, vars, st, d)? {
            Value::Err(_) => Ok(Value::Bool(true)),
            Value::Ok(_) => Ok(Value::Bool(false)),
            _ => Err(ill_typed("a Result operand")),
        }
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
    pub fn trace(&self, ir: &Ir, vars: &Vars<'_>, st: &mut Stats) -> Result<u64, GenError> {
        self.trace_at(ir, vars, st, 0)
    }

    fn trace_at(
        &self,
        ir: &Ir,
        vars: &Vars<'_>,
        st: &mut Stats,
        depth: usize,
    ) -> Result<u64, GenError> {
        if depth > MAX_EVAL_DEPTH {
            return Err(GenError::new(code::LIMIT, "", "the trace nests too deeply"));
        }
        let impure = || {
            GenError::new(
                code::W0701,
                "",
                "a subject call under an impure guard (VER04r 4.4)",
            )
        };
        if !ir.has_subject() {
            if self.may_wrap(ir) {
                self.go(ir, vars, st, depth)?;
            }
            return Ok(0);
        }
        let d = depth + 1;
        match ir {
            Ir::Subject { args, .. } => {
                let mut n = 1u64;
                for a in args {
                    n += self.trace_at(a, vars, st, d)?;
                }
                Ok(n)
            }
            Ir::If(c, t, e) => {
                if t.has_subject() || e.has_subject() {
                    if c.has_subject() {
                        return Err(impure());
                    }
                    let taken = self.go(c, vars, st, d)?.as_bool()?;
                    self.trace_at(if taken { t } else { e }, vars, st, d)
                } else {
                    self.trace_at(c, vars, st, d)
                }
            }
            Ir::Logic(op, a, b) => {
                if b.has_subject() {
                    if a.has_subject() {
                        return Err(impure());
                    }
                    let left = self.go(a, vars, st, d)?.as_bool()?;
                    let runs = match op {
                        Logic::And | Logic::Implies => left,
                        Logic::Or => !left,
                    };
                    if runs {
                        self.trace_at(b, vars, st, d)
                    } else {
                        Ok(0)
                    }
                } else {
                    self.trace_at(a, vars, st, d)
                }
            }
            other => {
                let mut n = 0u64;
                for c in other.children() {
                    n += self.trace_at(c, vars, st, d)?;
                }
                Ok(n)
            }
        }
    }

    /// The call count of `ir` when it does not depend on the case: no `if` or short
    /// circuit has a subject in a branch.  `None` means "trace each case".
    pub fn static_calls(&self, ir: &Ir) -> Option<u64> {
        if !ir.has_subject() {
            return Some(0);
        }
        match ir {
            Ir::Subject { args, .. } => {
                let mut n = 1u64;
                for a in args {
                    n += self.static_calls(a)?;
                }
                Some(n)
            }
            Ir::If(c, t, e) => {
                if t.has_subject() || e.has_subject() {
                    None
                } else {
                    self.static_calls(c)
                }
            }
            Ir::Logic(_, a, b) => {
                if b.has_subject() {
                    None
                } else {
                    self.static_calls(a)
                }
            }
            other => {
                let mut n = 0u64;
                for c in other.children() {
                    n += self.static_calls(c)?;
                }
                Some(n)
            }
        }
    }
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
    if !b.len().is_multiple_of(2) || (b.len() / 2) as u64 > MAX_IR_VALUE {
        return Err(bad());
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
        // a literal over MAX_IR_VALUE is refused before it is decoded
        let huge = "00".repeat(65_537);
        assert!(run(&Ir::Bytes(huge)).is_err());
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
    fn evaluation_recursion_is_bounded() {
        let mut ir = Ir::Bool(true);
        for _ in 0..(MAX_EVAL_DEPTH + 10) {
            ir = Ir::Not(Box::new(ir));
        }
        // the tree is far deeper than lowering allows; the evaluator refuses it rather than
        // recursing without bound (run on a thread with room to unwind to the limit)
        let h = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || run(&ir).unwrap_err().code)
            .unwrap();
        assert_eq!(h.join().unwrap(), code::LIMIT);
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
