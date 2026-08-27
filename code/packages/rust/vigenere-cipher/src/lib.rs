//! # Vigenere Cipher
//!
//! Polyalphabetic substitution cipher with full cryptanalysis (1553).
//!
//! The Vigenere cipher uses a repeating keyword to apply different Caesar
//! shifts at each position. It resisted cryptanalysis for 300 years until
//! Kasiski (1863) and Friedman (1920s) developed statistical attacks using
//! the Index of Coincidence and chi-squared frequency analysis.

pub mod analysis;
pub mod cipher;

pub use analysis::{break_cipher, find_key, find_key_length, BreakResult};
pub use cipher::{decrypt, encrypt};
