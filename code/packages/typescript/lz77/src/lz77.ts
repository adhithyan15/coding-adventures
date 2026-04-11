/**
 * LZ77 lossless compression algorithm (Lempel & Ziv, 1977).
 *
 * ## What Is LZ77?
 *
 * LZ77 replaces repeated byte sequences with compact backreferences into a
 * sliding window of recently seen data. It is the foundation of DEFLATE,
 * gzip, PNG, and zlib.
 *
 * ## The Sliding Window Model
 *
 * ```
 * ┌─────────────────────────────────┬──────────────────┐
 * │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
 * │  (already processed — the       │  (not yet seen —  │
 * │   last windowSize bytes)        │  next maxMatch)   │
 * └─────────────────────────────────┴──────────────────┘
 *                                    ↑
 *                                cursor (current position)
 * ```
 *
 * At each step, the encoder finds the longest match in the search buffer.
 * If found and long enough (≥ minMatch), emit a backreference token.
 * Otherwise, emit a literal token.
 *
 * ## Token: (offset, length, nextChar)
 *
 * - **offset**: Distance back the match starts (1..windowSize), or 0.
 * - **length**: Number of bytes the match covers (0 = literal).
 * - **nextChar**: Literal byte immediately after the match.
 *
 * ## Overlapping Matches
 *
 * When offset < length, the match extends into bytes not yet decoded.
 * The decoder must copy byte-by-byte (not bulk) to handle this.
 * Example: output = [A, B], token = (2, 5, Z):
 *   Copy byte-by-byte → [A, B, A, B, A, B, A], then append Z → ABABABAZ.
 *
 * ## The Series: CMP00 → CMP05
 *
 * - CMP00 (LZ77, 1977) — Sliding-window backreferences. This module.
 * - CMP01 (LZ78, 1978) — Explicit dictionary (trie), no sliding window.
 * - CMP02 (LZSS, 1982) — LZ77 + flag bits; eliminates wasted literals.
 * - CMP03 (LZW,  1984) — Pre-initialized dictionary; powers GIF.
 * - CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
 * - CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
 */

/** A single LZ77 token: (offset, length, nextChar). */
export interface Token {
  /** Distance back the match starts (1..windowSize), or 0 for literal. */
  readonly offset: number;
  /** Number of bytes the match covers (0 = literal). */
  readonly length: number;
  /** Literal byte immediately after the match (0..255). */
  readonly nextChar: number;
}

/**
 * Creates a Token.
 *
 * @param offset - Distance back the match starts.
 * @param length - Number of bytes the match covers.
 * @param nextChar - Literal byte after the match.
 */
export function token(offset: number, length: number, nextChar: number): Token {
  return { offset, length, nextChar };
}

/**
 * Finds the longest match in the search buffer.
 *
 * Scans the last `windowSize` bytes before `cursor` for the longest substring
 * matching the start of the lookahead buffer.
 *
 * @param data - Input bytes.
 * @param cursor - Current position (start of lookahead).
 * @param windowSize - Maximum lookback distance.
 * @param maxMatch - Maximum match length.
 * @returns [bestOffset, bestLength], both 0 if no match found.
 */
function findLongestMatch(
  data: Uint8Array,
  cursor: number,
  windowSize: number,
  maxMatch: number,
): [number, number] {
  let bestOffset = 0;
  let bestLength = 0;

  // The search buffer starts at most windowSize bytes back.
  const searchStart = Math.max(0, cursor - windowSize);

  // The lookahead cannot extend past the end of input.
  // Reserve 1 byte for nextChar.
  const lookaheadEnd = Math.min(cursor + maxMatch, data.length - 1);

  for (let pos = searchStart; pos < cursor; pos++) {
    let length = 0;
    // Match byte by byte. Matches may overlap (extend past cursor).
    while (
      cursor + length < lookaheadEnd &&
      data[pos + length] === data[cursor + length]
    ) {
      length++;
    }
    if (length > bestLength) {
      bestLength = length;
      bestOffset = cursor - pos; // Distance back from cursor.
    }
  }

  return [bestOffset, bestLength];
}

/**
 * Encodes data into an LZ77 token stream.
 *
 * Scans the input left-to-right, finding the longest match in the search
 * buffer for each position. If a match is long enough (≥ minMatch), emits
 * a backreference token; otherwise, emits a literal token.
 *
 * @param data - Input bytes.
 * @param windowSize - Maximum lookback distance (default 4096).
 * @param maxMatch - Maximum match length (default 255).
 * @param minMatch - Minimum length for a backreference (default 3).
 * @returns Array of Token objects representing the compressed stream.
 *
 * @example
 * ```ts
 * const tokens = encode(new Uint8Array([65, 66, 65, 66, 65, 66, 65, 66]));
 * // tokens.length === 3 (two literals + one backreference)
 * ```
 */
