//! # Typing a fault expression (VER04r 16.17 R67, 16.19 R94, 16.20 R96 to R98; slice S11c-2)
//!
//! The generator's evaluator is untyped for *source* (the lowerer types a document as it lowers it), but a
//! **fault row** is IR the generator writes itself, and since S11c-1 the oracle compares the type of every
//! fault expression with the declared type of the position it replaces.  This module is that comparison, and
//! `tests/data_eval_parity.rs` reproduces the `typing` rows of `data_eval_v1.oracle-refusals.json` through it.
//!
//! ```text
//!   fault expression ──type_of(expected)──▶ type ──fault_unify(type, declared)──▶ common type | refusal
//!        │                                     │
//!        └ every branch of an `if` and arm of a `match` is compared with `expected` as it is typed (R98)
//! ```
//!
//! ## The rules
//!
//! * **R67.** A position hands its declared type down exactly as R52 hands the expected type of a source
//!   expression down: both branches of an `if`, the payload of an `ok` and of a `some`, component `i` of a
//!   tuple, argument `i` of a constructor, the body of every arm of a `match`.  The one thing it changes is the
//!   type of a bytes literal: in a position whose expected type is plain `Bytes` it is a `Bytes`; anywhere
//!   else `Bytes<len>`.  The value never changes.
//! * **R94.** The declared type and the expression's type need a **common type** ([`fault_unify`]): equal
//!   types; `Nat` and `Int` (to `Int`); two `Bytes` of any widths (to `Bytes`); `Option` and `Result` of types
//!   that unify (a `Result` with the same error type, or an undetermined one).  Not "flows to": a fault may
//!   change a length.
//! * **R96.** Two tuples of one arity unify component by component under the **narrower** relation
//!   ([`fault_component`]): equal types, `Nat` with `Int`, `Option` and `Result` of related components, a
//!   nested tuple likewise.  Nothing else lifts below a tuple: `Bytes<1>` against `Bytes<2>` is refused.
//! * **R97.** An undetermined component (the error type of a bare `ok`, the payload of a bare `err`) unifies
//!   with whatever stands opposite it, and the common type takes the determined side.
//! * **R98.** Every branch is compared, and the type of an accepted row is the common type with every
//!   undetermined component filled in (never the placeholder, spelled `?` here).
//!
//! The typer for the rest (`ir.core`, `ir.fixed`, `ir.tuple` and `ir.data` nodes) is the oracle's
//! `ir_eval.Program._compile`, reduced to what a fault template can write.

use coding_adventures_warrant_grammar::fixed::FixedTy;

use crate::error::{code, GenError};
use crate::ir::{Cmp, Ir, Pat};
use crate::types::{Decls, Ty, Width};

/// The undetermined error type of a bare `ok` and payload type of a bare `err`.
const UNKNOWN: &str = "?";

fn unknown() -> Ty {
    Ty::Named(UNKNOWN.into())
}

fn is_unknown(t: &Ty) -> bool {
    matches!(t, Ty::Named(n) if n == UNKNOWN)
}

fn ir_err(message: &str) -> GenError {
    GenError::new(code::IR, "", message)
}

/// The refusal of R94: no common type of the expression (or one of its branches) with the declared type.
fn no_common_type(op: &str, what: &str) -> GenError {
    GenError::new(
        code::FAULT,
        "",
        format!("the {what} of a fault on `{op}` has no common type with the declared type"),
    )
}

/// The common type of two **tuple components** (R96, R97).
pub fn fault_component(a: &Ty, b: &Ty) -> Option<Ty> {
    if a == b {
        return Some(a.clone());
    }
    match (a, b) {
        (Ty::Nat, Ty::Int) | (Ty::Int, Ty::Nat) => Some(Ty::Int),
        (Ty::Option(x), Ty::Option(y)) => fault_component(x, y).map(|t| Ty::Option(Box::new(t))),
        (Ty::Result(x, e1), Ty::Result(y, e2)) => {
            let payload = if is_unknown(x) {
                Some((**y).clone())
            } else if is_unknown(y) {
                Some((**x).clone())
            } else {
                fault_component(x, y)
            };
            let error = if e2 == UNKNOWN { e1 } else { e2 };
            let error = if e1 == UNKNOWN { e2 } else { error };
            let compatible = e1 == e2 || e1 == UNKNOWN || e2 == UNKNOWN;
            match payload {
                Some(p) if compatible => Some(Ty::Result(Box::new(p), error.clone())),
                _ => None,
            }
        }
        (Ty::Tuple(x), Ty::Tuple(y)) if x.len() == y.len() => {
            let parts: Option<Vec<Ty>> = x.iter().zip(y).map(|(p, q)| fault_component(p, q)).collect();
            parts.map(Ty::Tuple)
        }
        _ => None,
    }
}

