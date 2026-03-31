//! # caesar-cipher
//!
//! Caesar cipher -- the oldest substitution cipher, with brute-force and
//! frequency analysis.
//!
//! ## What is a Caesar cipher?
//!
//! The Caesar cipher is one of the earliest known encryption techniques.
//! Julius Caesar used it in his private correspondence: each letter in the
//! plaintext is replaced by a letter a fixed number of positions further
//! along the alphabet.  If the shift is 3, then A becomes D, B becomes E,
//! and so on.  When we reach the end of the alphabet we wrap around, so
//! X becomes A, Y becomes B, Z becomes C.
//!
//! ```text
//! Plain:   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//! Shift 3: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
//! ```
//!
//! Decryption is the inverse operation: shift each letter backwards by the
//! same amount.  Equivalently, decrypt with shift `s` is the same as
//! encrypting with shift `26 - s`.
//!
//! ## Modules
//!
//! - [`cipher`] -- encrypt, decrypt, and ROT13
//! - [`analysis`] -- brute-force and frequency-analysis attacks
//!
//! ## Examples
//!
//! ```
//! use caesar_cipher::cipher;
//!
//! let ciphertext = cipher::encrypt("Hello, World!", 3);
//! assert_eq!(ciphertext, "Khoor, Zruog!");
//!
//! let plaintext = cipher::decrypt(&ciphertext, 3);
//! assert_eq!(plaintext, "Hello, World!");
//!
//! // ROT13 is its own inverse
//! let encoded = cipher::rot13("Hello");
//! assert_eq!(cipher::rot13(&encoded), "Hello");
//! ```
//!
//! This crate is part of the **coding-adventures** monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.

pub mod cipher;
pub mod analysis;
