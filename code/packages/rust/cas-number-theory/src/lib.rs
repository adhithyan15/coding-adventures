//! # cas-number-theory
//!
//! Number theory operations over the symbolic IR: GCD, LCM, primality,
//! integer factorization, Chinese Remainder Theorem, and Euler's totient.
//!
//! ## Modules
//!
//! | Module | Functions |
//! |--------|-----------|
//! | [`arithmetic`] | [`gcd`], [`lcm`], [`extended_gcd`], [`totient`], [`divisors`], [`moebius_mu`], [`jacobi_symbol`], [`integer_length`], [`mod_inverse`], [`mod_pow`] |
//! | [`primality`] | [`is_prime`], [`primes_up_to`], [`next_prime`], [`prev_prime`], [`nth_prime`] |
//! | [`factorize`] | [`factor_integer`], [`factorize_ir`] |
//! | [`crt`] | [`crt`] |
//!
//! ## Quick start
//!
//! ```rust
//! use cas_number_theory::{gcd, lcm, is_prime, factor_integer, crt};
//!
//! assert_eq!(gcd(12, 8), 4);
//! assert_eq!(lcm(4, 6), 12);
//! assert!(is_prime(97));
//! assert_eq!(factor_integer(360), vec![(2, 3), (3, 2), (5, 1)]);
//! assert_eq!(crt(&[2, 3, 2], &[3, 5, 7]), Some(23));
//! ```
//!
//! ## Stack position
//!
//! ```text
//! symbolic-ir  ←  cas-number-theory
//! ```

pub mod arithmetic;
pub mod crt;
pub mod factorize;
pub mod handlers;
pub mod primality;

// Re-export the full public API.

// arithmetic
pub use arithmetic::{
    divisors, extended_gcd, gcd, integer_length, jacobi_symbol, lcm, mod_inverse, mod_pow,
    moebius_mu, totient,
};

// primality
pub use primality::{is_prime, next_prime, nth_prime, prev_prime, primes_up_to};

// factorize
pub use factorize::{factor_integer, factorize_ir};

// handlers
pub use handlers::{
    build_number_theory_handler_table, chinese_remainder_handler, divisors_handler,
    factor_integer_handler, integer_length_handler, is_prime_handler, jacobi_symbol_handler,
    moebius_mu_handler, next_prime_handler, prev_prime_handler, totient_handler, Handler,
};

// crt
pub use crt::crt;
