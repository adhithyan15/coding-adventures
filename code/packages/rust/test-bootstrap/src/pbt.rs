//! # Property-Based Testing Engine (VER05)
//!
//! Level 2 property testing engine providing generator algebra, integrated
//! shrinking, deterministic PRNG replay with isolated sub-seeds, step budgets,
//! closed failure vocabulary, and privacy-preserving failure reporting.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::prng::{fnv1a64, SplitMix64};
use crate::tfv::Value;
use crate::verdict::Verdict;

/// Closed failure vocabulary for property testing (VER05 §5.1).
pub const FID_COUNTEREXAMPLE: &str = "property-counterexample";
pub const FID_EXHAUSTED: &str = "property-exhausted";
pub const FID_TIMEOUT: &str = "property-timeout";
pub const FID_PANIC: &str = "property-panic";

/// Check if a failure ID is in the closed PBT vocabulary (VER05 §5.1).
pub fn is_registered_failure_id(id: &str) -> bool {
    matches!(
        id,
        FID_COUNTEREXAMPLE | FID_EXHAUSTED | FID_TIMEOUT | FID_PANIC
    )
}

// --- Primitive Generators (VER05 §3.1) -------------------------------------

/// Generate a pseudo-random boolean.
pub fn gen_bool(rng: &mut SplitMix64) -> bool {
    rng.bounded(2) == 1
}

/// Generate a pseudo-random 8-bit unsigned integer.
pub fn gen_u8(rng: &mut SplitMix64) -> u8 {
    rng.bounded(256) as u8
}

/// Generate a pseudo-random 64-bit unsigned integer.
pub fn gen_u64(rng: &mut SplitMix64) -> u64 {
    rng.next_u64()
}

/// Generate a pseudo-random 128-bit signed integer.
pub fn gen_i128(rng: &mut SplitMix64) -> i128 {
    let w1 = rng.next_u64();
    let w2 = rng.next_u64();
    let val = ((w1 as u128) << 64) | (w2 as u128);
    val as i128
}

/// Reference implementation for primitive generator conformance (VER05 §6.2).
pub fn gen_primitive(seed: u64, type_name: &str, count: usize) -> Result<Vec<Value>, String> {
    let mut rng = SplitMix64::new(seed);
    let mut outputs = Vec::with_capacity(count);
    for _ in 0..count {
        match type_name {
            "bool" => outputs.push(Value::Bool(gen_bool(&mut rng))),
            "u8" => outputs.push(Value::Int((gen_u8(&mut rng) as u64).to_string())),
            "u64" => outputs.push(Value::Int(gen_u64(&mut rng).to_string())),
            "i128" => outputs.push(Value::Int(gen_i128(&mut rng).to_string())),
            other => return Err(format!("unknown primitive type: {other}")),
        }
    }
    Ok(outputs)
}

// --- Bounded Integer Generator (VER05 §3.1) ---------------------------------

/// Unbiased pseudo-random draw in `0..n` for `1 <= n <= 2^64`.
pub fn bounded_u64(rng: &mut SplitMix64, n: u64) -> u64 {
    if n <= (1 << 32) {
        rng.bounded(n)
    } else {
        let threshold = n.wrapping_neg() % n;
        loop {
            let x = rng.next_u64();
            if x >= threshold {
                return x % n;
            }
        }
    }
}

/// Generate an unbiased random integer in `[min_val, max_val]`.
pub fn gen_bounded_int(rng: &mut SplitMix64, min_val: i128, max_val: i128) -> i128 {
    assert!(min_val <= max_val, "min_val must be <= max_val");
    let span = (max_val - min_val) as u128 + 1;
    let offset = if span <= (1u128 << 32) {
        rng.bounded(span as u64) as u128
    } else if span < (1u128 << 64) {
        bounded_u64(rng, span as u64) as u128
    } else if span == (1u128 << 64) {
        rng.next_u64() as u128
    } else {
        let w1 = rng.next_u64() as u128;
        let w2 = rng.next_u64() as u128;
        let raw = (w1 << 64) | w2;
        raw % span
    };
    min_val + offset as i128
}

