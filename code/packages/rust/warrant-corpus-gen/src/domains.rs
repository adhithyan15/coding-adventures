//! # Binder domains: descriptors, labels and the VER04r 16.2 refusals
//!
//! A claim's binders range over *resolved descriptor trees* (VER04r 5.1): the finite,
//! ordered set of values of each binder **after** the budget demotion of VER04h section 3.
//! That resolution is the public API of `warrant-grammar` (slice S2): this module calls
//! [`resolve_binders`] and never re-derives an order, a size or a demotion, because "the
//! single largest cause of silent drift between a generator and a consumer" is exactly that.
//!
//! ```text
//!   claim AST ──resolve_binders──▶ Domain per binder ──describe──▶ descriptor JSON, label, size
//!                                        │
//!                                        └─ iter() / nth(i) ──▶ Value cache (small domains)
//! ```
//!
//! ## Descriptors and labels
//!
//! Descriptor names follow the independent oracle (`descriptor_for`), which VER04r 16.3
//! binds for the emitted JSON:
//!
//! ```text
//!   Domain                          descriptor                     label        size
//!   Bool                            {"d":"bool"}                   full            2
//!   U8 (or v1 U8)                   {"d":"u8_all"}                 full          256
//!   U64 (or v1 U64)                 {"d":"u64_edge"}               bounded-set   377
//!   Nat                             {"d":"nat_0_64"}               bounded-set    65
//!   Bytes                           {"d":"bytes_d1"}               bounded-set   347
//!   Bytes<N>                        {"d":"bytes_n","n":N}          full (N<=1) / sample (N>=2)
//!   8/16-bit fixed (not U8)         {"d":"fx_full","t":T}          full
//!   unsigned 32-bit fixed           {"d":"fx_edge","t":T}          bounded-set  313
//!   Result<T,E>                     {"d":"result","ok":D,"errs":[..]}   label of D
//! ```
//!
//! Every node also records its `size`.  `Bytes<N>` for `N >= 2` is `sample` (VER04r 9.3).
//!
//! ## What is refused, and why (VER04r 16.2, decided for S3 in 16.3 item 11e)
//!
//! The S2 API reports what the *generator* enumerates, and it differs from VER04r 5.1 in
//! five places.  Until S10 settles them with the census this slice produces
//! ([`crate::census`]), a binder that would need one of them is **refused**, naming the item:
//!
//! | 16.2 item | descriptor | refusal |
//! |---|---|---|
//! | 1, 2 | a `List` whose element domain is a byte string | `domain` |
//! | 3 | `NatCount` (the 73-value shift-count domain, not `0..=K`) | `domain` |
//! | 4 | a signed `fx_edge` (`I32`, `I64`: sorted, plus `-1..=-255`) | `domain` |
//! | 5 | `FxEdge` at 16 bits (a demoted `U16`/`I16`) | `domain` |
//!
//! (`ct-compare` hits none: its binders are `Bytes`, `Bytes<N>`, `U64` and `Bool`.)  A
//! composite (tuple, record, variant, option, list) is `ir.data`: refused naming the level.
//! `Domain` is `#[non_exhaustive]`, so the last arm of every match here refuses.
//!
//! ## Value caches
//!
//! Enumeration needs the value of binder `j` at index `i` once per case.  `Domain::nth`
//! rebuilds its leaf tables per call (documented in `domain.rs`), so a domain of at most
//! [`MAX_CACHED_VALUES`] values is materialised once through `Domain::iter` and its stream
//! encodings are cached beside it; a larger one falls back to `nth`.

use coding_adventures_warrant_grammar::codegen::CodegenError;
use coding_adventures_warrant_grammar::domain::{resolve_binders, Domain, DomainEnv};
use coding_adventures_warrant_grammar::parser::{Declaration, Expression, WarrantFile};

use crate::error::{code, Beyond, GenError};
use crate::json::Json;
use crate::lower::LoweredSuite;
use crate::stream::encode;
use crate::types::Ty;
use crate::value::Value;

/// A domain of at most this many values is materialised once.
pub const MAX_CACHED_VALUES: u64 = 1 << 20;
/// The descriptor depth limit of VER04r 5.1 and 14.
pub const MAX_DESCRIPTOR_DEPTH: usize = 16;
/// Binders per claim (VER04r 5.3).
pub const MAX_BINDERS: usize = 8;
/// Product ordinals per suite (`MAX_SUITE_PRODUCT`, VER04r 14).
pub const MAX_SUITE_PRODUCT: u64 = 1 << 28;

