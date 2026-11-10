//! # PRNG contracts: published vectors, an independent reference, bounded draws
//!
//! These tests pin `prng.rs` against evidence that is *not* the module's own
//! arithmetic:
//!
//! 1. **Published vectors.** SplitMix64 from seed 0 (Vigna's reference
//!    implementation, as quoted all over the literature) and from seed
//!    1234567 (the vector shipped in `rand_xoshiro`'s test-suite), and the
//!    FNV-1a test vectors from the FNV authors' page (`""`, `"a"`, `"foo"`,
//!    `"foobar"`).
//! 2. **An independent reference.** A second SplitMix64 written with `u128`
//!    arithmetic and an explicit `% 2^64` instead of wrapping operations, so
//!    a wrong constant or a wrong shift in `prng.rs` cannot also be wrong
//!    here by copy-paste of the same expression.
//! 3. **Bounded draws** compared against a from-the-spec `u128` rejection
//!    sampler, for every `n` in a dense range plus the 2^32 limit.
//! 4. **Sub-seed independence**: a test's seed depends on (run seed, id) and
//!    on nothing else, and distinct ids give distinct seeds over an
//!    exhaustive bounded domain.
//!
//! This is a libtest target on purpose: the harness under test must not be
//! the thing that reports on itself.

use std::collections::HashSet;

use coding_adventures_test_bootstrap::prng::{fnv1a64, shuffled, sub_seed, SplitMix64};

const TWO_64: u128 = 1 << 64;

/// SplitMix64 re-derived from TF01 §5.1 using u128 arithmetic.
struct Reference {
    state: u128,
}

impl Reference {
    fn new(seed: u64) -> Self {
        Reference {
            state: u128::from(seed),
        }
    }

    fn next(&mut self) -> u64 {
        self.state = (self.state + 0x9e37_79b9_7f4a_7c15) % TWO_64;
        let mut z = self.state;
        z = ((z ^ (z >> 30)) * 0xbf58_476d_1ce4_e5b9) % TWO_64;
        z = ((z ^ (z >> 27)) * 0x94d0_49bb_1331_11eb) % TWO_64;
        (z ^ (z >> 31)) as u64
    }

    /// `bounded(n)` exactly as TF01 §5.1 words it.
    fn bounded(&mut self, n: u64) -> u64 {
        let n = u128::from(n);
        let t = (TWO_64 - n) % n;
        loop {
            let x = u128::from(self.next());
            if x >= t {
                return (x % n) as u64;
            }
        }
    }
}

#[test]
fn splitmix64_seed_zero_published_vector() {
    let mut rng = SplitMix64::new(0);
    let got: Vec<u64> = (0..5).map(|_| rng.next_u64()).collect();
    assert_eq!(
        got,
        [
            0xe220_a839_7b1d_cdaf,
            0x6e78_9e6a_a1b9_65f4,
            0x06c4_5d18_8009_454f,
            0xf88b_b8a8_724c_81ec,
            0x1b39_896a_51a8_749b,
        ]
    );
}

#[test]
fn splitmix64_seed_1234567_published_vector() {
    let mut rng = SplitMix64::new(1_234_567);
    let got: Vec<u64> = (0..5).map(|_| rng.next_u64()).collect();
    assert_eq!(
        got,
        [
            6_457_827_717_110_365_317,
            3_203_168_211_198_807_973,
            9_817_491_932_198_370_423,
            4_593_380_528_125_082_431,
            16_408_922_859_458_223_821,
        ]
    );
}

#[test]
fn splitmix64_matches_the_independent_reference_over_many_seeds() {
    let mut seeds: Vec<u64> = vec![0, 1, 2, u64::MAX, u64::MAX - 1, 1 << 63, (1 << 63) - 1];
    // A spread of seeds produced by the generator itself, 1000 outputs each.
    let mut meta = SplitMix64::new(0xC0FFEE);
    seeds.extend((0..200).map(|_| meta.next_u64()));
    for seed in seeds {
        let mut got = SplitMix64::new(seed);
        let mut want = Reference::new(seed);
        for step in 0..1000 {
            assert_eq!(got.next_u64(), want.next(), "seed {seed:#x}, step {step}");
        }
    }
}

#[test]
fn splitmix64_state_wraps_across_the_u64_boundary() {
    // Seeds within one gamma of 2^64 force the wrapping add on the first step.
    for seed in [u64::MAX, u64::MAX - 0x9e37_79b9_7f4a_7c14, u64::MAX - 5] {
        let mut got = SplitMix64::new(seed);
        let mut want = Reference::new(seed);
        for _ in 0..16 {
            assert_eq!(got.next_u64(), want.next());
        }
    }
}

