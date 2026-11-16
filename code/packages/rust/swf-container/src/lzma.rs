//! Bounded LZMA decompressor and `ZwsConfig` for `ZWS` SWF containers.

use crate::error::SwfError;

/// 5-byte LZMA properties header alias.
pub type LzmaProperties = [u8; 5];

/// Configuration parameters extracted from a `ZWS` container header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZwsConfig {
    /// SWF format version (must be >= 13 for ZWS).
    pub version: u8,
    /// Total uncompressed SWF file size declared by header.
    pub uncompressed_file_length: u32,
    /// Compressed payload size declared by header.
    pub compressed_length: u32,
    /// Literal context bits (0..=8).
    pub lc: u8,
    /// Literal position bits (0..=4).
    pub lp: u8,
    /// Position bits (0..=4).
    pub pb: u8,
    /// Dictionary size in bytes.
    pub dict_size: u32,
}

impl ZwsConfig {
    /// Parses a `ZwsConfig` from the leading bytes of a `ZWS` SWF file.
    pub fn parse_zws_header(bytes: &[u8]) -> Result<Self, SwfError> {
        if bytes.len() < 17 {
            return Err(SwfError::UnexpectedEof);
        }
        if &bytes[0..3] != b"ZWS" {
            return Err(SwfError::InvalidSignature);
        }
        let version = bytes[3];
        if version < 13 {
            return Err(SwfError::UnsupportedVersion);
        }

        let uncompressed_file_length = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let compressed_length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        let prop_byte = bytes[12];
        if prop_byte >= 9 * 5 * 5 {
            return Err(SwfError::CorruptedData);
        }
        let lc = prop_byte % 9;
        let rem = prop_byte / 9;
        let lp = rem % 5;
        let pb = rem / 5;

        let dict_size = u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]);

        Ok(Self {
            version,
            uncompressed_file_length,
            compressed_length,
            lc,
            lp,
            pb,
            dict_size,
        })
    }

    /// Configured dictionary size in bytes.
    #[inline]
    pub fn zws_dict_size(&self) -> u32 {
        self.dict_size
    }
}

// LZMA Range Decoder
struct RangeDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    range: u32,
    code: u32,
}

impl<'a> RangeDecoder<'a> {
    fn new_range_decoder(data: &'a [u8]) -> Result<Self, SwfError> {
        if data.len() < 5 {
            return Err(SwfError::UnexpectedEof);
        }
        // First byte is usually 0
        let code = ((data[1] as u32) << 24)
            | ((data[2] as u32) << 16)
            | ((data[3] as u32) << 8)
            | (data[4] as u32);
        Ok(Self {
            data,
            pos: 5,
            range: 0xFFFF_FFFF,
            code,
        })
    }

    #[inline]
    fn bytes_consumed(&self) -> usize {
        self.pos
    }

    #[inline]
    fn normalize(&mut self) -> Result<(), SwfError> {
        if self.range < 0x0100_0000 {
            if self.pos >= self.data.len() {
                return Err(SwfError::UnexpectedEof);
            }
            self.code = (self.code << 8) | (self.data[self.pos] as u32);
            self.pos += 1;
            self.range <<= 8;
        }
        Ok(())
    }

    #[inline]
    fn decode_bit(&mut self, prob: &mut u16) -> Result<u32, SwfError> {
        let bound = (self.range >> 11) * (*prob as u32);
        if self.code < bound {
            self.range = bound;
            *prob += (2048 - *prob) >> 5;
            self.normalize()?;
            Ok(0)
        } else {
            self.range -= bound;
            self.code -= bound;
            *prob -= *prob >> 5;
            self.normalize()?;
            Ok(1)
        }
    }

    fn decode_direct_bits(&mut self, count: usize) -> Result<u32, SwfError> {
        let mut val: u32 = 0;
        for _ in 0..count {
            self.range >>= 1;
            self.code = self.code.wrapping_sub(self.range);
            let bit = if (self.code as i32) < 0 {
                self.code = self.code.wrapping_add(self.range);
                0
            } else {
                1
            };
            self.normalize()?;
            val = (val << 1) | bit;
        }
        Ok(val)
    }

    fn decode_bit_tree(&mut self, probs: &mut [u16]) -> Result<usize, SwfError> {
        let mut model_index = 1;
        while model_index < probs.len() {
            let bit = self.decode_bit(&mut probs[model_index])?;
            model_index = (model_index << 1) | (bit as usize);
        }
        Ok(model_index - probs.len())
    }

