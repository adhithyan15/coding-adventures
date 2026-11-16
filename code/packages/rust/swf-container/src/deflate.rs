//! Bounded RFC 1950 (zlib) and RFC 1951 (deflate) decompressor with anti-bomb defenses.

use crate::error::SwfError;

/// Computes the Adler-32 checksum of a byte slice per RFC 1950.
pub fn compute_adler32(data: &[u8]) -> u32 {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s2 << 16) | s1
}

/// LSB-first bit reader for DEFLATE streams.
struct LsbBitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: usize,
}

impl<'a> LsbBitReader<'a> {
    fn new_lsb(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn bytes_consumed(&self) -> usize {
        if self.bit_pos > 0 {
            self.byte_pos.saturating_add(1)
        } else {
            self.byte_pos
        }
    }

    fn read_bits(&mut self, count: usize) -> Result<u32, SwfError> {
        if count == 0 {
            return Ok(0);
        }
        let mut val: u32 = 0;
        let mut bits_read = 0;

        while bits_read < count {
            if self.byte_pos >= self.data.len() {
                return Err(SwfError::UnexpectedEof);
            }
            let byte = self.data[self.byte_pos];
            let available = 8 - self.bit_pos;
            let take = (count - bits_read).min(available);

            let chunk = (byte >> self.bit_pos) & ((1 << take) - 1);
            val |= (chunk as u32) << bits_read;

            self.bit_pos += take;
            bits_read += take;

            if self.bit_pos == 8 {
                self.byte_pos += 1;
                self.bit_pos = 0;
            }
        }

        Ok(val)
    }

    fn read_byte(&mut self) -> Result<u8, SwfError> {
        self.align_to_byte();
        if self.byte_pos >= self.data.len() {
            return Err(SwfError::UnexpectedEof);
        }
        let b = self.data[self.byte_pos];
        self.byte_pos += 1;
        Ok(b)
    }

    fn read_u16_le(&mut self) -> Result<u16, SwfError> {
        let b0 = self.read_byte()?;
        let b1 = self.read_byte()?;
        Ok(u16::from_le_bytes([b0, b1]))
    }

    fn align_to_byte(&mut self) {
        if self.bit_pos > 0 {
            self.byte_pos += 1;
            self.bit_pos = 0;
        }
    }
}

// Canonical Huffman table decoder
struct HuffmanTable {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl HuffmanTable {
    fn build_from_lengths(lengths: &[u8]) -> Result<Self, SwfError> {
        let mut counts = [0u16; 16];
        for &len in lengths {
            if len > 15 {
                return Err(SwfError::CorruptedData);
            }
            if len > 0 {
                counts[len as usize] += 1;
            }
        }

        let total_symbols: usize = counts.iter().map(|&c| c as usize).sum();
        let mut symbols = vec![0u16; total_symbols];
        let mut offsets = [0usize; 16];
        let mut sum = 0;
        for i in 1..=15 {
            offsets[i] = sum;
            sum += counts[i] as usize;
        }

        for (sym, &len) in lengths.iter().enumerate() {
            if len > 0 {
                let idx = offsets[len as usize];
                symbols[idx] = sym as u16;
                offsets[len as usize] += 1;
            }
        }

        Ok(Self { counts, symbols })
    }

