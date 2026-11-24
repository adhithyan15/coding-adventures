//! # Resolved binder domains and the post-state slot resolver (VER04r CV2-S2)
//!
//! This module is the *public* face of two things the evidence generator has
//! always had privately, published so that the v2 corpus emitter
//! (`warrant-corpus-gen`, slice S3) can reuse them instead of re-deriving them
//! -- "the single largest cause of silent drift between a generator and a
//! consumer" (VER04r section 5.1).  It changes no behaviour: the harness
//! generator and this module call the very same functions, and `gen_evidence
//! --check` is the proof for every adopter.
//!
//! ```text
//!   .warrant source ──parse──▶ binder types ──resolve_binders──▶ Domain tree per binder
//!                                              (aliases, records, variants, NatCount,
//!                                               budget demotion, W0706/W0707 checks)
//!                                                         │
//!                              size() / nth(i) / iter() ◀─┘   product_size / unrank_product
//!                                                             ProductIter (row-major)
//!
//!   op with `inout` params ──op_post_slot──▶ PostSlot (S1..S4) + the domains of the posts
//! ```
//!
//! ## What a [`Domain`] is
//!
//! A [`Domain`] is the *fully resolved descriptor tree* of VER04r section 5.1:
//! the finite, ordered set of values one binder ranges over, **after** the
//! budget demotion of VER04j section 2.6.  It is the generator's own type
//! (`evidence::Ty`) under its stable public name.  It answers the two total
//! functions every consumer needs:
//!
//! * [`Domain::domain_size`] -- how many values, with *checked* arithmetic:
//!   `None` means "does not fit in a `u64`" (or a `Bytes<N>` wider than
//!   [`MAX_FIXED_WIDTH`]) and must be treated as over budget, never as zero;
//! * [`Domain::nth`] -- the `i`-th value, `i < size`, without materialising the
//!   rest of the domain.
//!
//! ## Enumeration order (normative: VER04r 5.1 and 5.2)
//!
//! ```text
//!   leaf          the fixed lists of VER04i 2.6 (bool, u8_all, u64_edge, nat_0_64, bytes_d1, ...)
//!   tuple/record  Cartesian product, LAST component varying fastest
//!   variant       concatenation, in declaration order, of each constructor's field product
//!   Option<T>     None, then Some(x) for x in T
//!   Result<T,E>   Ok(x) for x in T, then each error case in declaration order
//!   List<T>       length 0; length 1 and 2 over the first 16 values of T;
//!                 length 3 over the first 4; lexicographic by value index
//!   claim         product of the binders, row-major, LAST binder fastest
//! ```
//!
//! ### Things to know
//!
//! * **Byte-string order.** `Bytes` and `Bytes<N>` list the length-2 and
//!   length-3 strings over `A = [00, 01, 80, ff]` *lexicographically* (first
//!   byte slowest), as VER04i section 2.6 and the v1 corpus compiler do.  The
//!   generated Rust harness's runtime prelude happens to count with the
//!   *first* byte fastest, and that order is frozen by the checked-in generated
//!   files, so it is deliberately not changed here.  For a bare `Bytes` or
//!   `Bytes<N>` binder the two orders enumerate the same *set* (they differ in
//!   which case a failure is reported at).  This module follows the spec.
//! * **Lists take a prefix, so the order change becomes a set change.**
//!   `List<T>` ranges over the first 16 (lengths 1 and 2) and first 4 (length
//!   3) values of `T`.  For `T = Bytes<2>` this module's first four values are
//!   `0000 0001 0080 00ff`, the harness prelude's are `0000 0100 8000 ff00`:
//!   `List<Bytes<2>>`, `Option<List<..>>` and so on are *different case sets* in
//!   the corpus and in the generated Rust harness, not merely reordered ones.
//!   The corpus follows the spec; the harness is frozen.  S3/S10 must settle
//!   which is normative for such types (VER04r 16).
//! * **`NatCount`.** A `Nat` that flows into a shift count or bit index has the
//!   73-value domain `0..=64` plus `65 127 128 129 255 256 65535 65536`
//!   (VER04j 3.6).  VER04r 5.1 describes `nat_count` as `0..=K`; the two are not
//!   the same set.  This module reports what the generator does; S3 decides how
//!   the corpus names it.
//! * **Signed edge sets.** A signed wide or demoted fixed-width leaf is the
//!   edge bit patterns read in two's complement *plus* `-1..=-255`, sorted
//!   ascending and de-duplicated (VER04j 2.6: `I16` 527 values, `I32` 559, `I64`
//!   623).  VER04r 5.1 says "the same bit patterns ... in the same order", which
//!   is the unsigned order and has no negatives.  The unsigned counts agree
//!   (`u64` 377, `u32` 313).  Again this module reports the generator's set, which
//!   the checked-in adopters' evidence pins; S3 and a VER04r erratum settle it.
//! * **`FxEdge` is for widths of 16 bits and up.** The unsigned edge formula
//!   repeats values at 8 bits (`0..=255` already contains `MAX` and the
//!   `MAX ^ (1<<i)` patterns), so an 8-bit edge set has duplicates.  The
//!   resolver never produces `FxEdge(U8|I8)` (an 8-bit binder is always full and
//!   never demoted); a hand-built one enumerates that duplicated set.
//!
//! ## Refinements are not part of a domain
//!
//! A binder's refinement (`x: U8 | x > 3`) *filters* the product; it does not
//! shrink the domain.  The product ordinal counts every tuple, the
//! *admitted-case ordinal* counts the admitted ones (VER04r 5.2).  Evaluating an
//! `admit` predicate needs the model evaluator and is the emitter's job; this
//! module hands it the binders' [`Parameter::refinement`] untouched (through
//! [`ResolvedBinders`]) and enumerates the *unfiltered* product.
//!
//! ## Values
//!
//! [`Domain::nth`] returns an [`EvalValue`], the evaluator's own value type, so
//! an emitter can feed a case straight to the model evaluator and the stream
//! encoder.  Leaves map as: `Bool` to `Bool`; `U8`/`U64` (v0) and `Fx(U8|U64)`
//! (v1) to `EvalValue::U8`/`U64`; the other fixed widths to `Fixed(width,
//! value)`; `Nat`/`NatCount` to `Nat`; byte strings to `Bytes`.  A `Result`
//! error case is `Result(Err((error type name, case)))`, the form the
//! evaluator's `x == E.CASE` and `match ... E.CASE` need (the name comes from the
//! declaration through [`DomainEnv`], and is invisible to `Domain` equality).  A
//! record or variant value is a `Con` whose `record` flag is the declaration's.
//! A `Domain` built by hand must give `Res` its real error type name and `Data`
//! its real flag for the same reason.
//!
//! ## Cost and limits (for the emitter)
//!
//! * `nth` is `O(descriptor + value)` but rebuilds leaf tables per call (a wide
//!   fixed-width leaf rebuilds its edge set, at most 623 entries); an emitter
//!   that calls it per case should cache the leaves' value lists.
//! * `resolve_binders` allows a descriptor of up to [`MAX_TYPE_NODES`] nodes and
//!   64 levels (the generator's own limits).  VER04r 5.1's 16-level descriptor
//!   limit is the *emitter's* to enforce.  `nth`/`domain_size` recurse over the
//!   tree, so a hand-built domain nested far deeper than the resolver can
//!   produce is the caller's responsibility.

