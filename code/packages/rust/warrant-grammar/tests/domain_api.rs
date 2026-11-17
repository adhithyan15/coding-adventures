//! VER04r CV2-S2: the public resolved-domain / post-slot API (`domain`), used the
//! way the v2 corpus emitter (S3) will use it -- from outside the crate.
//!
//! The pins here are written from the spec text (VER04r 5.1/5.2, VER04i 2.6,
//! VER04j 2.6/3.6/4.7), not read back from the implementation: [`materialize`]
//! is an *eager, recursive* enumerator built differently from the library's
//! unranking `nth`, and every property is checked against it.

use std::collections::BTreeSet;

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenErrorKind, DiagnosticCode, EvalValue};
use coding_adventures_warrant_grammar::domain::{
    inout_descriptor, op_post_slot, rank_product, resolve_binders, resolve_post_slot,
    resolve_post_slot_of, slot_shape, unrank_product, Demotion, Domain, DomainEnv, PostSlot,
    ProductIter, SlotShape, DOMAIN_BYTES_BUDGET, EXHAUSTIVE_BUDGET, MAX_FIXED_WIDTH,
};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const WXA: &str = include_str!("fixtures/wxa.warrant");
const WXG: &str = include_str!("fixtures/wxg.warrant");
const WX_FIXED: &str = include_str!("fixtures/wx_fixed.warrant");