/// The honesty label of a domain (VER04r 9.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Label {
    /// Every value of the type.
    Full,
    /// A fixed, documented subset (edge values and so on).
    BoundedSet,
    /// A sample (`Bytes<N>` for `N >= 2`).
    Sample,
}

impl Label {
    /// The label written in a corpus and in the oracle's rows.
    pub fn name(self) -> &'static str {
        match self {
            Label::Full => "full",
            Label::BoundedSet => "bounded-set",
            Label::Sample => "sample",
        }
    }
}

/// One binder, resolved.
#[derive(Debug, Clone)]
pub struct BinderDomain {
    /// The binder's name.
    pub name: String,
    /// Its declared type.
    pub ty: Ty,
    /// The resolved domain (S2 API).
    pub domain: Domain,
    /// `domain.domain_size()`, always present for a resolved binder.
    pub size: u64,
    /// The descriptor tree as JSON (every node has `size`).
    pub descriptor: Json,
    /// The honesty label.
    pub label: Label,
    values: Option<Vec<Value>>,
    encoded: Option<Vec<Vec<u8>>>,
}

impl BinderDomain {
    /// Resolve one binder from its domain: describe it, check it against its declared type
    /// and cache its values when it is small.
    ///
    /// # Errors
    ///
    /// The refusals of the module documentation; `W0706` for a size that does not fit a
    /// `u64`; `limit` past 16 descriptor levels; `W0701` if a value does not fit the
    /// declared type (a domain and a type that disagree).
    pub fn new(name: &str, ty: &Ty, domain: Domain) -> Result<BinderDomain, GenError> {
        let (descriptor, label, size) = describe(&domain, 1).map_err(|e| e.at(name))?;
        let (values, encoded) = if size <= MAX_CACHED_VALUES {
            let mut vs = Vec::with_capacity(size as usize);
            let mut es = Vec::with_capacity(size as usize);
            for ev in domain.iter() {
                let v = Value::from_eval(&ev).map_err(|e| e.at(name))?;
                let mut buf = Vec::new();
                encode(ty, &v, &mut buf).map_err(|e| e.at(name))?;
                vs.push(v);
                es.push(buf);
            }
            (Some(vs), Some(es))
        } else {
            (None, None)
        };
        Ok(BinderDomain {
            name: name.to_string(),
            ty: ty.clone(),
            domain,
            size,
            descriptor,
            label,
            values,
            encoded,
        })
    }

    /// The value at index `i` (`i < size`).
    ///
    /// # Errors
    ///
    /// `W0701` for an index past the size (a bug in the caller).
    pub fn value(&self, i: u64) -> Result<Value, GenError> {
        if let Some(vs) = &self.values {
            return vs.get(i as usize).cloned().ok_or_else(|| {
                GenError::new(code::W0701, &self.name, "a domain index is out of range")
            });
        }
        let ev = self.domain.nth(i).ok_or_else(|| {
            GenError::new(code::W0701, &self.name, "a domain index is out of range")
        })?;
        Value::from_eval(&ev).map_err(|e| e.at(&self.name))
    }

    /// Append the stream encoding of the value at index `i` to `out`.
    ///
    /// # Errors
    ///
    /// As [`BinderDomain::value`] and [`encode`].
    pub fn encode_into(&self, i: u64, out: &mut Vec<u8>) -> Result<(), GenError> {
        if let Some(es) = &self.encoded {
            let e = es.get(i as usize).ok_or_else(|| {
                GenError::new(code::W0701, &self.name, "a domain index is out of range")
            })?;
            out.extend_from_slice(e);
            return Ok(());
        }
        let v = self.value(i)?;
        encode(&self.ty, &v, out).map_err(|e| e.at(&self.name))
    }

    /// Swap two cached values (and encodings): the seeded-wrong "reordered enumeration"
    /// control.  `#[doc(hidden)]`; the values are only cached for small domains.
    #[doc(hidden)]
    pub fn sabotage_swap(&mut self, i: usize, j: usize) {
        if let (Some(v), Some(e)) = (self.values.as_mut(), self.encoded.as_mut()) {
            v.swap(i, j);
            e.swap(i, j);
        }
    }
}

