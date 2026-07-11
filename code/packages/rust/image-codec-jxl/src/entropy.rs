//! # Entropy coding helpers
//!
//! Bridges between the generic `rans` crate and the image-codec-specific
//! wire format used by `image-codec-jxl`.
//!
//! ## Wire format for one rANS block
//!
//! Each rANS block is self-describing:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │ 4 bytes LE │ num_symbols — count of coded items │
//! │ 4 bytes LE │ alphabet_size — number of counts   │
//! │ alphabet_size × 4 bytes LE │ frequency counts   │
//! │ 4 bytes LE │ data_len — compressed byte count   │
//! │ data_len bytes │ rANS bitstream                 │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! The frequency counts are the *raw* (non-normalised) histogram of the
//! input symbols; `AnsTable::new` performs the normalisation internally.
//! Every count is initialised to at least 1 so that all symbols in the
//! declared alphabet are reachable.
//!
//! ## Per-channel residual encoding
//!
//! Gradient residuals for 8bpp images lie in [−255, 255].  The rANS crate
//! uses a `u8` symbol type (alphabet ≤ 256).  To handle the full range we
//! use a **two-pass split**:
//!
//! 1. **Sign pass** — encode a sign symbol for every pixel:
//!    - 0 = residual is zero
//!    - 1 = positive
//!    - 2 = negative
//!
//! 2. **Magnitude pass** — encode `|residual| − 1` (in [0, 254]) for every
//!    non-zero pixel.  Zero-residual pixels contribute nothing to this stream.
//!
//! This cleanly fits each sub-stream into u8 (alphabet sizes 3 and 255
//! respectively) and keeps the two distributions separate for better
//! compression.

use rans::{AnsTable, RansDecoder, RansEncoder};

// ── Encoding ────────────────────────────────────────────────────────────────

/// Append a self-describing rANS block to `out`.
///
/// `symbols` is the sequence to encode.  `counts` is the raw frequency table
/// (length = alphabet size); counts must be ≥ 1 for every symbol that appears.
///
/// Symbols are pushed to the encoder in **reverse** order (as required by rANS)
/// then the block header + compressed data are appended to `out`.
pub fn encode_rans_block(symbols: &[u8], counts: &[u32], out: &mut Vec<u8>) {
    // Header: num_symbols
    out.extend_from_slice(&(symbols.len() as u32).to_le_bytes());
    // Header: alphabet_size
    out.extend_from_slice(&(counts.len() as u32).to_le_bytes());
    // Header: frequency counts
    for &c in counts {
        out.extend_from_slice(&c.to_le_bytes());
    }

    // Build table and encode.
    let table = AnsTable::new(counts).expect("rANS table build failed — invalid counts");
    let mut enc = RansEncoder::new(&table);
    for &s in symbols.iter().rev() {
        enc.put(s);
    }
    let data = enc.finish();

    // Header: data_len + compressed bytes.
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
}

/// Encode the residuals for one image channel (sign + magnitude blocks).
///
/// `residuals` must lie in [−255, 255] (guaranteed by gradient prediction on
/// 8bpp input).  Two rANS blocks are appended to `out`:
///
/// 1. Sign block (alphabet size 3)
/// 2. Magnitude block (alphabet size 255, for magnitudes 1–255 → indices 0–254)
pub fn encode_channel_residuals(residuals: &[i32], out: &mut Vec<u8>) {
    // ── Build sign and magnitude symbol sequences ────────────────────────
    let mut sign_counts = [1u32; 3]; // start at 1 so no symbol has freq 0
    let mut mag_counts = [1u32; 255];

    let mut signs = Vec::with_capacity(residuals.len());
    let mut mags: Vec<u8> = Vec::new();

    for &r in residuals {
        let (sign_sym, mag_sym) = if r == 0 {
            (0u8, None)
        } else if r > 0 {
            let m = (r.unsigned_abs() - 1).min(254) as u8;
            (1u8, Some(m))
        } else {
            let m = (r.unsigned_abs() - 1).min(254) as u8;
            (2u8, Some(m))
        };

        signs.push(sign_sym);
        sign_counts[sign_sym as usize] += 1;

        if let Some(m) = mag_sym {
            mags.push(m);
            mag_counts[m as usize] += 1;
        }
    }

    encode_rans_block(&signs, &sign_counts, out);
    encode_rans_block(&mags, &mag_counts, out);
}

// ── Decoding ────────────────────────────────────────────────────────────────