pub use crate::checker::{resolve_post_slot, slot_shape, PostSlot, SlotShape};
pub use crate::corpus::{driven_descriptor, inout_descriptor, DrivenDescriptor, InoutDescriptor};
use crate::codegen::{CodegenError, EvalValue};
use crate::evidence::{resolve_claim_tys, ty_of_type, TyEnv};
pub use crate::evidence::{
    claim_domain, product_size, DOMAIN_BYTES_BUDGET, EXHAUSTIVE_BUDGET, MAX_FIXED_WIDTH,
    MAX_TYPE_NODES,
};
use crate::fixed::{self, FixedTy};
use crate::parser::{Declaration, Expression, Parameter, TypeExpr, Version, WarrantFile};

/// The resolved descriptor tree of one binder (VER04r section 5.1).
///
/// This is the generator's internal binder type under its stable public name;
/// the variants are the descriptor nodes (`Fx` is `fx_full` for a narrow width
/// and `fx_edge` for a wide one, `FxEdge` is a narrow width *demoted* to its
/// edge set, `Data` is a record when it has one constructor named like itself
/// and a variant otherwise).
///
/// ```
/// use coding_adventures_warrant_grammar::domain::Domain;
/// use coding_adventures_warrant_grammar::codegen::EvalValue;
///
/// let d = Domain::Tuple(vec![Domain::Bool, Domain::Nat]);
/// assert_eq!(d.domain_size(), Some(2 * 65));
/// // Last component varying fastest: index 1 is (false, 1), index 65 is (true, 0).
/// assert_eq!(d.nth(1), Some(EvalValue::Tuple(vec![EvalValue::Bool(false), EvalValue::Nat(1)])));
/// assert_eq!(d.nth(65), Some(EvalValue::Tuple(vec![EvalValue::Bool(true), EvalValue::Nat(0)])));
/// assert_eq!(d.nth(130), None);
/// ```
pub use crate::evidence::Ty as Domain;
pub use crate::evidence::ErrName;

