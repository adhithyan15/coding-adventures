//! # Scytale Cipher
//!
//! Ancient Spartan transposition cipher (~700 BCE).
//!
//! The Scytale rearranges character positions using a columnar transposition.
//! Unlike substitution ciphers (Caesar, Atbash), no characters are replaced —
//! they are simply shuffled into a new order determined by the key.

pub mod cipher;

pub use cipher::{brute_force, decrypt, encrypt, BruteForceResult};