/// Decode a self-describing rANS block from `data`.
///
/// Returns `(symbols, bytes_consumed)`.
///
/// # Errors
///
/// Returns `Err` if the data is truncated, the header fields are inconsistent,
/// or rANS decoding fails.
pub fn decode_rans_block(data: &[u8]) -> Result<(Vec<u8>, usize), String> {
    // ── Parse header ────────────────────────────────────────────────────
    if data.len() < 8 {
        return Err("JXL: rANS block header too short (need 8 bytes for num_symbols + alphabet_size)".into());
    }
    let num_symbols = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let alphabet_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;

    // Sanity: alphabet_size must be ≤ 256 (rANS crate limit).
    if alphabet_size == 0 || alphabet_size > 256 {
        return Err(format!(
            "JXL: invalid rANS alphabet_size {} (must be 1–256)",
            alphabet_size
        ));
    }

    // DoS guard: a hostile input could claim num_symbols = u32::MAX (4 billion),
    // causing a multi-gigabyte Vec allocation and OOM.  We cap at 256 M symbols,
    // which covers a 16384×16384-pixel image — far beyond any realistic use case
    // for this teaching codec.
    const MAX_SYMBOLS: usize = 256 * 1024 * 1024; // 256 M
    if num_symbols > MAX_SYMBOLS {
        return Err(format!(
            "JXL: rANS block claims {} symbols, exceeding the {} limit",
            num_symbols, MAX_SYMBOLS
        ));
    }

    let counts_end = 8 + alphabet_size * 4;
    if data.len() < counts_end + 4 {
        return Err("JXL: rANS block truncated in frequency table".into());
    }
    let counts: Vec<u32> = (0..alphabet_size)
        .map(|i| u32::from_le_bytes(data[8 + i * 4..12 + i * 4].try_into().unwrap()))
        .collect();

    let data_len = u32::from_le_bytes(data[counts_end..counts_end + 4].try_into().unwrap()) as usize;
    let data_start = counts_end + 4;
    // Guard against usize overflow in data_start + data_len before we do the
    // bounds check below.  data_start is at most 8 + 256*4 + 4 = 1036 bytes,
    // so the checked_add is defensive but cheap.
    let data_end = data_start.checked_add(data_len).ok_or_else(|| {
        "JXL: rANS block data_len overflows address space".to_string()
    })?;

    if data.len() < data_end {
        return Err(format!(
            "JXL: rANS block claims {} compressed bytes but only {} remain",
            data_len,
            data.len().saturating_sub(data_start)
        ));
    }

    // ── Decode symbols ───────────────────────────────────────────────────
    let table = AnsTable::new(&counts).map_err(|e| format!("JXL: rANS table: {}", e))?;

    let symbols = if num_symbols == 0 {
        Vec::new()
    } else {
        let mut dec = RansDecoder::new(&table, &data[data_start..data_end])
            .map_err(|e| format!("JXL: rANS decoder init: {}", e))?;
        (0..num_symbols).map(|_| dec.get()).collect()
    };

    Ok((symbols, data_end))
}

/// Decode residuals for one channel from `data` (sign + magnitude blocks).
///
/// Returns `(residuals, bytes_consumed)`.
pub fn decode_channel_residuals(data: &[u8]) -> Result<(Vec<i32>, usize), String> {
    // Decode sign block.
    let (signs, sign_len) = decode_rans_block(data)?;
    // Decode magnitude block.
    let (mags, mag_len) = decode_rans_block(&data[sign_len..])?;

    // Reconstruct residuals.
    let mut mag_iter = mags.into_iter();
    let residuals: Result<Vec<i32>, String> = signs
        .into_iter()
        .map(|s| match s {
            0 => Ok(0i32),
            1 => {
                let m = mag_iter.next().ok_or_else(|| {
                    "JXL: magnitude stream exhausted before sign stream".to_string()
                })?;
                Ok(m as i32 + 1)
            }
            2 => {
                let m = mag_iter.next().ok_or_else(|| {
                    "JXL: magnitude stream exhausted before sign stream".to_string()
                })?;
                Ok(-(m as i32 + 1))
            }
            other => Err(format!("JXL: invalid sign symbol {}", other)),
        })
        .collect();

    Ok((residuals?, sign_len + mag_len))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn rt_rans(symbols: &[u8], alphabet: usize) {
        let mut counts = vec![1u32; alphabet];
        for &s in symbols {
            counts[s as usize] += 1;
        }
        let mut buf = Vec::new();
        encode_rans_block(symbols, &counts, &mut buf);
        let (decoded, consumed) = decode_rans_block(&buf).unwrap();
        assert_eq!(consumed, buf.len());
        assert_eq!(&decoded, symbols);
    }

    #[test]
    fn rans_block_round_trip_empty() {
        rt_rans(&[], 3);
    }

    #[test]
    fn rans_block_round_trip_single() {
        rt_rans(&[0], 2);
    }

    #[test]
    fn rans_block_round_trip_many() {
        let syms: Vec<u8> = (0u8..20).map(|i| i % 3).collect();
        rt_rans(&syms, 3);
    }

    fn rt_residuals(residuals: &[i32]) {
        let mut buf = Vec::new();
        encode_channel_residuals(residuals, &mut buf);
        let (decoded, consumed) = decode_channel_residuals(&buf).unwrap();
        assert_eq!(consumed, buf.len());
        assert_eq!(&decoded, residuals);
    }

    #[test]
    fn residuals_all_zero() {
        rt_residuals(&[0; 16]);
    }

    #[test]
    fn residuals_positive() {
        rt_residuals(&[1, 5, 10, 100, 127]);
    }

    #[test]
    fn residuals_negative() {
        rt_residuals(&[-1, -5, -127, -100]);
    }

    #[test]
    fn residuals_mixed() {
        let r: Vec<i32> = (0..50).map(|i| i - 25).collect();
        rt_residuals(&r);
    }

    #[test]
    fn residuals_extremes() {
        rt_residuals(&[255, -255, 0, 127, -128, 1, -1]);
    }
}
