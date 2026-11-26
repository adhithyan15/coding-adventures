//! # Canonical value streams and SHA-256 framing (VER04r 2.4, 5.4, 5.6)
//!
//! A corpus does not ship its inputs.  It ships *digests* of two byte streams per suite and
//! per chunk: the stream of every admitted case's input values and the stream of every
//! case's expected value.  A lane that skips, reorders or mis-evaluates a single case
//! produces a different digest.  This module writes those streams.
//!
//! ## Value encodings (`warrant-stream-v2`; every `v1` row keeps its bytes)
//!
//! ```text
//!   Bool                     00 or 01
//!   U8 U16 U32 U64  Nat      10, then 8 bytes little-endian, unsigned
//!   I8 I16 I32 I64           11, then 8 bytes two's complement, sign-extended to 64 bits
//!   Bytes  Bytes<N>          20, u32-LE length, the bytes
//!   Ok(v)  Err(E.CASE)       50 plus the inner value  /  51, u32-LE length, UTF-8 case name
//!   (v1, .., vn)             64, u32-LE component count, the components (`ir.tuple`)
//!   Option<T>                40 (none) / 41 plus the inner value                  (`ir.data`)
//!   record                   64, u32-LE field count, the fields (like a tuple)    (`ir.data`)
//!   variant                  65, u32 length + UTF-8 constructor, u32 field count, the fields
//! ```
//!
//! A tuple *value* carries the `64` tag and its count because nested composites must be
//! self-delimiting or two different values could serialise to the same bytes; a top-level binder
//! list is a plain concatenation of the binders' values.  (`List`, `30`, is the reserved `ir.list`.)
//!
//! ## Framing
//!
//! Every stream starts with a context prefix:
//!
//! ```text
//!   header  u32 len + suite id  u32 #binders  (u32 len + type name) per binder  u32 len + result type
//! ```
//!
//! The header is `warrant-stream-v1\0` if every type of the suite is in the v1 set
//! `{Bool U8 U64 Nat Bytes Bytes<N> Result<v1,E>}`, else `warrant-stream-v2\0` (VER04r 1.2).
//! A *chunk* stream appends the `u32-LE` chunk index to the prefix; chunk digests are
//! independent and combine by **listing**:
//!
//! ```text
//!   suite digest = SHA-256("warrant-chunks-v1\0" || u32-LE m || the m raw 32-byte chunk digests)
//! ```
//!
//! The v1 whole-suite digest is the SHA-256 of the *unchunked* stream: the prefix followed by
//! every admitted case, with no chunk index.  It is computed in the same pass through a
//! second hasher (`suite::Whole`).
//!
//! ## The slim base set
//!
//! ```text
//!   m <= 16          every chunk
//!   otherwise        k = ceil(m / 14),  o = FNV-1a-64(suite id) mod k
//!                    {0, m-1} + { o + j*k | o + j*k < m }       (at most 16 chunks)
//! ```
//!
//! A tiny worked example of the framing: for the empty list of digests the suite digest is
//! `SHA-256("warrant-chunks-v1\0" || 00000000)`.

use coding_adventures_sha256::sha256_hex;

use crate::error::{code, GenError};
use crate::types::{Decls, Ty, Width};
use crate::value::Value;

/// `warrant-stream-v1\0`.
pub const HEADER_V1: &[u8] = b"warrant-stream-v1\0";
/// `warrant-stream-v2\0`.
pub const HEADER_V2: &[u8] = b"warrant-stream-v2\0";
/// The suite-digest frame.
pub const CHUNKS_FRAME: &[u8] = b"warrant-chunks-v1\0";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The two stream headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header {
    /// `warrant-stream-v1`: every type of the suite is in the v1 set.
    V1,
    /// `warrant-stream-v2`.
    V2,
}

impl Header {
    /// The header bytes including the trailing NUL.
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Header::V1 => HEADER_V1,
            Header::V2 => HEADER_V2,
        }
    }

    /// The header text without the NUL, as the oracle's `stream_header` row writes it.
    pub fn name(self) -> &'static str {
        match self {
            Header::V1 => "warrant-stream-v1",
            Header::V2 => "warrant-stream-v2",
        }
    }
}

/// `u32` little-endian, refusing a length that does not fit.
pub fn put_u32(out: &mut Vec<u8>, n: usize) -> Result<(), GenError> {
    let n = u32::try_from(n)
        .map_err(|_| GenError::new(code::LIMIT, "", "a length does not fit a u32"))?;
    out.extend_from_slice(&n.to_le_bytes());
    Ok(())
}