/// The four-letter alphabet of the short byte-string domains (VER04i 2.6).
const ALPHABET: [u8; 4] = [0x00, 0x01, 0x80, 0xff];

/// The boundary shift counts a `NatCount` adds to `0..=64` (VER04j 3.6).
const COUNT_EXTRAS: [u64; 8] = [65, 127, 128, 129, 255, 256, 65535, 65536];

impl Domain {
    /// The `i`-th value of the domain in enumeration order, or `None` when
    /// `i >= domain_size()` (or the size overflows).
    ///
    /// Total, pure and deterministic; it never materialises the domain.  Cost is
    /// `O(size of the descriptor + size of the value)`, plus the at most 623-entry
    /// edge set of a wide fixed-width leaf -- never `O(i)`.
    ///
    /// ```
    /// use coding_adventures_warrant_grammar::domain::Domain;
    /// use coding_adventures_warrant_grammar::codegen::EvalValue;
    ///
    /// // u64_edge: 0..=255, then 1<<8 ..= 1<<63, then MAX, then MAX ^ (1<<i).
    /// assert_eq!(Domain::U64.domain_size(), Some(377));
    /// assert_eq!(Domain::U64.nth(255), Some(EvalValue::U64(255)));
    /// assert_eq!(Domain::U64.nth(256), Some(EvalValue::U64(1 << 8)));
    /// assert_eq!(Domain::U64.nth(312), Some(EvalValue::U64(u64::MAX)));
    /// assert_eq!(Domain::U64.nth(313), Some(EvalValue::U64(u64::MAX ^ 1)));
    /// assert_eq!(Domain::U64.nth(377), None);
    /// ```
    pub fn nth(&self, i: u64) -> Option<EvalValue> {
        if i >= self.domain_size()? {
            return None;
        }
        Some(self.nth_in_range(i))
    }

