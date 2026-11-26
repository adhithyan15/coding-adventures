//! # The types of the model IR
//!
//! The S2 checker publishes signatures but not the type of every sub-expression, and
//! three things in the lowering depend on expression types (the contextual type of a bare
//! decimal literal, the static widths of a `Bytes<N>` call, and whether an arithmetic node
//! wraps).  So this crate carries a small type of its own, [`Ty`], covering exactly what
//! `ir.core` can express (VER04r section 4.1):
//!
//! ```text
//!   Bool   Int   Nat   U8 U16 U32 U64 I8 I16 I32 I64
//!   Bytes  Bytes<4>  Bytes<N>           -- a literal width or a static width parameter
//!   Result<T, ErrorName>
//!   (T1,T2)  ((Bool,Nat),U8)            -- ir.tuple: 2 to 8 components, structural
//!   Option<T>  Point  Shape             -- ir.data: Option, a record, a variant (S11b)
//! ```
//!
//! Tuples `(T1,..,Tn)` (2 to 8 components) belong to `ir.tuple` (slice S10t-2); `Option<T>`, records
//! and variants (a [`Ty::Named`]) belong to `ir.data` (slice S11b, VER04r 16.14 [D46] to [D48]).
//! `List<T>` ([`Ty::List`]) is the *reserved* `ir.list`: it exists as a type only so that the
//! lowering can see one and refuse it by name (R27); it never reaches a corpus.
//!
//! ## The width is part of the type (VER04r 16.16 R51, [D60])
//!
//! A bytes literal is `Bytes<len>`, a `concat` of two literal widths is `Bytes<sum>`, and an `if` or
//! `match` whose branches agree keeps the width; `Some(e)` and `Ok(e)` wrap **that** type.  There is no
//! "recorded width beside the type" any more (S10t kept one, copied from the oracle, and refused
//! `Ok(concat(a, b"0102"))` for a `Result<Bytes<3>,E>`: R13(c), turned round by S11b).
//!
//! `Int` is the mathematical integer of VER04 section 3.  It exists only inside the IR
//! (the type of `Nat - Nat`, of a bare literal outside a `Nat` context); VER04r 2.1 allows
//! it in a `lit` node and nowhere else.
//!
//! [`Ty::name`] is the canonical spelling used in `ops`, binders and stream contexts
//! (`Result<Bytes,SelectError>` has **no space** after the comma, exactly as the v1 corpus
//! and the independent oracle write it).

use coding_adventures_warrant_grammar::fixed::FixedTy;

/// The width of a `Bytes<..>` type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Width {
    /// `Bytes<4>`.
    Lit(u64),
    /// `Bytes<N>`, where `N` is a static parameter of an operation (VER04i 2.5).
    Param(String),
}

/// A type of the model IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    /// `Bool`.
    Bool,
    /// `Int`, the model-internal mathematical integer.
    Int,
    /// `Nat`, the natural numbers.
    Nat,
    /// One of the eight fixed-width integer types.
    Fixed(FixedTy),
    /// `Bytes` (no width) or `Bytes<N>` (a literal or named width).
    Bytes(Option<Width>),
    /// `Result<T, E>`: the payload type and the *name* of the declared error type.
    Result(Box<Ty>, String),
    /// `Option<T>`: a real type of `ir.data` (slice S11b, [D48]).  The one use that adds no `ir.data`
    /// is the **intermediate use** of R29: a producer node directly under `unwrap_or`.
    Option(Box<Ty>),
    /// A record or a variant, by its declared name (`ir.data`).  Nominal: two are equal exactly when
    /// their names are; the fields live in the lowering's tables, never in the type.
    Named(String),
    /// `List<T>`: the reserved `ir.list`.  Only the lowering ever holds one, to refuse it (R27).
    List(Box<Ty>),
    /// `(T1, .., Tn)`, `2 <= n <= 8` (`ir.tuple`, VER04r 4.1, [D20]): structural, so two tuple
    /// types are equal exactly when their components are.
    Tuple(Vec<Ty>),
}

