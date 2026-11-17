//! # `corpus.profile`: the neutral line file of VER04r section 8.4
//!
//! The knobs of a v2 corpus live in one data file, read by the compiler (this crate) and by
//! the oracle's own independent reader, and by nothing else:
//!
//! ```text
//!   budget 16777216
//!   chunk_size 65536
//!   slim_stride auto
//!   max_ir_cost 32768
//!   slim_chunk eq_agrees 3
//!   fault multiply generator-shift
//! ```
//!
//! `max_ir_cost` is the **only** way to raise the per-expression IR cost bound above its
//! default of 2,000 (ceiling 65,536): no flag and no environment variable can, because an
//! override is a second place for the limit to live and the report could not show it
//! (VER04r 4.2 rule 6).
//!
//! The reader is deliberately strict and mirrors `python/warrant-oracle/profile.py` rule
//! for rule, so the two cannot disagree about what a profile means:
//!
//! * UTF-8, LF only (a CR anywhere is an error), at most 64 KiB;
//! * a line is single-space-separated tokens; a blank or whitespace-only line is skipped;
//! * `budget`, `chunk_size`, `slim_stride` and `max_ir_cost` take one value and occur once;
//!   `slim_chunk <claim> <index>` and `fault <op> <name>` may occur several times but never
//!   as an exact repeat of an earlier line; nothing else is accepted;
//! * an empty token (two spaces in a row, a leading or trailing space) is an error, and so
//!   are `budget 0` and `slim_stride 0` (VER04r 16.4 item 7; the oracle's reader accepts
//!   these until S4 part 2);
//! * a number is canonical decimal: ASCII digits only, no leading zero except `"0"`, at
//!   most 20 digits;
//! * `chunk_size` is a power of two in `2^10..=2^16`; `max_ir_cost` is `2,000..=65,536`.
//!
//! This slice reads and validates; the cross-checks against the document (a `slim_chunk`
//! past `m`, a fault the op's signature cannot take, the Rust `faults=` lists) are S3b/S3c.

use coding_adventures_sha256::sha256_hex;

use crate::cost::{DEFAULT_MAX_IR_COST, MAX_IR_COST_CEILING};
use crate::error::{code, GenError};

/// `corpus.profile` size cap.
pub const MAX_PROFILE_BYTES: usize = 64 * 1024;

/// `slim_stride`: `auto`, or a stride the profile sets explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlimStride {
    /// `slim_stride auto` (the default): `k = ceil(m / 14)`.
    Auto,
    /// An explicit stride (VER04r 5.6 allows only a value smaller than the automatic `k`;
    /// checked against `m` in S3b).
    Every(u64),
}

/// A parsed `corpus.profile`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Product ordinals per claim (default `2^24`).
    pub budget: u64,
    /// Product ordinals per chunk (default `2^16`).
    pub chunk_size: u64,
    /// The slim stride.
    pub slim_stride: SlimStride,
    /// The per-expression IR cost bound.
    pub max_ir_cost: u64,
    /// `slim_chunk <claim> <index>` lines, in file order.
    pub slim_chunks: Vec<(String, u64)>,
    /// `fault <op> <name>` lines, in file order.
    pub faults: Vec<(String, String)>,
    /// The SHA-256 of the file's bytes: `profile_sha256` in `corpus.json` (VER04r 1.4).
    pub sha256: String,
}

impl Default for Profile {
    /// The profile of an empty file: every default, and the digest of zero bytes.
    fn default() -> Profile {
        Profile {
            budget: 1 << 24,
            chunk_size: 1 << 16,
            slim_stride: SlimStride::Auto,
            max_ir_cost: DEFAULT_MAX_IR_COST,
            slim_chunks: Vec::new(),
            faults: Vec::new(),
            sha256: sha256_hex(b""),
        }
    }
}

fn bad(message: &str) -> GenError {
    GenError::new(code::PROFILE, "corpus.profile", message)
}

/// A canonical decimal: ASCII digits, no leading zero except `"0"`, at most 20 digits,
/// and small enough for a `u64`.
fn parse_int(text: &str) -> Result<u64, GenError> {
    let canonical = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && !(text.len() > 1 && text.starts_with('0'))
        && text.len() <= 20;
    if !canonical {
        return Err(bad("a profile value is a canonical decimal integer"));
    }
    text.parse::<u64>()
        .map_err(|_| bad("a profile value does not fit 64 bits"))
}

/// `budget 0` and `slim_stride 0` mean nothing (an empty corpus, a stride that never
/// advances): profile errors (VER04r 16.4 item 7).
fn nonzero(v: u64, key: &str) -> Result<u64, GenError> {
    if v == 0 {
        return Err(bad(&format!("{key} must be at least 1")));
    }
    Ok(v)
}