/// `u32` length then the UTF-8 bytes.
pub fn put_field(out: &mut Vec<u8>, text: &str) -> Result<(), GenError> {
    put_u32(out, text.len())?;
    out.extend_from_slice(text.as_bytes());
    Ok(())
}

/// Is `ty` in the v1 type set `{Bool, U8, U64, Nat, Bytes, Bytes<N>, Option<v1>, Result<v1,E>}`?
/// (`List` is in the set too but is the reserved `ir.list`; a tuple, a record and a variant are
/// not in it, so a suite with one anywhere in its types has the `warrant-stream-v2` header, and
/// `Option<U8>` keeps `warrant-stream-v1`.)
pub fn in_v1_set(ty: &Ty) -> bool {
    use coding_adventures_warrant_grammar::fixed::FixedTy;
    match ty {
        Ty::Bool | Ty::Nat | Ty::Bytes(_) => true,
        Ty::Fixed(t) => matches!(t, FixedTy::U8 | FixedTy::U64),
        Ty::Result(ok, _) | Ty::Option(ok) => in_v1_set(ok),
        Ty::Tuple(_) => false,
        // `Int` never appears in a value (VER04r 2.1); a record and a variant are v2 types; a list is
        // the reserved `ir.list`.
        Ty::Int | Ty::Named(_) | Ty::List(_) => false,
    }
}

/// The header of a suite: v1 iff every binder type and the result type are in the v1 set.
pub fn suite_header(binders: &[Ty], result: &Ty) -> Header {
    if binders.iter().all(in_v1_set) && in_v1_set(result) {
        Header::V1
    } else {
        Header::V2
    }
}

/// The shared stream prefix: header, suite id, binder types, result type.
///
/// # Errors
///
/// `limit` if a length does not fit a `u32`.
pub fn context_prefix(
    suite_id: &str,
    binders: &[Ty],
    result: &Ty,
    header: Header,
) -> Result<Vec<u8>, GenError> {
    let mut out = header.bytes().to_vec();
    put_field(&mut out, suite_id)?;
    put_u32(&mut out, binders.len())?;
    for t in binders {
        put_field(&mut out, &t.name())?;
    }
    put_field(&mut out, &result.name())?;
    Ok(out)
}

/// Encode `v`, a value of type `ty`, onto `out`, for a type that names no record or variant (every
/// type of `ir.core`, `ir.tuple`, `ir.fixed` and `Option`).  [`encode_in`] is the general form.
///
/// # Errors
///
/// As [`encode_in`].
pub fn encode(ty: &Ty, v: &Value, out: &mut Vec<u8>) -> Result<(), GenError> {
    encode_in(&Decls::default(), ty, v, out)
}