fn with_file<R>(src: &str, f: impl FnOnce(&WarrantFile<'_>) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

// ---------------------------------------------------------------------------
// The independent oracle
// ---------------------------------------------------------------------------

const A: [u8; 4] = [0x00, 0x01, 0x80, 0xff];

/// Every string of length `n` over `A`, first byte slowest (lexicographic), built
/// by recursion on the prefix -- the shape the v1 corpus compiler uses.
fn strings(n: usize) -> Vec<Vec<u8>> {
    if n == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for s in strings(n - 1) {
        for b in A {
            let mut t = s.clone();
            t.push(b);
            out.push(t);
        }
    }
    out
}

fn bytes_d1() -> Vec<Vec<u8>> {
    let mut v = vec![Vec::new()];
    v.extend((0..=255u8).map(|b| vec![b]));
    v.extend(strings(2));
    v.extend(strings(3));
    for n in 4..=8 {
        v.push(vec![0x00; n]);
        v.push(vec![0xa5; n]);
    }
    v
}

fn bytes_n(n: usize) -> Vec<Vec<u8>> {
    match n {
        0 => vec![Vec::new()],
        1 => (0..=255u8).map(|b| vec![b]).collect(),
        2 | 3 => strings(n),
        _ => {
            let mut v: Vec<Vec<u8>> = A.iter().map(|b| vec![*b; n]).collect();
            for pos in 0..n {
                for s in [0x01u8, 0x80, 0xff] {
                    let mut x = vec![0u8; n];
                    x[pos] = s;
                    v.push(x);
                }
            }
            v
        }
    }
}

/// The edge set of a fixed-width type from the VER04j 2.6 text: unsigned
/// `0..=255`, `1<<i` for `i = 8..w`, `MAX`, `MAX ^ (1<<i)`; signed the same bit
/// patterns read in two's complement plus `-1..=-255`, ascending, no duplicates.
fn fx_edge(t: FixedTy) -> Vec<i128> {
    let w = t.bits();
    let max: u128 = if w == 64 { u64::MAX as u128 } else { (1u128 << w) - 1 };
    let mut patterns: Vec<u128> = (0..=255u128).filter(|v| *v <= max).collect();
    patterns.extend((8..w).map(|i| 1u128 << i));
    patterns.push(max);
    patterns.extend((0..w).map(|i| max ^ (1u128 << i)));
    if !t.signed() {
        return patterns.into_iter().map(|p| p as i128).collect();
    }
    let sign = 1u128 << (w - 1);
    let mut set: BTreeSet<i128> = patterns
        .into_iter()
        .map(|p| if p & sign != 0 { p as i128 - (1i128 << w) } else { p as i128 })
        .collect();
    set.extend((1..=255i128).map(|k| -k).filter(|v| *v >= FixedTy::min(t)));
    set.into_iter().collect()
}

fn product(cs: &[Domain]) -> Vec<Vec<EvalValue>> {
    let mut rows = vec![Vec::new()];
    for c in cs {
        let vals = materialize(c);
        let mut next = Vec::new();
        for r in &rows {
            for v in &vals {
                let mut t = r.clone();
                t.push(v.clone());
                next.push(t);
            }
        }
        rows = next;
    }
    rows
}

/// The whole domain, eagerly, straight from VER04r 5.1.
fn materialize(d: &Domain) -> Vec<EvalValue> {
    match d {
        Domain::Bool => vec![EvalValue::Bool(false), EvalValue::Bool(true)],
        Domain::U8 => (0..=255u8).map(EvalValue::U8).collect(),
        Domain::Nat => (0..=64).map(EvalValue::Nat).collect(),
        Domain::NatCount => (0..=64u64)
            .chain([65, 127, 128, 129, 255, 256, 65535, 65536])
            .map(EvalValue::Nat)
            .collect(),
        Domain::U64 => fx_edge(FixedTy::U64)
            .into_iter()
            .map(|v| EvalValue::U64(v as u64))
            .collect(),
        Domain::Fx(f) if f.bits() <= 16 => (FixedTy::min(*f)..=FixedTy::max(*f)).map(|v| EvalValue::from_fixed(*f, v)).collect(),
        Domain::Fx(f) | Domain::FxEdge(f) => fx_edge(*f).into_iter().map(|v| EvalValue::from_fixed(*f, v)).collect(),
        Domain::Bytes => bytes_d1().into_iter().map(EvalValue::Bytes).collect(),
        Domain::BytesN(n) => bytes_n(*n as usize).into_iter().map(EvalValue::Bytes).collect(),
        Domain::Tuple(cs) => product(cs).into_iter().map(EvalValue::Tuple).collect(),
        Domain::Data { name, ctors } => {
            let mut out = Vec::new();
            for (index, (ctor, fields)) in ctors.iter().enumerate() {
                for fs in product(fields) {
                    out.push(EvalValue::Con {
                        ty: name.clone(),
                        index,
                        ctor: ctor.clone(),
                        record: ctors.len() == 1 && ctor == name,
                        fields: fs,
                    });
                }
            }
            out
        }
        Domain::Opt(t) => {
            let mut out = vec![EvalValue::Option(None)];
            out.extend(materialize(t).into_iter().map(|x| EvalValue::Option(Some(Box::new(x)))));
            out
        }
        Domain::Res(t, cases) => {
            let mut out: Vec<EvalValue> = materialize(t).into_iter().map(|x| EvalValue::Result(Ok(Box::new(x)))).collect();
            out.extend(cases.iter().map(|c| EvalValue::Result(Err((String::new(), c.clone())))));
            out
        }
        Domain::List(t) => {
            let all = materialize(t);
            let wide: Vec<&EvalValue> = all.iter().take(16).collect();
            let narrow: Vec<&EvalValue> = all.iter().take(4).collect();
            let mut out = vec![EvalValue::List(Vec::new())];
            for a in &wide {
                out.push(EvalValue::List(vec![(*a).clone()]));
            }
            for a in &wide {
                for b in &wide {
                    out.push(EvalValue::List(vec![(*a).clone(), (*b).clone()]));
                }
            }
            for a in &narrow {
                for b in &narrow {
                    for c in &narrow {
                        out.push(EvalValue::List(vec![(*a).clone(), (*b).clone(), (*c).clone()]));
                    }
                }
            }
            out
        }
    }
}

fn leaf_domains() -> Vec<Domain> {
    let mut v = vec![Domain::Bool, Domain::U8, Domain::U64, Domain::Nat, Domain::NatCount, Domain::Bytes];
    v.extend([0, 1, 2, 3, 4, 5, 16].map(Domain::BytesN));
    for f in FixedTy::ALL {
        v.push(Domain::Fx(f));
        v.push(Domain::FxEdge(f));
    }
    v
}

fn rec(name: &str, fields: Vec<Domain>) -> Domain {
    Domain::Data {
        name: name.into(),
        ctors: vec![(name.into(), fields)],
    }
}

fn composite_domains() -> Vec<Domain> {
    let shape = Domain::Data {
        name: "Shape".into(),
        ctors: vec![
            ("Circle".into(), vec![Domain::Bool]),
            ("Rect".into(), vec![Domain::Bool, Domain::BytesN(1)]),
            ("Empty".into(), vec![]),
        ],
    };
    vec![
        Domain::Tuple(vec![]),
        Domain::Tuple(vec![Domain::Bool, Domain::Nat]),
        Domain::Tuple(vec![Domain::BytesN(2), Domain::Bool, Domain::BytesN(0)]),
        rec("Span", vec![Domain::Fx(FixedTy::U8), Domain::Bool]),
        shape.clone(),
        Domain::Opt(Box::new(shape.clone())),
        Domain::Res(Box::new(Domain::Bool), vec!["TRUNCATED".into(), "OVERLONG".into()]),
        Domain::Res(Box::new(Domain::Tuple(vec![Domain::Bool, Domain::Bool])), vec!["E".into()]),
        // Lists: the length-1/2 rule uses the first 16 values, length 3 the first 4.
        Domain::List(Box::new(Domain::Bool)),
        Domain::List(Box::new(Domain::Nat)),
        Domain::List(Box::new(Domain::Opt(Box::new(Domain::Bool)))),
        Domain::List(Box::new(Domain::BytesN(0))),
        Domain::Opt(Box::new(Domain::List(Box::new(shape)))),
        Domain::Tuple(vec![Domain::Opt(Box::new(Domain::Tuple(vec![Domain::Bool, Domain::Bool]))), Domain::List(Box::new(Domain::Bool))]),
    ]
}

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

#[test]
fn leaf_sizes_are_the_documented_cardinalities() {
    let pin = |d: Domain, n: u64| assert_eq!(d.domain_size(), Some(n), "{d:?}");
    pin(Domain::Bool, 2);
    pin(Domain::U8, 256);
    pin(Domain::U64, 377);
    pin(Domain::Nat, 65);
    pin(Domain::NatCount, 73);
    pin(Domain::Bytes, 347);
    pin(Domain::BytesN(0), 1);
    pin(Domain::BytesN(1), 256);
    pin(Domain::BytesN(2), 16);
    pin(Domain::BytesN(3), 64);
    pin(Domain::BytesN(4), 16);
    pin(Domain::BytesN(16), 52);
    pin(Domain::Fx(FixedTy::U8), 256);
    pin(Domain::Fx(FixedTy::I8), 256);
    pin(Domain::Fx(FixedTy::U16), 65_536);
    pin(Domain::Fx(FixedTy::I16), 65_536);
    pin(Domain::FxEdge(FixedTy::U16), 281);
    pin(Domain::FxEdge(FixedTy::I16), 527);
    pin(Domain::Fx(FixedTy::U32), 313);
    pin(Domain::Fx(FixedTy::I32), 559);
    pin(Domain::Fx(FixedTy::U64), 377);
    pin(Domain::Fx(FixedTy::I64), 623);
}

#[test]
fn composite_sizes_follow_the_closed_forms() {
    // product, concatenation, +1, +#cases, 1 + d + d^2 + e^3.
    let t = Domain::Tuple(vec![Domain::U64, Domain::Nat]);
    assert_eq!(t.domain_size(), Some(377 * 65));
    let variant = Domain::Data {
        name: "V".into(),
        ctors: vec![("A".into(), vec![Domain::Bool]), ("B".into(), vec![])],
    };
    assert_eq!(variant.domain_size(), Some(2 + 1));
    assert_eq!(Domain::Opt(Box::new(Domain::U8)).domain_size(), Some(257));
    assert_eq!(Domain::Res(Box::new(Domain::Bool), vec!["X".into(), "Y".into()]).domain_size(), Some(4));
    // d = min(16, 65) = 16, e = min(4, 65) = 4.
    assert_eq!(Domain::List(Box::new(Domain::Nat)).domain_size(), Some(1 + 16 + 256 + 64));
    // d = e = 2.
    assert_eq!(Domain::List(Box::new(Domain::Bool)).domain_size(), Some(1 + 2 + 4 + 8));
    assert_eq!(Domain::Tuple(vec![]).domain_size(), Some(1));
}

#[test]
fn a_size_that_does_not_fit_is_none_never_zero() {
    // 256^8 = 2^64 wraps to 0 in a u64; it must be "over budget", not empty.
    let eight = Domain::Tuple(vec![Domain::U8; 8]);
    assert_eq!(eight.domain_size(), None);
    assert_eq!(eight.nth(0), None);
    assert_eq!(eight.iter().count(), 0);
    let seven = Domain::Tuple(vec![Domain::U8; 7]);
    assert_eq!(seven.domain_size(), Some(1u64 << 56));
    // A width over the cap has no domain.
    assert!(MAX_FIXED_WIDTH == 4096);
    assert_eq!(Domain::BytesN(MAX_FIXED_WIDTH + 1).domain_size(), None);
    assert_eq!(Domain::BytesN(MAX_FIXED_WIDTH).domain_size(), Some(3 * MAX_FIXED_WIDTH + 4));
    // An overflow nested anywhere propagates.
    assert_eq!(Domain::Opt(Box::new(eight.clone())).domain_size(), None);
    assert_eq!(Domain::List(Box::new(eight)).domain_size(), None);
    // The product helpers agree.
    assert_eq!(unrank_product(&[1 << 32, 1 << 32], 0), None);
    assert!(ProductIter::new(&[Domain::Tuple(vec![Domain::U8; 8])], 0).is_none());
    assert!(ProductIter::new(&vec![Domain::U8; 8], 0).is_none());
}

// ---------------------------------------------------------------------------
// Order: nth, iter and the eager oracle agree on every domain
// ---------------------------------------------------------------------------

fn assert_matches_oracle(d: &Domain) {
    let want = materialize(d);
    assert_eq!(Some(want.len() as u64), d.domain_size(), "size of {d:?}");
    let got: Vec<EvalValue> = d.iter().collect();
    assert_eq!(got, want, "sequential order of {d:?}");
    assert_eq!(d.iter().size_hint(), (want.len(), Some(want.len())));
    // nth must be random access into the same sequence, not merely agree when
    // walked in order: probe the ends, the middle and a stride.
    let n = want.len();
    let mut probes = vec![0, 1, n / 2, n.saturating_sub(2), n - 1];
    probes.extend((0..n).step_by(n / 97 + 1));
    for i in probes {
        assert_eq!(d.nth(i as u64).as_ref(), want.get(i), "nth({i}) of {d:?}");
    }
    assert_eq!(d.nth(n as u64), None, "nth(size) of {d:?}");
    assert_eq!(d.nth(u64::MAX), None);
}

#[test]
fn leaf_domains_enumerate_in_the_specified_order_and_nth_matches() {
    for d in leaf_domains() {
        assert_matches_oracle(&d);
    }
}

#[test]
fn composite_domains_enumerate_in_the_specified_order_and_nth_matches() {
    for d in composite_domains() {
        assert_matches_oracle(&d);
    }
}

#[test]
fn nth_on_every_index_of_a_mid_sized_composite_equals_sequential_enumeration() {
    let d = Domain::Tuple(vec![
        Domain::Opt(Box::new(Domain::BytesN(2))),
        Domain::List(Box::new(Domain::Bool)),
        Domain::Res(Box::new(Domain::Bool), vec!["E".into()]),
    ]);
    let seq: Vec<EvalValue> = d.iter().collect();
    for (i, v) in seq.iter().enumerate() {
        assert_eq!(d.nth(i as u64).as_ref(), Some(v), "index {i}");
    }
    assert_eq!(seq.len() as u64, d.domain_size().unwrap());
}

#[test]
fn literal_pins_of_the_orders_a_reader_would_check_first() {
    use EvalValue::{Bytes, Nat, U64};
    // bytes_d1: [], then single bytes, then length 2 lexicographic (first byte
    // slowest), length 3, then the 00/a5 fills of length 4..=8.
    assert_eq!(Domain::Bytes.nth(0), Some(Bytes(vec![])));
    assert_eq!(Domain::Bytes.nth(1), Some(Bytes(vec![0x00])));
    assert_eq!(Domain::Bytes.nth(256), Some(Bytes(vec![0xff])));
    assert_eq!(Domain::Bytes.nth(257), Some(Bytes(vec![0x00, 0x00])));
    assert_eq!(Domain::Bytes.nth(258), Some(Bytes(vec![0x00, 0x01])));
    assert_eq!(Domain::Bytes.nth(260), Some(Bytes(vec![0x00, 0xff])));
    assert_eq!(Domain::Bytes.nth(261), Some(Bytes(vec![0x01, 0x00])));
    assert_eq!(Domain::Bytes.nth(272), Some(Bytes(vec![0xff, 0xff])));
    assert_eq!(Domain::Bytes.nth(273), Some(Bytes(vec![0x00, 0x00, 0x00])));
    assert_eq!(Domain::Bytes.nth(336), Some(Bytes(vec![0xff, 0xff, 0xff])));
    assert_eq!(Domain::Bytes.nth(337), Some(Bytes(vec![0x00; 4])));
    assert_eq!(Domain::Bytes.nth(338), Some(Bytes(vec![0xa5; 4])));
    assert_eq!(Domain::Bytes.nth(346), Some(Bytes(vec![0xa5; 8])));
    // bytes_n(N >= 4): fills, then per position the bytes 01 80 ff.
    assert_eq!(Domain::BytesN(4).nth(3), Some(Bytes(vec![0xff; 4])));
    assert_eq!(Domain::BytesN(4).nth(4), Some(Bytes(vec![0x01, 0, 0, 0])));
    assert_eq!(Domain::BytesN(4).nth(6), Some(Bytes(vec![0xff, 0, 0, 0])));
    assert_eq!(Domain::BytesN(4).nth(7), Some(Bytes(vec![0, 0x01, 0, 0])));
    // u64_edge boundaries.
    assert_eq!(Domain::U64.nth(311), Some(U64(1 << 63)));
    assert_eq!(Domain::U64.nth(312), Some(U64(u64::MAX)));
    assert_eq!(Domain::U64.nth(376), Some(U64(u64::MAX ^ (1 << 63))));
    // nat_count: 0..=64 then the boundary shift counts.
    assert_eq!(Domain::NatCount.nth(64), Some(Nat(64)));
    assert_eq!(Domain::NatCount.nth(65), Some(Nat(65)));
    assert_eq!(Domain::NatCount.nth(72), Some(Nat(65536)));
    // Signed edge sets run ascending from the most negative value.
    assert_eq!(Domain::Fx(FixedTy::I32).nth(0), Some(EvalValue::Fixed(FixedTy::I32, i128::from(i32::MIN))));
    assert_eq!(Domain::Fx(FixedTy::I32).nth(558), Some(EvalValue::Fixed(FixedTy::I32, i128::from(i32::MAX))));
    // fx_full is numerically ascending, signed from MIN.
    assert_eq!(Domain::Fx(FixedTy::I8).nth(0), Some(EvalValue::Fixed(FixedTy::I8, -128)));
    assert_eq!(Domain::Fx(FixedTy::I8).nth(255), Some(EvalValue::Fixed(FixedTy::I8, 127)));
}

#[test]
fn composite_value_shapes_carry_the_constructor_and_record_flag() {
    let span = rec("Span", vec![Domain::Bool]);
    match span.nth(1) {
        Some(EvalValue::Con { ty, index, ctor, record, fields }) => {
            assert_eq!((ty.as_str(), index, ctor.as_str(), record), ("Span", 0, "Span", true));
            assert_eq!(fields, [EvalValue::Bool(true)]);
        }
        other => panic!("{other:?}"),
    }
    let v = Domain::Data {
        name: "Shape".into(),
        ctors: vec![("Circle".into(), vec![Domain::Bool]), ("Empty".into(), vec![])],
    };
    match v.nth(2) {
        Some(EvalValue::Con { index, ctor, record, fields, .. }) => {
            assert_eq!((index, ctor.as_str(), record, fields.len()), (1, "Empty", false, 0));
        }
        other => panic!("{other:?}"),
    }
    // An error case names the case; the descriptor does not know the error type.
    let r = Domain::Res(Box::new(Domain::Bool), vec!["OVERLONG".into()]);
    assert_eq!(r.nth(2), Some(EvalValue::Result(Err((String::new(), "OVERLONG".into())))));
    assert_eq!(r.nth(1), Some(EvalValue::Result(Ok(Box::new(EvalValue::Bool(true))))));
}

// ---------------------------------------------------------------------------
// Products
// ---------------------------------------------------------------------------

#[test]
fn product_unranking_is_row_major_last_binder_fastest_and_invertible() {
    let sizes = [3u64, 4, 5];
    let mut seen = Vec::new();
    for p in 0..60 {
        let idx = unrank_product(&sizes, p).unwrap();
        assert_eq!(rank_product(&sizes, &idx), Some(p));
        seen.push(idx);
    }
    // The first indices run through the LAST binder.
    assert_eq!(seen[0], [0, 0, 0]);
    assert_eq!(seen[1], [0, 0, 1]);
    assert_eq!(seen[5], [0, 1, 0]);
    assert_eq!(seen[20], [1, 0, 0]);
    assert_eq!(seen[59], [2, 3, 4]);
    // Strictly increasing lexicographically.
    assert!(seen.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(unrank_product(&sizes, 60), None);
    assert_eq!(unrank_product(&[3, 0, 5], 0), None, "an empty product has no ordinals");
    assert_eq!(rank_product(&sizes, &[0, 0]), None);
    assert_eq!(rank_product(&sizes, &[3, 0, 0]), None);
    assert_eq!(unrank_product(&[], 0), Some(vec![]));
    assert_eq!(unrank_product(&[], 1), None);
}

#[test]
fn product_iteration_from_any_ordinal_is_the_suffix_of_the_sequential_product() {
    let ds = [Domain::Bool, Domain::BytesN(2), Domain::Opt(Box::new(Domain::Bool))];
    let all: Vec<_> = ProductIter::new(&ds, 0).unwrap().collect();
    let total = ProductIter::new(&ds, 0).unwrap().total();
    assert_eq!(total, 2 * 16 * 3);
    assert_eq!(all.len() as u64, total);
    for (i, c) in all.iter().enumerate() {
        assert_eq!(c.ordinal, i as u64);
        // The values are the per-binder nth values at the unranked indices.
        for (k, d) in ds.iter().enumerate() {
            assert_eq!(d.nth(c.indices[k]).as_ref(), Some(&c.values[k]));
        }
    }
    for start in [0u64, 1, 2, 47, 48, 95, 96] {
        let tail: Vec<_> = ProductIter::new(&ds, start).unwrap().collect();
        assert_eq!(tail, all[start as usize..], "start {start}");
    }
    assert!(ProductIter::new(&ds, 97).is_none(), "past the end");
    assert_eq!(ProductIter::new(&ds, 96).unwrap().count(), 0, "start == P is empty, not an error");
}

#[test]
fn product_of_no_binders_is_one_empty_case() {
    let cases: Vec<_> = ProductIter::new(&[], 0).unwrap().collect();
    assert_eq!(cases.len(), 1);
    assert!(cases[0].values.is_empty());
}

// ---------------------------------------------------------------------------
// resolve_binders on real documents
// ---------------------------------------------------------------------------

fn claim<'a, 'f>(file: &'f WarrantFile<'a>, name: &str) -> (&'f [coding_adventures_warrant_grammar::parser::Parameter<'a>], &'f Expression<'a>) {
    for d in &file.declarations {
        if let Declaration::Claim(c) = d {
            if c.name == name {
                if let Expression::Quantified { binders, body, .. } = &c.body {
                    return (binders.as_slice(), body);
                }
            }
        }
    }
    panic!("claim {name}");
}

#[test]
fn a_claim_whose_product_fits_is_returned_undemoted() {
    with_file(WX_FIXED, |file| {
        let env = DomainEnv::new(file);
        let (binders, body) = claim(file, "add32_is_wrapping_add");
        let r = resolve_binders(&env, binders, Some(body), EXHAUSTIVE_BUDGET, "add32_is_wrapping_add", (0, 0)).unwrap();
        assert_eq!(r.domains, [Domain::Fx(FixedTy::U32), Domain::Fx(FixedTy::U32)]);
        assert!(r.demotions.is_empty());
        assert_eq!(r.product, 313 * 313);
        assert_eq!(r.sizes(), [313, 313]);
        // elem_cost of a scalar is 32 bytes (VALUE_OVERHEAD) per value.
        assert_eq!(r.bytes, 2 * 313 * 32);
        assert!(r.bytes <= DOMAIN_BYTES_BUDGET);
    });
}

#[test]
fn the_budget_demotes_the_leftmost_full_16_bit_leaf_first_and_records_it() {
    with_file(WX_FIXED, |file| {
        let env = DomainEnv::new(file);
        let (binders, body) = claim(file, "mul16_is_wrapping_mul");
        // 65536^2 = 2^32 > 400_000: demote the first binder (281 x 65536 is
        // still over), then the second (281 x 281 = 78_961).
        let r = resolve_binders(&env, binders, Some(body), EXHAUSTIVE_BUDGET, "mul16", (0, 0)).unwrap();
        assert_eq!(r.domains, [Domain::FxEdge(FixedTy::U16), Domain::FxEdge(FixedTy::U16)]);
        assert_eq!(
            r.demotions,
            [Demotion { binder: 0, path: vec![] }, Demotion { binder: 1, path: vec![] }]
        );
        assert_eq!(r.product, 281 * 281);
        // A budget that fits 281 x 65536 stops after the first demotion.
        let r = resolve_binders(&env, binders, Some(body), 281 * 65_536, "mul16", (0, 0)).unwrap();
        assert_eq!(r.domains, [Domain::FxEdge(FixedTy::U16), Domain::Fx(FixedTy::U16)]);
        assert_eq!(r.demotions.len(), 1);
        // A budget that fits the whole thing demotes nothing.
        let r = resolve_binders(&env, binders, Some(body), 1 << 32, "mul16", (0, 0)).unwrap();
        assert!(r.demotions.is_empty());
        assert_eq!(r.product, 1 << 32);
    });
}

#[test]
fn a_nat_that_flows_into_a_shift_count_gets_the_count_domain() {
    with_file(WX_FIXED, |file| {
        let env = DomainEnv::new(file);
        let (binders, body) = claim(file, "rotl32_is_rotate");
        let r = resolve_binders(&env, binders, Some(body), EXHAUSTIVE_BUDGET, "rotl32", (0, 0)).unwrap();
        assert_eq!(r.domains, [Domain::Fx(FixedTy::U32), Domain::NatCount]);
        assert_eq!(r.product, 313 * 73);
        // Without the claim body the refinement cannot be seen: a plain Nat.
        let r = resolve_binders(&env, binders, None, EXHAUSTIVE_BUDGET, "rotl32", (0, 0)).unwrap();
        assert_eq!(r.domains, [Domain::Fx(FixedTy::U32), Domain::Nat]);
        // shl16: U16 x NatCount = 4_784_128 > 400_000, so the U16 is demoted.
        let (binders, body) = claim(file, "shl16_is_total_shift");
        let r = resolve_binders(&env, binders, Some(body), EXHAUSTIVE_BUDGET, "shl16", (0, 0)).unwrap();
        assert_eq!(r.domains, [Domain::FxEdge(FixedTy::U16), Domain::NatCount]);
        assert_eq!(r.demotions, [Demotion { binder: 0, path: vec![] }]);
        assert_eq!(r.product, 281 * 73);
    });
}

#[test]
fn composite_binders_resolve_through_aliases_records_and_variants() {
    with_file(WXA, |file| {
        let env = DomainEnv::new(file);
        let (binders, body) = claim(file, "pick_matches_the_model");
        let r = resolve_binders(&env, binders, Some(body), EXHAUSTIVE_BUDGET, "pick", (0, 0)).unwrap();
        // forall o: Option<(U8, Bool)>, l: List<Bool>
        let opt = Domain::Opt(Box::new(Domain::Tuple(vec![Domain::Fx(FixedTy::U8), Domain::Bool])));
        assert_eq!(r.domains, [opt, Domain::List(Box::new(Domain::Bool))]);
        assert_eq!(r.sizes(), [1 + 512, 15]);
        // The resolved descriptors enumerate; the product is their row-major product.
        let all: Vec<_> = r.iter_from(0).unwrap().collect();
        assert_eq!(all.len() as u64, r.product);
        assert_eq!(all[0].values[0], EvalValue::Option(None));
        assert_eq!(all[1].values[1], r.domains[1].nth(1).unwrap());
        // A variant and an alias via the environment.
        let Declaration::Op(area) = file
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Op(o) if o.name == "area"))
            .unwrap()
        else {
            unreachable!()
        };
        let shape = env.domain_of_type(&area.params[0].param_type, area.span).unwrap();
        assert_eq!(shape.domain_size(), Some(256 + 65_536 + 1));
        let Declaration::Op(order) = file
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Op(o) if o.name == "order"))
            .unwrap()
        else {
            unreachable!()
        };
        let pair = env.domain_of_type(&order.return_type, order.span).unwrap();
        assert_eq!(pair, Domain::Tuple(vec![Domain::Fx(FixedTy::U8), Domain::Fx(FixedTy::U8)]), "alias Pair");
    });
}