/// Reference implementation for bounded generator conformance (VER05 §6.2).
pub fn gen_bounded(
    seed: u64,
    min_val: i128,
    max_val: i128,
    count: usize,
) -> Result<Vec<Value>, String> {
    if min_val > max_val {
        return Err("min_val must be <= max_val".into());
    }
    let mut rng = SplitMix64::new(seed);
    let mut outputs = Vec::with_capacity(count);
    for _ in 0..count {
        let val = gen_bounded_int(&mut rng, min_val, max_val);
        outputs.push(Value::Int(val.to_string()));
    }
    Ok(outputs)
}

// --- Bytes Generator (VER05 §3.1) -------------------------------------------

/// Generate a pseudo-random byte sequence with length in `[min_len, max_len]`.
pub fn gen_bytes_slice(rng: &mut SplitMix64, min_len: usize, max_len: usize) -> Vec<u8> {
    assert!(min_len <= max_len, "min_len must be <= max_len");
    let span = (max_len - min_len + 1) as u64;
    let length = min_len + rng.bounded(span) as usize;
    let mut data = Vec::with_capacity(length);
    for _ in 0..length {
        data.push(rng.bounded(256) as u8);
    }
    data
}

/// Reference implementation for bytes generator conformance (VER05 §6.2).
pub fn gen_bytes(
    seed: u64,
    min_len: usize,
    max_len: usize,
    count: usize,
) -> Result<Vec<Value>, String> {
    if min_len > max_len {
        return Err("min_len must be <= max_len".into());
    }
    let mut rng = SplitMix64::new(seed);
    let mut outputs = Vec::with_capacity(count);
    for _ in 0..count {
        let data = gen_bytes_slice(&mut rng, min_len, max_len);
        outputs.push(Value::Bytes(data));
    }
    Ok(outputs)
}

// --- Integrated Shrinking (VER05 §4) ---------------------------------------

/// Shrink an integer towards `target` while staying in `[min_val, max_val]`
/// and maintaining the falsification invariant `!prop(val)`.
pub fn shrink_int(
    initial: i128,
    target: i128,
    min_val: i128,
    max_val: i128,
    prop: impl Fn(i128) -> bool,
    max_steps: usize,
) -> (i128, usize) {
    assert!(!prop(initial), "initial value must falsify property");
    let mut current = initial;
    let mut steps = 0;

    if min_val <= target && target <= max_val && target != current && !prop(target) {
        return (target, 1);
    }

    while steps < max_steps {
        if current == target {
            break;
        }
        let diff = current - target;
        let mut progress = false;

        if diff.abs() > 1 {
            let step = diff.div_euclid(2);
            let candidate = current - step;
            if min_val <= candidate
                && candidate <= max_val
                && !prop(candidate)
                && candidate != current
            {
                current = candidate;
                steps += 1;
                progress = true;
            }
        }

        if !progress {
            let step = if diff > 0 { 1 } else { -1 };
            let candidate = current - step;
            if min_val <= candidate
                && candidate <= max_val
                && !prop(candidate)
                && candidate != current
            {
                current = candidate;
                steps += 1;
                progress = true;
            }
        }

        if !progress {
            break;
        }
    }

    (current, steps)
}