/// The common type of a fault expression's type and the declared type it is compared with (R94, R96, R97).
pub fn fault_unify(a: &Ty, b: &Ty) -> Option<Ty> {
    if a == b {
        return Some(a.clone());
    }
    match (a, b) {
        (Ty::Nat, Ty::Int) | (Ty::Int, Ty::Nat) => Some(Ty::Int),
        (Ty::Bytes(_), Ty::Bytes(_)) => Some(Ty::BYTES),
        (Ty::Option(x), Ty::Option(y)) => fault_unify(x, y).map(|t| Ty::Option(Box::new(t))),
        (Ty::Result(x, e1), Ty::Result(y, e2)) => {
            if !(e1 == e2 || e1 == UNKNOWN || e2 == UNKNOWN) {
                return None;
            }
            let payload = if is_unknown(x) {
                (**y).clone()
            } else if is_unknown(y) {
                (**x).clone()
            } else {
                fault_unify(x, y)?
            };
            let error = if e2 == UNKNOWN { e1 } else { e2 };
            Some(Ty::Result(Box::new(payload), error.clone()))
        }
        _ => fault_component(a, b),
    }
}

/// Assignability that tolerates the undetermined error type; `lift` is off below a tuple (the oracle's
/// `_assignable`).
fn assignable(src: &Ty, dst: &Ty, lift: bool) -> bool {
    if !lift {
        if src == dst {
            return true;
        }
        return match (src, dst) {
            (Ty::Option(x), Ty::Option(y)) => assignable(x, y, false),
            (Ty::Result(x, e1), Ty::Result(y, e2)) => {
                (is_unknown(x) || assignable(x, y, false)) && (e1 == UNKNOWN || e2 == UNKNOWN || e1 == e2)
            }
            (Ty::Tuple(x), Ty::Tuple(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(p, q)| assignable(p, q, false))
            }
            _ => false,
        };
    }
    match (src, dst) {
        (Ty::Result(x, e1), Ty::Result(y, e2)) => {
            (is_unknown(x) || assignable(x, y, true)) && (e1 == UNKNOWN || e2 == UNKNOWN || e1 == e2)
        }
        (Ty::Option(x), Ty::Option(y)) => assignable(x, y, true),
        (Ty::Tuple(_), Ty::Tuple(_)) => assignable(src, dst, false),
        _ => Ty::assignable(src, dst),
    }
}

/// Two `Result` types as one: payload and error type from whichever knows them (the oracle's `_merge_result`).
fn merge_result(a: &Ty, b: &Ty) -> Ty {
    match (a, b) {
        (Ty::Result(x, e1), Ty::Result(y, e2)) => Ty::Result(
            Box::new(if is_unknown(x) { (**y).clone() } else { (**x).clone() }),
            if e1 == UNKNOWN { e2.clone() } else { e1.clone() },
        ),
        _ => a.clone(),
    }
}

/// The common type of two branch or operand types (the oracle's `_common`).
fn common(a: &Ty, b: &Ty) -> Option<Ty> {
    match (a, b) {
        (Ty::Result(..), Ty::Result(..)) => Some(merge_result(a, b)),
        (Ty::Option(x), Ty::Option(y)) => common(x, y).map(|t| Ty::Option(Box::new(t))),
        (Ty::Tuple(x), Ty::Tuple(y)) => {
            if x.len() != y.len() {
                return None;
            }
            let parts: Option<Vec<Ty>> = x.iter().zip(y).map(|(p, q)| common_exact(p, q)).collect();
            parts.map(Ty::Tuple)
        }
        _ => Ty::unify(a, b),
    }
}