/// The binders of one suite and their product.
#[derive(Debug, Clone)]
pub struct SuiteDomains {
    /// The binders in order.
    pub binders: Vec<BinderDomain>,
    /// `P`, the number of product ordinals.
    pub product: u64,
}

fn refuse(item: &str, what: &str) -> GenError {
    GenError::new(
        code::DOMAIN,
        "",
        format!("{what}: VER04r 16.2 item {item} is undecided; S10 settles it with the S3b census"),
    )
}

fn weakest(a: Label, b: Label) -> Label {
    a.max(b)
}

fn leaf(name: &str, size: u64, extra: Option<(&str, Json)>) -> Json {
    let mut row = vec![
        ("d".to_string(), Json::str(name)),
        ("size".to_string(), Json::Int(size)),
    ];
    if let Some((k, v)) = extra {
        row.push((k.to_string(), v));
    }
    Json::Obj(row)
}

/// Describe a domain: its descriptor JSON, label and size.
///
/// # Errors
///
/// See the module documentation.
pub fn describe(d: &Domain, depth: usize) -> Result<(Json, Label, u64), GenError> {
    if depth > MAX_DESCRIPTOR_DEPTH {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "a descriptor tree is deeper than 16 levels (VER04r 5.1)",
        ));
    }
    let size = d.domain_size().ok_or_else(|| {
        GenError::new(
            code::W0706,
            "",
            "a domain size does not fit 64 bits (never silently zero)",
        )
    })?;
    use coding_adventures_warrant_grammar::fixed::FixedTy;
    Ok(match d {
        Domain::Bool => (leaf("bool", size, None), Label::Full, size),
        Domain::U8 | Domain::Fx(FixedTy::U8) => (leaf("u8_all", size, None), Label::Full, size),
        Domain::U64 | Domain::Fx(FixedTy::U64) => {
            (leaf("u64_edge", size, None), Label::BoundedSet, size)
        }
        Domain::Nat => (leaf("nat_0_64", size, None), Label::BoundedSet, size),
        Domain::Bytes => (leaf("bytes_d1", size, None), Label::BoundedSet, size),
        Domain::BytesN(n) => (
            leaf("bytes_n", size, Some(("n", Json::Int(*n)))),
            if *n <= 1 { Label::Full } else { Label::Sample },
            size,
        ),
        Domain::Fx(f) if f.bits() <= 16 => (
            leaf("fx_full", size, Some(("t", Json::str(f.name())))),
            Label::Full,
            size,
        ),
        Domain::Fx(f) if f.signed() => {
            return Err(refuse("4", "a signed fx_edge (I32, I64)"));
        }
        Domain::Fx(f) => (
            leaf("fx_edge", size, Some(("t", Json::str(f.name())))),
            Label::BoundedSet,
            size,
        ),
        Domain::FxEdge(_) => return Err(refuse("5", "a 16-bit fx_edge (a demoted U16 or I16)")),
        Domain::NatCount => return Err(refuse("3", "nat_count (the shift-count domain)")),
        Domain::Res(ok, errs, _) => {
            let (inner, label, _) = describe(ok, depth + 1)?;
            (
                Json::obj([
                    ("d", Json::str("result")),
                    ("ok", inner),
                    ("errs", Json::strs(errs.iter())),
                    ("size", Json::Int(size)),
                ]),
                label,
                size,
            )
        }
        Domain::List(inner) if is_byte_string(inner) => {
            return Err(refuse("1/2", "a list of byte strings"));
        }
        Domain::Tuple(_) | Domain::Data { .. } | Domain::Opt(_) | Domain::List(_) => {
            return Err(GenError::level(
                Beyond::IrData,
                "",
                "a tuple, record, variant, option or list binder needs ir.data",
            ));
        }
        // `Domain` is non-exhaustive: a kind this generator has not heard of is refused,
        // never skipped.
        _ => {
            return Err(GenError::new(
                code::DOMAIN,
                "",
                "a binder domain kind this generator does not know",
            ))
        }
    })
}