/// Parse a `corpus.profile`.
///
/// # Errors
///
/// `limit` past 64 KiB; `profile` for anything irregular (see the module documentation).
pub fn parse_profile(raw: &[u8]) -> Result<Profile, GenError> {
    if raw.len() > MAX_PROFILE_BYTES {
        return Err(GenError::new(
            code::LIMIT,
            "corpus.profile",
            "corpus.profile exceeds its size cap",
        ));
    }
    let text = std::str::from_utf8(raw).map_err(|_| bad("corpus.profile is not UTF-8"))?;
    if text.contains('\r') {
        return Err(bad("corpus.profile must use LF line endings"));
    }
    let mut prof = Profile {
        sha256: sha256_hex(raw),
        ..Profile::default()
    };
    let mut seen: Vec<&str> = Vec::new();
    for line in text.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(' ').collect();
        // An empty token (`fault  eq`, a leading or trailing space) is a malformed line, not
        // a name (VER04r 16.4 item 7).
        if parts.iter().any(|p| p.is_empty()) {
            return Err(bad("a profile line has an empty token"));
        }
        match (parts[0], parts.len()) {
            (key @ ("budget" | "chunk_size" | "slim_stride" | "max_ir_cost"), 2) => {
                if seen.contains(&key) {
                    return Err(bad("a profile key takes one value and occurs once"));
                }
                seen.push(key);
                match key {
                    "slim_stride" => {
                        prof.slim_stride = if parts[1] == "auto" {
                            SlimStride::Auto
                        } else {
                            SlimStride::Every(nonzero(parse_int(parts[1])?, "slim_stride")?)
                        }
                    }
                    "budget" => prof.budget = nonzero(parse_int(parts[1])?, "budget")?,
                    "chunk_size" => prof.chunk_size = parse_int(parts[1])?,
                    _ => prof.max_ir_cost = parse_int(parts[1])?,
                }
            }
            ("budget" | "chunk_size" | "slim_stride" | "max_ir_cost", _) => {
                return Err(bad("a profile key takes one value and occurs once"))
            }
            ("slim_chunk", 3) => {
                let entry = (parts[1].to_string(), parse_int(parts[2])?);
                if prof.slim_chunks.contains(&entry) {
                    return Err(bad("a slim_chunk line may not repeat"));
                }
                prof.slim_chunks.push(entry);
            }
            ("fault", 3) => {
                let entry = (parts[1].to_string(), parts[2].to_string());
                if prof.faults.contains(&entry) {
                    return Err(bad("a fault line may not repeat"));
                }
                prof.faults.push(entry);
            }
            _ => return Err(bad("an unknown or malformed profile line")),
        }
    }
    if !(DEFAULT_MAX_IR_COST..=MAX_IR_COST_CEILING).contains(&prof.max_ir_cost) {
        return Err(bad("max_ir_cost is 2,000..65,536"));
    }
    let cs = prof.chunk_size;
    if !cs.is_power_of_two() || !((1 << 10)..=(1 << 16)).contains(&cs) {
        return Err(bad("chunk_size is a power of two in 2^10..2^16"));
    }
    Ok(prof)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Profile, GenError> {
        parse_profile(s.as_bytes())
    }

    #[test]
    fn an_empty_profile_is_every_default() {
        let p = parse("").unwrap();
        assert_eq!(p, Profile::default());
        assert_eq!(p.budget, 16_777_216);
        assert_eq!(p.chunk_size, 65_536);
        assert_eq!(p.max_ir_cost, 2_000);
        assert_eq!(p.slim_stride, SlimStride::Auto);
        assert_eq!(
            p.sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_documented_profile_parses_and_keeps_order() {
        let text = "budget 16777216\nchunk_size 65536\nslim_stride auto\nmax_ir_cost 32768\n\
                    slim_chunk eq_agrees 3\nslim_chunk eq_agrees 1\nfault multiply generator-shift\n\
                    fault eq invert\n";
        let p = parse(text).unwrap();
        assert_eq!(p.max_ir_cost, 32_768);
        assert_eq!(
            p.slim_chunks,
            [("eq_agrees".to_string(), 3), ("eq_agrees".to_string(), 1)]
        );
        assert_eq!(
            p.faults,
            [
                ("multiply".to_string(), "generator-shift".to_string()),
                ("eq".to_string(), "invert".to_string())
            ]
        );
        assert_eq!(p.sha256, sha256_hex(text.as_bytes()));
    }

    #[test]
    fn a_numeric_slim_stride_and_blank_lines_are_accepted() {
        let p = parse("\n  \nslim_stride 7\n\t\n").unwrap();
        assert_eq!(p.slim_stride, SlimStride::Every(7));
    }

    #[test]
    fn crlf_is_refused_anywhere() {
        let e = parse("budget 16777216\r\n").unwrap_err();
        assert_eq!(e.code, code::PROFILE);
        assert!(e.message.contains("LF"));
        assert!(parse("fault a b\rfault c d\n").is_err());
    }

    #[test]
    fn a_duplicate_single_valued_key_is_refused() {
        for key in [
            "budget 1048576",
            "chunk_size 1024",
            "slim_stride auto",
            "max_ir_cost 4000",
        ] {
            let text = format!("{key}\n{key}\n");
            assert_eq!(parse(&text).unwrap_err().code, code::PROFILE, "{key}");
        }
    }

    #[test]
    fn repeatable_keys_may_repeat_with_different_values_but_never_exactly() {
        assert!(parse("fault a b\nfault a c\nfault d b\nslim_chunk c 0\nslim_chunk c 1\nslim_chunk d 0\n").is_ok());
        for text in [
            "fault a b\nfault a b\n",
            "slim_chunk c 0\nslim_chunk c 0\n",
            "fault a b\nfault c d\nfault a b\n",
        ] {
            assert_eq!(parse(text).unwrap_err().code, code::PROFILE, "{text:?}");
        }
    }

    #[test]
    fn empty_tokens_and_zero_budget_or_stride_are_refused() {
        for text in [
            "fault  eq\n",
            "fault eq \n",
            "slim_chunk  c 1\n",
            "slim_chunk c  \n",
            "budget  5\n",
            "slim_stride \n",
            "budget 0\n",
            "slim_stride 0\n",
        ] {
            assert_eq!(parse(text).unwrap_err().code, code::PROFILE, "{text:?}");
        }
        assert_eq!(parse("budget 1\nslim_stride 1\n").unwrap().budget, 1);
    }

    #[test]
    fn chunk_size_must_be_a_power_of_two_in_range() {
        assert!(parse("chunk_size 1024\n").is_ok());
        assert!(parse("chunk_size 65536\n").is_ok());
        for bad in ["3000", "512", "131072", "0", "1000"] {
            assert!(parse(&format!("chunk_size {bad}\n")).is_err(), "{bad}");
        }
    }

    #[test]
    fn max_ir_cost_is_2000_to_65536() {
        assert!(parse("max_ir_cost 2000\n").is_ok());
        assert!(parse("max_ir_cost 65536\n").is_ok());
        assert!(parse("max_ir_cost 70000\n").is_err());
        assert!(parse("max_ir_cost 65537\n").is_err());
        assert!(parse("max_ir_cost 1999\n").is_err());
    }

    #[test]
    fn numbers_are_canonical_decimals() {
        for bad in [
            "007",
            "+5",
            "-5",
            "1_000",
            "0x10",
            "",
            "\u{0663}",
            "99999999999999999999999",
        ] {
            let line = format!("budget {bad}\n");
            assert!(parse(&line).is_err(), "{bad:?}");
        }
        // 20 digits but past u64
        assert!(parse("budget 99999999999999999999\n").is_err());
        assert_eq!(
            parse("budget 18446744073709551615\n").unwrap().budget,
            u64::MAX
        );
    }

    #[test]
    fn unknown_malformed_and_wrongly_spaced_lines_are_refused() {
        for bad in [
            "frobnicate 1\n",
            "budget\n",
            "budget 1 2\n",
            "fault onlyone\n",
            "fault a b c\n",
            "slim_chunk c\n",
            " budget 1\n",
            "budget  1\n",
            "slim_chunk c x\n",
        ] {
            assert_eq!(parse(bad).unwrap_err().code, code::PROFILE, "{bad:?}");
        }
    }

    #[test]
    fn size_cap_and_encoding() {
        let big = vec![b'\n'; MAX_PROFILE_BYTES + 1];
        assert_eq!(parse_profile(&big).unwrap_err().code, code::LIMIT);
        assert!(parse_profile(&vec![b'\n'; MAX_PROFILE_BYTES]).is_ok());
        assert_eq!(
            parse_profile(&[0xff, 0xfe]).unwrap_err().code,
            code::PROFILE
        );
    }

    #[test]
    fn errors_never_echo_the_input() {
        let e = parse("secret-token 1\n").unwrap_err();
        assert!(!e.to_string().contains("secret-token"));
    }
}