    /// Every value in order, lazily.  `domain_size() == None` yields nothing.
    ///
    /// ```
    /// use coding_adventures_warrant_grammar::domain::Domain;
    /// use coding_adventures_warrant_grammar::codegen::EvalValue;
    ///
    /// let all: Vec<EvalValue> = Domain::Opt(Box::new(Domain::Bool)).iter().collect();
    /// assert_eq!(all.len(), 3); // None, Some(false), Some(true)
    /// ```
    pub fn iter(&self) -> DomainIter<'_> {
        DomainIter {
            domain: self,
            next: 0,
            size: self.domain_size().unwrap_or(0),
        }
    }

    /// `nth` for an `i` already known to be `< domain_size()`.
    fn nth_in_range(&self, i: u64) -> EvalValue {
        match self {
            Domain::Bool => EvalValue::Bool(i == 1),
            Domain::U8 => EvalValue::U8(i as u8),
            Domain::Nat => EvalValue::Nat(i),
            Domain::NatCount => EvalValue::Nat(if i <= 64 {
                i
            } else {
                COUNT_EXTRAS[(i - 65) as usize]
            }),
            Domain::U64 => EvalValue::U64(match i {
                0..=255 => i,
                // 256 + k  is  1 << (8 + k)  for k in 0..56.
                256..=311 => 1u64 << (8 + (i - 256)),
                312 => u64::MAX,
                _ => u64::MAX ^ (1u64 << (i - 313)),
            }),
            Domain::Fx(f) if f.bits() <= 16 => EvalValue::from_fixed(*f, FixedTy::min(*f) + i128::from(i)),
            Domain::Fx(f) | Domain::FxEdge(f) => EvalValue::from_fixed(*f, fixed::edge_domain(*f)[i as usize]),
            Domain::Bytes => EvalValue::Bytes(bytes_d1_nth(i)),
            // The base domain, then the `00`/`a5` fills per hinted length,
            // in the same order the prelude enumerates.
            Domain::BytesHinted(ls) => {
                if i < 347 {
                    EvalValue::Bytes(bytes_d1_nth(i))
                } else {
                    let j = i - 347;
                    let l = ls[(j / 2) as usize] as usize;
                    let b = if j.is_multiple_of(2) { 0x00 } else { 0xa5 };
                    EvalValue::Bytes(vec![b; l])
                }
            }
            Domain::BytesN(n) => EvalValue::Bytes(bytes_n_nth(*n, i)),
            Domain::Tuple(cs) => EvalValue::Tuple(unrank_components(cs, i)),
            Domain::Data { name, ctors, record } => {
                let mut rest = i;
                for (index, (ctor, fields)) in ctors.iter().enumerate() {
                    let n = product_size(fields).expect("size checked by the caller");
                    if rest < n {
                        return EvalValue::Con {
                            ty: name.clone(),
                            index,
                            ctor: ctor.clone(),
                            record: *record,
                            fields: unrank_components(fields, rest),
                        };
                    }
                    rest -= n;
                }
                unreachable!("i < the sum of the constructor products")
            }
            Domain::Opt(t) => {
                if i == 0 {
                    EvalValue::Option(None)
                } else {
                    EvalValue::Option(Some(Box::new(t.nth_in_range(i - 1))))
                }
            }
            Domain::Res(t, cases, err) => {
                let n = t.domain_size().expect("size checked by the caller");
                if i < n {
                    EvalValue::Result(Ok(Box::new(t.nth_in_range(i))))
                } else {
                    EvalValue::Result(Err((err.0.clone(), cases[(i - n) as usize].clone())))
                }
            }
            // A marked binder enumerates exactly like `List<Bytes>`; only the
            // property relation differs (re-chunking in the prelude).
            Domain::ListDriven => Domain::List(Box::new(Domain::Bytes)).nth_in_range(i),
            Domain::List(t) => {
                let n = t.domain_size().expect("size checked by the caller");
                let (d, e) = (n.min(16), n.min(4));
                let at = |j: u64| t.nth_in_range(j);
                let mut rest = i;
                if rest == 0 {
                    return EvalValue::List(Vec::new());
                }
                rest -= 1;
                if rest < d {
                    return EvalValue::List(vec![at(rest)]);
                }
                rest -= d;
                if rest < d * d {
                    return EvalValue::List(vec![at(rest / d), at(rest % d)]);
                }
                rest -= d * d;
                EvalValue::List(vec![at(rest / (e * e)), at((rest / e) % e), at(rest % e)])
            }
        }
    }
}

/// The values of the Cartesian product of `components` at `i`, last component
/// fastest (`i` is known to be below the product size).
fn unrank_components(components: &[Domain], mut i: u64) -> Vec<EvalValue> {
    let mut idx = vec![0u64; components.len()];
    for (slot, c) in idx.iter_mut().zip(components).rev() {
        let n = c.domain_size().expect("size checked by the caller");
        *slot = i % n;
        i /= n;
    }
    components
        .iter()
        .zip(idx)
        .map(|(c, j)| c.nth_in_range(j))
        .collect()
}

/// `bytes_d1` (VER04i 2.6): `[]`; each single byte; every length-2 then every
/// length-3 string over [`ALPHABET`] lexicographically; for `L = 4..=8` the
/// pair `00`x`L`, `a5`x`L`.  347 values.
fn bytes_d1_nth(i: u64) -> Vec<u8> {
    match i {
        0 => Vec::new(),
        1..=256 => vec![(i - 1) as u8],
        257..=272 => alphabet_string(2, i - 257),
        273..=336 => alphabet_string(3, i - 273),
        _ => {
            let k = i - 337;
            let len = 4 + (k / 2) as usize;
            vec![if k & 1 == 0 { 0x00 } else { 0xa5 }; len]
        }
    }
}