    fn decode_reverse_bit_tree(&mut self, probs: &mut [u16]) -> Result<usize, SwfError> {
        let mut model_index = 1;
        let mut symbol = 0;
        let mut bit_mask = 1;
        while model_index < probs.len() {
            let bit = self.decode_bit(&mut probs[model_index])?;
            model_index = (model_index << 1) | (bit as usize);
            if bit != 0 {
                symbol |= bit_mask;
            }
            bit_mask <<= 1;
        }
        Ok(symbol)
    }
}

// LenCoder for match / rep match lengths
struct LenCoder {
    choice: u16,
    choice2: u16,
    low: [u16; 16 * 8],
    mid: [u16; 16 * 8],
    high: [u16; 256],
}

impl LenCoder {
    fn new_len_coder() -> Self {
        Self {
            choice: 1024,
            choice2: 1024,
            low: [1024; 16 * 8],
            mid: [1024; 16 * 8],
            high: [1024; 256],
        }
    }

    fn decode(
        &mut self,
        range: &mut RangeDecoder<'_>,
        pos_state: usize,
    ) -> Result<usize, SwfError> {
        if range.decode_bit(&mut self.choice)? == 0 {
            let offset = pos_state << 3;
            let len = range.decode_bit_tree(&mut self.low[offset..offset + 8])?;
            Ok(len)
        } else if range.decode_bit(&mut self.choice2)? == 0 {
            let offset = pos_state << 3;
            let len = range.decode_bit_tree(&mut self.mid[offset..offset + 8])?;
            Ok(8 + len)
        } else {
            let len = range.decode_bit_tree(&mut self.high)?;
            Ok(16 + len)
        }
    }
}