#[test]
fn splitmix64_is_a_pure_function_of_the_seed() {
    let mut a = SplitMix64::new(77);
    let mut b = a.clone();
    for _ in 0..64 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
    // Distinct seeds diverge immediately (the finaliser is a bijection, so
    // first outputs of distinct states can never coincide).
    let firsts: HashSet<u64> = (0..5000u64)
        .map(|s| SplitMix64::new(s).next_u64())
        .collect();
    assert_eq!(firsts.len(), 5000);
}

#[test]
fn fnv1a64_published_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foo"), 0xdcb2_7518_fed9_d577);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn fnv1a64_is_over_bytes_not_chars() {
    // "é" is two UTF-8 bytes; hashing chars-as-u8 would give a different value.
    let by_bytes = fnv1a64("é".as_bytes());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in [0xc3u8, 0xa9] {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    assert_eq!(by_bytes, h);
}

#[test]
fn bounded_matches_the_spec_sampler_for_every_small_n() {
    for n in 1..=3000u64 {
        let mut got = SplitMix64::new(n);
        let mut want = Reference::new(n);
        for _ in 0..20 {
            assert_eq!(got.bounded(n), want.bounded(n), "n = {n}");
        }
    }
}

#[test]
fn bounded_matches_the_spec_sampler_at_the_large_edges() {
    let edges = [
        (1u64 << 32) - 1,
        1 << 32,
        (1 << 31) + 1,
        (1 << 31),
        (1 << 31) - 1,
        3_000_000_019,
        4_294_967_291, // the largest prime below 2^32
    ];
    for n in edges {
        for seed in 0..50u64 {
            let mut got = SplitMix64::new(seed);
            let mut want = Reference::new(seed);
            for _ in 0..40 {
                assert_eq!(got.bounded(n), want.bounded(n), "n = {n}, seed = {seed}");
            }
        }
    }
}

#[test]
fn bounded_one_is_always_zero_and_consumes_exactly_one_output() {
    // threshold = (2^64 - 1) % 1 = 0, so the first draw is always accepted:
    // two generators stay in lock-step after one `bounded(1)` and one `next_u64`.
    let mut a = SplitMix64::new(5);
    let mut b = SplitMix64::new(5);
    assert_eq!(a.bounded(1), 0);
    b.next_u64();
    assert_eq!(a.next_u64(), b.next_u64());
}

#[test]
fn bounded_rejection_rule_is_unbiased_on_a_tiny_word() {
    // The algorithm on a 4-bit "word": n = 6, t = (16 - 6) % 6 = 4, so the
    // accepted outputs are 4..=15 (12 values) and every residue 0..5 has
    // exactly two preimages. Plain `x % 6` over 0..=15 would give residues
    // 0..=3 three preimages and 4..=5 only two.
    let (word, n) = (16u32, 6u32);
    let t = (word - n) % n;
    let mut counts = [0u32; 6];
    for x in t..word {
        counts[(x % n) as usize] += 1;
    }
    assert_eq!(counts, [2; 6]);
}

#[test]
fn bounded_rejects_outputs_below_the_threshold() {
    // Random draws never reach the rejection branch (an output below
    // t < 2^32 has probability 2^-32), so a statistical test cannot tell
    // `bounded` from a version with no threshold. The finalizer is a
    // bijection and the state that outputs 0 is 0, so the seed -gamma
    // (2^64 - 0x9e3779b97f4a7c15) has first output exactly 0.
    let seed = 0x61c8_8646_80b5_83eb_u64;
    let mut probe = SplitMix64::new(seed);
    let outputs = [probe.next_u64(), probe.next_u64(), probe.next_u64()];
    assert_eq!(outputs[0], 0);
    assert_eq!(outputs[1], 16_294_208_416_658_607_535);
    // n = 3: t = (2^64 - 3) mod 3 = 1, so the output 0 is rejected and the
    // answer comes from the second output (16294208416658607535 mod 3 = 1,
    // where accepting 0 would have answered 0).
    for (n, want) in [(3u64, 1u64), (6, 1), (10, 5), (1000, 535), (7, 2)] {
        let mut rng = SplitMix64::new(seed);
        assert_eq!(rng.bounded(n), want, "n = {n}");
        let mut reference = Reference::new(seed);
        assert_eq!(reference.bounded(n), want, "reference, n = {n}");
        // Two outputs were consumed when the first was rejected (n = 7 has
        // t = 2 and so also rejects 0).
        assert_eq!(
            rng.next_u64(),
            outputs[2],
            "n = {n} consumed the wrong number of outputs"
        );
    }
    // n = 4 divides 2^64: threshold 0, nothing is ever rejected, so 0 is
    // accepted and exactly one output is consumed.
    let mut rng = SplitMix64::new(seed);
    assert_eq!(rng.bounded(4), 0);
    assert_eq!(rng.next_u64(), outputs[1]);
}