/// A short name for a domain tree, for reports: the descriptor name of a leaf
/// (`bytes_d1`, `u64_edge`, `fx_edge(I64)`), `nat_count` for the shift-count domain,
/// `fx_edge16(U16)` for a demoted 16-bit leaf, and the composite kinds spelled out.
pub fn kind_name(d: &Domain) -> String {
    use coding_adventures_warrant_grammar::fixed::FixedTy;
    match d {
        Domain::Bool => "bool".into(),
        Domain::U8 | Domain::Fx(FixedTy::U8) => "u8_all".into(),
        Domain::U64 | Domain::Fx(FixedTy::U64) => "u64_edge".into(),
        Domain::Nat => "nat_0_64".into(),
        Domain::NatCount => "nat_count".into(),
        Domain::Bytes => "bytes_d1".into(),
        Domain::BytesN(n) => format!("bytes_n({n})"),
        Domain::Fx(f) if f.bits() <= 16 => format!("fx_full({})", f.name()),
        Domain::Fx(f) => format!("fx_edge({})", f.name()),
        Domain::FxEdge(f) => format!("fx_edge16({})", f.name()),
        Domain::Tuple(cs) => format!(
            "tuple({})",
            cs.iter().map(kind_name).collect::<Vec<_>>().join(", ")
        ),
        Domain::Data { name, record, .. } => {
            format!("{}({name})", if *record { "record" } else { "variant" })
        }
        Domain::Opt(t) => format!("option({})", kind_name(t)),
        Domain::Res(t, _, _) => format!("result({})", kind_name(t)),
        Domain::List(t) => format!("list({})", kind_name(t)),
        _ => "unknown".into(),
    }
}

fn is_byte_string(d: &Domain) -> bool {
    matches!(d, Domain::Bytes | Domain::BytesN(_))
}

/// The VER04r 16.2 items a domain tree touches, for the census: `"1/2"` (list of byte
/// strings), `"3"` (`nat_count`), `"4"` (signed `fx_edge`), `"5"` (`FxEdge` at 16 bits).
/// Independent of whether the domain is otherwise `ir.core`.
pub fn s162_items(d: &Domain) -> Vec<&'static str> {
    let mut out = Vec::new();
    fn walk(d: &Domain, out: &mut Vec<&'static str>, depth: usize) {
        if depth > 64 {
            return;
        }
        let mut add = |s: &'static str| {
            if !out.contains(&s) {
                out.push(s);
            }
        };
        match d {
            Domain::NatCount => add("3"),
            Domain::FxEdge(_) => add("5"),
            Domain::Fx(f) if f.bits() > 16 && f.signed() => add("4"),
            Domain::List(inner) => {
                if is_byte_string(inner) {
                    add("1/2");
                }
                walk(inner, out, depth + 1);
            }
            Domain::Tuple(cs) => cs.iter().for_each(|c| walk(c, out, depth + 1)),
            Domain::Data { ctors, .. } => ctors
                .iter()
                .flat_map(|(_, fs)| fs.iter())
                .for_each(|c| walk(c, out, depth + 1)),
            Domain::Opt(t) | Domain::Res(t, _, _) => walk(t, out, depth + 1),
            _ => {}
        }
    }
    walk(d, &mut out, 0);
    out
}

fn strip<'e, 'a>(mut e: &'e Expression<'a>) -> &'e Expression<'a> {
    while let Expression::Grouped { inner, .. } = e {
        e = inner;
    }
    e
}

/// Map the S2 resolver's error to this crate's: its code (`W0706`, `W0707`, otherwise
/// `W0701`) and one of our own sentences.
pub fn map_codegen(e: &CodegenError, at: &str) -> GenError {
    let c = match e.code.to_string().as_str() {
        "W0706" => code::W0706,
        "W0707" => code::W0707,
        _ => code::W0701,
    };
    GenError::new(
        c,
        at,
        "the binder domains cannot be resolved (the product or a width is over budget, or a type has no enumeration)",
    )
}

/// A claim's binders, its body past the quantifier, and its span.
type ClaimParts<'f, 'a> = (
    Vec<&'f coding_adventures_warrant_grammar::parser::Parameter<'a>>,
    Option<&'f Expression<'a>>,
    (usize, usize),
);

/// The binder parameters and body of the claim named `claim`, if the document declares it.
fn claim_parts<'f, 'a>(file: &'f WarrantFile<'a>, claim: &str) -> Option<ClaimParts<'f, 'a>> {
    let decl = file.declarations.iter().find_map(|d| match d {
        Declaration::Claim(c) if c.name == claim => Some(c),
        _ => None,
    })?;
    let body = strip(&decl.body);
    Some(match body {
        Expression::Quantified { binders, body, .. } => {
            (binders.iter().collect(), Some(strip(body)), decl.span)
        }
        other => (Vec::new(), Some(other), decl.span),
    })
}

