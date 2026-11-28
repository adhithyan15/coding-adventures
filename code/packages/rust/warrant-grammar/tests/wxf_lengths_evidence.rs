//! WX-F slice 5b: lengths hints, tested by *running* what the generator
//! emits. The generated harness (`tests/generated/wxf_lengths_evidence.rs`,
//! checked in and drift-tested by `tests/wxf_lengths_regen.rs`) is included
//! as a module: its hinted domains, the property bias and the manifest text
//! are asserted against an honest summing subject.

use coding_adventures_warrant_grammar::evidence::Ty as GenTy;

#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod harness {
    /// An honest summing subject: `sum1` is the 20-byte wrapped sum, so all
    /// claim suites pass and the domain assertions below see real drives.
    pub mod wxf_subject {
        #[derive(Debug, Default)]
        pub struct Digest {
            acc: Vec<u8>,
        }

        impl Digest {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn update(&mut self, chunk: &[u8]) {
                self.acc.extend_from_slice(chunk);
            }
            pub fn sum1(&self) -> [u8; 20] {
                let mut out = [0u8; 20];
                for (i, b) in self.acc.iter().enumerate() {
                    out[i % 20] = out[i % 20].wrapping_add(*b);
                }
                out
            }
        }
    }

    include!("generated/wxf_lengths_evidence.rs");

    /// `n` draws of `ty` from a seeded stream: the relation bias entry point
    /// the property and fuzz suites share.
    pub fn fresh_many(ty: Ty, seed: u64, n: usize) -> Vec<V> {
        let mut rng = SplitMix64::new(seed);
        (0..n).map(|_| fresh(ty, &mut RngSrc(&mut rng))).collect()
    }

    /// `n` relations of `prev` in `mode` for binder type `ty`.
    pub fn relate_many(prev: V, ty: Ty, mode: u8, seed: u64, n: usize) -> Vec<V> {
        let mut rng = SplitMix64::new(seed);
        (0..n)
            .map(|_| relate(&prev, ty, mode, &mut RngSrc(&mut rng)))
            .collect()
    }
}

use harness::{fresh_many, relate_many, Ty, V};

const HINTS: &[u64] = &[55, 64];
const BASE: u64 = 347;

#[test]
fn hinted_domain_holds_base_plus_fills_in_order() {
    let base = harness::domain(Ty::Bytes);
    let full = harness::domain(Ty::BytesHinted(HINTS));
    assert_eq!(base.len() as u64, BASE);
    assert_eq!(full.len() as u64, BASE + 2 * HINTS.len() as u64);
    assert_eq!(&full[..BASE as usize], &base[..], "base first, unchanged");
    let tail = &full[BASE as usize..];
    let expect: Vec<V> = HINTS
        .iter()
        .flat_map(|l| {
            [
                V::Bytes(vec![0x00; *l as usize]),
                V::Bytes(vec![0xa5; *l as usize]),
            ]
        })
        .collect();
    assert_eq!(tail, expect.as_slice());
}

#[test]
fn closed_forms_match_on_both_sides() {
    let ty = Ty::BytesHinted(HINTS);
    let size = harness::domain_size(ty).expect("sized");
    assert_eq!(harness::domain(ty).len() as u64, size);
    // The generator's mirror agrees exactly (budget decisions match).
    let gen = GenTy::BytesHinted(HINTS.to_vec());
    assert_eq!(gen.domain_size(), Some(size));
    assert_eq!(gen.domain_bytes(), harness::domain_bytes(ty));
}

#[test]
fn fresh_draws_hinted_lengths_about_a_quarter_of_the_time() {
    let is_fill = |v: &V| match v {
        V::Bytes(b) if b.len() == 55 || b.len() == 64 => {
            b.iter().all(|x| *x == 0x00) || b.iter().all(|x| *x == 0xa5)
        }
        _ => false,
    };
    // Deterministic: the same seed draws the same stream twice.
    let first = fresh_many(Ty::BytesHinted(HINTS), 7, 4000);
    let second = fresh_many(Ty::BytesHinted(HINTS), 7, 4000);
    assert_eq!(first, second);
    // Binomial(4000, 1/4): mean 1000, sd ~27; +-110 is 4 sigma either way.
    let fills = first.iter().filter(|v| is_fill(v)).count();
    assert!(
        (890..=1110).contains(&fills),
        "hinted fraction off: {fills}/4000 fills"
    );
}