/// The `j`-th string of length `len` over [`ALPHABET`], lexicographic (first
/// byte slowest).
fn alphabet_string(len: usize, mut j: u64) -> Vec<u8> {
    let mut out = vec![0u8; len];
    for slot in out.iter_mut().rev() {
        *slot = ALPHABET[(j % 4) as usize];
        j /= 4;
    }
    out
}

/// `bytes_n(N)` (VER04i 2.6): `N = 0` the empty string; `N = 1` all 256 bytes;
/// `N <= 3` the length-`N` members of `bytes_d1`; `N >= 4` the four uniform
/// fills over [`ALPHABET`], then for each position and each of `01 80 ff` a
/// zero string with that one byte set.
fn bytes_n_nth(n: u64, i: u64) -> Vec<u8> {
    let n = n as usize;
    match n {
        0 => Vec::new(),
        1 => vec![i as u8],
        2 | 3 => alphabet_string(n, i),
        _ if i < 4 => vec![ALPHABET[i as usize]; n],
        _ => {
            let j = (i - 4) as usize;
            let mut v = vec![0u8; n];
            v[j / 3] = ALPHABET[1 + j % 3];
            v
        }
    }
}

/// Lazy sequential enumeration of a [`Domain`] ([`Domain::iter`]).
#[derive(Clone, Debug)]
pub struct DomainIter<'a> {
    domain: &'a Domain,
    next: u64,
    size: u64,
}

impl Iterator for DomainIter<'_> {
    type Item = EvalValue;

    fn next(&mut self) -> Option<EvalValue> {
        if self.next >= self.size {
            return None;
        }
        let v = self.domain.nth_in_range(self.next);
        self.next += 1;
        Some(v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = usize::try_from(self.size - self.next).unwrap_or(usize::MAX);
        (left, Some(left))
    }
}

// ---------------------------------------------------------------------------
// Products (VER04r section 5.2)
// ---------------------------------------------------------------------------

/// Mixed-radix unranking of a product ordinal: the per-binder value indices of
/// product ordinal `p`, **last binder fastest** (VER04r 5.2).
///
/// `sizes` are the binders' `domain_size()` values.  `None` when `p` is not a
/// valid ordinal (`p >= product`), when the product overflows a `u64`, or when
/// some size is zero (there is no ordinal in an empty product).
///
/// ```
/// use coding_adventures_warrant_grammar::domain::unrank_product;
/// assert_eq!(unrank_product(&[2, 3], 0), Some(vec![0, 0]));
/// assert_eq!(unrank_product(&[2, 3], 4), Some(vec![1, 1]));
/// assert_eq!(unrank_product(&[2, 3], 6), None);
/// assert_eq!(unrank_product(&[], 0), Some(vec![])); // the empty product has one ordinal
/// ```
pub fn unrank_product(sizes: &[u64], p: u64) -> Option<Vec<u64>> {
    let total = checked_product(sizes)?;
    if p >= total {
        return None;
    }
    let mut idx = vec![0u64; sizes.len()];
    let mut rest = p;
    for (slot, n) in idx.iter_mut().zip(sizes).rev() {
        *slot = rest % n;
        rest /= n;
    }
    Some(idx)
}

/// The inverse of [`unrank_product`]: the product ordinal of per-binder value
/// indices.  `None` when the lengths differ, an index is out of its range, or
/// the product overflows.
///
/// ```
/// use coding_adventures_warrant_grammar::domain::{rank_product, unrank_product};
/// let sizes = [4, 5, 6];
/// for p in 0..120 {
///     assert_eq!(rank_product(&sizes, &unrank_product(&sizes, p).unwrap()), Some(p));
/// }
/// ```
pub fn rank_product(sizes: &[u64], indices: &[u64]) -> Option<u64> {
    checked_product(sizes)?;
    if sizes.len() != indices.len() {
        return None;
    }
    let mut p = 0u64;
    for (n, i) in sizes.iter().zip(indices) {
        if i >= n {
            return None;
        }
        p = p * n + i;
    }
    Some(p)
}

