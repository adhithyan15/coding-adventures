//! # Vigenere Cipher
//!
//! Polyalphabetic substitution cipher with full cryptanalysis (1553).
//!
//! The Vigenere cipher uses a repeating keyword to apply different Caesar
//! shifts at each position. It resisted cryptanalysis for 300 years until
//! Kasiski (1863) and Friedman (1920s) developed statistical attacks using
//! the Index of Coincidence and chi-squared frequency analysis.

pub mod cipher;
pub mod analysis;

pub use cipher::{encrypt, decrypt};
pub use analysis::{find_key_length, find_key, break_cipher, BreakResult};
