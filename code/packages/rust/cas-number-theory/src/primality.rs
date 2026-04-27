//! Primality testing and prime generation.
//!
//! # Algorithms
//!
//! - **`is_prime`** — trial division up to `√n`.  Correct for all `i64` inputs;
//!   O(√n) time.  Fast enough for values up to ~10^12 (√n ≈ 10^6 trials).
//!
//! - **`primes_up_to`** — Sieve of Eratosthenes.  Marks composites in an
//!   array of size `n+1`; collects unmarked indices.  O(n log log n) time,
//!   O(n) space.  Practical for `limit ≤ ~10^8`.
//!
//! ```text
//! Sieve up to 20:
//! Start:  [F, F, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T]
//!          0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20
//!
//! p=2: mark 4, 6, 8, 10, 12, 14, 16, 18, 20
//! p=3: mark 9, 15
//! p=5: 5*5=25 > 20, stop
//!
//! Primes: 2, 3, 5, 7, 11, 13, 17, 19
//! ```

// ---------------------------------------------------------------------------
// Core test
// ---------------------------------------------------------------------------

/// Return `true` if `n` is a prime number.
///
/// Uses trial division with an optimised step sequence:
/// - Handle 2 and 3 explicitly.
/// - Test only numbers of the form `6k ± 1` (every prime > 3 fits this).
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::is_prime;
///
/// assert!(is_prime(2));
/// assert!(is_prime(7));
/// assert!(is_prime(97));
/// assert!(!is_prime(0));
/// assert!(!is_prime(1));
/// assert!(!is_prime(4));
/// assert!(!is_prime(100));
/// ```
pub fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true; // 2 and 3 are prime
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    // Test candidates of the form 6k±1.  Every composite number has a
    // prime factor ≤ its square root, so we only need to go up to √n.
    let mut k = 5i64;
    while k * k <= n {
        if n % k == 0 || n % (k + 2) == 0 {
            return false;
        }
        k += 6;
    }
    true
}

// ---------------------------------------------------------------------------
// Prime enumeration
// ---------------------------------------------------------------------------

/// Return all primes up to and including `limit` using the Sieve of
/// Eratosthenes.
///
/// Returns an empty vector if `limit < 2`.
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::primes_up_to;
///
/// assert_eq!(primes_up_to(20), vec![2, 3, 5, 7, 11, 13, 17, 19]);
/// assert_eq!(primes_up_to(2), vec![2]);
/// assert_eq!(primes_up_to(1), vec![]);
/// ```
pub fn primes_up_to(limit: u64) -> Vec<u64> {
    if limit < 2 {
        return vec![];
    }
    let n = limit as usize + 1;
    // `sieve[i]` starts as `true` (potentially prime) and is set to `false`
    // when we find a factor.
    let mut sieve = vec![true; n];
    sieve[0] = false; // 0 is not prime
    sieve[1] = false; // 1 is not prime

    // Standard sieve: for each prime p, mark all multiples starting at p².
    // We only need to iterate p up to √limit.
    let mut p = 2usize;
    while p * p < n {
        if sieve[p] {
            let mut multiple = p * p;
            while multiple < n {
                sieve[multiple] = false;
                multiple += p;
            }
        }
        p += 1;
    }

    sieve
        .iter()
        .enumerate()
        .filter_map(|(i, &is_p)| if is_p { Some(i as u64) } else { None })
        .collect()
}

/// Return the smallest prime strictly greater than `n`.
///
/// Starts at `n + 1` and increments until a prime is found.
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::next_prime;
///
/// assert_eq!(next_prime(0), 2);
/// assert_eq!(next_prime(2), 3);
/// assert_eq!(next_prime(10), 11);
/// assert_eq!(next_prime(13), 17);
/// ```
pub fn next_prime(n: i64) -> i64 {
    let mut candidate = if n < 2 { 2 } else { n + 1 };
    loop {
        if is_prime(candidate) {
            return candidate;
        }
        candidate += 1;
    }
}

/// Return the `k`-th prime (1-indexed: `nth_prime(1) = 2`).
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::nth_prime;
///
/// assert_eq!(nth_prime(1), 2);
/// assert_eq!(nth_prime(4), 7);
/// assert_eq!(nth_prime(10), 29);
/// ```
pub fn nth_prime(k: usize) -> i64 {
    assert!(k >= 1, "nth_prime: k must be ≥ 1");
    let mut count = 0;
    let mut candidate = 2i64;
    loop {
        if is_prime(candidate) {
            count += 1;
            if count == k {
                return candidate;
            }
        }
        candidate += 1;
    }
}