/// The product of `sizes` in checked arithmetic (`None` on overflow).  An empty
/// product is `1`; a zero size makes the product `0`.
fn checked_product(sizes: &[u64]) -> Option<u64> {
    sizes.iter().try_fold(1u64, |acc, n| acc.checked_mul(*n))
}

/// One case of the row-major product: its product ordinal, the per-binder value
/// indices and the values themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductCase {
    /// The product ordinal (not the admitted-case ordinal; refinements are the
    /// emitter's to evaluate).
    pub ordinal: u64,
    /// The index of each binder's value within its own domain.
    pub indices: Vec<u64>,
    /// The binder values, in binder order.
    pub values: Vec<EvalValue>,
}

/// Row-major iteration over the product of binder domains, starting at any
/// product ordinal -- the chunk jump of VER04r 5.4: unrank `c * chunk_size`,
/// then enumerate, never touching an earlier case.
///
/// ```
/// use coding_adventures_warrant_grammar::domain::{Domain, ProductIter};
/// use coding_adventures_warrant_grammar::codegen::EvalValue;
///
/// let ds = [Domain::Bool, Domain::Bool];
/// let from_two: Vec<_> = ProductIter::new(&ds, 2).unwrap().map(|c| c.ordinal).collect();
/// assert_eq!(from_two, [2, 3]);
/// let first = ProductIter::new(&ds, 1).unwrap().next().unwrap();
/// assert_eq!(first.values, [EvalValue::Bool(false), EvalValue::Bool(true)]);
/// ```
#[derive(Clone, Debug)]
pub struct ProductIter<'a> {
    domains: &'a [Domain],
    sizes: Vec<u64>,
    next: u64,
    total: u64,
}

impl<'a> ProductIter<'a> {
    /// Start at product ordinal `start` (`start == product` is allowed and
    /// yields nothing).  `None` when the product overflows a `u64` or `start`
    /// is past the end.
    pub fn new(domains: &'a [Domain], start: u64) -> Option<ProductIter<'a>> {
        let sizes: Vec<u64> = domains.iter().map(Domain::domain_size).collect::<Option<_>>()?;
        let total = checked_product(&sizes)?;
        (start <= total).then_some(ProductIter {
            domains,
            sizes,
            next: start,
            total,
        })
    }

    /// The number of product ordinals in all (`P` of VER04r 5.2).
    pub fn total(&self) -> u64 {
        self.total
    }
}

impl Iterator for ProductIter<'_> {
    type Item = ProductCase;

    fn next(&mut self) -> Option<ProductCase> {
        if self.next >= self.total {
            return None;
        }
        let ordinal = self.next;
        self.next += 1;
        let indices = unrank_product(&self.sizes, ordinal)?;
        let values = self
            .domains
            .iter()
            .zip(&indices)
            .map(|(d, i)| d.nth_in_range(*i))
            .collect();
        Some(ProductCase {
            ordinal,
            indices,
            values,
        })
    }
}

// ---------------------------------------------------------------------------
// The resolver: binders of a claim -> domains after demotion
// ---------------------------------------------------------------------------

/// What the generator needs to know about a document's type declarations to
/// turn a declared binder type into a [`Domain`]: aliases, records, variants
/// and error types.  Build it once per document.
pub struct DomainEnv<'a> {
    env: TyEnv<'a>,
    v1: bool,
}

impl<'a> DomainEnv<'a> {
    /// The environment of `file`'s declarations.  Fixed-width names, composite
    /// types and aliases resolve only in a `v1` document, exactly as in the
    /// generator.
    pub fn new(file: &WarrantFile<'a>) -> DomainEnv<'a> {
        DomainEnv {
            env: TyEnv::new(file),
            v1: file.version == Version::V1,
        }
    }

    /// The domain of one declared type, *before* any shift-count refinement or
    /// budget demotion (use [`resolve_binders`] for a claim's binders).
    /// `span` is only used in the error.  Errors are the generator's: `W0701`
    /// for a type it cannot enumerate, `W0706` for a type that expands to more
    /// than [`MAX_TYPE_NODES`] nodes, `W0707` for a `Bytes<N>` over
    /// [`MAX_FIXED_WIDTH`].
    pub fn domain_of_type(&self, t: &TypeExpr<'a>, span: (usize, usize)) -> Result<Domain, CodegenError> {
        ty_of_type(t, &self.env, self.v1, span, 0)
    }
}

/// One budget demotion: a leftmost full `U16`/`I16` leaf was cut to its edge
/// set.  `binder` is the binder's index; `path` the component path inside it
/// (a tuple component or record field index, and for a variant the constructor
/// name before the field index, so `a.Q.0` is `["Q", "0"]` on binder `a`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Demotion {
    pub binder: usize,
    pub path: Vec<String>,
}

