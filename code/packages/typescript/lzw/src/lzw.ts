// =============================================================================
// CodingAdventures LZW
// =============================================================================
//
// LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
// Part of the CMP compression series in the coding-adventures monorepo.
//
// What Is LZW?
// ------------
//
// LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
// added before encoding begins (codes 0–255). This eliminates LZ78's mandatory
// next_char byte — every symbol is already in the dictionary, so the encoder
// can emit pure codes.
//
// With only codes to transmit, LZW uses variable-width bit-packing: codes start
// at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
//
// Reserved Codes
// --------------
//
//   0–255:  Pre-seeded single-byte entries.
//   256:    CLEAR_CODE — reset to initial 256-entry state.
//   257:    STOP_CODE  — end of code stream.
//   258+:   Dynamically added entries.
//
// Wire Format (CMP03)
// -------------------
//
//   Bytes 0–3:  original_length (big-endian uint32)
//   Bytes 4+:   bit-packed variable-width codes, LSB-first
//
// The Tricky Token
// ----------------
//
// During decoding the decoder may receive code C == nextCode (not yet added).
// This happens when the input has the form xyx...x. The fix:
//
//   entry = dict[prevCode] + [dict[prevCode][0]]
//
// The Series: CMP00 -> CMP05
// --------------------------
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF. (this module)
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
// =============================================================================

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const CLEAR_CODE = 256;
export const STOP_CODE = 257;
export const INITIAL_NEXT_CODE = 258;
export const INITIAL_CODE_SIZE = 9;
export const MAX_CODE_SIZE = 16;

// ---------------------------------------------------------------------------
// Bit I/O helpers
// ---------------------------------------------------------------------------

/**
 * BitWriter accumulates variable-width codes into a byte array, LSB-first.
 *
 * Bits within each byte are filled from the least-significant end. This
 * matches the GIF and Unix compress conventions.
 */
export class BitWriter {
  private buf = 0; // bit accumulator (JS number, up to 32 safe bits)
  private bitPos = 0; // valid bits in buf
  private output: number[] = [];

  /**
   * Write `code` using exactly `codeSize` bits.
   *
   * We use two-step logic to avoid JavaScript's 32-bit shift limitation:
   * shift by at most 24 bits at a time using multiplication when necessary.
   */
  write(code: number, codeSize: number): void {
    // Use multiplication for large shifts to avoid JS 32-bit truncation.
    this.buf = (this.buf + code * Math.pow(2, this.bitPos)) >>> 0;
    this.bitPos += codeSize;
    while (this.bitPos >= 8) {
      this.output.push(this.buf & 0xff);
      this.buf = Math.floor(this.buf / 256);
      this.bitPos -= 8;
    }
  }

  flush(): void {
    if (this.bitPos > 0) {
      this.output.push(this.buf & 0xff);
      this.buf = 0;
      this.bitPos = 0;
    }
  }

  bytes(): Uint8Array {
    return new Uint8Array(this.output);
  }
}

/**
 * BitReader reads variable-width codes from a byte array, LSB-first.
 */
export class BitReader {
  private pos = 0;
  private buf = 0;
  private bitPos = 0;

  constructor(private readonly data: Uint8Array) {}

  /**
   * Read and return the next `codeSize`-bit code.
   * Throws if the stream is exhausted before enough bits are available.
   */
  read(codeSize: number): number {
    while (this.bitPos < codeSize) {
      if (this.pos >= this.data.length) {
        throw new Error("unexpected end of bit stream");
      }
      this.buf = (this.buf + this.data[this.pos]! * Math.pow(2, this.bitPos)) >>> 0;
      this.pos++;
      this.bitPos += 8;
    }
    const code = this.buf & ((1 << codeSize) - 1);
    this.buf = Math.floor(this.buf / Math.pow(2, codeSize));
    this.bitPos -= codeSize;
    return code;
  }