fn common_exact(a: &Ty, b: &Ty) -> Option<Ty> {
    if a == b {
        return Some(a.clone());
    }
    match (a, b) {
        (Ty::Result(..), Ty::Result(..)) => {
            (assignable(a, b, false) || assignable(b, a, false)).then(|| merge_result(a, b))
        }
        (Ty::Option(x), Ty::Option(y)) => common_exact(x, y).map(|t| Ty::Option(Box::new(t))),
        (Ty::Tuple(x), Ty::Tuple(y)) if x.len() == y.len() => {
            let parts: Option<Vec<Ty>> = x.iter().zip(y).map(|(p, q)| common_exact(p, q)).collect();
            parts.map(Ty::Tuple)
        }
        _ => None,
    }
}

/// The types a pattern binds when it matches a value of type `ty`.
fn bind_types(pat: &Pat, ty: &Ty, decls: &Decls, out: &mut Vec<(String, Ty)>) -> Result<(), GenError> {
    match pat {
        Pat::Wild | Pat::ErrAny | Pat::Err(_) | Pat::None | Pat::Bool(_) => {}
        Pat::Bind(n) => out.push((n.clone(), ty.clone())),
        Pat::Ok(inner) => match ty {
            Ty::Result(ok, _) => bind_types(inner, ok, decls, out)?,
            _ => return Err(ir_err("an ok pattern needs a Result")),
        },
        Pat::Some(inner) => match ty {
            Ty::Option(t) => bind_types(inner, t, decls, out)?,
            _ => return Err(ir_err("a some pattern needs an Option")),
        },
        Pat::Tuple(parts) => match ty {
            Ty::Tuple(tys) if tys.len() == parts.len() => {
                for (p, t) in parts.iter().zip(tys) {
                    bind_types(p, t, decls, out)?;
                }
            }
            _ => return Err(ir_err("a tuple pattern needs a tuple of its arity")),
        },
        Pat::Ctor(name, parts) => {
            let Ty::Named(owner) = ty else {
                return Err(ir_err("a constructor pattern needs a record or variant"));
            };
            let fields = decls
                .fields(owner, name)
                .filter(|f| f.len() == parts.len())
                .ok_or_else(|| ir_err("a constructor pattern names no constructor of its type"))?;
            for (p, t) in parts.iter().zip(fields) {
                bind_types(p, t, decls, out)?;
            }
        }
    }
    Ok(())
}

/// The typer of one fault row: the declarations, the op's name (for the messages) and the typing environment.
pub struct FaultTyper<'a> {
    /// The document's records and variants.
    pub decls: &'a Decls,
    /// The faulted operation (named in a refusal).
    pub op: &'a str,
    /// `true`: every branch is compared with the expected type as it is typed (R98).
    pub branch_check: bool,
}

type Env = Vec<(String, Ty)>;