#[test]
fn bounded_three_is_statistically_flat() {
    // 300_000 draws, each bucket within 1% of a third. (The exactness
    // claim is carried by the spec-sampler comparison above; this guards
    // against a gross distribution bug such as an off-by-one range.)
    let mut rng = SplitMix64::new(2024);
    let mut buckets = [0u32; 3];
    for _ in 0..300_000 {
        buckets[rng.bounded(3) as usize] += 1;
    }
    for count in buckets {
        assert!((99_000..=101_000).contains(&count), "{buckets:?}");
    }
}

#[test]
fn sub_seed_is_the_first_output_of_the_xored_seed() {
    for run_seed in [0u64, 1, 42, u64::MAX] {
        for id in ["", "a", "ct-compare/eq", "x/y.z_9-A"] {
            let want = Reference::new(run_seed ^ fnv1a64(id.as_bytes())).next();
            assert_eq!(sub_seed(run_seed, id), want, "{run_seed} {id}");
        }
    }
}

#[test]
fn sub_seed_depends_only_on_seed_and_id() {
    // Adding or removing other tests changes nothing: the function takes no
    // other input, and the shuffle position does not feed it.
    let alone = sub_seed(9, "needle");
    for extra in 0..50 {
        let ids: Vec<String> = (0..extra)
            .map(|i| format!("hay{i}"))
            .chain(["needle".to_string()])
            .collect();
        let _ = shuffled(9, &ids);
        assert_eq!(sub_seed(9, "needle"), alone);
    }
}

#[test]
fn sub_seeds_are_distinct_over_an_exhaustive_id_domain() {
    // Every id of length 1..=4 over a 6-letter alphabet, and every run seed in 0..4.
    let alphabet = ['a', 'b', 'z', '0', '/', '-'];
    let mut ids: Vec<String> = Vec::new();
    let mut frontier = vec![String::new()];
    for _ in 0..4 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for c in alphabet {
                next.push(format!("{prefix}{c}"));
            }
        }
        ids.extend(next.iter().cloned());
        frontier = next;
    }
    assert_eq!(ids.len(), 6 + 36 + 216 + 1296);
    for run_seed in 0..4u64 {
        let seeds: HashSet<u64> = ids.iter().map(|id| sub_seed(run_seed, id)).collect();
        assert_eq!(
            seeds.len(),
            ids.len(),
            "collision under run seed {run_seed}"
        );
    }
    // FNV-1a itself has no collisions on this domain either.
    let hashes: HashSet<u64> = ids.iter().map(|id| fnv1a64(id.as_bytes())).collect();
    assert_eq!(hashes.len(), ids.len());
}

#[test]
fn shuffle_is_a_permutation_of_the_sorted_ids_for_every_seed() {
    let ids = ["d", "a", "c", "b", "e", "ab", "A", "_", "0"];
    let mut sorted: Vec<String> = ids.iter().map(|s| (*s).to_owned()).collect();
    sorted.sort();
    for seed in 0..500u64 {
        let mut got = shuffled(seed, &ids);
        got.sort();
        assert_eq!(got, sorted, "seed {seed}");
    }
}

#[test]
fn shuffle_matches_an_independent_fisher_yates() {
    // Re-derive TF01 §5.2 step 3 from the reference generator.
    let ids = ["delta", "alpha", "charlie", "bravo", "echo", "foxtrot"];
    for seed in [0u64, 1, 7, 99, u64::MAX] {
        let mut order: Vec<&str> = ids.to_vec();
        order.sort_unstable();
        let mut rng = Reference::new(seed);
        for i in (1..order.len()).rev() {
            let j = rng.bounded(i as u64 + 1) as usize;
            order.swap(i, j);
        }
        assert_eq!(shuffled(seed, &ids), order, "seed {seed}");
    }
}

#[test]
fn shuffle_uses_byte_order_not_locale_or_length() {
    // Uppercase sorts before lowercase, '-' before digits before letters.
    let got = shuffled(0, &["b", "B", "a", "A", "-", "0", "_"]);
    let mut by_bytes = vec!["b", "B", "a", "A", "-", "0", "_"];
    by_bytes.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
    let mut got_sorted = got.clone();
    got_sorted.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
    assert_eq!(got_sorted, by_bytes);
}

#[test]
fn shuffle_of_zero_and_one_ids_is_trivial() {
    let none: [&str; 0] = [];
    assert!(shuffled(3, &none).is_empty());
    assert_eq!(shuffled(3, &["only"]), ["only"]);
}

#[test]
fn shuffle_reaches_every_permutation_of_three_ids() {
    // Fisher–Yates over a uniform bounded() must be able to produce all 3! = 6
    // orders; a broken swap index (e.g. always swapping with i-1) cannot.
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    for seed in 0..200u64 {
        seen.insert(shuffled(seed, &["a", "b", "c"]));
    }
    assert_eq!(seen.len(), 6);
}
