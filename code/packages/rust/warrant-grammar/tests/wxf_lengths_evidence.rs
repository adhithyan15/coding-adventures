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
}

use harness::{fresh_many, Ty, V};

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