/// The S2-resolved domains of the claim named `claim` and the product, **without** checking
/// them against this generator (no descriptors, no 16.2 refusals): what the census reads.
///
/// # Errors
///
/// The S2 resolver's `W0706`/`W0707`/`W0701`; `W0701` if the claim is not declared.
pub fn claim_domains(
    file: &WarrantFile<'_>,
    env: &DomainEnv<'_>,
    claim: &str,
    budget: u64,
) -> Result<(Vec<Domain>, u64), GenError> {
    let (params, body, span) = claim_parts(file, claim)
        .ok_or_else(|| GenError::new(code::W0701, claim, "the claim is not in the document"))?;
    let owned: Vec<_> = params.into_iter().cloned().collect();
    let r = resolve_binders(env, &owned, body, budget, claim, span)
        .map_err(|e| map_codegen(&e, claim))?;
    Ok((r.domains, r.product))
}

/// Resolve the binder domains of every claim named `claim` in `file` against `budget` (the
/// S2 [`resolve_binders`], then this module's descriptors).
///
/// # Errors
///
/// The S2 resolver's `W0706`/`W0707`/`W0701`, the 16.2 refusals, `limit` for more than 8
/// binders or a product past `2^28`.
pub fn resolve_claim(
    file: &WarrantFile<'_>,
    env: &DomainEnv<'_>,
    suite: &LoweredSuite,
    budget: u64,
) -> Result<SuiteDomains, GenError> {
    let at = |e: GenError| e.at(&suite.claim);
    let (params, body, span) = claim_parts(file, &suite.claim).ok_or_else(|| {
        GenError::new(
            code::W0701,
            &suite.claim,
            "the claim is not in the document",
        )
    })?;
    if params.len() > MAX_BINDERS || suite.binders.len() > MAX_BINDERS {
        return Err(at(GenError::new(
            code::LIMIT,
            "",
            "a claim has more than 8 binders (VER04r 5.3)",
        )));
    }
    let owned: Vec<_> = params.into_iter().cloned().collect();
    let resolved = resolve_binders(env, &owned, body, budget, &suite.claim, span)
        .map_err(|e| map_codegen(&e, &suite.claim))?;
    if resolved.domains.len() != suite.binders.len() {
        return Err(at(GenError::new(
            code::W0701,
            "",
            "the resolved binders and the lowered binders disagree",
        )));
    }
    if resolved.product > MAX_SUITE_PRODUCT {
        return Err(at(GenError::new(
            code::LIMIT,
            "",
            "a suite has more than 2^28 product ordinals (VER04r 14)",
        )));
    }
    let mut binders = Vec::with_capacity(owned.len());
    for ((name, ty), d) in suite.binders.iter().zip(resolved.domains) {
        // Name the claim, not the binder: every refusal of a suite is attributed the same way.
        binders.push(BinderDomain::new(name, ty, d).map_err(|mut e| {
            e.at = suite.claim.clone();
            e
        })?);
    }
    Ok(SuiteDomains {
        binders,
        product: resolved.product,
    })
}