impl Ty {
    /// The unconstrained byte string type.
    pub const BYTES: Ty = Ty::Bytes(None);

    /// `Bytes<n>` for a literal width.
    pub fn bytes_n(n: u64) -> Ty {
        Ty::Bytes(Some(Width::Lit(n)))
    }

    /// The canonical spelling, e.g. `"U64"`, `"Bytes<N>"`, `"Result<Bytes,SelectError>"`.
    pub fn name(&self) -> String {
        match self {
            Ty::Bool => "Bool".into(),
            Ty::Int => "Int".into(),
            Ty::Nat => "Nat".into(),
            Ty::Fixed(t) => t.name().into(),
            Ty::Bytes(None) => "Bytes".into(),
            Ty::Bytes(Some(Width::Lit(n))) => format!("Bytes<{n}>"),
            Ty::Bytes(Some(Width::Param(p))) => format!("Bytes<{p}>"),
            Ty::Result(ok, err) => format!("Result<{},{err}>", ok.name()),
            Ty::Option(inner) => format!("Option<{}>", inner.name()),
            Ty::List(inner) => format!("List<{}>", inner.name()),
            Ty::Named(name) => name.clone(),
            Ty::Tuple(parts) => format!(
                "({})",
                parts.iter().map(Ty::name).collect::<Vec<_>>().join(",")
            ),
        }
    }

    /// Is this type, or a type inside it, a tuple?  The type half of R9 (VER04r 16.10): a binder,
    /// parameter, result or sub-expression whose static type is or contains a tuple needs
    /// `ir.tuple` even where the IR names no tuple tag.
    pub fn contains_tuple(&self) -> bool {
        match self {
            Ty::Tuple(_) => true,
            Ty::Result(ok, _) | Ty::Option(ok) | Ty::List(ok) => ok.contains_tuple(),
            _ => false,
        }
    }

    /// Is this type, or a type inside it, a record, a variant or an `Option`?  The type half of R9
    /// for `ir.data` (VER04r 16.14 R29, Appendix A.2): such a type adds `ir.data` wherever it is the
    /// type of a binder, a parameter, a result or a sub-expression (the one exception, a producer
    /// under `unwrap_or`, is the lowerer's).  It also looks inside a tuple (`(Option<U8>,Bool)`).
    pub fn contains_data(&self) -> bool {
        match self {
            Ty::Option(_) | Ty::Named(_) => true,
            Ty::Tuple(parts) => parts.iter().any(Ty::contains_data),
            Ty::Result(ok, _) | Ty::List(ok) => ok.contains_data(),
            _ => false,
        }
    }

    /// Is this type, or a type inside it, a list (the reserved `ir.list`, R27)?
    pub fn contains_list(&self) -> bool {
        match self {
            Ty::List(_) => true,
            Ty::Tuple(parts) => parts.iter().any(Ty::contains_list),
            Ty::Result(ok, _) | Ty::Option(ok) => ok.contains_list(),
            _ => false,
        }
    }

    /// The levels this type brings by itself, sorted: `ir.tuple` for a tuple, `ir.data` for an
    /// `Option`, a record or a variant, anywhere inside it (R9, R29).
    pub fn type_levels(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.contains_data() {
            out.push("ir.data");
        }
        if self.contains_tuple() {
            out.push("ir.tuple");
        }
        out
    }

    /// The fixed-width type, if this is one.
    pub fn fixed(&self) -> Option<FixedTy> {
        match self {
            Ty::Fixed(t) => Some(*t),
            _ => None,
        }
    }

    /// Is this one of the eight fixed-width integer types?
    pub fn is_fixed(&self) -> bool {
        self.fixed().is_some()
    }