#[test]
fn an_unfittable_claim_is_w0706_and_never_silently_sampled() {
    with_file(WX_FIXED, |file| {
        let env = DomainEnv::new(file);
        let (binders, body) = claim(file, "add32_is_wrapping_add");
        // 313 x 313 = 97_969 > 1_000 and a U32 has no further demotion.
        let e = resolve_binders(&env, binders, Some(body), 1_000, "c", (7, 9)).unwrap_err();
        assert_eq!(e.code, DiagnosticCode::W0706);
        assert_eq!(e.span, (7, 9));
        let CodegenErrorKind::UnsupportedTarget(msg) = &e.kind else { panic!("{e:?}") };
        assert!(msg.contains("claim c exhaustive domain 97969"), "{msg}");
        assert!(msg.contains("exceeds budget 1000 cases"), "{msg}");
    });
}

#[test]
fn eight_u8_binders_overflow_to_w0706_not_to_an_empty_domain() {
    // 256^8 wraps to 0; the checked arithmetic must refuse it.
    let src = "warrant t v1\n  op f(a: U8, b: U8, c: U8, d: U8, e: U8, f: U8, g: U8, h: U8) -> U8\n";
    with_file(src, |file| {
        let env = DomainEnv::new(file);
        let Declaration::Op(op) = &file.declarations[0] else { unreachable!() };
        let e = resolve_binders(&env, &op.params, None, u64::MAX, "f", op.span).unwrap_err();
        assert_eq!(e.code, DiagnosticCode::W0706);
        let CodegenErrorKind::UnsupportedTarget(msg) = &e.kind else { panic!("{e:?}") };
        assert!(msg.contains("overflowing u64"), "{msg}");
    });
}