/// Shrink a sequence of integers towards `elem_target` and minimal length
/// while maintaining the falsification invariant `!prop(slice)`.
pub fn shrink_list(
    initial: &[i128],
    elem_target: i128,
    prop: impl Fn(&[i128]) -> bool,
    max_steps: usize,
) -> (Vec<i128>, usize) {
    assert!(!prop(initial), "initial list must falsify property");
    let mut current = initial.to_vec();
    let mut steps = 0;

    if !prop(&[]) && !current.is_empty() {
        return (Vec::new(), 1);
    }

    // Phase 1a: Bisection
    while current.len() > 1 && steps < max_steps {
        let mid = current.len() / 2;
        let left = &current[..mid];
        if !prop(left) {
            current = left.to_vec();
            steps += 1;
            continue;
        }
        let right = &current[mid..];
        if !prop(right) {
            current = right.to_vec();
            steps += 1;
            continue;
        }
        break;
    }

    // Phase 1b: Single-element deletion from right to left
    if !current.is_empty() {
        let mut i = current.len() - 1;
        while current.len() > 1 && steps < max_steps {
            let mut cand = current[..i].to_vec();
            cand.extend_from_slice(&current[i + 1..]);
            if !prop(&cand) {
                current = cand;
                steps += 1;
                if i >= current.len() {
                    i = current.len() - 1;
                }
            } else if i == 0 {
                break;
            } else {
                i -= 1;
            }
        }
    }

    // Phase 2: Element-wise minimization
    for i in 0..current.len() {
        while steps < max_steps {
            if current[i] == elem_target {
                break;
            }
            let diff = current[i] - elem_target;
            let mut progress = false;
            if diff.abs() > 1 {
                let step = diff.div_euclid(2);
                let cand_val = current[i] - step;
                let mut cand_list = current.clone();
                cand_list[i] = cand_val;
                if !prop(&cand_list) {
                    current = cand_list;
                    steps += 1;
                    progress = true;
                }
            }
            if !progress {
                let step = if diff > 0 { 1 } else { -1 };
                let cand_val = current[i] - step;
                let mut cand_list = current.clone();
                cand_list[i] = cand_val;
                if !prop(&cand_list) {
                    current = cand_list;
                    steps += 1;
                    progress = true;
                }
            }
            if !progress {
                break;
            }
        }
    }

    (current, steps)
}

/// Shrink a byte sequence towards zero bytes and minimal length.
pub fn shrink_bytes(
    initial: &[u8],
    prop: impl Fn(&[u8]) -> bool,
    max_steps: usize,
) -> (Vec<u8>, usize) {
    let init_i128: Vec<i128> = initial.iter().map(|&b| b as i128).collect();
    let (shrunk_i128, steps) = shrink_list(
        &init_i128,
        0,
        |slice| {
            let bytes: Vec<u8> = slice.iter().map(|&v| v as u8).collect();
            prop(&bytes)
        },
        max_steps,
    );
    let shrunk_bytes: Vec<u8> = shrunk_i128.iter().map(|&v| v as u8).collect();
    (shrunk_bytes, steps)
}

/// Simulate deterministic replay to reproduce initial failing draw and minimal witness.
pub fn simulate_replay<F>(seed: u64, prop: F) -> Result<(i128, i128), &'static str>
where
    F: Fn(i128) -> bool,
{
    let mut rng = SplitMix64::new(seed);
    let mut initial_fail: Option<i128> = None;
    for _ in 0..1000 {
        let val = rng.bounded(1000) as i128;
        if !prop(val) {
            initial_fail = Some(val);
            break;
        }
    }
    let initial_fail = initial_fail.ok_or("property was not falsified within 1000 draws")?;
    let (minimal, _) = shrink_int(initial_fail, 0, 0, 1000, &prop, 1000);
    Ok((initial_fail, minimal))
}

// --- Generator Combinators DSL (VER05 §3) -----------------------------------

/// Sampling function signature for generators.
pub type GeneratorFn<T> = Box<dyn Fn(&mut SplitMix64) -> Result<T, String>>;

/// Composable typed generator function.
pub struct Gen<T> {
    generate_fn: GeneratorFn<T>,
}

impl<T: 'static> Gen<T> {
    /// Construct a generator from a raw sampling closure.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut SplitMix64) -> Result<T, String> + 'static,
    {
        Gen {
            generate_fn: Box::new(f),
        }
    }

    /// Sample one instance with the provided PRNG.
    pub fn sample(&self, rng: &mut SplitMix64) -> Result<T, String> {
        (self.generate_fn)(rng)
    }

    /// Transform generated values with a function.
    pub fn map<U: 'static, F>(self, f: F) -> Gen<U>
    where
        F: Fn(T) -> U + 'static,
    {
        Gen::new(move |rng| self.sample(rng).map(&f))
    }

    /// Constrain generated values with a precondition filter and max retries.
    pub fn filter<P>(self, predicate: P, max_retries: usize) -> Gen<T>
    where
        P: Fn(&T) -> bool + 'static,
    {
        Gen::new(move |rng| {
            for _ in 0..max_retries {
                let candidate = self.sample(rng)?;
                if predicate(&candidate) {
                    return Ok(candidate);
                }
            }
            Err(FID_EXHAUSTED.to_string())
        })
    }
}