    /// `Int`, `Nat` or a fixed-width type: what `< + - *` accept.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Nat | Ty::Fixed(_))
    }

    /// Is this `Bytes` or `Bytes<..>`?
    pub fn is_bytes(&self) -> bool {
        matches!(self, Ty::Bytes(_))
    }

    /// The common type of two operand types, or `None`: the grammar checker's `common_type`
    /// (VER04 section 3, VER04r 16.16 R51, [D60]).
    ///
    /// ```text
    ///   Nat   with Int                 -> Int          (Nat widens)
    ///   Bytes<N> with Bytes<M> / Bytes -> Bytes        (a differing or forgotten width is forgotten)
    ///   Result<A,E> with Result<B,E>   -> Result<unify(A,B),E>
    ///   Option<A> with Option<B>       -> Option<unify(A,B)>        (by the payload, as Result does)
    ///   anything else must be equal; nothing else is implicit
    /// ```
    ///
    /// A tuple, a record, a variant and a list never lift: they are equal or have no common type
    /// (before S11b a tuple unified component by component, the oracle's CV2-F18 leniency, R61).
    pub fn unify(a: &Ty, b: &Ty) -> Option<Ty> {
        if a == b {
            return Some(a.clone());
        }
        match (a, b) {
            (Ty::Nat, Ty::Int) | (Ty::Int, Ty::Nat) => Some(Ty::Int),
            (Ty::Bytes(_), Ty::Bytes(_)) => Some(Ty::BYTES),
            (Ty::Result(x, e1), Ty::Result(y, e2)) if e1 == e2 => {
                Ty::unify(x, y).map(|t| Ty::Result(Box::new(t), e1.clone()))
            }
            (Ty::Option(x), Ty::Option(y)) => Ty::unify(x, y).map(|t| Ty::Option(Box::new(t))),
            _ => None,
        }
    }

    /// Can a value of type `src` flow to a place of type `dst`?  The grammar checker's
    /// `is_assignable_to` (R51): a type to itself; `Nat` to `Int`; `Bytes<N>` to `Bytes`; `Option<S>`
    /// to `Option<T>` and `Result<S,E>` to `Result<T,E>` when `S` flows to `T`.  Nothing else: a
    /// tuple, a record, a variant and a list are invariant, also under an `Option`.
    pub fn assignable(src: &Ty, dst: &Ty) -> bool {
        if src == dst {
            return true;
        }
        match (src, dst) {
            (Ty::Nat, Ty::Int) => true,
            (Ty::Bytes(_), Ty::Bytes(None)) => true,
            (Ty::Result(x, e1), Ty::Result(y, e2)) if e1 == e2 => Ty::assignable(x, y),
            (Ty::Option(x), Ty::Option(y)) => Ty::assignable(x, y),
            _ => false,
        }
    }

    /// Replace static width names by their values (`Bytes<N>` with `N = 4` is
    /// `Bytes<4>`).  Names with no value are kept.
    pub fn substitute(&self, statics: &[(String, u64)]) -> Ty {
        match self {
            Ty::Bytes(Some(Width::Param(p))) => statics
                .iter()
                .find(|(name, _)| name == p)
                .map(|(_, v)| Ty::bytes_n(*v))
                .unwrap_or_else(|| self.clone()),
            Ty::Result(ok, e) => Ty::Result(Box::new(ok.substitute(statics)), e.clone()),
            Ty::Option(inner) => Ty::Option(Box::new(inner.substitute(statics))),
            Ty::List(inner) => Ty::List(Box::new(inner.substitute(statics))),
            Ty::Tuple(parts) => Ty::Tuple(parts.iter().map(|p| p.substitute(statics)).collect()),
            other => other.clone(),
        }
    }
}

/// The declared records and variants of a document: what a value of a [`Ty::Named`] type needs to be
/// encoded, written as JSON or TFV, or compared with a fault (`ir.data`, S11c-2).
///
/// A record is a single constructor named like the record; a variant lists its constructors.  A value
/// of either is a [`crate::value::Value::Con`] carrying only the constructor's name, so the field types
/// (which decide `10` or `11` for an integer, the width of a `Bytes<N>`) live here.  Cloning is cheap
/// (a reference count).
#[derive(Debug, Clone, Default)]
pub struct Decls(std::rc::Rc<DeclsInner>);

#[derive(Debug, Default)]
struct DeclsInner {
    /// `name -> [(constructor, field types)]`; a record has one constructor, named like it.
    adts: std::collections::HashMap<String, Vec<(String, Vec<Ty>)>>,
    /// Names of the records (the others are variants).
    records: std::collections::HashSet<String>,
}