  exhausted(): boolean {
    return this.pos >= this.data.length && this.bitPos === 0;
  }
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

/**
 * Encode `data` into an array of LZW codes including CLEAR_CODE and STOP_CODE.
 *
 * Returns [codes, originalLength]. The encode dictionary maps sequence strings
 * to codes. The encoder walks the input byte-by-byte extending the current
 * prefix; when the prefix + new byte is not in the dict, the current prefix's
 * code is emitted, the new sequence is added to the dict, and the prefix
 * resets to just the new byte.
 */
export function encodeCodes(data: Uint8Array): [number[], number] {
  const originalLength = data.length;
  const encDict = new Map<string, number>();
  for (let b = 0; b < 256; b++) {
    encDict.set(String.fromCharCode(b), b);
  }

  let nextCode = INITIAL_NEXT_CODE;
  const maxEntries = 1 << MAX_CODE_SIZE;
  const codes: number[] = [CLEAR_CODE];

  let w = ""; // current working prefix as a string

  for (const byte of data) {
    const wb = w + String.fromCharCode(byte);
    if (encDict.has(wb)) {
      w = wb;
    } else {
      codes.push(encDict.get(w)!);

      if (nextCode < maxEntries) {
        encDict.set(wb, nextCode);
        nextCode++;
      } else if (nextCode === maxEntries) {
        // Dictionary full — emit CLEAR and reset.
        codes.push(CLEAR_CODE);
        encDict.clear();
        for (let b = 0; b < 256; b++) {
          encDict.set(String.fromCharCode(b), b);
        }
        nextCode = INITIAL_NEXT_CODE;
      }

      w = String.fromCharCode(byte);
    }
  }

  if (w.length > 0) {
    codes.push(encDict.get(w)!);
  }

  codes.push(STOP_CODE);
  return [codes, originalLength];
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/**
 * Decode an array of LZW codes back to a byte array.
 *
 * Handles CLEAR_CODE (reset), STOP_CODE (done), and the tricky-token
 * edge case (code === nextCode).
 */
export function decodeCodes(codes: number[]): number[] {
  // Decode dictionary: index → byte sequence (Uint8Array for efficiency).
  const decDict: Uint8Array[] = [];
  for (let b = 0; b < 256; b++) {
    decDict.push(new Uint8Array([b]));
  }
  decDict.push(new Uint8Array(0)); // 256 = CLEAR placeholder
  decDict.push(new Uint8Array(0)); // 257 = STOP  placeholder

  let nextCode = INITIAL_NEXT_CODE;
  const output: number[] = [];
  let prevCode = -1; // -1 = no previous code

  for (const code of codes) {
    if (code === CLEAR_CODE) {
      while (decDict.length > 258) decDict.pop();
      decDict.length = 258;
      // Re-initialise the 256 single-byte entries.
      for (let b = 0; b < 256; b++) {
        decDict[b] = new Uint8Array([b]);
      }
      decDict[256] = new Uint8Array(0);
      decDict[257] = new Uint8Array(0);
      nextCode = INITIAL_NEXT_CODE;
      prevCode = -1;
      continue;
    }

    if (code === STOP_CODE) break;

    let entry: Uint8Array;

    if (code < decDict.length) {
      entry = decDict[code]!;
    } else if (code === nextCode) {
      // Tricky token.
      if (prevCode < 0) continue; // malformed
      const prev = decDict[prevCode]!;
      entry = new Uint8Array(prev.length + 1);
      entry.set(prev);
      entry[prev.length] = prev[0]!;
    } else {
      continue; // invalid code
    }

    for (const b of entry) output.push(b);

    if (prevCode >= 0 && nextCode < (1 << MAX_CODE_SIZE)) {
      const prev = decDict[prevCode]!;
      const newEntry = new Uint8Array(prev.length + 1);
      newEntry.set(prev);
      newEntry[prev.length] = entry[0]!;
      decDict.push(newEntry);
      nextCode++;
    }

    prevCode = code;
  }

  return output;
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

/**
 * Pack an array of LZW codes into the CMP03 wire format.
 *
 * Header: 4-byte big-endian original_length.
 * Body:   LSB-first variable-width bit-packed codes.
 */
export function packCodes(codes: number[], originalLength: number): Uint8Array {
  const writer = new BitWriter();
  let codeSize = INITIAL_CODE_SIZE;
  let nextCode = INITIAL_NEXT_CODE;

  for (const code of codes) {
    writer.write(code, codeSize);

    if (code === CLEAR_CODE) {
      codeSize = INITIAL_CODE_SIZE;
      nextCode = INITIAL_NEXT_CODE;
    } else if (code !== STOP_CODE) {
      if (nextCode < (1 << MAX_CODE_SIZE)) {
        nextCode++;
        if (nextCode > (1 << codeSize) && codeSize < MAX_CODE_SIZE) {
          codeSize++;
        }
      }
    }
  }
  writer.flush();

  const body = writer.bytes();
  const result = new Uint8Array(4 + body.length);
  const view = new DataView(result.buffer);
  view.setUint32(0, originalLength, false); // big-endian
  result.set(body, 4);
  return result;
}

/**
 * Unpack CMP03 wire-format bytes into an array of LZW codes.
 *
 * Returns [codes, originalLength]. Stops at STOP_CODE or stream exhaustion.
 */
export function unpackCodes(data: Uint8Array): [number[], number] {
  if (data.length < 4) {
    return [[CLEAR_CODE, STOP_CODE], 0];
  }

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const originalLength = view.getUint32(0, false);

  const reader = new BitReader(data.slice(4));
  const codes: number[] = [];
  let codeSize = INITIAL_CODE_SIZE;
  let nextCode = INITIAL_NEXT_CODE;

  while (!reader.exhausted()) {
    let code: number;
    try {
      code = reader.read(codeSize);
    } catch {
      break;
    }

    codes.push(code);

    if (code === STOP_CODE) {
      break;
    } else if (code === CLEAR_CODE) {
      codeSize = INITIAL_CODE_SIZE;
      nextCode = INITIAL_NEXT_CODE;
    } else if (nextCode < (1 << MAX_CODE_SIZE)) {
      nextCode++;
      if (nextCode > (1 << codeSize) && codeSize < MAX_CODE_SIZE) {
        codeSize++;
      }
    }
  }

  return [codes, originalLength];
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Compress `data` using LZW and return CMP03 wire-format bytes.
 */
export function compress(data: Uint8Array): Uint8Array {
  const [codes, originalLength] = encodeCodes(data);
  return packCodes(codes, originalLength);
}

/**
 * Decompress CMP03 wire-format `data` and return the original bytes.
 */
export function decompress(data: Uint8Array): Uint8Array {
  const [codes, originalLength] = unpackCodes(data);
  const result = decodeCodes(codes);
  const trimmed = result.slice(0, originalLength);
  return new Uint8Array(trimmed);
}