fn flat_bytes(v: &V) -> Vec<u8> {
    match v {
        V::List(vs) => vs
            .iter()
            .flat_map(|e| match e {
                V::Bytes(b) => b.clone(),
                _ => panic!("marked lists hold bytes, got {e:?}"),
            })
            .collect(),
        other => panic!("expected a list, got {other:?}"),
    }
}

#[test]
fn rechunking_preserves_bytes_under_a_new_split() {
    let prev = V::List(vec![
        V::Bytes(vec![0x00, 0x01]),
        V::Bytes(vec![0x02]),
        V::Bytes(vec![0x03, 0x04, 0x05]),
    ]);
    let want = flat_bytes(&prev);
    for mode in [2u8, 3u8] {
        let outs = relate_many(prev.clone(), Ty::ListDriven, mode, 11, 64);
        // Same bytes, always two chunks (one split point).
        for o in &outs {
            assert_eq!(flat_bytes(o), want, "mode {mode} changed bytes");
            match o {
                V::List(vs) => assert_eq!(vs.len(), 2, "mode {mode} shape"),
                other => panic!("mode {mode}: {other:?}"),
            }
        }
        // The split point varies: not every draw repeats one chunking.
        let distinct: std::collections::HashSet<Vec<usize>> = outs
            .iter()
            .map(|o| match o {
                V::List(vs) => vs
                    .iter()
                    .map(|e| match e {
                        V::Bytes(b) => b.len(),
                        _ => 0,
                    })
                    .collect(),
                _ => vec![],
            })
            .collect();
        assert!(distinct.len() >= 3, "mode {mode}: splits never vary");
    }
}

#[test]
fn rechunking_edges_and_unmarked_control() {
    // Empty concatenations split into two empty chunks; singletons split too.
    let empty = relate_many(V::List(vec![]), Ty::ListDriven, 2, 1, 1);
    assert_eq!(
        empty[0],
        V::List(vec![V::Bytes(vec![]), V::Bytes(vec![])]),
        "empty edge"
    );
    let one = relate_many(
        V::List(vec![V::Bytes(vec![0x09])]),
        Ty::ListDriven,
        3,
        1,
        1,
    );
    assert_eq!(flat_bytes(&one[0]), vec![0x09]);
    // Unmarked lists still clone on modes 2-3 (the pre-existing relation).
    let prev = V::List(vec![V::Bytes(vec![0x01]), V::Bytes(vec![0x02])]);
    for mode in [2u8, 3u8] {
        let outs = relate_many(prev.clone(), Ty::List(&Ty::Bytes), mode, 5, 8);
        assert!(outs.iter().all(|o| *o == prev), "mode {mode} must clone");
    }
}

#[test]
fn marked_binders_enumerate_like_plain_lists() {
    let marked = Ty::ListDriven;
    let plain = Ty::List(&Ty::Bytes);
    assert_eq!(harness::domain(marked), harness::domain(plain));
    assert_eq!(harness::domain_size(marked), harness::domain_size(plain));
    assert_eq!(harness::domain_bytes(marked), harness::domain_bytes(plain));
}

#[test]
fn manifest_records_hinted_domains() {
    use coding_adventures_test_bootstrap::run::Options;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(harness::Fault::None);
    let mut tap = Vec::new();
    let (_summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut tap)
        .expect("run");
    std::panic::set_hook(previous);
    let out = std::env::temp_dir().join(format!("wxf_lengths_{}.json", std::process::id()));
    std::env::set_var("WARRANT_EVIDENCE_OUT", &out);
    harness::write_manifest_for(&outcomes);
    std::env::remove_var("WARRANT_EVIDENCE_OUT");
    let manifest = std::fs::read_to_string(&out).expect("manifest written");
    let _ = std::fs::remove_file(&out);
    assert!(
        manifest.contains("bytes_d1+lengths(55,64)"),
        "{manifest}"
    );
    // The honesty check: every suite passes, so the assertions above see a
    // working harness, not vacuous failures (negctls fail: `scan([m]) ==
    // scan([m])` is a self-comparison no drive rewrite can catch).
    for o in &outcomes {
        if o.id.contains("/negctl/") {
            assert!(!o.passed(), "self-comparison caught a fault: {}", o.id);
        } else {
            assert!(o.passed(), "honest suite failed: {}", o.id);
        }
    }
}