    fn decode_symbol(&self, reader: &mut LsbBitReader<'_>) -> Result<u16, SwfError> {
        let mut code: u32 = 0;
        let mut first: u32 = 0;
        let mut index: usize = 0;

        for len in 1..=15 {
            let bit = reader.read_bits(1)?;
            code = (code << 1) | bit;
            let count = self.counts[len] as u32;

            if code < first + count {
                return Ok(self.symbols[index + (code - first) as usize]);
            }

            index += count as usize;
            first = (first + count) << 1;
        }

        Err(SwfError::CorruptedData)
    }
}

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn check_output_bounds(
    output_len: usize,
    bytes_in: usize,
    max_uncompressed_bytes: usize,
    max_ratio: f64,
) -> Result<(), SwfError> {
    if output_len > max_uncompressed_bytes {
        return Err(SwfError::OutputLimitExceeded);
    }
    // Beyond 1 MB (1_048_576 bytes), enforce ratio limit
    if output_len > 1_048_576 {
        let in_eff = bytes_in.max(1);
        let ratio = output_len as f64 / in_eff as f64;
        if ratio > max_ratio {
            return Err(SwfError::DecompressionBombDetected);
        }
    }
    Ok(())
}

fn decode_huffman_block(
    reader: &mut LsbBitReader<'_>,
    ll_table: &HuffmanTable,
    dist_table: &HuffmanTable,
    output: &mut Vec<u8>,
    max_uncompressed_bytes: usize,
    max_ratio: f64,
) -> Result<(), SwfError> {
    loop {
        let sym = ll_table.decode_symbol(reader)?;
        if sym < 256 {
            output.push(sym as u8);
            if output.len().is_multiple_of(8192) {
                check_output_bounds(
                    output.len(),
                    reader.bytes_consumed(),
                    max_uncompressed_bytes,
                    max_ratio,
                )?;
            }
        } else if sym == 256 {
            // End of block
            break;
        } else if sym <= 285 {
            let idx = (sym - 257) as usize;
            let extra = LENGTH_EXTRA[idx] as usize;
            let length = LENGTH_BASE[idx] as usize + reader.read_bits(extra)? as usize;

            let dist_sym = dist_table.decode_symbol(reader)? as usize;
            if dist_sym >= 30 {
                return Err(SwfError::CorruptedData);
            }
            let dist_extra = DIST_EXTRA[dist_sym] as usize;
            let distance = DIST_BASE[dist_sym] as usize + reader.read_bits(dist_extra)? as usize;

            if distance == 0 || distance > output.len() {
                return Err(SwfError::CorruptedData);
            }

            for _ in 0..length {
                let idx = output.len() - distance;
                let b = output[idx];
                output.push(b);
            }

            if output.len().is_multiple_of(8192) {
                check_output_bounds(
                    output.len(),
                    reader.bytes_consumed(),
                    max_uncompressed_bytes,
                    max_ratio,
                )?;
            }
        } else {
            return Err(SwfError::CorruptedData);
        }
    }

    check_output_bounds(
        output.len(),
        reader.bytes_consumed(),
        max_uncompressed_bytes,
        max_ratio,
    )
}

/// Decompresses an RFC 1950 zlib stream with bounded output size and expansion ratio checks.
pub fn zlib_decompress_bounded(
    data: &[u8],
    max_uncompressed_bytes: usize,
    max_ratio: f64,
) -> Result<Vec<u8>, SwfError> {
    if data.len() < 6 {
        return Err(SwfError::UnexpectedEof);
    }

    let cmf = data[0];
    let flg = data[1];

    if !(cmf as u16 * 256 + flg as u16).is_multiple_of(31) {
        return Err(SwfError::CorruptedData);
    }

    let cm = cmf & 0x0F;
    if cm != 8 {
        return Err(SwfError::CorruptedData);
    }

    let fdict = (flg >> 5) & 1;
    if fdict != 0 {
        return Err(SwfError::CorruptedData);
    }

    // Payload is data[2..data.len() - 4]
    let deflate_payload = &data[2..data.len() - 4];
    let expected_adler = u32::from_be_bytes([
        data[data.len() - 4],
        data[data.len() - 3],
        data[data.len() - 2],
        data[data.len() - 1],
    ]);

    let mut reader = LsbBitReader::new_lsb(deflate_payload);
    let mut output = Vec::with_capacity(deflate_payload.len().min(65536));

    let mut is_final = false;
    while !is_final {
        let bfinal = reader.read_bits(1)?;
        let btype = reader.read_bits(2)?;
        is_final = bfinal != 0;

        match btype {
            0b00 => {
                // Stored block
                reader.align_to_byte();
                let len = reader.read_u16_le()? as usize;
                let nlen = reader.read_u16_le()? as usize;
                if (len ^ 0xFFFF) != nlen {
                    return Err(SwfError::CorruptedData);
                }
                for _ in 0..len {
                    let b = reader.read_byte()?;
                    output.push(b);
                }
                check_output_bounds(
                    output.len(),
                    reader.bytes_consumed(),
                    max_uncompressed_bytes,
                    max_ratio,
                )?;
            }
            0b01 => {
                // Fixed Huffman
                let mut ll_lengths = vec![8u8; 288];
                for (i, len) in ll_lengths.iter_mut().enumerate() {
                    if i <= 143 {
                        *len = 8;
                    } else if i <= 255 {
                        *len = 9;
                    } else if i <= 279 {
                        *len = 7;
                    } else {
                        *len = 8;
                    }
                }
                let dist_lengths = vec![5u8; 32];
                let ll_table = HuffmanTable::build_from_lengths(&ll_lengths)?;
                let dist_table = HuffmanTable::build_from_lengths(&dist_lengths)?;
                decode_huffman_block(
                    &mut reader,
                    &ll_table,
                    &dist_table,
                    &mut output,
                    max_uncompressed_bytes,
                    max_ratio,
                )?;
            }
            0b10 => {
                // Dynamic Huffman
                let hlit = reader.read_bits(5)? as usize + 257;
                let hdist = reader.read_bits(5)? as usize + 1;
                let hclen = reader.read_bits(4)? as usize + 4;

                let mut cl_lengths = [0u8; 19];
                for i in 0..hclen {
                    cl_lengths[CL_ORDER[i]] = reader.read_bits(3)? as u8;
                }
                let cl_table = HuffmanTable::build_from_lengths(&cl_lengths)?;

                let total_codes = hlit + hdist;
                let mut code_lengths = Vec::with_capacity(total_codes);

                while code_lengths.len() < total_codes {
                    let sym = cl_table.decode_symbol(&mut reader)?;
                    if sym <= 15 {
                        code_lengths.push(sym as u8);
                    } else if sym == 16 {
                        if code_lengths.is_empty() {
                            return Err(SwfError::CorruptedData);
                        }
                        let prev = *code_lengths.last().unwrap();
                        let repeat = reader.read_bits(2)? as usize + 3;
                        code_lengths.resize(code_lengths.len() + repeat, prev);
                    } else if sym == 17 {
                        let repeat = reader.read_bits(3)? as usize + 3;
                        code_lengths.resize(code_lengths.len() + repeat, 0);
                    } else if sym == 18 {
                        let repeat = reader.read_bits(7)? as usize + 11;
                        code_lengths.resize(code_lengths.len() + repeat, 0);
                    } else {
                        return Err(SwfError::CorruptedData);
                    }
                }

                if code_lengths.len() > total_codes {
                    return Err(SwfError::CorruptedData);
                }

                let ll_table = HuffmanTable::build_from_lengths(&code_lengths[..hlit])?;
                let dist_table = HuffmanTable::build_from_lengths(&code_lengths[hlit..])?;
                decode_huffman_block(
                    &mut reader,
                    &ll_table,
                    &dist_table,
                    &mut output,
                    max_uncompressed_bytes,
                    max_ratio,
                )?;
            }
            _ => return Err(SwfError::CorruptedData),
        }
    }

    let actual_adler = compute_adler32(&output);
    if actual_adler != expected_adler {
        return Err(SwfError::ChecksumMismatch);
    }

    Ok(output)
}