export function encode(
  data: Uint8Array,
  windowSize = 4096,
  maxMatch = 255,
  minMatch = 3,
): Token[] {
  const tokens: Token[] = [];
  let cursor = 0;

  while (cursor < data.length) {
    // Edge case: last byte has no room for nextChar after a match.
    if (cursor === data.length - 1) {
      tokens.push(token(0, 0, data[cursor]!));
      cursor++;
      continue;
    }

    const [offset, length] = findLongestMatch(data, cursor, windowSize, maxMatch);

    if (length >= minMatch) {
      // Emit a backreference token.
      const nextChar = data[cursor + length]!;
      tokens.push(token(offset, length, nextChar));
      cursor += length + 1;
    } else {
      // Emit a literal token (no match or too short).
      tokens.push(token(0, 0, data[cursor]!));
      cursor++;
    }
  }

  return tokens;
}

/**
 * Decodes an LZ77 token stream back into the original bytes.
 *
 * Processes each token: if length > 0, copies length bytes byte-by-byte
 * from the search buffer (handling overlapping matches), then appends
 * nextChar.
 *
 * @param tokens - The token stream (output of encode).
 * @param initialBuffer - Optional seed for the search buffer (streaming use).
 * @returns Reconstructed bytes.
 *
 * @example
 * ```ts
 * const tokens = [token(0, 0, 65), token(1, 3, 68)];
 * decode(tokens); // Uint8Array [65, 65, 65, 65, 68] = "AAAAD"
 * ```
 */
export function decode(tokens: Token[], initialBuffer: Uint8Array = new Uint8Array()): Uint8Array {
  // Use a growing array for efficient byte-by-byte appending.
  const output: number[] = [...initialBuffer];

  for (const tok of tokens) {
    if (tok.length > 0) {
      // Copy length bytes from position (output.length - offset).
      const start = output.length - tok.offset;
      // Copy byte-by-byte to handle overlapping matches (offset < length).
      for (let i = 0; i < tok.length; i++) {
        output.push(output[start + i]!);
      }
    }
    // Always append nextChar.
    output.push(tok.nextChar);
  }

  return new Uint8Array(output);
}

/**
 * Serialises a token list to bytes using a fixed-width format.
 *
 * Format:
 * - 4 bytes: token count (big-endian uint32)
 * - N × 4 bytes: each token as (offset: uint16 BE, length: uint8, nextChar: uint8)
 *
 * This is a teaching format, not an industry one. Production compressors use
 * variable-width bit-packing (see DEFLATE, zstd).
 *
 * @param tokens - Token list to serialise.
 * @returns Binary representation.
 */
export function serialiseTokens(tokens: Token[]): Uint8Array {
  const buf = new ArrayBuffer(4 + tokens.length * 4);
  const view = new DataView(buf);

  // Write token count as big-endian uint32.
  view.setUint32(0, tokens.length, false);

  for (let i = 0; i < tokens.length; i++) {
    const base = 4 + i * 4;
    const tok = tokens[i]!;
    view.setUint16(base, tok.offset, false);     // 2 bytes: offset
    view.setUint8(base + 2, tok.length);          // 1 byte: length
    view.setUint8(base + 3, tok.nextChar);        // 1 byte: nextChar
  }

  return new Uint8Array(buf);
}

/**
 * Deserialises bytes back into a token list.
 *
 * Inverse of serialiseTokens.
 *
 * @param data - Binary data (output of serialiseTokens).
 * @returns Token list.
 */
export function deserialiseTokens(data: Uint8Array): Token[] {
  if (data.length < 4) return [];

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const count = view.getUint32(0, false);
  const tokens: Token[] = [];

  for (let i = 0; i < count; i++) {
    const base = 4 + i * 4;
    if (base + 4 > data.length) break;

    tokens.push(
      token(
        view.getUint16(base, false),
        view.getUint8(base + 2),
        view.getUint8(base + 3),
      ),
    );
  }

  return tokens;
}

/**
 * Compresses data using LZ77.
 *
 * One-shot API: encode then serialise the token stream to bytes.
 *
 * @param data - Input bytes.
 * @param windowSize - Maximum lookback distance (default 4096).
 * @param maxMatch - Maximum match length (default 255).
 * @param minMatch - Minimum match length for backreferences (default 3).
 * @returns Compressed bytes.
 *
 * @example
 * ```ts
 * const compressed = compress(new TextEncoder().encode("AAAAAAA"));
 * decompress(compressed); // Uint8Array encoding of "AAAAAAA"
 * ```
 */
export function compress(
  data: Uint8Array,
  windowSize = 4096,
  maxMatch = 255,
  minMatch = 3,
): Uint8Array {
  const tokens = encode(data, windowSize, maxMatch, minMatch);
  return serialiseTokens(tokens);
}

/**
 * Decompresses data that was compressed with compress().
 *
 * Deserialises the byte stream into tokens, then decodes.
 *
 * @param data - Compressed bytes.
 * @returns Original uncompressed bytes.
 */
export function decompress(data: Uint8Array): Uint8Array {
  const tokens = deserialiseTokens(data);
  return decode(tokens);
}