#[test]
fn a_type_the_generator_cannot_enumerate_is_refused_with_its_code() {
    // A symbolic-width byte string has no finite domain.
    let src = "warrant t v1\n  op f<N: Nat>(a: Bytes<N>) -> U8\n";
    with_file(src, |file| {
        let env = DomainEnv::new(file);
        let Declaration::Op(op) = &file.declarations[0] else { unreachable!() };
        let e = resolve_binders(&env, &op.params, None, EXHAUSTIVE_BUDGET, "f", op.span);
        assert_eq!(e.unwrap_err().code, DiagnosticCode::W0701);
    });
}

#[test]
fn v0_documents_resolve_without_demotion_or_fixed_widths() {
    let src = "warrant t\n  op f(a: U8, n: Nat, d: Bytes, x: U64) -> U8\n";
    with_file(src, |file| {
        let env = DomainEnv::new(file);
        let Declaration::Op(op) = &file.declarations[0] else { unreachable!() };
        let r = resolve_binders(&env, &op.params, None, u64::MAX, "f", op.span);
        // 256 * 65 * 347 * 377 fits a u64: resolved as declared.
        let r = r.unwrap();
        assert_eq!(r.domains, [Domain::U8, Domain::Nat, Domain::Bytes, Domain::U64]);
        assert!(r.demotions.is_empty());
        assert_eq!(r.product, 256 * 65 * 347 * 377);
        // v0 U8 / U64 are the evaluator's U8 / U64 values.
        assert_eq!(r.domains[0].nth(7), Some(EvalValue::U8(7)));
        assert_eq!(r.domains[3].nth(256), Some(EvalValue::U64(1 << 8)));
    });
}