/// Encode `v`, a value of type `ty`, onto `out`; `decls` are the document's records and variants.
///
/// # Errors
///
/// `W0701` for a value that does not fit its declared type: a `Nat` or unsigned value past
/// `2^64-1` or negative (a `Nat` *value* is in `0..=2^64-1`, VER04r 2.3), a signed value
/// outside `i64`, a value outside a fixed-width type's own range (all three say ``a value is outside the
/// range of <type> (VER04r 2.3)``, the oracle's sentence), an `Int` anywhere (VER04r 2.1), or a value of
/// the wrong shape.
pub fn encode_in(decls: &Decls, ty: &Ty, v: &Value, out: &mut Vec<u8>) -> Result<(), GenError> {
    let bad = |what: &str| GenError::new(code::W0701, "", format!("a stream value {what}"));
    // The oracle's sentence (`stream.make_encoder`, VER04r 2.3), so a refusal reads alike in both tools.
    let out_of_range = || {
        GenError::new(
            code::W0701,
            "",
            format!("a value is outside the range of {} (VER04r 2.3)", ty.name()),
        )
    };
    match (ty, v) {
        // VER04r 2.1: `Int` is model-internal, so a value of it has no stream form (checked first, whatever
        // the value, as the oracle's `make_encoder` does).
        (Ty::Int, _) => {
            return Err(GenError::new(
                code::W0701,
                "",
                "an `Int` value has no stream encoding (VER04r 2.1)",
            ))
        }
        (Ty::Bool, Value::Bool(b)) => out.push(u8::from(*b)),
        (Ty::Nat | Ty::Fixed(_), Value::Int(i)) => {
            // A `Nat` or unsigned value is `0..=2^64-1` (the row is 8 bytes wide), a signed one `i64`, and a
            // fixed-width type its own range: an exact `Int` result such as `a - 1` can leave all of them.
            if let Some(t) = ty.fixed() {
                if *i < t.min() || *i > t.max() {
                    return Err(out_of_range());
                }
            }
            let signed = ty.fixed().is_some_and(|t| t.signed());
            if signed {
                let n = i64::try_from(*i).map_err(|_| out_of_range())?;
                out.push(0x11);
                out.extend_from_slice(&n.to_le_bytes());
            } else {
                let n = u64::try_from(*i).map_err(|_| out_of_range())?;
                out.push(0x10);
                out.extend_from_slice(&n.to_le_bytes());
            }
        }
        (Ty::Bytes(width), Value::Bytes(b)) => {
            if let Some(Width::Lit(n)) = width {
                if b.len() as u64 != *n {
                    return Err(bad("does not have the declared Bytes<N> length"));
                }
            }
            out.push(0x20);
            put_u32(out, b.len())?;
            out.extend_from_slice(b);
        }
        (Ty::Result(ok, _), Value::Ok(inner)) => {
            out.push(0x50);
            encode_in(decls, ok, inner, out)?;
        }
        (Ty::Result(..), Value::Err(case)) => {
            out.push(0x51);
            put_field(out, case)?;
        }
        (Ty::Tuple(parts), Value::Tuple(items)) if parts.len() == items.len() => {
            out.push(0x64);
            put_u32(out, items.len())?;
            for (t, v) in parts.iter().zip(items) {
                encode_in(decls, t, v, out)?;
            }
        }
        (Ty::Option(_), Value::Opt(None)) => out.push(0x40),
        (Ty::Option(inner), Value::Opt(Some(v))) => {
            out.push(0x41);
            encode_in(decls, inner, v, out)?;
        }
        (Ty::Named(name), Value::Con(ctor, items)) => {
            let fields = decls
                .fields(name, ctor)
                .filter(|f| f.len() == items.len())
                .ok_or_else(|| bad("names a constructor its type does not declare"))?;
            if decls.is_record(name) {
                // a record is encoded like a tuple of its fields
                out.push(0x64);
                put_u32(out, items.len())?;
            } else {
                out.push(0x65);
                put_field(out, ctor)?;
                put_u32(out, items.len())?;
            }
            for (t, v) in fields.iter().zip(items) {
                encode_in(decls, t, v, out)?;
            }
        }
        _ => return Err(bad("does not have its declared shape")),
    }
    Ok(())
}

/// `SHA-256("warrant-chunks-v1\0" || u32 m || the m raw chunk digests)`, as hex.
///
/// # Errors
///
/// `limit` if `m` does not fit a `u32`.
pub fn chunk_digest_framing(digests: &[[u8; 32]]) -> Result<String, GenError> {
    let mut buf = CHUNKS_FRAME.to_vec();
    put_u32(&mut buf, digests.len())?;
    for d in digests {
        buf.extend_from_slice(d);
    }
    Ok(sha256_hex(&buf))
}

/// FNV-1a, 64 bit, over the UTF-8 bytes (VER04r 5.6), in wrapping arithmetic.
pub fn fnv1a64(text: &str) -> u64 {
    text.bytes().fold(FNV_OFFSET, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(FNV_PRIME)
    })
}

/// The automatic slim stride for `m` chunks: `ceil(m / 14)`.
pub fn auto_stride(m: u64) -> u64 {
    m.div_ceil(14)
}

