//! Bounded MSB-first SWF bitstream writer (`SwfBitWriter`) with checked bit-widths (`SWF08`).

#![forbid(unsafe_code)]

use swf_container::{SwfColorTransform, SwfMatrix};

use crate::error::{CorpusError, MAX_BIT_WIDTH, MAX_CORPUS_SWF_BYTES};

/// Computes the minimum signed bit width (`0..=MAX_BIT_WIDTH`) needed to represent all values in `vals`.
pub fn min_signed_bit_width(vals: &[i32]) -> Result<u8, CorpusError> {
    if vals.iter().all(|&v| v == 0) {
        return Ok(0);
    }
    for bits in 1..=MAX_BIT_WIDTH {
        let min_val = -(1i64 << (bits - 1));
        let max_val = (1i64 << (bits - 1)) - 1;
        if vals.iter().all(|&v| {
            let v64 = v as i64;
            v64 >= min_val && v64 <= max_val
        }) {
            return Ok(bits);
        }
    }
    Err(CorpusError::BitWidthExceeded)
}

/// Computes the minimum unsigned bit width (`0..=MAX_BIT_WIDTH`) needed to represent `val`.
pub fn min_unsigned_bit_width(val: u32) -> Result<u8, CorpusError> {
    if val == 0 {
        return Ok(0);
    }
    for bits in 1..=MAX_BIT_WIDTH {
        let max_val = (1u64 << bits) - 1;
        if (val as u64) <= max_val {
            return Ok(bits);
        }
    }
    Err(CorpusError::BitWidthExceeded)
}

/// Bounded MSB-first bitstream encoder for SWF `RECT`, `MATRIX`, `CXFORM`, shape records, and ADPCM.
#[derive(Debug, Clone, Default)]
pub struct SwfBitWriter {
    bytes: Vec<u8>,
    cur_byte: u8,
    used_bits: u8,
}

impl SwfBitWriter {
    /// Creates an empty `SwfBitWriter`.
    pub fn bit_writer_new() -> Self {
        Self {
            bytes: Vec::new(),
            cur_byte: 0,
            used_bits: 0,
        }
    }

    /// Writes `bits` unsigned bits (`0..=MAX_BIT_WIDTH`) MSB-first.
    pub fn bit_writer_write_ub(&mut self, value: u32, bits: u8) -> Result<(), CorpusError> {
        if bits > MAX_BIT_WIDTH {
            return Err(CorpusError::BitWidthExceeded);
        }
        if bits == 0 {
            return Ok(());
        }
        for shift in (0..bits).rev() {
            let bit = ((value >> shift) & 1) as u8;
            self.cur_byte = (self.cur_byte << 1) | bit;
            self.used_bits += 1;
            if self.used_bits == 8 {
                if self.bytes.len() >= MAX_CORPUS_SWF_BYTES {
                    return Err(CorpusError::SwfSizeExceeded);
                }
                self.bytes.push(self.cur_byte);
                self.cur_byte = 0;
                self.used_bits = 0;
            }
        }
        Ok(())
    }

    /// Writes `bits` signed two's-complement bits (`0..=MAX_BIT_WIDTH`) MSB-first.
    pub fn bit_writer_write_sb(&mut self, value: i32, bits: u8) -> Result<(), CorpusError> {
        if bits > MAX_BIT_WIDTH {
            return Err(CorpusError::BitWidthExceeded);
        }
        if bits == 0 {
            return if value == 0 {
                Ok(())
            } else {
                Err(CorpusError::BitWidthExceeded)
            };
        }
        let min_val = -(1i64 << (bits - 1));
        let max_val = (1i64 << (bits - 1)) - 1;
        let v64 = value as i64;
        if v64 < min_val || v64 > max_val {
            return Err(CorpusError::BitWidthExceeded);
        }
        self.bit_writer_write_ub(value as u32, bits)
    }

    /// Writes a 16.16 fixed-point float encoded in `bits` signed bits.
    pub fn bit_writer_write_fb(&mut self, value: f64, bits: u8) -> Result<(), CorpusError> {
        if !value.is_finite() {
            return Err(CorpusError::IntegerOverflow);
        }
        let scaled = (value * 65536.0).round();
        if scaled < (i32::MIN as f64) || scaled > (i32::MAX as f64) {
            return Err(CorpusError::IntegerOverflow);
        }
        self.bit_writer_write_sb(scaled as i32, bits)
    }

    /// Pads the current byte with zero bits if not already byte-aligned.
    pub fn bit_writer_align(&mut self) -> Result<(), CorpusError> {
        if self.used_bits > 0 {
            if self.bytes.len() >= MAX_CORPUS_SWF_BYTES {
                return Err(CorpusError::SwfSizeExceeded);
            }
            let padded = self.cur_byte << (8 - self.used_bits);
            self.bytes.push(padded);
            self.cur_byte = 0;
            self.used_bits = 0;
        }
        Ok(())
    }

    /// Appends raw aligned bytes (aligning first if needed).
    pub fn bit_writer_write_bytes(&mut self, raw: &[u8]) -> Result<(), CorpusError> {
        self.bit_writer_align()?;
        if self
            .bytes
            .len()
            .checked_add(raw.len())
            .is_none_or(|t| t > MAX_CORPUS_SWF_BYTES)
        {
            return Err(CorpusError::SwfSizeExceeded);
        }
        self.bytes.extend_from_slice(raw);
        Ok(())
    }