impl FaultTyper<'_> {
    /// Type `ir` with `expected` as the type its position asks for and require a common type with it
    /// (R67, R94); every branch is compared on the way (R98).  `what` is `"expression"` or `"rewrite"`.
    ///
    /// # Errors
    ///
    /// `fault` for a pair with no common type; `ir` for an ill-typed tree (a constructor operand that is not
    /// its field's type, ...).
    pub fn typed(&self, ir: &Ir, env: &Env, declared: Option<&Ty>, what: &str) -> Result<Ty, GenError> {
        let ty = self.type_of(ir, env, declared)?;
        if let Some(d) = declared {
            if fault_unify(&ty, d).is_none() {
                return Err(no_common_type(self.op, what));
            }
        }
        Ok(ty)
    }

    fn check_branch(&self, got: &Ty, expected: Option<&Ty>) -> Result<(), GenError> {
        if let (true, Some(want)) = (self.branch_check, expected) {
            if fault_unify(got, want).is_none() {
                return Err(no_common_type(self.op, "expression"));
            }
        }
        Ok(())
    }

    fn type_of(&self, ir: &Ir, env: &Env, expected: Option<&Ty>) -> Result<Ty, GenError> {
        let sub = |child: &Ir, want: Option<&Ty>| self.type_of(child, env, want);
        Ok(match ir {
            Ir::Bool(_) => Ty::Bool,
            Ir::Int { ty, .. } => ty.clone(),
            Ir::Bytes(hex) => {
                if matches!(expected, Some(Ty::Bytes(None))) {
                    Ty::BYTES
                } else {
                    Ty::bytes_n((hex.len() / 2) as u64)
                }
            }
            Ir::Var(name) => env
                .iter()
                .rev()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| ir_err("an unbound IR variable"))?,
            Ir::Err(_) => Ty::Result(Box::new(unknown()), UNKNOWN.into()),
            Ir::Subject { .. } | Ir::Model { .. } => {
                return Err(ir_err("a fault expression calls nothing"));
            }
            Ir::Len(a) => {
                if !sub(a, None)?.is_bytes() {
                    return Err(ir_err("len needs Bytes at ir.core"));
                }
                Ty::Nat
            }
            Ir::Ok(a) => {
                let want = match expected {
                    Some(Ty::Result(ok, _)) => Some(&**ok),
                    _ => None,
                };
                Ty::Result(Box::new(sub(a, want)?), UNKNOWN.into())
            }
            Ir::Not(a) => {
                if sub(a, None)? != Ty::Bool {
                    return Err(ir_err("not needs Bool"));
                }
                Ty::Bool
            }
            Ir::IsErr(a) => {
                if !matches!(sub(a, None)?, Ty::Result(..)) {
                    return Err(ir_err("is_err needs a Result"));
                }
                Ty::Bool
            }
            Ir::Neg(_, a) => match sub(a, None)? {
                t @ Ty::Fixed(_) => t,
                Ty::Nat | Ty::Int => Ty::Int,
                _ => return Err(ir_err("neg needs a number")),
            },
            Ir::Logic(_, a, b) => {
                if sub(a, None)? != Ty::Bool || sub(b, None)? != Ty::Bool {
                    return Err(ir_err("a logical operator needs Bool operands"));
                }
                Ty::Bool
            }
            Ir::If(c, t, e) => {
                let cond = sub(c, None)?;
                let tt = sub(t, expected)?;
                let et = sub(e, expected)?;
                self.check_branch(&tt, expected)?;
                self.check_branch(&et, expected)?;
                if cond != Ty::Bool {
                    return Err(ir_err("an if condition is Bool"));
                }
                common(&tt, &et).ok_or_else(|| ir_err("the branches of an if have no common type"))?
            }
            Ir::Concat(a, b) => {
                let (x, y) = (sub(a, None)?, sub(b, None)?);
                if !x.is_bytes() || !y.is_bytes() {
                    return Err(ir_err("concat needs Bytes"));
                }
                match (&x, &y) {
                    (Ty::Bytes(Some(Width::Lit(m))), Ty::Bytes(Some(Width::Lit(n)))) => Ty::bytes_n(m + n),
                    _ => Ty::BYTES,
                }
            }
            Ir::Take(a, n) | Ir::Drop(a, n) => {
                if !sub(a, None)?.is_bytes() || !matches!(sub(n, None)?, Ty::Nat | Ty::Int) {
                    return Err(ir_err("take/drop need Bytes and a count"));
                }
                Ty::BYTES
            }
            Ir::Cmp(op, a, b) => {
                let (x, y) = (sub(a, None)?, sub(b, None)?);
                match op {
                    Cmp::Eq | Cmp::Ne => {
                        let results = matches!(x, Ty::Result(..)) || matches!(y, Ty::Result(..));
                        if results {
                            if !(matches!(x, Ty::Result(..)) && matches!(y, Ty::Result(..))) {
                                return Err(ir_err("comparing a Result with a non-Result"));
                            }
                        } else if common(&x, &y).is_none() {
                            return Err(ir_err("equality needs operands of one type"));
                        }
                    }
                    _ => {
                        let num = |t: &Ty| t.is_numeric();
                        if !(num(&x) && num(&y)) || ((x.is_fixed() || y.is_fixed()) && x != y) {
                            return Err(ir_err("an ordering needs two numbers of one kind"));
                        }
                    }
                }
                Ty::Bool
            }
            Ir::Arith(_, _, a, b) => {
                let (x, y) = (sub(a, None)?, sub(b, None)?);
                if x.is_fixed() || y.is_fixed() {
                    if x != y {
                        return Err(ir_err("a fixed-width operation needs operands of one type"));
                    }
                    x
                } else if matches!(x, Ty::Nat | Ty::Int) && matches!(y, Ty::Nat | Ty::Int) {
                    Ty::Int
                } else {
                    return Err(ir_err("arithmetic needs numbers"));
                }
            }
            Ir::Tuple(parts) => {
                let wants: Vec<Option<&Ty>> = match expected {
                    Some(Ty::Tuple(tys)) if tys.len() == parts.len() => tys.iter().map(Some).collect(),
                    _ => vec![None; parts.len()],
                };
                let tys = parts
                    .iter()
                    .zip(wants)
                    .map(|(p, w)| sub(p, w))
                    .collect::<Result<Vec<_>, _>>()?;
                Ty::Tuple(tys)
            }
            Ir::Project(base, index) => match sub(base, None)? {
                Ty::Tuple(tys) => tys
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| ir_err("a projection index outside the tuple"))?,
                _ => return Err(ir_err("project needs a tuple")),
            },
            Ir::Match { scrutinee, arms, .. } => {
                let s = sub(scrutinee, None)?;
                let mut merged: Option<Ty> = None;
                for (pat, body) in arms {
                    let mut binds = Vec::new();
                    bind_types(pat, &s, self.decls, &mut binds)?;
                    let mut inner = env.clone();
                    inner.extend(binds);
                    let kid = self.type_of(body, &inner, expected)?;
                    self.check_branch(&kid, expected)?;
                    merged = Some(match merged {
                        None => kid,
                        Some(m) => common(&m, &kid)
                            .ok_or_else(|| ir_err("the arms of a match have no common type"))?,
                    });
                }
                merged.ok_or_else(|| ir_err("a match has arms"))?
            }
            Ir::Fixed { tag, ty, args, .. } => {
                let kids = args
                    .iter()
                    .map(|a| sub(a, None))
                    .collect::<Result<Vec<_>, _>>()?;
                fixed_type(tag, *ty, &kids)?
            }
            Ir::Some(a) => {
                let want = match expected {
                    Some(Ty::Option(t)) => Some(&**t),
                    _ => None,
                };
                Ty::Option(Box::new(sub(a, want)?))
            }
            Ir::None(t) => Ty::Option(Box::new(t.clone())),
            Ir::Ctor { name, args } => {
                let (owner, fields) = self
                    .decls
                    .owner(name)
                    .ok_or_else(|| ir_err("a ctor names no declared constructor"))?;
                if args.len() != fields.len() {
                    return Err(ir_err("a ctor has the wrong number of operands"));
                }
                let kids = args
                    .iter()
                    .zip(fields)
                    .map(|(a, f)| sub(a, Some(f)))
                    .collect::<Result<Vec<_>, _>>()?;
                for (k, f) in kids.iter().zip(fields) {
                    if !assignable(k, f, true) {
                        return Err(ir_err("a ctor operand does not have its field's type"));
                    }
                }
                Ty::Named(owner.to_string())
            }
            Ir::Field(base, index) => {
                let b = sub(base, None)?;
                let Ty::Named(owner) = &b else {
                    return Err(ir_err("field needs a record"));
                };
                if !self.decls.is_record(owner) {
                    return Err(ir_err("field needs a record"));
                }
                self.decls
                    .fields(owner, owner)
                    .and_then(|f| f.get(*index))
                    .cloned()
                    .ok_or_else(|| ir_err("a field index outside the record"))?
            }
        })
    }
}