/// The slim base set of a suite of `m` chunks, before witness chunks and profile lines.
/// `stride` overrides the automatic stride (the profile's `slim_stride`, which the caller
/// has checked is smaller than [`auto_stride`]).
pub fn slim_base_set(suite_id: &str, m: u64, stride: Option<u64>) -> Vec<u64> {
    if m <= 16 {
        return (0..m).collect();
    }
    let k = stride.unwrap_or_else(|| auto_stride(m)).max(1);
    let o = fnv1a64(suite_id) % k;
    let mut set = vec![0, m - 1];
    let mut c = o;
    while c < m {
        set.push(c);
        c += k;
    }
    set.sort_unstable();
    set.dedup();
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    fn enc(ty: &Ty, v: &Value) -> Vec<u8> {
        let mut out = Vec::new();
        encode(ty, v, &mut out).unwrap();
        out
    }

    // Every expected byte string below is written by hand from the VER04r 2.4 table.

    #[test]
    fn scalar_encodings() {
        assert_eq!(enc(&Ty::Bool, &Value::Bool(false)), [0x00]);
        assert_eq!(enc(&Ty::Bool, &Value::Bool(true)), [0x01]);
        assert_eq!(
            enc(&Ty::Nat, &Value::Int(1)),
            [0x10, 1, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            enc(&Ty::Fixed(FixedTy::U8), &Value::Int(255)),
            [0x10, 255, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            enc(&Ty::Fixed(FixedTy::U64), &Value::Int(i128::from(u64::MAX))),
            [0x10, 255, 255, 255, 255, 255, 255, 255, 255]
        );
        // the worked example of 2.4: -1 as I8 is 11 ffffffffffffffff
        assert_eq!(
            enc(&Ty::Fixed(FixedTy::I8), &Value::Int(-1)),
            [0x11, 255, 255, 255, 255, 255, 255, 255, 255]
        );
        assert_eq!(
            enc(&Ty::Fixed(FixedTy::I16), &Value::Int(-2)),
            [0x11, 254, 255, 255, 255, 255, 255, 255, 255]
        );
        assert_eq!(
            enc(&Ty::Fixed(FixedTy::I64), &Value::Int(i128::from(i64::MIN))),
            [0x11, 0, 0, 0, 0, 0, 0, 0, 0x80]
        );
        assert_eq!(
            enc(&Ty::Fixed(FixedTy::I32), &Value::Int(5)),
            [0x11, 5, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn bytes_and_result_encodings() {
        assert_eq!(
            enc(&Ty::bytes_n(2), &Value::bytes(&[0xaa, 0xbb])),
            [0x20, 2, 0, 0, 0, 0xaa, 0xbb]
        );
        assert_eq!(enc(&Ty::BYTES, &Value::bytes(&[])), [0x20, 0, 0, 0, 0]);
        let r = Ty::Result(Box::new(Ty::BYTES), "E".into());
        assert_eq!(
            enc(&r, &Value::Ok(Box::new(Value::bytes(&[7])))),
            [0x50, 0x20, 1, 0, 0, 0, 7]
        );
        assert_eq!(
            enc(&r, &Value::Err("BAD".into())),
            [0x51, 3, 0, 0, 0, b'B', b'A', b'D']
        );
    }

    #[test]
    fn values_that_do_not_fit_their_type_are_refused() {
        let mut out = Vec::new();
        // a Nat value is 0..=2^64-1
        assert!(encode(&Ty::Nat, &Value::Int(1 << 64), &mut out).is_err());
        assert!(encode(&Ty::Nat, &Value::Int(-1), &mut out).is_err());
        // fixed-width range (L2): U8 300 and I8 200 are not values of their types
        assert!(encode(&Ty::Fixed(FixedTy::U8), &Value::Int(300), &mut out).is_err());
        assert!(encode(&Ty::Fixed(FixedTy::U8), &Value::Int(255), &mut out).is_ok());
        assert!(encode(&Ty::Fixed(FixedTy::I8), &Value::Int(200), &mut out).is_err());
        assert!(encode(&Ty::Fixed(FixedTy::I8), &Value::Int(-128), &mut out).is_ok());
        // Bytes<N> has exactly N bytes
        assert!(encode(&Ty::bytes_n(2), &Value::bytes(&[1]), &mut out).is_err());
        assert!(encode(&Ty::bytes_n(2), &Value::bytes(&[1, 2]), &mut out).is_ok());
        assert!(encode(&Ty::Fixed(FixedTy::U64), &Value::Int(-1), &mut out).is_err());
        assert!(encode(&Ty::Fixed(FixedTy::I64), &Value::Int(1 << 63), &mut out).is_err());
        // Int has no stream encoding
        let e = encode(&Ty::Int, &Value::Int(0), &mut out).unwrap_err();
        assert!(e.message.contains("Int"));
        // wrong shapes
        assert!(encode(&Ty::Bool, &Value::Int(0), &mut out).is_err());
        assert!(encode(&Ty::BYTES, &Value::Bool(true), &mut out).is_err());
        assert!(encode(&Ty::Nat, &Value::bytes(&[]), &mut out).is_err());
        assert!(encode(
            &Ty::Result(Box::new(Ty::Bool), "E".into()),
            &Value::Bool(true),
            &mut out
        )
        .is_err());
    }

    #[test]
    fn the_v1_set_and_the_header_choice() {
        for t in [
            Ty::Bool,
            Ty::Nat,
            Ty::BYTES,
            Ty::bytes_n(16),
            Ty::Fixed(FixedTy::U8),
            Ty::Fixed(FixedTy::U64),
            Ty::Result(Box::new(Ty::BYTES), "E".into()),
        ] {
            assert!(in_v1_set(&t), "{t:?}");
        }
        for t in [
            Ty::Int,
            Ty::Fixed(FixedTy::U16),
            Ty::Fixed(FixedTy::U32),
            Ty::Fixed(FixedTy::I8),
            Ty::Fixed(FixedTy::I64),
            Ty::Result(Box::new(Ty::Fixed(FixedTy::I8)), "E".into()),
        ] {
            assert!(!in_v1_set(&t), "{t:?}");
        }
        assert_eq!(suite_header(&[Ty::Bool, Ty::BYTES], &Ty::Bool), Header::V1);
        assert_eq!(
            suite_header(&[Ty::Bool, Ty::Fixed(FixedTy::U16)], &Ty::Bool),
            Header::V2
        );
        // a v2 result type alone makes the suite v2
        assert_eq!(
            suite_header(&[Ty::Bool], &Ty::Fixed(FixedTy::I8)),
            Header::V2
        );
        assert_eq!(Header::V1.name(), "warrant-stream-v1");
        assert_eq!(Header::V2.bytes(), b"warrant-stream-v2\0");
    }

    #[test]
    fn the_context_prefix_is_header_id_arity_types_result() {
        let p = context_prefix("s/x", &[Ty::BYTES, Ty::Bool], &Ty::Bool, Header::V1).unwrap();
        let mut want = b"warrant-stream-v1\0".to_vec();
        want.extend_from_slice(&[3, 0, 0, 0]);
        want.extend_from_slice(b"s/x");
        want.extend_from_slice(&[2, 0, 0, 0]);
        want.extend_from_slice(&[5, 0, 0, 0]);
        want.extend_from_slice(b"Bytes");
        want.extend_from_slice(&[4, 0, 0, 0]);
        want.extend_from_slice(b"Bool");
        want.extend_from_slice(&[4, 0, 0, 0]);
        want.extend_from_slice(b"Bool");
        assert_eq!(p, want);
        // a different binder-type spelling is a different prefix (Bytes<4> is not Bytes)
        let q = context_prefix("s/x", &[Ty::bytes_n(4), Ty::Bool], &Ty::Bool, Header::V1).unwrap();
        assert_ne!(p, q);
    }

    #[test]
    fn chunk_framing_is_the_listing_of_raw_digests() {
        // the empty listing: SHA-256("warrant-chunks-v1\0" 00000000)
        let mut buf = CHUNKS_FRAME.to_vec();
        buf.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(chunk_digest_framing(&[]).unwrap(), sha256_hex(&buf));
        // order and count matter
        let (a, b) = ([1u8; 32], [2u8; 32]);
        let ab = chunk_digest_framing(&[a, b]).unwrap();
        assert_ne!(ab, chunk_digest_framing(&[b, a]).unwrap());
        assert_ne!(ab, chunk_digest_framing(&[a]).unwrap());
        let mut buf = CHUNKS_FRAME.to_vec();
        buf.extend_from_slice(&[2, 0, 0, 0]);
        buf.extend_from_slice(&a);
        buf.extend_from_slice(&b);
        assert_eq!(ab, sha256_hex(&buf));
    }

    #[test]
    fn fnv1a64_known_answers() {
        // Published FNV-1a 64-bit test vectors.
        assert_eq!(fnv1a64(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64("foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn the_slim_base_set_follows_the_section_5_6_formula() {
        // m <= 16: every chunk
        assert_eq!(slim_base_set("x", 1, None), [0]);
        assert_eq!(slim_base_set("x", 16, None), (0..16).collect::<Vec<_>>());
        // m = 256: k = ceil(256/14) = 19; the set is {0, 255} + every 19th from o
        assert_eq!(auto_stride(256), 19);
        let id = "warrant/gf256/exhaustive/mul_agrees";
        let o = fnv1a64(id) % 19;
        let set = slim_base_set(id, 256, None);
        assert!(set.contains(&0) && set.contains(&255));
        assert!(set.windows(2).all(|w| w[0] < w[1]));
        assert!(set.len() <= 16);
        let mut want: Vec<u64> = vec![0, 255];
        want.extend((o..256).step_by(19));
        want.sort_unstable();
        want.dedup();
        assert_eq!(set, want);
        // m = 17: k = 2
        assert_eq!(auto_stride(17), 2);
        assert_eq!(auto_stride(16), 2);
        assert_eq!(auto_stride(14), 1);
        let s17 = slim_base_set("x", 17, None);
        assert_eq!(s17.first(), Some(&0));
        assert_eq!(s17.last(), Some(&16));
        // an explicit smaller stride gives at least as many chunks
        let denser = slim_base_set(id, 256, Some(7));
        assert!(denser.len() > set.len());
        assert!(denser.contains(&0) && denser.contains(&255));
        // stride 0 cannot loop forever
        assert!(!slim_base_set(id, 64, Some(0)).is_empty());
    }
}