impl Gen<bool> {
    pub fn bool_gen() -> Self {
        Gen::new(|rng| Ok(gen_bool(rng)))
    }
}

impl Gen<u8> {
    pub fn u8_gen() -> Self {
        Gen::new(|rng| Ok(gen_u8(rng)))
    }
}

impl Gen<u64> {
    pub fn u64_gen() -> Self {
        Gen::new(|rng| Ok(gen_u64(rng)))
    }
}

impl Gen<i128> {
    pub fn i128_gen() -> Self {
        Gen::new(|rng| Ok(gen_i128(rng)))
    }

    pub fn bounded(min_val: i128, max_val: i128) -> Self {
        Gen::new(move |rng| Ok(gen_bounded_int(rng, min_val, max_val)))
    }
}

impl Gen<Vec<u8>> {
    pub fn bytes_gen(min_len: usize, max_len: usize) -> Self {
        Gen::new(move |rng| Ok(gen_bytes_slice(rng, min_len, max_len)))
    }
}

/// Uniform selection among a list of generators.
pub fn one_of<T: 'static>(gens: Vec<Gen<T>>) -> Gen<T> {
    assert!(!gens.is_empty(), "one_of requires at least one generator");
    Gen::new(move |rng| {
        let index = rng.bounded(gens.len() as u64) as usize;
        gens[index].sample(rng)
    })
}

/// Weighted selection among generators with positive frequencies.
pub fn frequency<T: 'static>(weighted: Vec<(u64, Gen<T>)>) -> Gen<T> {
    assert!(
        !weighted.is_empty(),
        "frequency requires at least one generator"
    );
    let total_weight: u64 = weighted.iter().map(|(w, _)| *w).sum();
    assert!(total_weight > 0, "frequency total weight must be > 0");
    Gen::new(move |rng| {
        let mut pick = rng.bounded(total_weight);
        for (w, gen) in &weighted {
            if pick < *w {
                return gen.sample(rng);
            }
            pick -= *w;
        }
        weighted.last().unwrap().1.sample(rng)
    })
}

/// Generate a list of elements with length in `[min_len, max_len]`.
pub fn list<T: 'static>(elem_gen: Gen<T>, min_len: usize, max_len: usize) -> Gen<Vec<T>> {
    assert!(min_len <= max_len, "min_len must be <= max_len");
    Gen::new(move |rng| {
        let span = (max_len - min_len + 1) as u64;
        let length = min_len + rng.bounded(span) as usize;
        let mut res = Vec::with_capacity(length);
        for _ in 0..length {
            res.push(elem_gen.sample(rng)?);
        }
        Ok(res)
    })
}

// --- Property Verification Harness (VER05 §5) -------------------------------

/// Execution options for property testing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PbtOptions {
    pub num_tests: usize,
    pub max_retries: usize,
    pub max_shrink_steps: usize,
    pub is_ci: bool,
}

impl Default for PbtOptions {
    fn default() -> Self {
        PbtOptions {
            num_tests: 100,
            max_retries: 100,
            max_shrink_steps: 1000,
            is_ci: std::env::var("CI").is_ok(),
        }
    }
}

