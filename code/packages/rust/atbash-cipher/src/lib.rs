//! # atbash-cipher
//!
//! A Rust implementation of the Atbash cipher, one of the oldest known
//! substitution ciphers.
//!
//! ## What is the Atbash Cipher?
//!
//! The Atbash cipher reverses the alphabet: A maps to Z, B maps to Y, C maps
//! to X, and so on. The name comes from the Hebrew alphabet: Aleph-Tav-Beth-Shin.
//!
//! ```text
//! Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//! Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
//! ```
//!
//! ## The Formula
//!
//! Given a letter at position `p` (where A=0, B=1, ..., Z=25):
//!
//! ```text
//! encrypted_position = 25 - p
//! ```
//!
//! ## Self-Inverse Property
//!
//! The cipher is self-inverse: `encrypt(encrypt(text)) == text`.
//! This is because `f(f(x)) = 25 - (25 - x) = x`.
//!
//! ## Usage
//!
//! ```
//! use atbash_cipher::{encrypt, decrypt};
//!
//! assert_eq!(encrypt("HELLO"), "SVOOL");
//! assert_eq!(decrypt("SVOOL"), "HELLO");
//! assert_eq!(encrypt("Hello, World! 123"), "Svool, Dliow! 123");
//! ```

pub mod cipher;

// Re-export the main functions at the crate root for convenience.
pub use cipher::{decrypt, encrypt};