    /// Writes an SWF bit-packed `RECT` in twips and byte-aligns.
    pub fn bit_writer_write_rect(
        &mut self,
        xmin: i32,
        xmax: i32,
        ymin: i32,
        ymax: i32,
    ) -> Result<(), CorpusError> {
        let nbits = min_signed_bit_width(&[xmin, xmax, ymin, ymax])?;
        self.bit_writer_write_ub(nbits as u32, 5)?;
        self.bit_writer_write_sb(xmin, nbits)?;
        self.bit_writer_write_sb(xmax, nbits)?;
        self.bit_writer_write_sb(ymin, nbits)?;
        self.bit_writer_write_sb(ymax, nbits)?;
        self.bit_writer_align()
    }

    /// Writes an SWF bit-packed `MATRIX` record and byte-aligns.
    pub fn bit_writer_write_matrix(&mut self, m: &SwfMatrix) -> Result<(), CorpusError> {
        if !m.scale_x.is_finite()
            || !m.scale_y.is_finite()
            || !m.rotate_skew_0.is_finite()
            || !m.rotate_skew_1.is_finite()
        {
            return Err(CorpusError::IntegerOverflow);
        }
        let has_scale = (m.scale_x - 1.0).abs() > 1e-6 || (m.scale_y - 1.0).abs() > 1e-6;
        self.bit_writer_write_ub(u32::from(has_scale), 1)?;
        if has_scale {
            let sx = (m.scale_x * 65536.0).round();
            let sy = (m.scale_y * 65536.0).round();
            if !sx.is_finite()
                || !sy.is_finite()
                || sx < i32::MIN as f64
                || sx > i32::MAX as f64
                || sy < i32::MIN as f64
                || sy > i32::MAX as f64
            {
                return Err(CorpusError::IntegerOverflow);
            }
            let sx_i = sx as i32;
            let sy_i = sy as i32;
            let nbits = min_signed_bit_width(&[sx_i, sy_i])?;
            self.bit_writer_write_ub(nbits as u32, 5)?;
            self.bit_writer_write_sb(sx_i, nbits)?;
            self.bit_writer_write_sb(sy_i, nbits)?;
        }

        let has_rotate = m.rotate_skew_0.abs() > 1e-6 || m.rotate_skew_1.abs() > 1e-6;
        self.bit_writer_write_ub(u32::from(has_rotate), 1)?;
        if has_rotate {
            let r0 = (m.rotate_skew_0 * 65536.0).round();
            let r1 = (m.rotate_skew_1 * 65536.0).round();
            if !r0.is_finite()
                || !r1.is_finite()
                || r0 < i32::MIN as f64
                || r0 > i32::MAX as f64
                || r1 < i32::MIN as f64
                || r1 > i32::MAX as f64
            {
                return Err(CorpusError::IntegerOverflow);
            }
            let r0_i = r0 as i32;
            let r1_i = r1 as i32;
            let nbits = min_signed_bit_width(&[r0_i, r1_i])?;
            self.bit_writer_write_ub(nbits as u32, 5)?;
            self.bit_writer_write_sb(r0_i, nbits)?;
            self.bit_writer_write_sb(r1_i, nbits)?;
        }

        let n_translate = min_signed_bit_width(&[m.translate_x, m.translate_y])?;
        self.bit_writer_write_ub(n_translate as u32, 5)?;
        self.bit_writer_write_sb(m.translate_x, n_translate)?;
        self.bit_writer_write_sb(m.translate_y, n_translate)?;
        self.bit_writer_align()
    }

    /// Writes an SWF bit-packed `CXFORMWITHALPHA` record and byte-aligns.
    pub fn bit_writer_write_cxform_alpha(
        &mut self,
        cx: &SwfColorTransform,
    ) -> Result<(), CorpusError> {
        let has_add = cx.r_add != 0 || cx.g_add != 0 || cx.b_add != 0 || cx.a_add != 0;
        let has_mult = cx.r_mult != 256 || cx.g_mult != 256 || cx.b_mult != 256 || cx.a_mult != 256;
        let mut vals = Vec::with_capacity(8);
        if has_mult {
            vals.extend_from_slice(&[cx.r_mult, cx.g_mult, cx.b_mult, cx.a_mult]);
        }
        if has_add {
            vals.extend_from_slice(&[cx.r_add, cx.g_add, cx.b_add, cx.a_add]);
        }
        let nbits = if vals.is_empty() {
            0
        } else {
            let b = min_signed_bit_width(&vals)?;
            if b > 15 {
                return Err(CorpusError::BitWidthExceeded);
            }
            b
        };
        self.bit_writer_write_ub(u32::from(has_add), 1)?;
        self.bit_writer_write_ub(u32::from(has_mult), 1)?;
        self.bit_writer_write_ub(nbits as u32, 4)?;
        if has_mult {
            self.bit_writer_write_sb(cx.r_mult, nbits)?;
            self.bit_writer_write_sb(cx.g_mult, nbits)?;
            self.bit_writer_write_sb(cx.b_mult, nbits)?;
            self.bit_writer_write_sb(cx.a_mult, nbits)?;
        }
        if has_add {
            self.bit_writer_write_sb(cx.r_add, nbits)?;
            self.bit_writer_write_sb(cx.g_add, nbits)?;
            self.bit_writer_write_sb(cx.b_add, nbits)?;
            self.bit_writer_write_sb(cx.a_add, nbits)?;
        }
        self.bit_writer_align()
    }

    /// Returns the total byte length if padded to the next byte boundary.
    #[inline]
    pub fn bit_writer_byte_len(&self) -> usize {
        self.bytes.len() + usize::from(self.used_bits > 0)
    }

    /// Consumes the writer, byte-aligns, and returns the encoded byte buffer.
    pub fn bit_writer_into_bytes(mut self) -> Result<Vec<u8>, CorpusError> {
        self.bit_writer_align()?;
        Ok(self.bytes)
    }
}
