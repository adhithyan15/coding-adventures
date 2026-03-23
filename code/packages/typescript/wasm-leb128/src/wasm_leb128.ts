/**
 * wasm_leb128.ts — LEB128 variable-length integer encoding for WebAssembly
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * WHAT IS LEB128?
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * LEB128 stands for "Little-Endian Base 128." It is a variable-length encoding
 * for integers — instead of always using a fixed 4 or 8 bytes, small numbers
 * take fewer bytes, and large numbers take more. WebAssembly uses LEB128 for
 * virtually every integer value in its binary (.wasm) format: function indices,
 * memory sizes, instruction immediates, section lengths, etc.
 *
 * The key insight: computers normally use 8 bits (one byte) to store a number.
 * LEB128 uses only 7 of those bits for actual data; the 8th bit (the most
 * significant, or "high" bit) is a *continuation flag*:
 *
 *   bit 7 (0x80) = 1  →  "more bytes follow"
 *   bit 7 (0x80) = 0  →  "this is the last byte"
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * WORKED EXAMPLE: Encoding 624485
 * ─────────────────────────────────────────────────────────────────────────────
 *
 *   624485 in binary (21 bits):
 *     0b 0010_0110  0001_0000  1100_0101
 *
 *   Step 1 — split into 7-bit groups, least-significant first (little-endian):
 *     Group 0 (bits 0–6):   110_0101  = 0x65 = 101
 *     Group 1 (bits 7–13):  000_1110  = 0x0E = 14
 *     Group 2 (bits 14–19): 010_0110  = 0x26 = 38
 *
 *   Step 2 — set the continuation bit (0x80) on every group except the last:
 *     Group 0: 0x65 | 0x80 = 0xE5   (more bytes follow)
 *     Group 1: 0x0E | 0x80 = 0x8E   (more bytes follow)
 *     Group 2: 0x26         = 0x26   (last byte, no continuation bit)
 *
 *   Encoded: [0xE5, 0x8E, 0x26]  ← 3 bytes instead of 4
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * SIGNED vs UNSIGNED
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * Unsigned LEB128 (ULEB128): treats the integer as a non-negative value.
 * Signed LEB128 (SLEB128): uses two's complement representation. After
 * decoding all 7-bit groups, if the most-significant bit of the last group
 * is 1, the value is negative and we must "sign-extend" it.
 *
 *   Sign extension example: -2 encoded as [0x7E]
 *     0x7E = 0b0111_1110
 *     Continuation bit = 0  →  last (and only) byte
 *     7-bit payload = 0b111_1110 = 126
 *     MSB of payload = 1  →  sign-extend: result = 126 - 128 = -2  ✓
 *
 *   Truth table for sign extension (7-bit groups, 32-bit result):
 *   ┌──────────────┬────────────────┬──────────────────────┐
 *   │ 7-bit value  │ MSB set?       │ Sign-extended result │
 *   ├──────────────┼────────────────┼──────────────────────┤
 *   │  0b000_0001  │ No  (0)        │  +1                  │
 *   │  0b011_1111  │ No  (0)        │  +63                 │
 *   │  0b100_0000  │ Yes (1)        │  -64                 │
 *   │  0b111_1111  │ Yes (1)        │  -1                  │
 *   └──────────────┴────────────────┴──────────────────────┘
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * 32-BIT LIMITATION
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * JavaScript's `number` type is a 64-bit IEEE 754 float. It can represent all
 * integers exactly up to 2^53 - 1 (Number.MAX_SAFE_INTEGER), which covers the
 * full u32 and i32 ranges used by WebAssembly 1.0.
 *
 * However, WASM also uses i64/u64 (64-bit) values in some contexts. Those
 * require BigInt in JavaScript. This package handles u32/i32 only. The maximum
 * byte width is therefore 5 (ceil(32/7) = 5).
 *
 * ─────────────────────────────────────────────────────────────────────────────
 */

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/**
 * LEB128Error is thrown when decoding encounters invalid data.
 *
 * Common cases:
 *   - Unterminated sequence: bytes end while the continuation bit is still 1
 *   - Value exceeds the 32-bit range (overflow)
 */