/// The weakest label over a suite's binders (`full` for none).
pub fn suite_label(binders: &[BinderDomain]) -> Label {
    binders.iter().fold(Label::Full, |a, b| weakest(a, b.label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    fn d(domain: Domain) -> (String, Label, u64) {
        let (json, label, size) = describe(&domain, 1).unwrap();
        (json.render().unwrap(), label, size)
    }

    #[test]
    fn leaf_descriptors_follow_the_oracle_names_labels_and_sizes() {
        let (j, l, n) = d(Domain::Bool);
        assert_eq!((l, n), (Label::Full, 2));
        assert!(j.contains("\"d\": \"bool\"") && j.contains("\"size\": 2"));
        assert_eq!(d(Domain::U8).1, Label::Full);
        assert_eq!(d(Domain::U8).2, 256);
        let (j, l, n) = d(Domain::U64);
        assert_eq!((l, n), (Label::BoundedSet, 377));
        assert!(j.contains("u64_edge"));
        assert_eq!(d(Domain::Nat), (d(Domain::Nat).0, Label::BoundedSet, 65));
        assert!(d(Domain::Nat).0.contains("nat_0_64"));
        let (j, l, n) = d(Domain::Bytes);
        assert_eq!((l, n), (Label::BoundedSet, 347));
        assert!(j.contains("bytes_d1"));
        // Bytes<N>: full for N <= 1, sample from 2 (VER04r 9.3)
        for (n, label, size) in [
            (0u64, Label::Full, 1u64),
            (1, Label::Full, 256),
            (2, Label::Sample, 16),
            (4, Label::Sample, 16),
            (16, Label::Sample, 52),
        ] {
            let (j, l, s) = d(Domain::BytesN(n));
            assert_eq!((l, s), (label, size), "Bytes<{n}>");
            assert!(
                j.contains("bytes_n") && j.contains(&format!("\"n\": {n}")),
                "{j}"
            );
        }
    }

    #[test]
    fn v1_u8_and_u64_binders_keep_the_oracle_names() {
        // a v1 document resolves U8/U64 binders to Fx(U8)/Fx(U64): the same value sets, and
        // the oracle names them u8_all/u64_edge whatever the document version
        assert_eq!(d(Domain::Fx(FixedTy::U8)).0, d(Domain::U8).0);
        assert_eq!(d(Domain::Fx(FixedTy::U64)).0, d(Domain::U64).0);
        assert_eq!(d(Domain::Fx(FixedTy::U8)).2, 256);
        assert_eq!(d(Domain::Fx(FixedTy::U64)).2, 377);
    }

    #[test]
    fn narrow_fixed_widths_are_fx_full_and_unsigned_32_is_fx_edge() {
        let (j, l, n) = d(Domain::Fx(FixedTy::I8));
        assert_eq!((l, n), (Label::Full, 256));
        assert!(j.contains("fx_full") && j.contains("\"t\": \"I8\""));
        let (_, l, n) = d(Domain::Fx(FixedTy::U16));
        assert_eq!((l, n), (Label::Full, 65_536));
        assert_eq!(d(Domain::Fx(FixedTy::I16)).2, 65_536);
        let (j, l, n) = d(Domain::Fx(FixedTy::U32));
        assert_eq!((l, n), (Label::BoundedSet, 313));
        assert!(j.contains("fx_edge") && j.contains("U32"));
    }

    #[test]
    fn a_result_descriptor_nests_and_takes_the_label_of_its_ok_domain() {
        let r = Domain::Res(
            Box::new(Domain::Bytes),
            vec!["A".into(), "B".into()],
            coding_adventures_warrant_grammar::domain::ErrName("E".into()),
        );
        let (j, l, n) = d(r);
        assert_eq!((l, n), (Label::BoundedSet, 349));
        assert!(
            j.contains("\"d\": \"result\"") && j.contains("\"errs\"") && j.contains("bytes_d1")
        );
    }

    #[test]
    fn the_16_2_descriptors_are_refused_naming_the_item() {
        let cases = [
            (Domain::NatCount, "item 3"),
            (Domain::Fx(FixedTy::I64), "item 4"),
            (Domain::Fx(FixedTy::I32), "item 4"),
            (Domain::FxEdge(FixedTy::U16), "item 5"),
            (Domain::FxEdge(FixedTy::I16), "item 5"),
            (Domain::List(Box::new(Domain::Bytes)), "item 1/2"),
            (Domain::List(Box::new(Domain::BytesN(2))), "item 1/2"),
        ];
        for (dom, item) in cases {
            let e = describe(&dom, 1).unwrap_err();
            assert_eq!(e.code, code::DOMAIN, "{dom:?}");
            assert!(e.message.contains(item), "{dom:?}: {}", e.message);
        }
    }

    #[test]
    fn composites_are_ir_data_and_a_deep_tree_is_a_limit_error() {
        for dom in [
            Domain::Tuple(vec![Domain::Bool]),
            Domain::Opt(Box::new(Domain::Bool)),
            Domain::List(Box::new(Domain::Bool)),
        ] {
            assert_eq!(describe(&dom, 1).unwrap_err().level, Some("ir.data"));
        }
        // 17 nested results
        let mut deep = Domain::Bool;
        for _ in 0..16 {
            deep = Domain::Res(
                Box::new(deep),
                vec!["X".into()],
                coding_adventures_warrant_grammar::domain::ErrName("E".into()),
            );
        }
        // 16 results + the leaf = 17 levels
        assert_eq!(describe(&deep, 1).unwrap_err().code, code::LIMIT);
        let mut ok = Domain::Bool;
        for _ in 0..15 {
            ok = Domain::Res(
                Box::new(ok),
                vec!["X".into()],
                coding_adventures_warrant_grammar::domain::ErrName("E".into()),
            );
        }
        assert!(describe(&ok, 1).is_ok());
    }

    #[test]
    fn the_16_2_scan_finds_items_inside_composites_without_refusing() {
        assert_eq!(s162_items(&Domain::NatCount), ["3"]);
        assert_eq!(s162_items(&Domain::Bool), Vec::<&str>::new());
        let nested = Domain::Tuple(vec![
            Domain::Fx(FixedTy::I64),
            Domain::Opt(Box::new(Domain::List(Box::new(Domain::Bytes)))),
            Domain::FxEdge(FixedTy::U16),
            Domain::NatCount,
        ]);
        let mut items = s162_items(&nested);
        items.sort_unstable();
        assert_eq!(items, ["1/2", "3", "4", "5"]);
        // a fine unsigned wide leaf and a v1 U64 are not on the list
        assert!(s162_items(&Domain::Fx(FixedTy::U32)).is_empty());
        assert!(s162_items(&Domain::Fx(FixedTy::U64)).is_empty());
        assert!(s162_items(&Domain::Fx(FixedTy::I8)).is_empty());
        // a list of Nat is not a list of byte strings
        assert!(s162_items(&Domain::List(Box::new(Domain::Nat))).is_empty());
    }

    #[test]
    fn a_binder_caches_values_and_encodings_that_agree_with_nth() {
        let b = BinderDomain::new("a", &Ty::bytes_n(1), Domain::BytesN(1)).unwrap();
        assert_eq!(b.size, 256);
        assert_eq!(b.value(7).unwrap(), Value::bytes(&[7]));
        let mut out = Vec::new();
        b.encode_into(7, &mut out).unwrap();
        assert_eq!(out, [0x20, 1, 0, 0, 0, 7]);
        // out of range
        assert_eq!(b.value(256).unwrap_err().code, code::W0701);
        assert!(b.encode_into(256, &mut Vec::new()).is_err());
        // a binder over the cache limit falls back to `nth`: Fx(U16) is 65,536 values (below
        // the limit); a hand-built Data-free big domain is not available at ir.core, so the
        // uncached path is exercised through a one-off binder with caching disabled
        let mut big =
            BinderDomain::new("x", &Ty::Fixed(FixedTy::U16), Domain::Fx(FixedTy::U16)).unwrap();
        assert_eq!(big.value(65_535).unwrap(), Value::Int(65_535));
        big.values = None;
        big.encoded = None;
        assert_eq!(big.value(300).unwrap(), Value::Int(300));
        let mut out = Vec::new();
        big.encode_into(300, &mut out).unwrap();
        assert_eq!(out, [0x10, 0x2c, 0x01, 0, 0, 0, 0, 0, 0]);
        assert!(big.value(65_536).is_err());
        assert!(big.encode_into(65_536, &mut Vec::new()).is_err());
    }

    #[test]
    fn a_domain_that_disagrees_with_its_declared_type_is_refused() {
        // a Bool domain for a Bytes binder: the value does not fit its type
        let e = BinderDomain::new("a", &Ty::BYTES, Domain::Bool).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.at, "a");
    }

    #[test]
    fn the_swap_sabotage_reorders_values_and_encodings_together() {
        let mut b = BinderDomain::new("a", &Ty::bytes_n(1), Domain::BytesN(1)).unwrap();
        b.sabotage_swap(0, 1);
        assert_eq!(b.value(0).unwrap(), Value::bytes(&[1]));
        let mut out = Vec::new();
        b.encode_into(1, &mut out).unwrap();
        assert_eq!(out, [0x20, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn labels_order_full_below_bounded_below_sample() {
        assert!(Label::Full < Label::BoundedSet && Label::BoundedSet < Label::Sample);
        assert_eq!(Label::Sample.name(), "sample");
        assert_eq!(Label::BoundedSet.name(), "bounded-set");
        assert_eq!(Label::Full.name(), "full");
        let a = BinderDomain::new("a", &Ty::Bool, Domain::Bool).unwrap();
        let b = BinderDomain::new("b", &Ty::bytes_n(4), Domain::BytesN(4)).unwrap();
        assert_eq!(suite_label(std::slice::from_ref(&a)), Label::Full);
        assert_eq!(suite_label(&[a, b]), Label::Sample);
        assert_eq!(suite_label(&[]), Label::Full);
    }
}
