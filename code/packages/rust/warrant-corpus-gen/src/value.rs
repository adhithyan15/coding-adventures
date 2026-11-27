//! # Values of the `ir.core` evaluator
//!
//! The evaluator is dynamically typed: the types live on the IR (the [`crate::ir::Ir::Arith`]
//! annotation, the binder and result types of a suite), and a value only needs to carry
//! what the operators look at.
//!
//! ```text
//!   Value::Bool(bool)          Bool
//!   Value::Int(i128)           Nat, Int and all eight fixed-width types (the IR says which)
//!   Value::Bytes(Rc<[u8]>)     Bytes and Bytes<N>
//!   Value::Ok(Box<Value>)      Ok(v)       of a Result
//!   Value::Err(Rc<str>)        Err(E.CASE) of a Result (the case name; errors are nominal)
//!   Value::Tuple(Vec<Value>)   (v1, .., vn)  an `ir.tuple` value, 2 to 8 components (S10t-2)
//!   Value::Opt(Option<Box<Value>>)  none / some(v)  inside an `ir.fixed` expression only (S10c-2)
//! ```
//!
//! An `i128` holds every `U64` and every `I64` exactly, and every exact `Nat`/`Int`
//! intermediate the window of VER04r 3.2 allows (`-(2^127-1)..=2^127-1`), so one integer
//! variant is enough.  Equality is structural (VER04 section 3): `Ok(a) == Ok(b)` iff
//! `a == b`, an `Err` equals an `Err` with the same case, and values of different variants
//! are never equal.  `Rc` keeps a clone of a `Bytes` binder O(1): the evaluator clones a
//! variable on every read.

use std::rc::Rc;

use coding_adventures_warrant_grammar::codegen::EvalValue;

use crate::error::{code, Beyond, GenError};

/// A value of the evaluator.  See the module documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `true` / `false`.
    Bool(bool),
    /// An integer of any numeric type.
    Int(i128),
    /// A byte string.
    Bytes(Rc<[u8]>),
    /// `Ok(v)`.
    Ok(Box<Value>),
    /// `Err(E.CASE)`: the case name.
    Err(Rc<str>),
    /// `(v1, .., vn)`: the components in order (`ir.tuple`).  Equality is component-wise.
    Tuple(Vec<Value>),
    /// `none` or `some(v)`: the IR-only intermediate of the nine `ir.fixed` producers (`try`,
    /// `checked_*`, `div`, `rem`, `div_trunc`, `rem_trunc`, `at`), consumed by `unwrap_or` alone.  It
    /// is never a binder, a parameter, a result or an encoded value (VER04r 16.10 [D23]).
    Opt(Option<Box<Value>>),
    /// A record or a variant (`ir.data`, S11c-2): the constructor's name and its fields in declaration
    /// order.  A record's constructor is named like the record, so equality is structural and needs no
    /// type; the declared field types (which the stream, JSON and TFV writers need) are [`crate::types::Decls`].
    Con(Rc<str>, Vec<Value>),
}

impl Value {
    /// A byte-string value.
    pub fn bytes(b: &[u8]) -> Value {
        Value::Bytes(Rc::from(b))
    }

    /// Convert a value of the S2 `domain` API ([`EvalValue`]) to an evaluator value.
    ///
    /// # Errors
    ///
    /// `level [ir.list]` for a list (the reserved level, R27); options, records and variants convert
    /// and never reach a binder of a document this generator emits.  A tuple converts
    /// component by component (`ir.tuple`).
    pub fn from_eval(v: &EvalValue) -> Result<Value, GenError> {
        Ok(match v {
            EvalValue::Bool(b) => Value::Bool(*b),
            EvalValue::Int(i) => Value::Int(*i),
            EvalValue::Nat(n) => Value::Int(i128::from(*n)),
            EvalValue::U8(n) => Value::Int(i128::from(*n)),
            EvalValue::U64(n) => Value::Int(i128::from(*n)),
            EvalValue::Fixed(_, n) => Value::Int(*n),
            EvalValue::Bytes(b) => Value::bytes(b),
            EvalValue::Result(Ok(inner)) => Value::Ok(Box::new(Value::from_eval(inner)?)),
            EvalValue::Result(Err((_, case))) => Value::Err(Rc::from(case.as_str())),
            EvalValue::Tuple(items) => Value::Tuple(
                items
                    .iter()
                    .map(Value::from_eval)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            EvalValue::Option(None) => Value::Opt(None),
            EvalValue::Option(Some(inner)) => Value::Opt(Some(Box::new(Value::from_eval(inner)?))),
            EvalValue::Con { ctor, fields, .. } => Value::Con(
                Rc::from(ctor.as_str()),
                fields
                    .iter()
                    .map(Value::from_eval)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            EvalValue::List(_) => {
                return Err(GenError::level(
                    Beyond::IrList,
                    "",
                    "a list value is the reserved level ir.list",
                ))
            }
        })
    }

    /// The Boolean, or an internal-error refusal (the IR was typed, so this is a bug in a
    /// hand-built tree).
    pub fn as_bool(&self) -> Result<bool, GenError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(ill_typed("a Bool operand")),
        }
    }

