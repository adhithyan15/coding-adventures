//! MSB-first bit reader for SWF bit-packed primitives.

use crate::cxform::SwfColorTransform;
use crate::error::SwfError;
use crate::matrix::SwfMatrix;
use crate::rect::SwfRect;

/// A zero-allocation MSB-first bitstream reader over a byte slice.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: usize, // 0..8 bits consumed from data[byte_pos]
}

impl<'a> BitReader<'a> {
    /// Creates a new `BitReader` positioned at the start of `data`.
    #[inline]
    pub const fn from_bits(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Total number of bits remaining in the reader.
    #[inline]
    pub fn bits_remaining(&self) -> usize {
        let total_bits = self.data.len().saturating_mul(8);
        let consumed = self.byte_pos.saturating_mul(8) + self.bit_pos;
        total_bits.saturating_sub(consumed)
    }

    /// Number of full or partial bytes consumed so far.
    #[inline]
    pub fn bytes_consumed(&self) -> usize {
        if self.bit_pos > 0 {
            self.byte_pos.saturating_add(1)
        } else {
            self.byte_pos
        }
    }

    /// Aligns reader to the next whole byte boundary.
    #[inline]
    pub fn align_to_byte(&mut self) {
        if self.bit_pos > 0 {
            self.byte_pos = self.byte_pos.saturating_add(1);
            self.bit_pos = 0;
        }
    }

    /// Reads `n` unsigned bits (0..=32) from the MSB of current bytes.
    pub fn read_ub(&mut self, n: usize) -> Result<u32, SwfError> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(SwfError::InvalidBitField);
        }
        if self.bits_remaining() < n {
            return Err(SwfError::UnexpectedEof);
        }

        let mut val: u32 = 0;
        let mut bits_needed = n;

        while bits_needed > 0 {
            if self.byte_pos >= self.data.len() {
                return Err(SwfError::UnexpectedEof);
            }
            let current_byte = self.data[self.byte_pos];
            let bits_available = 8 - self.bit_pos;

            if bits_needed <= bits_available {
                let shift = bits_available - bits_needed;
                let mask = ((1u32 << bits_needed) - 1) as u8;
                let chunk = (current_byte >> shift) & mask;
                val = (val << bits_needed) | (chunk as u32);
                self.bit_pos += bits_needed;
                if self.bit_pos == 8 {
                    self.byte_pos += 1;
                    self.bit_pos = 0;
                }
                bits_needed = 0;
            } else {
                let mask = ((1u32 << bits_available) - 1) as u8;
                let chunk = current_byte & mask;
                val = (val << bits_available) | (chunk as u32);
                bits_needed -= bits_available;
                self.byte_pos += 1;
                self.bit_pos = 0;
            }
        }

        Ok(val)
    }

    /// Reads `n` signed bits (0..=32), sign-extending to `i32`.
    pub fn read_sb(&mut self, n: usize) -> Result<i32, SwfError> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(SwfError::InvalidBitField);
        }
        let u = self.read_ub(n)?;
        let shift = 32 - n;
        Ok(((u as i32) << shift) >> shift)
    }

    /// Reads `n` signed bits as a 16.16 fixed-point float.
    pub fn read_fb(&mut self, n: usize) -> Result<f64, SwfError> {
        let s = self.read_sb(n)?;
        Ok(s as f64 / 65536.0)
    }

    /// Reads an SWF bit-packed `RECT` record and byte-aligns.
    pub fn read_rect(&mut self) -> Result<SwfRect, SwfError> {
        let nbits = self.read_ub(5)? as usize;
        let xmin = self.read_sb(nbits)?;
        let xmax = self.read_sb(nbits)?;
        let ymin = self.read_sb(nbits)?;
        let ymax = self.read_sb(nbits)?;
        self.align_to_byte();
        Ok(SwfRect::create_rect(xmin, xmax, ymin, ymax))
    }

    /// Reads an SWF bit-packed `MATRIX` record and byte-aligns.
    pub fn read_matrix(&mut self) -> Result<SwfMatrix, SwfError> {
        let has_scale = self.read_ub(1)? != 0;
        let (scale_x, scale_y) = if has_scale {
            let n = self.read_ub(5)? as usize;
            (self.read_fb(n)?, self.read_fb(n)?)
        } else {
            (1.0, 1.0)
        };

        let has_rotate = self.read_ub(1)? != 0;
        let (rotate_skew_0, rotate_skew_1) = if has_rotate {
            let n = self.read_ub(5)? as usize;
            (self.read_fb(n)?, self.read_fb(n)?)
        } else {
            (0.0, 0.0)
        };

        let n_translate = self.read_ub(5)? as usize;
        let translate_x = self.read_sb(n_translate)?;
        let translate_y = self.read_sb(n_translate)?;

        self.align_to_byte();

        Ok(SwfMatrix {
            scale_x,
            scale_y,
            rotate_skew_0,
            rotate_skew_1,
            translate_x,
            translate_y,
        })
    }

    /// Reads an SWF bit-packed `CXFORM` record and byte-aligns.
    pub fn read_cxform(&mut self) -> Result<SwfColorTransform, SwfError> {
        let has_add = self.read_ub(1)? != 0;
        let has_mult = self.read_ub(1)? != 0;
        let nbits = self.read_ub(4)? as usize;

        let (r_mult, g_mult, b_mult) = if has_mult {
            (
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
            )
        } else {
            (256, 256, 256)
        };

        let (r_add, g_add, b_add) = if has_add {
            (
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
            )
        } else {
            (0, 0, 0)
        };

        self.align_to_byte();

        Ok(SwfColorTransform {
            r_mult,
            g_mult,
            b_mult,
            a_mult: 256,
            r_add,
            g_add,
            b_add,
            a_add: 0,
        })
    }

    /// Reads an SWF bit-packed `CXFORMWITHALPHA` record and byte-aligns.
    pub fn read_cxform_with_alpha(&mut self) -> Result<SwfColorTransform, SwfError> {
        let has_add = self.read_ub(1)? != 0;
        let has_mult = self.read_ub(1)? != 0;
        let nbits = self.read_ub(4)? as usize;

        let (r_mult, g_mult, b_mult, a_mult) = if has_mult {
            (
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
            )
        } else {
            (256, 256, 256, 256)
        };

        let (r_add, g_add, b_add, a_add) = if has_add {
            (
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
                self.read_sb(nbits)?,
            )
        } else {
            (0, 0, 0, 0)
        };

        self.align_to_byte();

        Ok(SwfColorTransform {
            r_mult,
            g_mult,
            b_mult,
            a_mult,
            r_add,
            g_add,
            b_add,
            a_add,
        })
    }
}