// ---------------------------------------------------------------------------
// The post-state slot resolver
// ---------------------------------------------------------------------------

#[test]
fn op_post_slot_resolves_each_slot_rule_over_the_wxg_fixture() {
    with_file(WXG, |file| {
        let env = DomainEnv::new(file);
        let slot = |op: &str| op_post_slot(&env, file, op).unwrap().map(|s| s.slot);
        assert_eq!(slot("wipe"), Some(PostSlot::S1));
        assert_eq!(slot("bump"), Some(PostSlot::S1));
        assert_eq!(slot("clear_nonempty"), Some(PostSlot::S2));
        assert_eq!(slot("push_byte"), Some(PostSlot::S3 { h: 1 }));
        assert_eq!(slot("pair"), Some(PostSlot::S3 { h: 2 }));
        assert_eq!(slot("encode"), Some(PostSlot::S4 { h: 1 }));
        // No such operation.
        assert_eq!(slot("nope"), None);
        // The detail: the posts and the result.
        let s = op_post_slot(&env, file, "push_byte").unwrap().unwrap();
        assert_eq!(s.inout, ["out"]);
        assert_eq!(s.posts, [Domain::Bytes]);
        assert_eq!(s.result, Domain::Tuple(vec![Domain::Nat, Domain::Bytes]));
    });
}

