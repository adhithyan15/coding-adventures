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
//!   Nat that is a shift count       {"d":"nat_shift"}              bounded-set   73
//!   8/16-bit fixed (not U8)         {"d":"fx_full","t":T}          full
//!   16/32/64-bit fixed (not U64)    {"d":"fx_edge","t":T}          bounded-set  281 313 527 559 623
//!   Result<T,E>                     {"d":"result","ok":D,"errs":[..]}   label of D
//!   (T1,..,Tn)                      {"d":"tuple","of":[D1,..,Dn]}       weakest label of the Di
//! ```
//!
//! A tuple domain is the Cartesian product of its components, the **last** component varying
//! fastest (`ir.tuple`, S10t-2); its `size` is the product, checked.
//!
//! Every node also records its `size`.  `Bytes<N>` for `N >= 2` is `sample` (VER04r 9.3).
//!
//! ## What is refused, and why (VER04r 16.2, decided in S10c, [D21] [D25] [D26])
//!
//! The S2 API reported what the *generator* enumerates, which differed from VER04r 5.1 in five
//! places; S3 refused a binder that needed one of them, naming the item, until S10 settled them
//! with the census.  S10c-1 (the oracle) and S10c-2 (this generator) settle three:
//!
//! | 16.2 item | descriptor | decision |
//! |---|---|---|
//! | 1, 2 | a `List` whose element domain is a byte string | still refused (`domain`): `ir.data` |
//! | 3 | the 73-value shift-count domain | **`{"d":"nat_shift","size":73}`** [D26] (a leaf, no `k`) |
//! | 4 | a signed `fx_edge` (`I32`, `I64`: sorted, plus `-1..=-255`) | **`fx_edge`** [D21]: `I16` 527, `I32` 559, `I64` 623 |
//! | 5 | `FxEdge` at 16 bits (a demoted `U16`/`I16`) | **`fx_edge`** [D25]: by demotion only, 281 / 527 |
//!
//! `nat_shift` is chosen by the S2 resolver for a `Nat` binder that is *directly* the count of
//! `shl shr sar rotl rotr bit` in the claim body (R10: parentheses do not matter, an expression of
//! the binder does not select, and an `admit` never does).  An 8-bit `fx_edge` has no descriptor
//! [D25]: the resolver never demotes an 8-bit binder, and [`describe`] refuses a hand-built one.
//! A composite (record, variant, option, list) is `ir.data`: refused naming the level (a tuple is
//! `ir.tuple` and is described).  `Domain` is `#[non_exhaustive]`, so the last arm of every match
//! here refuses.
//!
//! [`descriptor_size`] is the **consumer-side** check of VER04r 5.1: it recomputes the `size` of a
//! descriptor tree in checked arithmetic and refuses a malformed, mis-sized or illegal leaf with the
//! oracle's own message (`tests/fixed_eval_parity.rs` reproduces the oracle's verdict table).
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
use crate::stream::encode_in;
use crate::types::{Decls, Ty};
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
    decls: Decls,
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
        BinderDomain::new_in(&Decls::default(), name, ty, domain)
    }

    /// [`BinderDomain::new`] for a document that declares records and variants (`decls`).
    ///
    /// # Errors
    ///
    /// As [`BinderDomain::new`].
    pub fn new_in(
        decls: &Decls,
        name: &str,
        ty: &Ty,
        domain: Domain,
    ) -> Result<BinderDomain, GenError> {
        let (descriptor, label, size) = describe(&domain, 1).map_err(|e| e.at(name))?;
        let (values, encoded) = if size <= MAX_CACHED_VALUES {
            let mut vs = Vec::with_capacity(size as usize);
            let mut es = Vec::with_capacity(size as usize);
            for ev in domain.iter() {
                let v = Value::from_eval(&ev).map_err(|e| e.at(name))?;
                let mut buf = Vec::new();
                encode_in(decls, ty, &v, &mut buf).map_err(|e| e.at(name))?;
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
            decls: decls.clone(),
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
        encode_in(&self.decls, &self.ty, &v, out).map_err(|e| e.at(&self.name))
    }

    /// Swap two cached values (and encodings): the seeded-wrong "reordered enumeration"
    /// control.  Only with the `test-hooks` feature; the values are only cached for small
    /// domains, and an index out of range changes nothing.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn sabotage_swap(&mut self, i: usize, j: usize) {
        if let (Some(v), Some(e)) = (self.values.as_mut(), self.encoded.as_mut()) {
            if i < v.len() && j < v.len() {
                v.swap(i, j);
                e.swap(i, j);
            }
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
    /// The demotions the budget applied, in order, as the oracle writes them (see
    /// [`demotion_path`]).
    pub demoted: Vec<String>,
}

fn refuse(item: &str, what: &str) -> GenError {
    GenError::new(
        code::DOMAIN,
        "",
        format!("{what}: VER04r 16.2 item {item} is undecided; S11 (ir.data) settles it"),
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
        // [D21] a wide leaf, signed or not; [D25] a demoted 16-bit leaf, which is the same
        // descriptor.  An 8-bit `fx_edge` has none (the resolver never makes one).
        Domain::Fx(f) | Domain::FxEdge(f) if f.bits() >= 16 && *f != FixedTy::U64 => (
            leaf("fx_edge", size, Some(("t", Json::str(f.name())))),
            Label::BoundedSet,
            size,
        ),
        Domain::FxEdge(_) => {
            return Err(GenError::new(
                code::DOMAIN,
                "",
                "fx_edge is for 16-, 32- and 64-bit types other than U64 (D25)",
            ))
        }
        // [D26] the shift-count domain: a leaf of 73 values, no `k`.
        Domain::NatCount => (leaf("nat_shift", size, None), Label::BoundedSet, size),
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
        Domain::Tuple(parts) => {
            let mut described = Vec::with_capacity(parts.len());
            let mut label = Label::Full;
            for part in parts {
                let (j, l, _) = describe(part, depth + 1)?;
                label = weakest(label, l);
                described.push(j);
            }
            (
                Json::obj([
                    ("d", Json::str("tuple")),
                    ("of", Json::arr(described)),
                    ("size", Json::Int(size)),
                ]),
                label,
                size,
            )
        }
        // `ir.data` (S11c-2): an `Option` is `{"d":"option","of":D}` (its label that of D); a record is the
        // `tuple` of its fields; a variant is `{"d":"variant","ctors":[[name,[D..]],..]}` (the weakest label
        // of every field of every constructor, `full` when no constructor has a field).
        Domain::Opt(inner) => {
            let (j, label, _) = describe(inner, depth + 1)?;
            (
                Json::obj([
                    ("d", Json::str("option")),
                    ("of", j),
                    ("size", Json::Int(size)),
                ]),
                label,
                size,
            )
        }
        Domain::Data { ctors, record, .. } => {
            let mut label = Label::Full;
            let mut described: Vec<(&str, Vec<Json>)> = Vec::with_capacity(ctors.len());
            for (name, fields) in ctors {
                let mut parts = Vec::with_capacity(fields.len());
                for f in fields {
                    let (j, l, _) = describe(f, depth + 1)?;
                    label = weakest(label, l);
                    parts.push(j);
                }
                described.push((name.as_str(), parts));
            }
            if *record {
                let (_, parts) = described.pop().unwrap_or_default();
                (
                    Json::obj([
                        ("d", Json::str("tuple")),
                        ("of", Json::arr(parts)),
                        ("size", Json::Int(size)),
                    ]),
                    label,
                    size,
                )
            } else {
                (
                    Json::obj([
                        ("d", Json::str("variant")),
                        (
                            "ctors",
                            Json::arr(described.into_iter().map(|(n, parts)| {
                                Json::arr([Json::str(n), Json::arr(parts)])
                            })),
                        ),
                        ("size", Json::Int(size)),
                    ]),
                    label,
                    size,
                )
            }
        }
        Domain::List(_) => {
            return Err(GenError::level(
                Beyond::IrList,
                "",
                "a list binder is the reserved level ir.list",
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

/// Why a descriptor tree is not acceptable to a consumer: the oracle's `generation` (a malformed,
/// mis-sized or illegal descriptor, `W0706` for a size past 64 bits) or `limit` (too deep).  The
/// message is the oracle's own words, so the verdict table of `fixed-eval-v1` compares byte for byte.
fn bad(message: &str) -> GenError {
    GenError::new(code::DOMAIN, "", message)
}

/// Recompute the `size` of a descriptor tree and check it **as a consumer must** (VER04r 5.1).
///
/// Every node's declared `size` must equal the recomputed one (in checked arithmetic: a size of
/// 2^64 or more is refused, as `W0706` would be), the tree is at most 16 levels deep, a node has
/// exactly the keys its kind allows, and the leaves are legal: `fx_full` is for 8- and 16-bit types
/// other than `U8`; `fx_edge` for 16-, 32- and 64-bit types other than `U64`, **never 8-bit**
/// [D25]; `nat_shift` has exactly 73 values [D26].  Returns the size.
///
/// ```
/// use coding_adventures_warrant_corpus_gen::domains::descriptor_size;
/// use coding_adventures_warrant_corpus_gen::json::Json;
///
/// let leaf = Json::obj([("d", Json::str("nat_shift")), ("size", Json::Int(73))]);
/// assert_eq!(descriptor_size(&leaf, 1).unwrap(), 73);
/// let wrong = Json::obj([("d", Json::str("nat_shift")), ("size", Json::Int(72))]);
/// assert!(descriptor_size(&wrong, 1).is_err());
/// ```
///
/// # Errors
///
/// `domain` for everything above except depth, which is `limit`.
pub fn descriptor_size(d: &Json, depth: usize) -> Result<u64, GenError> {
    use coding_adventures_warrant_grammar::fixed::{edge_domain, FixedTy};
    if depth > MAX_DESCRIPTOR_DEPTH {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "descriptor tree deeper than 16 levels",
        ));
    }
    let malformed = || bad("a malformed descriptor");
    let Json::Obj(pairs) = d else {
        return Err(malformed());
    };
    let get = |k: &str| pairs.iter().find(|(n, _)| n == k).map(|(_, v)| v);
    let (Some(Json::Str(kind)), Some(Json::Int(declared))) = (get("d"), get("size")) else {
        return Err(malformed());
    };
    let need = |keys: &[&str]| {
        let mut want: Vec<&str> = vec!["d", "size"];
        want.extend_from_slice(keys);
        let mut have: Vec<&str> = pairs.iter().map(|(n, _)| n.as_str()).collect();
        want.sort_unstable();
        have.sort_unstable();
        if want == have {
            Ok(())
        } else {
            Err(bad("a descriptor has the wrong keys"))
        }
    };
    let fixed_type = || -> Result<FixedTy, GenError> {
        match get("t") {
            Some(Json::Str(name)) => FixedTy::from_name(name)
                .ok_or_else(|| bad("a descriptor names no fixed-width type")),
            _ => Err(bad("a descriptor names no fixed-width type")),
        }
    };
    let checked = |size: Option<u64>| {
        size.ok_or_else(|| bad("W0706: a descriptor size does not fit 64 bits"))
    };
    let leaf_size = |dom: &Domain| dom.domain_size().ok_or_else(malformed);
    let size: u64 = match kind.as_str() {
        "bool" | "u8_all" | "u64_edge" | "nat_0_64" | "bytes_d1" | "nat_shift" => {
            need(&[])?;
            match kind.as_str() {
                "bool" => 2,
                "u8_all" => 256,
                "u64_edge" => 377,
                "nat_0_64" => 65,
                "bytes_d1" => leaf_size(&Domain::Bytes)?,
                _ => 73,
            }
        }
        "bytes_n" => {
            need(&["n"])?;
            match get("n") {
                Some(Json::Int(n)) if *n <= 4096 => leaf_size(&Domain::BytesN(*n))?,
                _ => return Err(bad("bytes_n needs a width in 0..4096")),
            }
        }
        "nat_count" => {
            need(&["k"])?;
            match get("k") {
                Some(Json::Int(k)) if *k <= 4096 => k + 1,
                _ => return Err(bad("nat_count needs K in 0..4096")),
            }
        }
        "fx_full" => {
            need(&["t"])?;
            let t = fixed_type()?;
            if t == FixedTy::U8 || t.bits() > 16 {
                return Err(bad("fx_full is for 8- and 16-bit types other than U8"));
            }
            1u64 << t.bits()
        }
        "fx_edge" => {
            need(&["t"])?;
            let t = fixed_type()?;
            if t == FixedTy::U64 || t.bits() < 16 {
                return Err(bad(
                    "fx_edge is for 16-, 32- and 64-bit types other than U64 (D25)",
                ));
            }
            edge_domain(t).len() as u64
        }
        "tuple" => {
            need(&["of"])?;
            let Some(Json::Arr(parts)) = get("of").filter(|v| !matches!(v, Json::Arr(a) if a.is_empty())) else {
                return Err(bad("a tuple descriptor has components"));
            };
            let mut size = 1u64;
            for part in parts {
                size = checked(size.checked_mul(descriptor_size(part, depth + 1)?))?;
            }
            size
        }
        "option" => {
            need(&["of"])?;
            let inner = descriptor_size(get("of").ok_or_else(malformed)?, depth + 1)?;
            checked(inner.checked_add(1))?
        }
        "result" => {
            need(&["ok", "errs"])?;
            let errs = match get("errs") {
                Some(Json::Arr(e)) if e.iter().all(|x| matches!(x, Json::Str(_))) => e.len() as u64,
                _ => return Err(bad("a result descriptor lists error cases")),
            };
            let ok = descriptor_size(get("ok").ok_or_else(malformed)?, depth + 1)?;
            checked(ok.checked_add(errs))?
        }
        "list" => {
            need(&["of"])?;
            let inner = descriptor_size(get("of").ok_or_else(malformed)?, depth + 1)?;
            let (d16, d4) = (inner.min(16), inner.min(4));
            checked(
                1u64.checked_add(d16)
                    .and_then(|x| x.checked_add(d16 * d16))
                    .and_then(|x| x.checked_add(d4 * d4 * d4)),
            )?
        }
        "variant" => {
            need(&["ctors"])?;
            let Some(Json::Arr(ctors)) = get("ctors").filter(|v| !matches!(v, Json::Arr(a) if a.is_empty())) else {
                return Err(bad("a variant descriptor has constructors"));
            };
            let mut size = 0u64;
            for ctor in ctors {
                let Json::Arr(pair) = ctor else {
                    return Err(bad("a malformed variant constructor"));
                };
                let [_, Json::Arr(fields)] = pair.as_slice() else {
                    return Err(bad("a malformed variant constructor"));
                };
                let mut product = 1u64;
                for field in fields {
                    product = checked(product.checked_mul(descriptor_size(field, depth + 1)?))?;
                }
                size = checked(size.checked_add(product))?;
            }
            size
        }
        _ => return Err(bad("an unknown descriptor kind")),
    };
    if *declared != size {
        return Err(bad("a descriptor's size is not its recomputed size"));
    }
    Ok(size)
}

/// A short name for a domain tree, for reports: the descriptor name of a leaf
/// (`bytes_d1`, `u64_edge`, `fx_edge(I64)`), `nat_shift` for the shift-count domain,
/// `fx_edge16(U16)` for a demoted 16-bit leaf, and the composite kinds spelled out.
pub fn kind_name(d: &Domain) -> String {
    use coding_adventures_warrant_grammar::fixed::FixedTy;
    match d {
        Domain::Bool => "bool".into(),
        Domain::U8 | Domain::Fx(FixedTy::U8) => "u8_all".into(),
        Domain::U64 | Domain::Fx(FixedTy::U64) => "u64_edge".into(),
        Domain::Nat => "nat_0_64".into(),
        Domain::NatCount => "nat_shift".into(),
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

/// The VER04r 16.2 items a domain tree touches **that are still undecided**, for the census: only
/// `"1/2"` (a list of byte strings, `ir.data`).  Items 3, 4 and 5 are settled since S10c ([D26],
/// [D21], [D25]) and are described, not refused.  Independent of whether the domain is otherwise
/// `ir.core`.
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
            Domain::List(inner) => {
                if is_byte_string(inner) {
                    add("1/2");
                }
                walk(inner, out, depth + 1);
            }
            Domain::Tuple(cs) => cs.iter().for_each(|c| walk(c, out, depth + 1)),
            Domain::Data { ctors, .. } => ctors
                .iter()
                .flat_map(|(_, fields)| fields.iter())
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
    resolve_claim_in(file, env, suite, budget, &Decls::default())
}

/// [`resolve_claim`] for a document that declares records and variants (`decls`: the stream encodings
/// of their values need the field types).
///
/// # Errors
///
/// As [`resolve_claim`].
pub fn resolve_claim_in(
    file: &WarrantFile<'_>,
    env: &DomainEnv<'_>,
    suite: &LoweredSuite,
    budget: u64,
    decls: &Decls,
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
        binders.push(BinderDomain::new_in(decls, name, ty, d).map_err(|mut e| {
            e.at = suite.claim.clone();
            e
        })?);
    }
    let demoted = resolved
        .demotions
        .iter()
        .map(|d| demotion_path(&resolved_domains(&binders), d))
        .collect();
    Ok(SuiteDomains {
        binders,
        product: resolved.product,
        demoted,
    })
}

fn resolved_domains(binders: &[BinderDomain]) -> Vec<&Domain> {
    binders.iter().map(|b| &b.domain).collect()
}

/// The oracle's spelling of one demotion (`domains.demote`, VER04j 2.6): the binder's index as
/// decimal text, then `.k` for component `k` of a tuple and `.ok` for the payload of a `Result`
/// (`.some` for an option, `.elem` for a list: not `ir.core`/`ir.tuple`).  The S2 resolver records
/// the same walk without the `Result` step (`Demotion::path` holds tuple indices only), so it is
/// rebuilt by walking the **resolved** tree of the binder to the demoted leaf.
pub fn demotion_path(domains: &[&Domain], d: &coding_adventures_warrant_grammar::domain::Demotion) -> String {
    let mut out = d.binder.to_string();
    let mut path = d.path.iter();
    let mut cur = domains[d.binder];
    loop {
        match cur {
            Domain::Res(inner, _, _) => {
                out.push_str(".ok");
                cur = inner;
            }
            Domain::Opt(inner) => {
                out.push_str(".some");
                cur = inner;
            }
            Domain::List(inner) => {
                out.push_str(".elem");
                cur = inner;
            }
            // a record is its fields like a tuple (`.k`); a variant names the constructor first (`.Done.1`):
            // the S2 resolver records `[ctor, k]` for a variant and `[k]` for a record (VER04r 5.3 [D54])
            Domain::Data { ctors, record, .. } => {
                let fields = if *record {
                    ctors.first().map(|(_, f)| f)
                } else {
                    let Some(name) = path.next() else { break };
                    out.push('.');
                    out.push_str(name);
                    ctors.iter().find(|(c, _)| c == name).map(|(_, f)| f)
                };
                let Some(fields) = fields else { break };
                let Some(k) = path.next() else { break };
                out.push('.');
                out.push_str(k);
                match k.parse::<usize>().ok().and_then(|k| fields.get(k)) {
                    Some(next) => cur = next,
                    None => break,
                }
            }
            Domain::Tuple(parts) => {
                let Some(k) = path.next() else { break };
                out.push('.');
                out.push_str(k);
                match k.parse::<usize>().ok().and_then(|k| parts.get(k)) {
                    Some(next) => cur = next,
                    None => break,
                }
            }
            _ => break,
        }
    }
    out
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
    fn the_settled_16_2_items_are_described_and_the_undecided_one_is_refused() {
        // [D26] the shift-count leaf: 73 values, no `k`, bounded-set
        let (j, l, n) = d(Domain::NatCount);
        assert_eq!((l, n), (Label::BoundedSet, 73));
        assert_eq!(
            j.split_whitespace().collect::<String>(),
            r#"{"d":"nat_shift","size":73}"#
        );
        // [D21] signed edge sets, [D25] a demoted 16-bit leaf: the same descriptor
        for (dom, ty, size) in [
            (Domain::Fx(FixedTy::I32), "I32", 559),
            (Domain::Fx(FixedTy::I64), "I64", 623),
            (Domain::FxEdge(FixedTy::U16), "U16", 281),
            (Domain::FxEdge(FixedTy::I16), "I16", 527),
        ] {
            let (j, l, n) = d(dom);
            assert_eq!((l, n), (Label::BoundedSet, size), "{ty}");
            assert_eq!(
                j.split_whitespace().collect::<String>(),
                format!(r#"{{"d":"fx_edge","size":{size},"t":"{ty}"}}"#)
            );
        }
        // [D25] an 8-bit fx_edge has no descriptor, and neither has a U64 one (it is u64_edge)
        for dom in [Domain::FxEdge(FixedTy::U8), Domain::FxEdge(FixedTy::I8), Domain::FxEdge(FixedTy::U64)] {
            let e = describe(&dom, 1).unwrap_err();
            assert_eq!(e.code, code::DOMAIN, "{dom:?}");
            assert!(e.message.contains("D25"), "{}", e.message);
        }
        // items 1 and 2 stay undecided until ir.data
        for dom in [
            Domain::List(Box::new(Domain::Bytes)),
            Domain::List(Box::new(Domain::BytesN(2))),
        ] {
            let e = describe(&dom, 1).unwrap_err();
            assert_eq!(e.code, code::DOMAIN, "{dom:?}");
            assert!(e.message.contains("item 1/2"), "{}", e.message);
        }
    }

    #[test]
    fn the_consumer_check_recomputes_sizes_and_rejects_illegal_leaves() {
        let leaf = |d: &str, size: u64, t: Option<&str>| {
            let mut row = vec![("d", Json::str(d)), ("size", Json::Int(size))];
            if let Some(t) = t {
                row.push(("t", Json::str(t)));
            }
            Json::obj(row)
        };
        let ok = |j: Json| descriptor_size(&j, 1).unwrap();
        assert_eq!(ok(leaf("nat_shift", 73, None)), 73);
        assert_eq!(ok(leaf("fx_edge", 527, Some("I16"))), 527);
        assert_eq!(ok(leaf("fx_edge", 281, Some("U16"))), 281);
        assert_eq!(ok(leaf("fx_full", 65_536, Some("I16"))), 65_536);
        let pair = Json::obj([
            ("d", Json::str("tuple")),
            ("of", Json::arr([leaf("nat_shift", 73, None), leaf("bool", 2, None)])),
            ("size", Json::Int(146)),
        ]);
        assert_eq!(ok(pair), 146);
        for (j, message) in [
            (leaf("fx_edge", 256, Some("U8")), "D25"),
            (leaf("fx_edge", 256, Some("I8")), "D25"),
            (leaf("fx_edge", 377, Some("U64")), "D25"),
            (leaf("fx_edge", 377, Some("I16")), "recomputed size"),
            (leaf("fx_edge", 314, Some("U32")), "recomputed size"),
            (leaf("fx_full", 256, Some("U8")), "fx_full is for 8- and 16-bit"),
            (leaf("nat_shift", 72, None), "recomputed size"),
            (leaf("bool", 2, Some("U8")), "wrong keys"),
            (leaf("nat_zzz", 1, None), "unknown descriptor kind"),
            (Json::Int(1), "malformed"),
        ] {
            let e = descriptor_size(&j, 1).unwrap_err();
            assert_eq!(e.code, code::DOMAIN);
            assert!(e.message.contains(message), "{}: {message}", e.message);
        }
        // 17 levels is a limit error, and a size past 2^64 is W0706's, never a wrap
        let mut deep = leaf("bool", 2, None);
        for _ in 0..16 {
            deep = Json::obj([
                ("d", Json::str("option")),
                ("of", deep),
                ("size", Json::Int(3)),
            ]);
        }
        assert_eq!(descriptor_size(&deep, 1).unwrap_err().code, code::LIMIT);
        let big = |size: u64| leaf("u8_all", size, None);
        let mut tuple = Json::obj([
            ("d", Json::str("tuple")),
            ("of", Json::arr(vec![big(256); 9])),
            ("size", Json::Int(0)),
        ]);
        assert!(descriptor_size(&tuple, 1).unwrap_err().message.contains("W0706"));
        if let Json::Obj(p) = &mut tuple {
            p[1].1 = Json::arr(vec![big(256); 3]);
            p[2].1 = Json::Int(256 * 256 * 256);
        }
        assert_eq!(descriptor_size(&tuple, 1).unwrap(), 1 << 24);
    }

    #[test]
    fn a_tuple_is_described_and_other_composites_are_ir_data_and_a_deep_tree_is_a_limit_error() {
        // ir.tuple (S10t-2): the Cartesian product, the weakest label of its components
        let pair = Domain::Tuple(vec![Domain::Bool, Domain::BytesN(2)]);
        let (json, label, size) = describe(&pair, 1).unwrap();
        assert_eq!(
            json.render().unwrap().split_whitespace().collect::<String>(),
            r#"{"d":"tuple","of":[{"d":"bool","size":2},{"d":"bytes_n","n":2,"size":16}],"size":32}"#
        );
        assert_eq!((label, size), (Label::Sample, 32));
        // ir.data (S11c-2): an Option is described (also inside a tuple); a list is the reserved ir.list
        let opt = Domain::Tuple(vec![Domain::Bool, Domain::Opt(Box::new(Domain::Bool))]);
        let (json, label, size) = describe(&opt, 1).unwrap();
        assert_eq!((label, size), (Label::Full, 6));
        assert!(json.render().unwrap().contains(r#""d": "option""#));
        assert_eq!(
            describe(&Domain::List(Box::new(Domain::Bool)), 1).unwrap_err().level,
            Some("ir.list")
        );
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
    fn the_16_2_scan_reports_only_the_undecided_item_inside_composites() {
        // items 3, 4 and 5 are settled (S10c): they are described, so the scan no longer counts them
        assert!(s162_items(&Domain::NatCount).is_empty());
        assert!(s162_items(&Domain::Fx(FixedTy::I64)).is_empty());
        assert!(s162_items(&Domain::FxEdge(FixedTy::U16)).is_empty());
        assert_eq!(s162_items(&Domain::Bool), Vec::<&str>::new());
        let nested = Domain::Tuple(vec![
            Domain::Fx(FixedTy::I64),
            Domain::Opt(Box::new(Domain::List(Box::new(Domain::Bytes)))),
            Domain::FxEdge(FixedTy::U16),
            Domain::NatCount,
        ]);
        assert_eq!(s162_items(&nested), ["1/2"]);
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