/// The binders of one claim after resolution (VER04r 5.1, 5.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBinders {
    /// One domain per binder, in binder order, **after** demotion.
    pub domains: Vec<Domain>,
    /// The demotions applied, in order.  A demoted binder appears in the
    /// manifest so the label stays honest (VER04r 5.3).
    pub demotions: Vec<Demotion>,
    /// `P`: the number of product ordinals (the unfiltered product).
    pub product: u64,
    /// The sum of the binders' worst-case domain bytes, checked against
    /// [`DOMAIN_BYTES_BUDGET`].
    pub bytes: u64,
}

impl ResolvedBinders {
    /// The per-binder domain sizes, ready for [`unrank_product`].
    ///
    /// `None` only if a domain was replaced by hand with one whose size does not
    /// fit a `u64`; the resolver's own output always has sizes.
    pub fn sizes(&self) -> Option<Vec<u64>> {
        self.domains.iter().map(Domain::domain_size).collect()
    }

    /// Row-major iteration over the product from ordinal `start`.
    pub fn iter_from(&self, start: u64) -> Option<ProductIter<'_>> {
        ProductIter::new(&self.domains, start)
    }
}

/// Resolve the binders of a claim to their [`Domain`]s: declared types, the
/// shift-count refinement (a `Nat` binder that flows into `shl`/`shr`/`sar`/
/// `rotl`/`rotr`/`bit` of the claim body `claim_body` becomes `NatCount`, VER04j
/// 3.6), and the budget demotion of VER04j 2.6 -- the same function the
/// evidence generator runs, so a lane never re-runs demotion.
///
/// `budget` is the product-ordinal budget (the generator's default is
/// [`EXHAUSTIVE_BUDGET`]; VER04r 5.3 uses `2^24` for corpus v2).  The
/// [`DOMAIN_BYTES_BUDGET`] applies as well.  `label` names the claim in the
/// error and `span` is where it points.
///
/// # Errors
///
/// A [`CodegenError`] exactly as the generator reports it: `W0701` for a binder
/// type that cannot be enumerated, `W0706` when the product, or the domain
/// bytes, are still over budget after every possible demotion (or overflow a
/// `u64`; "no silent sampling"), `W0707` for an over-wide `Bytes<N>`.
///
/// Demotion runs only in a `v1` document; a `v0` document's binders are
/// returned as declared.
///
/// ```
/// use coding_adventures_warrant_grammar::domain::{resolve_binders, Domain, DomainEnv};
/// use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
/// use coding_adventures_warrant_grammar::parser::{parse_document, Declaration, ParserLimits};
/// use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
///
/// let src = "warrant t v1\n  op f(a: U16, b: Bool) -> U16\n  claim c:\n    forall a: U16, b: Bool. f(a, b) == a\n";
/// let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
/// let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
/// let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
///
/// let env = DomainEnv::new(&file);
/// let Declaration::Op(op) = &file.declarations[0] else { unreachable!() };
/// // U16 is full (65 536 values); times Bool that is 131 072 <= 400 000.
/// let r = resolve_binders(&env, &op.params, None, 400_000, "f", op.span).unwrap();
/// assert_eq!(r.product, 131_072);
/// assert!(r.demotions.is_empty());
/// // A tighter budget demotes the U16 to its edge set (281 values).
/// let r = resolve_binders(&env, &op.params, None, 1_000, "f", op.span).unwrap();
/// assert_eq!(r.domains[0].domain_size(), Some(281));
/// assert_eq!(r.demotions[0].binder, 0);
/// assert!(matches!(r.domains[1], Domain::Bool));
/// ```
pub fn resolve_binders<'a>(
    env: &DomainEnv<'a>,
    binders: &[Parameter<'a>],
    claim_body: Option<&Expression<'a>>,
    budget: u64,
    label: &str,
    span: (usize, usize),
) -> Result<ResolvedBinders, CodegenError> {
    let (domains, demoted) = resolve_claim_tys(binders, claim_body, &env.env, env.v1, budget, label, span)?;
    let (product, bytes) = claim_domain(&domains).expect("an in-budget claim fits a u64");
    Ok(ResolvedBinders {
        domains,
        demotions: demoted
            .into_iter()
            .map(|(binder, path)| Demotion { binder, path })
            .collect(),
        product,
        bytes,
    })
}