impl Decls {
    /// The declarations of a lowered document's `types`.
    pub fn new(types: &[crate::lower::LoweredType]) -> Decls {
        let mut inner = DeclsInner::default();
        for t in types {
            match t {
                crate::lower::LoweredType::Record { name, fields } => {
                    inner.records.insert(name.clone());
                    inner.adts.insert(
                        name.clone(),
                        vec![(name.clone(), fields.iter().map(|(_, t)| t.clone()).collect())],
                    );
                }
                crate::lower::LoweredType::Variant { name, ctors } => {
                    inner.adts.insert(name.clone(), ctors.clone());
                }
            }
        }
        Decls(std::rc::Rc::new(inner))
    }

    /// The constructors of the record or variant `name`.
    pub fn ctors(&self, name: &str) -> Option<&[(String, Vec<Ty>)]> {
        self.0.adts.get(name).map(Vec::as_slice)
    }

    /// Is `name` a declared record?
    pub fn is_record(&self, name: &str) -> bool {
        self.0.records.contains(name)
    }

    /// The type that declares constructor `ctor` and its field types (a record's constructor is named like
    /// the record).
    pub fn owner(&self, ctor: &str) -> Option<(&str, &[Ty])> {
        self.0.adts.iter().find_map(|(name, ctors)| {
            ctors
                .iter()
                .find(|(c, _)| c == ctor)
                .map(|(_, f)| (name.as_str(), f.as_slice()))
        })
    }