/// Is the range of fixed type `source` inside that of `target`?
fn inside(source: FixedTy, target: FixedTy) -> bool {
    target.min() <= source.min() && source.max() <= target.max()
}

/// The result type of an `ir.fixed` node (the oracle's `_fixed_type`; operand types are those of Appendix A.1).
fn fixed_type(tag: &str, stat: Option<FixedTy>, types: &[Ty]) -> Result<Ty, GenError> {
    let bad = |why: &str| ir_err(why);
    let fixed = |t: &Ty| t.fixed();
    let signed = |t: &Ty| t.fixed().is_some_and(|f| f.signed());
    let one = || types.first().cloned().ok_or_else(|| bad("a built-in lost an operand"));
    let two = || match types {
        [a, b] => Ok((a.clone(), b.clone())),
        _ => Err(bad("a built-in lost an operand")),
    };
    let stat_ty = || stat.map(Ty::Fixed).ok_or_else(|| bad("a static type argument is a fixed-width type name"));
    Ok(match tag {
        "max_value" | "min_value" => stat_ty()?,
        "widen" | "wrap" | "try" => {
            let x = one()?;
            let target = stat.ok_or_else(|| bad("a static type argument is a fixed-width type name"))?;
            match fixed(&x) {
                Some(src) => {
                    if tag == "widen" && !inside(src, target) {
                        return Err(bad("widen needs a source range inside the target"));
                    }
                }
                None => {
                    if tag == "widen" || !matches!(x, Ty::Int | Ty::Nat) {
                        return Err(bad("a conversion needs a fixed-width, Int or Nat operand"));
                    }
                }
            }
            if tag == "try" {
                Ty::Option(Box::new(Ty::Fixed(target)))
            } else {
                Ty::Fixed(target)
            }
        }
        "to_int" | "to_nat" => {
            let x = one()?;
            if fixed(&x).is_none() || (tag == "to_nat" && signed(&x)) {
                return Err(bad("to_int needs a fixed-width operand and to_nat an unsigned one"));
            }
            if tag == "to_int" {
                Ty::Int
            } else {
                Ty::Nat
            }
        }
        "from_be" | "from_le" => {
            let x = one()?;
            let target = stat.ok_or_else(|| bad("a static type argument is a fixed-width type name"))?;
            if x != Ty::bytes_n(u64::from(target.bits() / 8)) {
                return Err(bad("from_be/from_le need Bytes of exactly the target's width"));
            }
            Ty::Fixed(target)
        }
        "bit_not" | "popcount" | "clz" | "ctz" | "be_bytes" | "le_bytes" => {
            let x = one()?;
            let Some(f) = fixed(&x) else {
                return Err(bad("a bit operation needs a fixed-width operand"));
            };
            match tag {
                "bit_not" => x,
                "be_bytes" | "le_bytes" => Ty::bytes_n(u64::from(f.bits() / 8)),
                _ => Ty::Nat,
            }
        }
        "shl" | "shr" | "sar" | "rotl" | "rotr" | "bit" => {
            let (x, n) = two()?;
            if fixed(&x).is_none() || n != Ty::Nat {
                return Err(bad("a shift needs a fixed-width operand and a Nat count"));
            }
            if tag == "sar" && !signed(&x) {
                return Err(bad("sar is for signed operands"));
            }
            if tag == "bit" {
                Ty::Bool
            } else {
                x
            }
        }
        "bit_and" | "bit_or" | "bit_xor" | "checked_add" | "checked_sub" | "checked_mul" => {
            let (a, b) = two()?;
            if fixed(&a).is_none() || a != b {
                return Err(bad("operands must be fixed-width values of one type"));
            }
            if tag.starts_with("bit_") {
                a
            } else {
                Ty::Option(Box::new(a))
            }
        }
        "div" | "rem" => {
            let (a, b) = two()?;
            if fixed(&a).is_some() || fixed(&b).is_some() {
                if a != b || signed(&a) {
                    return Err(bad("div and rem take two unsigned operands of one type, or Int/Nat"));
                }
                Ty::Option(Box::new(a))
            } else if !matches!(a, Ty::Int | Ty::Nat) || !matches!(b, Ty::Int | Ty::Nat) {
                return Err(bad("div and rem take two unsigned operands of one type, or Int/Nat"));
            } else {
                Ty::Option(Box::new(Ty::Int))
            }
        }
        "div_trunc" | "rem_trunc" => {
            let (a, b) = two()?;
            if fixed(&a).is_none() || a != b || !signed(&a) {
                return Err(bad("div_trunc and rem_trunc take two signed operands of one type"));
            }
            Ty::Option(Box::new(a))
        }
        "imin" | "imax" => {
            if types.iter().any(|t| !matches!(t, Ty::Int | Ty::Nat)) {
                return Err(bad("imin and imax take Int or Nat operands"));
            }
            Ty::Int
        }
        "at" => {
            let (b, i) = two()?;
            if !b.is_bytes() || !matches!(i, Ty::Nat | Ty::Int) {
                return Err(bad("at needs Bytes and a Nat or Int index"));
            }
            Ty::Option(Box::new(Ty::Fixed(FixedTy::U8)))
        }
        "unwrap_or" => {
            let (o, d) = two()?;
            match o {
                Ty::Option(inner) if assignable(&d, &inner, true) => *inner,
                _ => return Err(bad("unwrap_or needs an Option and a default of its type")),
            }
        }
        _ => return Err(bad("an unknown ir.fixed tag")),
    })
}