// ---------------------------------------------------------------------------
// The post-state slot resolver over domains
// ---------------------------------------------------------------------------

/// The post-state slot of an operation over [`Domain`]s: `posts` are the
/// domains of its `inout` parameters in parameter order, `ret` its declared
/// result domain.  This is the generator's own definition (VER04k 4.2: S1 plain,
/// S2 `Result` of plain, S3 tuple whose last components are the posts, S4
/// `Result` of such a tuple); [`resolve_post_slot`] is the same rule over any
/// `PartialEq` type.
///
/// ```
/// use coding_adventures_warrant_grammar::domain::{resolve_post_slot_of, Domain, PostSlot};
/// let bytes = Domain::Bytes;
/// // op push_byte(v: U8, out: inout Bytes) -> (Nat, Bytes)
/// let ret = Domain::Tuple(vec![Domain::Nat, Domain::Bytes]);
/// assert_eq!(resolve_post_slot_of(&[bytes], &ret), Some(PostSlot::S3 { h: 1 }));
/// ```
pub fn resolve_post_slot_of(posts: &[Domain], ret: &Domain) -> Option<PostSlot> {
    let shape = match ret {
        Domain::Res(t, _, _) => match &**t {
            Domain::Tuple(cs) => SlotShape::ResultTuple(cs.clone()),
            other => SlotShape::ResultPlain(other.clone()),
        },
        Domain::Tuple(cs) => SlotShape::Tuple(cs.clone()),
        other => SlotShape::Plain(other.clone()),
    };
    resolve_post_slot(posts, &shape)
}

/// The resolved post-state slot of one operation with `inout` parameters, with
/// the domains of everything the slot names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpPostSlot {
    /// `S1` .. `S4` and, for the tuple forms, the host-returned component count.
    pub slot: PostSlot,
    /// The `inout` parameters' names, in parameter order.
    pub inout: Vec<String>,
    /// Their domains (the pre-state's type, which is also the post-state's).
    pub posts: Vec<Domain>,
    /// The declared result's domain.
    pub result: Domain,
}

/// The post-state slot of operation `op` in a `v1` document, or `Ok(None)` when
/// the operation does not exist, has no `inout` parameter, or its result type
/// has no slot for them (the checker refuses the last case, so it does not
/// occur in a checked document).
///
/// # Errors
///
/// The generator's type errors if a parameter or the result is not an
/// enumerable type.
pub fn op_post_slot<'a>(
    env: &DomainEnv<'a>,
    file: &WarrantFile<'a>,
    op: &str,
) -> Result<Option<OpPostSlot>, CodegenError> {
    let Some(decl) = file.declarations.iter().find_map(|d| match d {
        Declaration::Op(o) if o.name == op => Some(o),
        _ => None,
    }) else {
        return Ok(None);
    };
    let mut inout = Vec::new();
    let mut posts = Vec::new();
    for p in decl.params.iter().filter(|p| p.inout) {
        inout.push(p.name.to_string());
        posts.push(ty_of_type(&p.param_type, &env.env, true, p.span, 0)?);
    }
    if posts.is_empty() {
        return Ok(None);
    }
    let result = ty_of_type(&decl.return_type, &env.env, true, decl.span, 0)?;
    Ok(resolve_post_slot_of(&posts, &result).map(|slot| OpPostSlot {
        slot,
        inout,
        posts,
        result,
    }))
}