    /// The field types of constructor `ctor` of type `name`.
    pub fn fields(&self, name: &str, ctor: &str) -> Option<&[Ty]> {
        self.ctors(name)?
            .iter()
            .find(|(c, _)| c == ctor)
            .map(|(_, f)| f.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_bytes() -> Ty {
        Ty::Result(Box::new(Ty::BYTES), "SelectError".into())
    }

    #[test]
    fn names_are_the_canonical_spelling() {
        assert_eq!(Ty::Bool.name(), "Bool");
        assert_eq!(Ty::Int.name(), "Int");
        assert_eq!(Ty::Nat.name(), "Nat");
        assert_eq!(Ty::Fixed(FixedTy::U64).name(), "U64");
        assert_eq!(Ty::Fixed(FixedTy::I8).name(), "I8");
        assert_eq!(Ty::BYTES.name(), "Bytes");
        assert_eq!(Ty::bytes_n(4).name(), "Bytes<4>");
        assert_eq!(Ty::Bytes(Some(Width::Param("N".into()))).name(), "Bytes<N>");
        assert_eq!(result_bytes().name(), "Result<Bytes,SelectError>");
    }

    #[test]
    fn classification_helpers() {
        assert!(Ty::Fixed(FixedTy::U8).is_fixed() && Ty::Fixed(FixedTy::U8).is_numeric());
        assert_eq!(Ty::Fixed(FixedTy::I16).fixed(), Some(FixedTy::I16));
        assert_eq!(Ty::Nat.fixed(), None);
        assert!(Ty::Nat.is_numeric() && Ty::Int.is_numeric() && !Ty::Bool.is_numeric());
        assert!(Ty::bytes_n(2).is_bytes() && !Ty::Nat.is_bytes());
        assert!(!result_bytes().is_numeric());
    }

    #[test]
    fn unify_widens_nat_forgets_widths_and_nothing_else_is_implicit() {
        assert_eq!(Ty::unify(&Ty::Nat, &Ty::Int), Some(Ty::Int));
        assert_eq!(Ty::unify(&Ty::Int, &Ty::Nat), Some(Ty::Int));
        assert_eq!(Ty::unify(&Ty::bytes_n(2), &Ty::bytes_n(3)), Some(Ty::BYTES));
        assert_eq!(
            Ty::unify(&Ty::bytes_n(2), &Ty::bytes_n(2)),
            Some(Ty::bytes_n(2))
        );
        assert_eq!(Ty::unify(&Ty::BYTES, &Ty::bytes_n(2)), Some(Ty::BYTES));
        assert_eq!(
            Ty::unify(&Ty::Fixed(FixedTy::U8), &Ty::Fixed(FixedTy::U64)),
            None
        );
        assert_eq!(Ty::unify(&Ty::Fixed(FixedTy::U8), &Ty::Nat), None);
        assert_eq!(Ty::unify(&Ty::Bool, &Ty::Nat), None);
        let narrow = Ty::Result(Box::new(Ty::bytes_n(1)), "E".into());
        let wide = Ty::Result(Box::new(Ty::bytes_n(2)), "E".into());
        assert_eq!(
            Ty::unify(&narrow, &wide),
            Some(Ty::Result(Box::new(Ty::BYTES), "E".into()))
        );
        let other_err = Ty::Result(Box::new(Ty::bytes_n(1)), "F".into());
        assert_eq!(Ty::unify(&narrow, &other_err), None);
        let bad_inner = Ty::Result(Box::new(Ty::Bool), "E".into());
        assert_eq!(Ty::unify(&narrow, &bad_inner), None);
    }

    #[test]
    fn assignable_is_directional() {
        assert!(Ty::assignable(&Ty::Nat, &Ty::Int));
        assert!(!Ty::assignable(&Ty::Int, &Ty::Nat));
        assert!(Ty::assignable(&Ty::bytes_n(4), &Ty::BYTES));
        assert!(!Ty::assignable(&Ty::BYTES, &Ty::bytes_n(4)));
        assert!(!Ty::assignable(&Ty::bytes_n(4), &Ty::bytes_n(5)));
        assert!(Ty::assignable(&Ty::bytes_n(4), &Ty::bytes_n(4)));
        let narrow = Ty::Result(Box::new(Ty::bytes_n(1)), "E".into());
        assert!(Ty::assignable(
            &narrow,
            &Ty::Result(Box::new(Ty::BYTES), "E".into())
        ));
        assert!(!Ty::assignable(
            &narrow,
            &Ty::Result(Box::new(Ty::BYTES), "F".into())
        ));
        assert!(!Ty::assignable(&Ty::Bool, &Ty::Nat));
    }

    #[test]
    fn tuples_are_structural_named_without_spaces_and_counted_by_r9() {
        let pair = Ty::Tuple(vec![Ty::Bool, Ty::Nat]);
        assert_eq!(pair.name(), "(Bool,Nat)");
        let nested = Ty::Tuple(vec![pair.clone(), Ty::bytes_n(0)]);
        assert_eq!(nested.name(), "((Bool,Nat),Bytes<0>)");
        assert_eq!(
            Ty::Result(Box::new(pair.clone()), "E".into()).name(),
            "Result<(Bool,Nat),E>"
        );
        assert!(pair.contains_tuple() && nested.contains_tuple());
        assert!(Ty::Result(Box::new(pair.clone()), "E".into()).contains_tuple());
        assert!(!Ty::Result(Box::new(Ty::Bool), "E".into()).contains_tuple());
        assert!(!Ty::bytes_n(2).contains_tuple());
        // a tuple never lifts, in `unify` or in `assignable` (R51, R61): equal or nothing
        let wide = Ty::Tuple(vec![Ty::Bool, Ty::Int]);
        assert_eq!(Ty::unify(&pair, &wide), None);
        assert_eq!(Ty::unify(&pair, &pair.clone()), Some(pair.clone()));
        assert_eq!(Ty::unify(&pair, &Ty::Tuple(vec![Ty::Bool])), None);
        assert!(!Ty::assignable(&pair, &wide));
        assert!(!Ty::assignable(&wide, &pair));
        assert!(Ty::assignable(&pair, &pair.clone()));
        let narrow = Ty::Tuple(vec![Ty::bytes_n(1)]);
        assert!(!Ty::assignable(&narrow, &Ty::Tuple(vec![Ty::BYTES, Ty::Bool])));
        // a tuple is never a Result, and substitute looks inside
        assert_eq!(Ty::unify(&pair, &Ty::Bool), None);
        let param = Ty::Tuple(vec![Ty::Bytes(Some(Width::Param("N".into()))), Ty::Bool]);
        assert_eq!(
            param.substitute(&[("N".into(), 4)]),
            Ty::Tuple(vec![Ty::bytes_n(4), Ty::Bool])
        );
    }

    #[test]
    fn substitute_replaces_known_static_widths_only() {
        let n = Ty::Bytes(Some(Width::Param("N".into())));
        assert_eq!(n.substitute(&[("N".into(), 4)]), Ty::bytes_n(4));
        assert_eq!(n.substitute(&[("M".into(), 4)]), n);
        let r = Ty::Result(Box::new(n.clone()), "E".into());
        assert_eq!(
            r.substitute(&[("N".into(), 16)]),
            Ty::Result(Box::new(Ty::bytes_n(16)), "E".into())
        );
        assert_eq!(Ty::Nat.substitute(&[("N".into(), 1)]), Ty::Nat);
    }

    #[test]
    fn option_lifts_by_its_payload_and_records_and_variants_are_nominal() {
        let opt = |t: Ty| Ty::Option(Box::new(t));
        // R51: an Option flows and unifies by its payload, as a Result does
        assert!(Ty::assignable(&opt(Ty::Nat), &opt(Ty::Int)));
        assert!(!Ty::assignable(&opt(Ty::Int), &opt(Ty::Nat)));
        assert!(Ty::assignable(&opt(Ty::bytes_n(4)), &opt(Ty::BYTES)));
        assert!(Ty::assignable(&opt(opt(Ty::Nat)), &opt(opt(Ty::Int))));
        assert_eq!(Ty::unify(&opt(Ty::Nat), &opt(Ty::Int)), Some(opt(Ty::Int)));
        assert_eq!(Ty::unify(&opt(Ty::Nat), &opt(Ty::Bool)), None);
        // ... but nothing lifts through a tuple, also under an Option
        let nn = Ty::Tuple(vec![Ty::Nat, Ty::Nat]);
        let ii = Ty::Tuple(vec![Ty::Int, Ty::Int]);
        assert!(!Ty::assignable(&opt(nn.clone()), &opt(ii.clone())));
        assert_eq!(Ty::unify(&opt(nn), &opt(ii)), None);
        // a record or variant is its name
        let (p, q) = (Ty::Named("P".into()), Ty::Named("Q".into()));
        assert_eq!(p.name(), "P");
        assert!(Ty::assignable(&p, &p.clone()) && !Ty::assignable(&p, &q));
        assert_eq!(Ty::unify(&p, &q), None);
        // R9, the type half: ir.data for an Option, a record or a variant, anywhere inside
        assert_eq!(opt(Ty::Bool).type_levels(), ["ir.data"]);
        assert_eq!(p.type_levels(), ["ir.data"]);
        assert_eq!(
            Ty::Tuple(vec![opt(Ty::Bool), Ty::Bool]).type_levels(),
            ["ir.data", "ir.tuple"]
        );
        assert_eq!(
            Ty::Result(Box::new(opt(Ty::Bool)), "E".into()).type_levels(),
            ["ir.data"]
        );
        assert!(Ty::Bool.type_levels().is_empty());
        assert!(Ty::Result(Box::new(nn_test()), "E".into()).type_levels() == ["ir.tuple"]);
        // a list is never counted (it is refused), only found
        assert!(Ty::List(Box::new(Ty::Bool)).contains_list());
        assert!(opt(Ty::List(Box::new(Ty::Bool))).contains_list());
        assert!(!p.contains_list());
        assert_eq!(Ty::List(Box::new(Ty::Nat)).name(), "List<Nat>");
        assert_eq!(opt(Ty::bytes_n(2)).name(), "Option<Bytes<2>>");
        assert_eq!(
            opt(Ty::Bytes(Some(Width::Param("N".into()))))
                .substitute(&[("N".into(), 8)]),
            opt(Ty::bytes_n(8))
        );
    }

    fn nn_test() -> Ty {
        Ty::Tuple(vec![Ty::Nat, Ty::Nat])
    }
}