/// Decompresses an LZMA stream bounded by output caps and expansion ratio check.
pub fn lzma_decompress_bounded(
    data: &[u8],
    expected_len: usize,
    config: &ZwsConfig,
    max_uncompressed_bytes: usize,
    max_ratio: f64,
) -> Result<Vec<u8>, SwfError> {
    if expected_len > max_uncompressed_bytes {
        return Err(SwfError::OutputLimitExceeded);
    }

    let mut range = RangeDecoder::new_range_decoder(data)?;
    let mut output: Vec<u8> = Vec::with_capacity(expected_len.min(65536));

    let mut is_match = [1024u16; 12 * 16];
    let mut is_rep = [1024u16; 12];
    let mut is_rep_g0 = [1024u16; 12];
    let mut is_rep_g1 = [1024u16; 12];
    let mut is_rep_g2 = [1024u16; 12];
    let mut is_rep0_long = [1024u16; 12 * 16];
    let mut pos_slot = [[1024u16; 64]; 4];
    let mut spec_pos = [1024u16; 115];
    let mut align_probs = [1024u16; 16];

    let mut len_coder = LenCoder::new_len_coder();
    let mut rep_len_coder = LenCoder::new_len_coder();

    let lit_size = 0x300 << (config.lc + config.lp);
    let mut lit_probs = vec![1024u16; lit_size];

    let pos_mask = (1 << config.pb) - 1;
    let lit_pos_mask = (1 << config.lp) - 1;

    let mut state = 0usize;
    let mut rep0 = 0usize;
    let mut rep1 = 0usize;
    let mut rep2 = 0usize;
    let mut rep3 = 0usize;

    while output.len() < expected_len {
        let pos_state = output.len() & pos_mask;
        let match_idx = (state << 4) + pos_state;

        if range.decode_bit(&mut is_match[match_idx])? == 0 {
            // Literal
            let prev_byte = *output.last().unwrap_or(&0) as usize;
            let lit_ctx =
                ((output.len() & lit_pos_mask) << config.lc) + (prev_byte >> (8 - config.lc));
            let sub_probs = &mut lit_probs[lit_ctx * 0x300..(lit_ctx + 1) * 0x300];

            let lit = if state < 7 {
                let mut sym = 1;
                while sym < 0x100 {
                    let bit = range.decode_bit(&mut sub_probs[sym])?;
                    sym = (sym << 1) | (bit as usize);
                }
                (sym - 0x100) as u8
            } else {
                let mut match_byte = if rep0 < output.len() {
                    output[output.len() - 1 - rep0] as usize
                } else {
                    0
                };
                let mut sym = 1;
                let mut match_bit_enabled = true;
                while sym < 0x100 {
                    let match_bit = if match_bit_enabled {
                        (match_byte >> 7) & 1
                    } else {
                        0
                    };
                    match_byte <<= 1;
                    let prob_idx = 0x100 + (match_bit << 8) + sym;
                    let bit = range.decode_bit(&mut sub_probs[prob_idx])?;
                    sym = (sym << 1) | (bit as usize);
                    if match_bit_enabled && bit as usize != match_bit {
                        match_bit_enabled = false;
                    }
                }
                (sym - 0x100) as u8
            };

            output.push(lit);
            state = if state < 4 {
                0
            } else if state < 10 {
                state - 3
            } else {
                state - 6
            };
        } else {
            // Match or Rep Match
            let len: usize;
            if range.decode_bit(&mut is_rep[state])? == 1 {
                // Rep match
                if range.decode_bit(&mut is_rep_g0[state])? == 0 {
                    let rep0_long_idx = (state << 4) + pos_state;
                    if range.decode_bit(&mut is_rep0_long[rep0_long_idx])? == 0 {
                        state = if state < 7 { 9 } else { 11 };
                        let b = *output
                            .get(
                                output
                                    .len()
                                    .checked_sub(1 + rep0)
                                    .ok_or(SwfError::CorruptedData)?,
                            )
                            .ok_or(SwfError::CorruptedData)?;
                        output.push(b);
                        continue;
                    }
                } else {
                    let dist: usize;
                    if range.decode_bit(&mut is_rep_g1[state])? == 0 {
                        dist = rep1;
                    } else {
                        if range.decode_bit(&mut is_rep_g2[state])? == 0 {
                            dist = rep2;
                        } else {
                            dist = rep3;
                            rep3 = rep2;
                        }
                        rep2 = rep1;
                    }
                    rep1 = rep0;
                    rep0 = dist;
                }
                len = rep_len_coder.decode(&mut range, pos_state)? + 2;
                state = if state < 7 { 8 } else { 11 };
            } else {
                // Normal match
                rep3 = rep2;
                rep2 = rep1;
                rep1 = rep0;
                state = if state < 7 { 7 } else { 10 };
                len = len_coder.decode(&mut range, pos_state)? + 2;

                let len_state = len.min(5) - 2;
                let slot = range.decode_bit_tree(&mut pos_slot[len_state])?;

                if slot < 4 {
                    rep0 = slot;
                } else {
                    let num_direct_bits = (slot >> 1) - 1;
                    let mut dist = (2 | (slot & 1)) << num_direct_bits;

                    if slot < 14 {
                        let offset = dist - slot;
                        let slice = spec_pos
                            .get_mut(offset..offset + (1 << num_direct_bits))
                            .ok_or(SwfError::CorruptedData)?;
                        let extra = range.decode_reverse_bit_tree(slice)?;
                        dist += extra;
                    } else {
                        let direct = range.decode_direct_bits(num_direct_bits - 4)? as usize;
                        dist += direct << 4;
                        let align = range.decode_reverse_bit_tree(&mut align_probs)?;
                        dist += align;
                    }
                    rep0 = dist;
                }
            }

            if rep0 >= output.len() {
                // End of stream or corrupt
                if rep0 == 0xFFFF_FFFF {
                    break;
                }
                return Err(SwfError::CorruptedData);
            }

            for _ in 0..len {
                if output.len() >= expected_len {
                    break;
                }
                let idx = output.len() - 1 - rep0;
                let b = output[idx];
                output.push(b);
            }
        }

        // Bomb defenses check
        if output.len() > max_uncompressed_bytes {
            return Err(SwfError::OutputLimitExceeded);
        }
        if output.len() > 1_048_576 {
            let bytes_in = range.bytes_consumed().max(1);
            if output.len() as f64 / bytes_in as f64 > max_ratio {
                return Err(SwfError::DecompressionBombDetected);
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lzma_decode_direct_bits_wrapping() {
        let raw = [0u8, 0x20, 0x00, 0x00, 0x00, 0x00];
        let mut rd = RangeDecoder::new_range_decoder(&raw).unwrap();
        // Force range > code to exercise wrapping_sub and wrapping_add
        rd.range = 0x8000_0000;
        rd.code = 0x2000_0000;
        let bit = rd.decode_direct_bits(1).unwrap();
        assert_eq!(bit, 0);
    }

    #[test]
    fn test_lzma_spec_pos_slot_13_slice_bounds() {
        let mut spec_pos = [1024u16; 115];
        let slot = 13usize;
        let num_direct_bits = (slot >> 1) - 1; // 5
        let dist = (2 | (slot & 1)) << num_direct_bits; // 96
        let offset = dist - slot; // 83
        let slice = spec_pos
            .get_mut(offset..offset + (1 << num_direct_bits))
            .expect("slot 13 slice within 115 elements");
        assert_eq!(slice.len(), 32);
        assert_eq!(offset + (1 << num_direct_bits), 115);
    }
}