/// Execute a property test over `options.num_tests` random evaluations.
pub fn check_property<T, G, P, S>(
    seed: u64,
    options: &PbtOptions,
    gen: G,
    prop: P,
    shrink: S,
) -> Verdict
where
    T: std::fmt::Debug + Clone + 'static,
    G: Fn(&mut SplitMix64) -> Result<T, String>,
    P: Fn(&T) -> bool + std::panic::RefUnwindSafe,
    S: Fn(T, &dyn Fn(&T) -> bool) -> (T, usize),
{
    let mut rng = SplitMix64::new(seed);
    for iteration in 1..=options.num_tests {
        let candidate = match gen(&mut rng) {
            Ok(c) => c,
            Err(err) if err == FID_EXHAUSTED => {
                return Verdict::fail(
                    FID_EXHAUSTED,
                    format!("filter precondition rejected all candidates at iteration {iteration}"),
                );
            }
            Err(err) => {
                return Verdict::fail(
                    FID_EXHAUSTED,
                    format!("generator error at iteration {iteration}: {err}"),
                );
            }
        };

        let prop_ref = &prop;
        let cand_ref = &candidate;
        let outcome = catch_unwind(AssertUnwindSafe(|| prop_ref(cand_ref)));
        match outcome {
            Err(_) => {
                return Verdict::fail(
                    FID_PANIC,
                    format!("property evaluation panicked at iteration {iteration}"),
                );
            }
            Ok(false) => {
                // Property falsified: execute integrated shrinking
                let shrink_pred = |val: &T| prop_ref(val);
                let (shrunk, steps) = shrink(candidate.clone(), &shrink_pred);

                if options.is_ci {
                    // Privacy-preserving mode: redact raw values, emit digest (VER05 §5.2)
                    let repro_str = format!("iter:{iteration};steps:{steps};shrunk:{shrunk:?}");
                    let digest = fnv1a64(repro_str.as_bytes());
                    return Verdict::fail(
                        FID_COUNTEREXAMPLE,
                        format!(
                            "property falsified after {iteration} iterations (shrunk in {steps} steps, repro-fnv: {digest:016x})"
                        ),
                    );
                } else {
                    return Verdict::fail(
                        FID_COUNTEREXAMPLE,
                        format!(
                            "property falsified at iteration {iteration}: initial={candidate:?}, minimal={shrunk:?} after {steps} shrink steps"
                        ),
                    );
                }
            }
            Ok(true) => {}
        }
    }

    Verdict::Pass
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_failure_ids_match_closed_vocabulary() {
        assert!(is_registered_failure_id(FID_COUNTEREXAMPLE));
        assert!(is_registered_failure_id(FID_EXHAUSTED));
        assert!(is_registered_failure_id(FID_TIMEOUT));
        assert!(is_registered_failure_id(FID_PANIC));
        assert!(!is_registered_failure_id("unregistered-id"));
        assert!(!is_registered_failure_id(""));
    }

    #[test]
    fn primitive_generators_produce_reproducible_values() {
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(42);
        assert_eq!(gen_bool(&mut rng1), gen_bool(&mut rng2));
        assert_eq!(gen_u8(&mut rng1), gen_u8(&mut rng2));
        assert_eq!(gen_u64(&mut rng1), gen_u64(&mut rng2));
        assert_eq!(gen_i128(&mut rng1), gen_i128(&mut rng2));

        let prim_bool = gen_primitive(0, "bool", 3).unwrap();
        assert_eq!(prim_bool.len(), 3);
        let prim_u8 = gen_primitive(0, "u8", 4).unwrap();
        assert_eq!(prim_u8.len(), 4);
        let prim_u64 = gen_primitive(0, "u64", 2).unwrap();
        assert_eq!(prim_u64.len(), 2);
        let prim_i128 = gen_primitive(0, "i128", 2).unwrap();
        assert_eq!(prim_i128.len(), 2);
        assert!(gen_primitive(0, "unknown_type", 1).is_err());
    }

    #[test]
    fn bounded_generators_stay_strictly_in_bounds() {
        let mut rng = SplitMix64::new(12345);
        for _ in 0..100 {
            let v = gen_bounded_int(&mut rng, -50, 50);
            assert!((-50..=50).contains(&v));
        }
        let b_vals = gen_bounded(99, 10, 20, 5).unwrap();
        assert_eq!(b_vals.len(), 5);
        assert!(gen_bounded(0, 50, 10, 1).is_err());

        // Test small bounded_u64
        let small_b = bounded_u64(&mut rng, 100);
        assert!(small_b < 100);
        let large_b = bounded_u64(&mut rng, (1 << 33) + 10);
        assert!(large_b < (1 << 33) + 10);

        // Test large bounds (span > 2^32 and span >= 2^64)
        let large_span_val = gen_bounded_int(&mut rng, 0, 1099511627776);
        assert!((0..=1099511627776).contains(&large_span_val));

        let max_u64_span = gen_bounded_int(&mut rng, 0, u64::MAX as i128);
        assert!((0..=u64::MAX as i128).contains(&max_u64_span));

        let huge_span = gen_bounded_int(&mut rng, 0, (u64::MAX as i128) + 100);
        assert!((0..=(u64::MAX as i128) + 100).contains(&huge_span));
    }

    #[test]
    fn bytes_generator_respects_bounds() {
        let mut rng = SplitMix64::new(777);
        for _ in 0..50 {
            let b = gen_bytes_slice(&mut rng, 2, 8);
            assert!(b.len() >= 2 && b.len() <= 8);
        }
        let b_vals = gen_bytes(13, 4, 8, 3).unwrap();
        assert_eq!(b_vals.len(), 3);
        assert!(gen_bytes(0, 10, 2, 1).is_err());
    }

    #[test]
    fn shrink_int_minimizes_correctly() {
        let (min1, steps1) = shrink_int(54321, 0, 0, 100000, |x| x <= 100, 1000);
        assert_eq!(min1, 101);
        assert!(steps1 > 0);

        let (min2, steps2) = shrink_int(-80, 0, -200, 200, |x| (-10..=50).contains(&x), 1000);
        assert_eq!(min2, -11);
        assert!(steps2 > 0);

        // Immediate target match
        let (min3, steps3) = shrink_int(100, 0, 0, 200, |x| x < 0, 1000);
        assert_eq!(min3, 0);
        assert_eq!(steps3, 1);
    }

    #[test]
    fn shrink_list_and_bytes_minimize_correctly() {
        let list_data = vec![1, 2, 3, 3, 4, 5];
        let (min_list, _) =
            shrink_list(&list_data, 0, |l| l.windows(2).all(|w| w[0] != w[1]), 1000);
        assert_eq!(min_list, vec![3, 3]);

        // Empty list match
        let (empty_min, empty_steps) = shrink_list(&[1, 2, 3], 0, |_| false, 1000);
        assert!(empty_min.is_empty());
        assert_eq!(empty_steps, 1);

        // Bytes shrinking
        let (min_bytes, steps_bytes) =
            shrink_bytes(&[1, 2, 3, 4, 5, 6, 7, 8], |b| b.len() < 5, 1000);
        assert_eq!(min_bytes, vec![0, 0, 0, 0, 0]);
        assert_eq!(steps_bytes, 16);
    }

    #[test]
    fn simulate_replay_reproduces_witness() {
        let (init, min) = simulate_replay(42, |x| x <= 100).unwrap();
        assert_eq!(init, 413);
        assert_eq!(min, 101);
    }

    #[test]
    fn combinators_and_property_runner_pass_and_fail() {
        let bool_gen = Gen::bool_gen();
        let u8_gen = Gen::u8_gen();
        let u64_gen = Gen::u64_gen();
        let i128_gen = Gen::i128_gen();
        let bounded_gen = Gen::bounded(-10, 10);
        let bytes_gen = Gen::bytes_gen(1, 4);

        let mut rng = SplitMix64::new(123);
        assert!(bool_gen.sample(&mut rng).is_ok());
        assert!(u8_gen.sample(&mut rng).is_ok());
        assert!(u64_gen.sample(&mut rng).is_ok());
        assert!(i128_gen.sample(&mut rng).is_ok());
        assert!(bounded_gen.sample(&mut rng).is_ok());
        assert!(bytes_gen.sample(&mut rng).is_ok());

        let mapped = Gen::bounded(1, 10).map(|x| x * 2);
        let val = mapped.sample(&mut rng).unwrap();
        assert_eq!(val % 2, 0);

        let filtered = Gen::bounded(1, 10).filter(|x| x % 2 == 0, 50);
        assert_eq!(filtered.sample(&mut rng).unwrap() % 2, 0);

        let exhausted = Gen::bounded(1, 10).filter(|_| false, 10);
        assert!(exhausted.sample(&mut rng).is_err());

        let one_of_gen = one_of(vec![Gen::bounded(1, 1), Gen::bounded(2, 2)]);
        let o = one_of_gen.sample(&mut rng).unwrap();
        assert!(o == 1 || o == 2);

        let freq_gen = frequency(vec![(1, Gen::bounded(1, 1)), (2, Gen::bounded(2, 2))]);
        let f = freq_gen.sample(&mut rng).unwrap();
        assert!(f == 1 || f == 2);

        let list_gen = list(Gen::bounded(1, 5), 2, 4);
        let l = list_gen.sample(&mut rng).unwrap();
        assert!(l.len() >= 2 && l.len() <= 4);

        let default_opts = PbtOptions::default();
        assert_eq!(default_opts.num_tests, 100);
        assert_eq!(default_opts.max_retries, 100);

        // Check property passing
        let opts = PbtOptions {
            num_tests: 50,
            max_retries: 50,
            max_shrink_steps: 100,
            is_ci: false,
        };
        let pass_verdict = check_property(
            42,
            &opts,
            |r| Ok(gen_bounded_int(r, 0, 10)),
            |&x| x >= 0,
            |cand, _| (cand, 0),
        );
        assert_eq!(pass_verdict, Verdict::Pass);

        // Check property failing and shrinking (local mode)
        let fail_verdict = check_property(
            42,
            &opts,
            |r| Ok(gen_bounded_int(r, 0, 200)),
            |&x| x <= 50,
            |cand, prop| shrink_int(cand, 0, 0, 200, |x| prop(&x), 1000),
        );
        match fail_verdict {
            Verdict::Fail {
                failure_id,
                message,
            } => {
                assert_eq!(failure_id, FID_COUNTEREXAMPLE);
                assert!(message.contains("minimal=51"));
            }
            Verdict::Pass => panic!("expected property to fail"),
        }

        // Check property failing in CI mode (privacy-preserving)
        let ci_opts = PbtOptions {
            num_tests: 50,
            max_retries: 50,
            max_shrink_steps: 100,
            is_ci: true,
        };
        let ci_fail_verdict = check_property(
            42,
            &ci_opts,
            |r| Ok(gen_bounded_int(r, 0, 200)),
            |&x| x <= 50,
            |cand, prop| shrink_int(cand, 0, 0, 200, |x| prop(&x), 1000),
        );
        match ci_fail_verdict {
            Verdict::Fail {
                failure_id,
                message,
            } => {
                assert_eq!(failure_id, FID_COUNTEREXAMPLE);
                assert!(message.contains("repro-fnv:"));
            }
            Verdict::Pass => panic!("expected property to fail"),
        }

        // Check property panic catching
        let panic_verdict = check_property(
            42,
            &opts,
            |r| Ok(gen_bounded_int(r, 0, 10)),
            |&x| {
                if x > 5 {
                    panic!("deliberate property panic");
                }
                true
            },
            |cand, _| (cand, 0),
        );
        match panic_verdict {
            Verdict::Fail {
                failure_id,
                message,
            } => {
                assert_eq!(failure_id, FID_PANIC);
                assert!(message.contains("panicked"));
            }
            Verdict::Pass => panic!("expected panic failure"),
        }

        // Check filter exhaustion failure
        let exhaust_verdict = check_property(
            42,
            &opts,
            |_| Err(FID_EXHAUSTED.to_string()),
            |_: &i32| true,
            |cand, _| (cand, 0),
        );
        match exhaust_verdict {
            Verdict::Fail { failure_id, .. } => {
                assert_eq!(failure_id, FID_EXHAUSTED);
            }
            Verdict::Pass => panic!("expected exhausted failure"),
        }

        // Check other generator error failure
        let gen_err_verdict = check_property(
            42,
            &opts,
            |_| Err("custom-error".to_string()),
            |_: &i32| true,
            |cand, _| (cand, 0),
        );
        match gen_err_verdict {
            Verdict::Fail {
                failure_id,
                message,
            } => {
                assert_eq!(failure_id, FID_EXHAUSTED);
                assert!(message.contains("custom-error"));
            }
            Verdict::Pass => panic!("expected generator error failure"),
        }
    }
}