#[test]
fn an_operation_without_inout_parameters_has_no_post_slot() {
    with_file(WXA, |file| {
        let env = DomainEnv::new(file);
        assert_eq!(op_post_slot(&env, file, "split").unwrap(), None);
    });
}

#[test]
fn domain_post_slots_agree_with_the_checker_and_the_corpus_descriptor_names() {
    with_file(WXG, |file| {
        let checked = check_warrant(file).expect("checks");
        let env = DomainEnv::new(file);
        let mut compared = 0;
        for d in &file.declarations {
            let Declaration::Op(op) = d else { continue };
            let ours = op_post_slot(&env, file, op.name).unwrap();
            let theirs = inout_descriptor(file, &checked, op.name);
            assert_eq!(ours.as_ref().map(|s| s.slot.name()), theirs.as_ref().map(|t| t.post_slot), "{}", op.name);
            if let (Some(o), Some(t)) = (&ours, &theirs) {
                assert_eq!(o.inout, t.inout, "{}", op.name);
                compared += 1;
            }
        }
        assert!(compared >= 9, "only {compared} inout operations compared");
    });
}

#[test]
fn resolve_post_slot_of_is_the_generic_rule_over_domains() {
    let bytes = Domain::Bytes;
    let nat = Domain::Nat;
    let tup = |cs: Vec<Domain>| Domain::Tuple(cs);
    let res = |t: Domain| Domain::Res(Box::new(t), vec!["E".into()]);
    let posts = [bytes.clone()];
    assert_eq!(resolve_post_slot_of(&posts, &bytes), Some(PostSlot::S1));
    assert_eq!(resolve_post_slot_of(&posts, &res(bytes.clone())), Some(PostSlot::S2));
    assert_eq!(resolve_post_slot_of(&posts, &tup(vec![nat.clone(), bytes.clone()])), Some(PostSlot::S3 { h: 1 }));
    assert_eq!(resolve_post_slot_of(&posts, &res(tup(vec![nat.clone(), bytes.clone()]))), Some(PostSlot::S4 { h: 1 }));
    // The last k components must be the posts, in order.
    assert_eq!(resolve_post_slot_of(&posts, &tup(vec![bytes.clone(), nat.clone()])), None);
    assert_eq!(resolve_post_slot_of(&[bytes.clone(), nat.clone()], &bytes), None);
    // It is the same rule as the re-exported generic resolver.
    let shape = SlotShape::Tuple(vec![nat.clone(), bytes.clone()]);
    assert_eq!(resolve_post_slot_of(&posts, &tup(vec![nat, bytes.clone()])), resolve_post_slot(&posts, &shape));
    // slot_shape is re-exported alongside (checker types).
    let _ = slot_shape;
}
