//! # `Int` never reaches a corpus (VER04r 2.1, CV2-F17)
//!
//! `Int` is the mathematical integer of the model IR: the type of an intermediate (`Nat - Nat`) and of a
//! `lit` node, and nothing a corpus can *write*.  The lowering accepts it wherever the grammar does (the
//! committed `data-forms-v1` is a lowering-only fixture of documents that name it in a record, an op and a
//! binder), so the refusal is a separate **document check** that runs after lowering and before anything
//! is costed or emitted, exactly where the oracle's `derive` runs `refuse_int_in_corpus`:
//!
//! ```text
//!   lower_document ──▶ refuse_int_in_corpus ──▶ cost ──▶ enumerate ──▶ emit
//!         (accepts Int)       (W0701)
//! ```
//!
//! ## What is looked at
//!
//! ```text
//!   a record field          corpus["types"][R]["fields"]       "a record field"
//!   a variant field         corpus["types"][V]["ctors"]        "a variant field"
//!   an op parameter/result  corpus["ops"][f]["params"], "ret"  "an op signature"
//!   a binder                a claim's `forall` binders         "a binder"
//! ```
//!
//! at any depth: `Option<Int>`, `Result<Int, E>`, `(Int, Bool)`, `Option<(Bool, Option<Int>)>`.  An alias
//! is resolved away by the lowering, so `model Z = Int` then `record R(i: Z)` is a record field of `Int`.
//! When several positions are wrong the *first kind in the order above* names the refusal (the oracle's
//! precedence), so the message does not depend on declaration order.
//!
//! ## What is not looked at (and why)
//!
//! * A model's *result* and a model's *body*: they are never written to a corpus.
//! * A model's *parameters*: they ARE written (`corpus["models"][m]["params"]`), contrary to D23, but
//!   `arithmetic` and `fixed_forms_v1` use them, so this is an **open owner question**, left as the oracle
//!   leaves it.
//! * A `lit` node's type and the argument of a `none`: IR nodes, never a corpus position.
//!
//! The message is the oracle's, byte for byte after the `W0701: ` that [`GenError`]'s `Display` writes.

use crate::error::{code, GenError};
use crate::lower::{Lowered, LoweredType};
use crate::types::Ty;

/// Does `ty` name `Int` at any depth (through `Option`, `Result`, a tuple or a list)?  A record or
/// variant is a name: its own fields are checked as fields.
pub fn mentions_int(ty: &Ty) -> bool {
    match ty {
        Ty::Int => true,
        Ty::Result(inner, _) | Ty::Option(inner) | Ty::List(inner) => mentions_int(inner),
        Ty::Tuple(parts) => parts.iter().any(mentions_int),
        Ty::Bool | Ty::Nat | Ty::Fixed(_) | Ty::Bytes(_) | Ty::Named(_) => false,
    }
}

/// Refuse (`W0701`) a document that names `Int` in a position a corpus writes.
///
/// # Errors
///
/// `W0701`, ``W0701: `Int` is model-internal and may not be <where> of a corpus (VER04r 2.1)``, where
/// `<where>` is "a record field", "a variant field", "an op signature" or "a binder".
pub fn refuse_int_in_corpus(lowered: &Lowered) -> Result<(), GenError> {
    let in_fields = |want_record: bool| {
        lowered.types.iter().any(|t| match t {
            LoweredType::Record { fields, .. } => want_record && fields.iter().any(|(_, ty)| mentions_int(ty)),
            LoweredType::Variant { ctors, .. } => {
                !want_record && ctors.iter().any(|(_, tys)| tys.iter().any(mentions_int))
            }
        })
    };
    let place = if in_fields(true) {
        Some("a record field")
    } else if in_fields(false) {
        Some("a variant field")
    } else if lowered
        .ops
        .iter()
        .any(|op| op.params.iter().any(|(_, t)| mentions_int(t)) || mentions_int(&op.ret))
    {
        Some("an op signature")
    } else if lowered
        .suites
        .iter()
        .any(|s| s.binders.iter().any(|(_, t)| mentions_int(t)))
    {
        Some("a binder")
    } else {
        None
    };
    match place {
        None => Ok(()),
        Some(place) => Err(GenError::new(
            code::W0701,
            "",
            format!("`Int` is model-internal and may not be {place} of a corpus (VER04r 2.1)"),
        )),
    }
}
