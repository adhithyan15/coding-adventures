//! SHA-256 digests as the gate handles them: a 32-byte value with one spelling.
//!
//! # Why a type
//!
//! Every file in an environment chain is named by the SHA-256 of its exact bytes (VER32
//! 3.7, 8.3). A digest that is a loose `String` can be upper-case, short, carry a stray
//! space or the `sha256:` prefix twice, and then two spellings of one digest compare
//! unequal (or worse, two *different* digests compare equal after a sloppy normalisation).
//! [`Digest`] parses one spelling (64 lowercase hex digits, with an optional `sha256:`
//! prefix that is stripped exactly once) and prints one spelling (the bare hex), so equality
//! of values is equality of bytes.
//!
//! # The environment fold (VER32 8.3)
//!
//! ```text
//!   H0 = SHA-256("pif-env-1\n")
//!   Hi = SHA-256(H(i-1) || file_sha256_i)
//! ```
//!
//! `||` is concatenation of the two **raw 32-byte values** (64 bytes in all). VER32 does not
//! say whether the operands are raw bytes or their hex spellings; raw bytes is the usual
//! reading of a hash chain and is what this crate does. The choice is recorded in the
//! README so a second implementation can match it.

use coding_adventures_sha256::sha256;

/// A SHA-256 value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    /// The digest of `bytes`, computed by the repository's own SHA-256.
    pub fn of(bytes: &[u8]) -> Digest {
        Digest(sha256(bytes))
    }

    /// Parse `<64 lowercase hex>` or `sha256:<64 lowercase hex>`. Anything else (upper
    /// case, wrong length, whitespace, a second prefix) is `None`.
    pub fn parse(s: &str) -> Option<Digest> {
        let hex = s.strip_prefix("sha256:").unwrap_or(s);
        if hex.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        let b = hex.as_bytes();
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = (nibble(b[2 * i])? << 4) | nibble(b[2 * i + 1])?;
        }
        Some(Digest(out))
    }

    /// The 64 lowercase hex digits.
    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push(char::from(HEX[usize::from(b >> 4)]));
            s.push(char::from(HEX[usize::from(b & 15)]));
        }
        s
    }

    /// The environment fold: the hash of the state followed by the next file's digest.
    pub fn fold(&self, file: &Digest) -> Digest {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&self.0);
        buf[32..].copy_from_slice(&file.0);
        Digest::of(&buf)
    }

    /// `H0` of the environment fold.
    pub fn env_start() -> Digest {
        Digest::of(b"pif-env-1\n")
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hex())
    }
}