    /// The integer, or an internal-error refusal.
    pub fn as_int(&self) -> Result<i128, GenError> {
        match self {
            Value::Int(i) => Ok(*i),
            _ => Err(ill_typed("a numeric operand")),
        }
    }

    /// The bytes, or an internal-error refusal.
    pub fn as_bytes(&self) -> Result<&Rc<[u8]>, GenError> {
        match self {
            Value::Bytes(b) => Ok(b),
            _ => Err(ill_typed("a Bytes operand")),
        }
    }
}

/// A refusal for a tree that is not well typed (cannot happen for a lowered document).
pub(crate) fn ill_typed(what: &str) -> GenError {
    GenError::new(
        code::W0701,
        "",
        format!("the evaluator needs {what} (an ill-typed hand-built IR tree)"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    #[test]
    fn domain_values_convert_to_one_integer_variant() {
        assert_eq!(
            Value::from_eval(&EvalValue::Bool(true)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            Value::from_eval(&EvalValue::U64(u64::MAX)).unwrap(),
            Value::Int(i128::from(u64::MAX))
        );
        assert_eq!(Value::from_eval(&EvalValue::U8(7)).unwrap(), Value::Int(7));
        assert_eq!(Value::from_eval(&EvalValue::Nat(9)).unwrap(), Value::Int(9));
        assert_eq!(
            Value::from_eval(&EvalValue::Int(-3)).unwrap(),
            Value::Int(-3)
        );
        assert_eq!(
            Value::from_eval(&EvalValue::Fixed(FixedTy::I8, -128)).unwrap(),
            Value::Int(-128)
        );
        assert_eq!(
            Value::from_eval(&EvalValue::Bytes(vec![1, 2])).unwrap(),
            Value::bytes(&[1, 2])
        );
        let ok = EvalValue::Result(Ok(Box::new(EvalValue::Bytes(vec![0xff]))));
        assert_eq!(
            Value::from_eval(&ok).unwrap(),
            Value::Ok(Box::new(Value::bytes(&[0xff])))
        );
        let err = EvalValue::Result(Err(("E".into(), "BAD".into())));
        assert_eq!(Value::from_eval(&err).unwrap(), Value::Err(Rc::from("BAD")));
    }

    #[test]
    fn tuples_convert_component_by_component_and_compare_structurally() {
        let t = EvalValue::Tuple(vec![
            EvalValue::Bool(true),
            EvalValue::Tuple(vec![EvalValue::Nat(3), EvalValue::Bytes(vec![1])]),
        ]);
        let v = Value::from_eval(&t).unwrap();
        assert_eq!(
            v,
            Value::Tuple(vec![
                Value::Bool(true),
                Value::Tuple(vec![Value::Int(3), Value::bytes(&[1])])
            ])
        );
        assert_ne!(v, Value::Tuple(vec![Value::Bool(true)]));
        // a list inside a tuple is still the reserved ir.list
        let bad = EvalValue::Tuple(vec![EvalValue::Bool(true), EvalValue::List(vec![])]);
        assert_eq!(Value::from_eval(&bad).unwrap_err().level, Some("ir.list"));
    }

    #[test]
    fn options_records_and_variants_convert_and_a_list_is_still_refused() {
        assert_eq!(Value::from_eval(&EvalValue::Option(None)).unwrap(), Value::Opt(None));
        let some = EvalValue::Option(Some(Box::new(EvalValue::Bool(true))));
        assert_eq!(
            Value::from_eval(&some).unwrap(),
            Value::Opt(Some(Box::new(Value::Bool(true))))
        );
        let con = EvalValue::Con {
            ty: "T".into(),
            index: 0,
            ctor: "T".into(),
            record: true,
            fields: vec![EvalValue::Nat(2)],
        };
        assert_eq!(
            Value::from_eval(&con).unwrap(),
            Value::Con(Rc::from("T"), vec![Value::Int(2)])
        );
        // a list is the reserved ir.list, also under an Ok
        let e = Value::from_eval(&EvalValue::List(vec![])).unwrap_err();
        assert_eq!(e.level, Some("ir.list"));
        let nested = EvalValue::Result(Ok(Box::new(EvalValue::List(vec![]))));
        assert!(Value::from_eval(&nested).is_err());
    }

    #[test]
    fn equality_is_structural_and_variants_never_mix() {
        assert_eq!(Value::Int(1), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Bool(true));
        assert_ne!(Value::bytes(&[1]), Value::Int(1));
        assert_eq!(
            Value::Ok(Box::new(Value::Int(1))),
            Value::Ok(Box::new(Value::Int(1)))
        );
        assert_ne!(
            Value::Ok(Box::new(Value::Int(1))),
            Value::Err(Rc::from("X"))
        );
        assert_ne!(Value::Err(Rc::from("X")), Value::Err(Rc::from("Y")));
    }

    #[test]
    fn accessors_refuse_the_wrong_variant() {
        assert!(Value::Bool(true).as_bool().unwrap());
        assert_eq!(Value::Int(4).as_int().unwrap(), 4);
        assert_eq!(&**Value::bytes(&[9]).as_bytes().unwrap(), &[9u8][..]);
        assert_eq!(Value::Int(1).as_bool().unwrap_err().code, code::W0701);
        assert!(Value::Bool(true).as_int().is_err());
        assert!(Value::Int(1).as_bytes().is_err());
    }
}