export class LEB128Error extends Error {
  constructor(message: string) {
    super(message);
    this.name = "LEB128Error";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The continuation flag occupies bit 7 of each encoded byte.
 * If this bit is 1, another byte follows. If 0, this is the final byte.
 *
 *   0x80 = 0b1000_0000
 */
const CONTINUATION_BIT = 0x80;

/**
 * The payload mask extracts the lower 7 bits from an encoded byte.
 *
 *   0x7F = 0b0111_1111
 */
const PAYLOAD_MASK = 0x7f;

/**
 * Maximum bytes for a 32-bit LEB128 value.
 * ceil(32 / 7) = 5.  Five 7-bit groups = 35 bits, which covers 32 bits.
 */
const MAX_LEB128_BYTES_32 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// Decoding
// ─────────────────────────────────────────────────────────────────────────────

/**
 * decodeUnsigned — decode a ULEB128-encoded unsigned integer.
 *
 * Algorithm:
 *   result = 0
 *   shift  = 0
 *   for each byte b starting at `offset`:
 *     payload = b & 0x7F          // extract lower 7 bits
 *     result |= payload << shift  // place them at the correct bit position
 *     shift += 7
 *     if (b & 0x80) == 0: break   // no continuation bit → done
 *
 * Returns [value, bytesConsumed].
 *
 * @param data   - The byte array containing the encoded value.
 * @param offset - The byte index at which to start reading (default 0).
 * @returns      [decoded value (non-negative integer), number of bytes consumed]
 * @throws LEB128Error if the sequence is unterminated or exceeds 5 bytes.
 *
 * Example:
 *   decodeUnsigned(new Uint8Array([0xE5, 0x8E, 0x26]))
 *   → [624485, 3]
 */
export function decodeUnsigned(
  data: Uint8Array,
  offset: number = 0
): [number, number] {
  let result = 0;
  let shift = 0;
  let bytesConsumed = 0;

  for (let i = offset; i < data.length; i++) {
    // Guard: LEB128 for a 32-bit value must not exceed 5 bytes.
    // If we've already read 5 bytes without finding a terminator, the data
    // is either corrupt or represents a value too large for 32 bits.
    if (bytesConsumed >= MAX_LEB128_BYTES_32) {
      throw new LEB128Error(
        `LEB128 sequence exceeds maximum ${MAX_LEB128_BYTES_32} bytes for a 32-bit value`
      );
    }

    const byte = data[i];
    const payload = byte & PAYLOAD_MASK; // lower 7 bits

    // Shift the 7 payload bits to their correct position in `result`.
    // After byte 0: bits 0–6; after byte 1: bits 7–13; etc.
    result |= payload << shift;
    shift += 7;
    bytesConsumed++;

    // If the continuation bit is NOT set, this is the last byte.
    if ((byte & CONTINUATION_BIT) === 0) {
      // Convert to unsigned 32-bit to handle the full u32 range correctly.
      // The `| 0` trick converts to signed i32; `>>> 0` converts to u32.
      return [result >>> 0, bytesConsumed];
    }
  }

  // If we exit the loop without returning, the input was truncated.
  throw new LEB128Error(
    `LEB128 sequence is unterminated: reached end of data at offset ${offset + bytesConsumed} ` +
      `without finding a byte with continuation bit = 0`
  );
}

/**
 * decodeSigned — decode a SLEB128-encoded signed integer (two's complement).
 *
 * The decoding loop is identical to ULEB128, but after the loop we check
 * whether sign extension is needed.
 *
 * Sign extension rule:
 *   If (shift < 32) AND (last byte's bit 6 is 1), the value is negative.
 *   We extend the sign by OR-ing all remaining high bits with 1s:
 *
 *     result |= -(1 << shift)
 *
 *   This works because:
 *   - `1 << shift` sets bit `shift`
 *   - Negating it in two's complement fills all bits above `shift` with 1
 *   - OR-ing extends the sign bit throughout the upper bits of `result`
 *
 * Example: decode [0x7E] as signed
 *   byte 0x7E: payload = 0x7E & 0x7F = 0x7E = 0b111_1110
 *   result = 0x7E = 126, shift = 7, continuation bit = 0 → done
 *   shift (7) < 32, last byte MSB (bit 6 of 0x7E) = 1 → sign extend
 *   result |= -(1 << 7) = result | 0xFFFFFF80 = -2  ✓
 *
 * Returns [value, bytesConsumed].
 *
 * @param data   - The byte array containing the encoded value.
 * @param offset - The byte index at which to start reading (default 0).
 * @returns      [decoded signed value, number of bytes consumed]
 * @throws LEB128Error if the sequence is unterminated or exceeds 5 bytes.
 */
export function decodeSigned(
  data: Uint8Array,
  offset: number = 0
): [number, number] {
  let result = 0;
  let shift = 0;
  let bytesConsumed = 0;
  let lastByte = 0;

  for (let i = offset; i < data.length; i++) {
    if (bytesConsumed >= MAX_LEB128_BYTES_32) {
      throw new LEB128Error(
        `LEB128 sequence exceeds maximum ${MAX_LEB128_BYTES_32} bytes for a 32-bit value`
      );
    }

    const byte = data[i];
    lastByte = byte;
    const payload = byte & PAYLOAD_MASK;

    result |= payload << shift;
    shift += 7;
    bytesConsumed++;

    if ((byte & CONTINUATION_BIT) === 0) {
      // Sign extension: if the most-significant bit of the last 7-bit group
      // is 1, the value is negative and we must extend the sign.
      //
      // `shift < 32` guards against the case where we already consumed all
      // 32 bits — in that case no sign extension is needed (or possible).
      //
      // `lastByte & 0x40` checks bit 6 of the last byte, which is the MSB
      // of its 7-bit payload.
      //   0x40 = 0b0100_0000  ← bit 6
      if (shift < 32 && (lastByte & 0x40) !== 0) {
        // Fill all bits from position `shift` upward with 1s.
        result |= -(1 << shift);
      }
      return [result | 0, bytesConsumed]; // `| 0` coerces to signed i32
    }
  }

  throw new LEB128Error(
    `LEB128 sequence is unterminated: reached end of data at offset ${offset + bytesConsumed} ` +
      `without finding a byte with continuation bit = 0`
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Encoding
// ─────────────────────────────────────────────────────────────────────────────

/**
 * encodeUnsigned — encode a non-negative integer as ULEB128.
 *
 * Algorithm:
 *   do:
 *     byte   = value & 0x7F     // take the lowest 7 bits
 *     value  = value >>> 7      // logical right-shift (unsigned) by 7
 *     if value != 0:
 *       byte |= 0x80            // set continuation bit
 *     emit byte
 *   while value != 0
 *
 * The `do...while` ensures at least one byte is emitted even for value=0.
 *
 * @param value - A non-negative integer in the range [0, 2^32 - 1].
 * @returns     Uint8Array of ULEB128-encoded bytes.
 *
 * Example:
 *   encodeUnsigned(624485)
 *   → Uint8Array [0xE5, 0x8E, 0x26]
 */
export function encodeUnsigned(value: number): Uint8Array {
  // Treat `value` as an unsigned 32-bit integer.
  // `>>> 0` is the JavaScript idiom for unsigned 32-bit truncation.
  let remaining = value >>> 0;
  const bytes: number[] = [];

  do {
    // Extract the lowest 7 bits as the next byte's payload.
    let byte = remaining & PAYLOAD_MASK;

    // Shift off the 7 bits we just consumed.
    // `>>> 7` is a *logical* (unsigned) right shift — important for large u32
    // values whose sign bit would be set if we used `>> 7` (arithmetic shift).
    remaining = remaining >>> 7;

    // If there are still non-zero bits left, more bytes will follow.
    if (remaining !== 0) {
      byte |= CONTINUATION_BIT;
    }

    bytes.push(byte);
  } while (remaining !== 0);

  return new Uint8Array(bytes);
}

/**
 * encodeSigned — encode an integer as SLEB128 (signed LEB128).
 *
 * The algorithm is similar to ULEB128, but the termination condition
 * accounts for both positive and negative values:
 *
 *   Positive: stop when remaining == 0 AND bit 6 of the last byte == 0
 *             (so the decoder won't sign-extend a positive value)
 *   Negative: stop when remaining == -1 AND bit 6 of the last byte == 1
 *             (so the decoder won't fail to sign-extend a negative value)
 *
 * This is expressed as a combined condition:
 *
 *   done = (remaining == 0 && (byte & 0x40) == 0)
 *       || (remaining == -1 && (byte & 0x40) != 0)
 *
 * Example: encode -2
 *   -2 in 32-bit two's complement = 0xFFFFFFFE = 0b1111...1111_1110
 *   byte 0: 0xFFFFFFFE & 0x7F = 0x7E = 0b111_1110
 *           0xFFFFFFFE >> 7   = 0xFFFFFFFF = -1 (arithmetic shift preserves sign)
 *           remaining (-1), byte & 0x40 = 0x40 ≠ 0 → done
 *   Emit: [0x7E]  ✓
 *
 * @param value - An integer in the range [-2^31, 2^31 - 1].
 * @returns     Uint8Array of SLEB128-encoded bytes.
 */
export function encodeSigned(value: number): Uint8Array {
  // Coerce to a signed 32-bit integer.
  let remaining = value | 0;
  const bytes: number[] = [];
  let done = false;

  do {
    // Extract the lowest 7 bits.
    let byte = remaining & PAYLOAD_MASK;

    // Arithmetic right-shift by 7, preserving the sign bit.
    // In JavaScript, `>>` is arithmetic (sign-extending), unlike `>>>`.
    remaining = remaining >> 7;

    // Termination check:
    //   - Positive path: remaining = 0 and sign bit of payload = 0
    //     (the decoder will NOT sign-extend, which is correct for positive values)
    //   - Negative path: remaining = -1 and sign bit of payload = 1
    //     (the decoder WILL sign-extend, which is correct for negative values)
    if (
      (remaining === 0 && (byte & 0x40) === 0) ||
      (remaining === -1 && (byte & 0x40) !== 0)
    ) {
      done = true;
    } else {
      byte |= CONTINUATION_BIT;
    }

    bytes.push(byte);
  } while (!done);

  return new Uint8Array(bytes);
}
