/**
 * lzss.ts — LZSS lossless compression algorithm (1982).
 *
 * LZSS (Lempel-Ziv-Storer-Szymanski) refines LZ77 by replacing the mandatory
 * `next_char` byte after every token with a flag-bit scheme:
 *
 *   Literal → 1 byte  (flag bit = 0)
 *   Match   → 3 bytes (flag bit = 1: offset uint16 BE + length uint8)
 *
 * Tokens are grouped in blocks of 8. Each block starts with a flag byte
 * (bit 0 = first token, bit 7 = eighth token).
 *
 * Wire format (CMP02):
 *   Bytes 0–3: original_length (big-endian uint32)
 *   Bytes 4–7: block_count     (big-endian uint32)
 *   Bytes 8+:  blocks
 *     Each block: [1-byte flag] [1 or 3 bytes per symbol]
 *
 * Series:
 *   CMP00 (LZ77, 1977) — Sliding-window backreferences.
 *   CMP01 (LZ78, 1978) — Explicit dictionary (trie).
 *   CMP02 (LZSS, 1982) — LZ77 + flag bits. ← this file
 *   CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
 *   CMP04 (Huffman, 1952) — Entropy coding.
 *   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
 */

// ─── Token types ─────────────────────────────────────────────────────────────

/** A single literal byte in the LZSS token stream. */
export interface Literal {
  readonly kind: "literal";
  readonly byte: number; // 0–255
}

/** A back-reference match in the LZSS token stream. */
export interface Match {
  readonly kind: "match";
  readonly offset: number; // 1..windowSize
  readonly length: number; // minMatch..maxMatch
}

/** Union of the two LZSS token types. */
export type Token = Literal | Match;

/** Construct a Literal token. */
export function literal(byte: number): Literal {
  return { kind: "literal", byte };
}

/** Construct a Match token. */
export function match(offset: number, length: number): Match {
  return { kind: "match", offset, length };
}

// ─── Encoder ─────────────────────────────────────────────────────────────────

/** Default sliding-window parameters matching the CMP02 spec. */
export const DEFAULT_WINDOW_SIZE = 4096;
export const DEFAULT_MAX_MATCH = 255;
export const DEFAULT_MIN_MATCH = 3;

/**
 * Find the longest match for data[cursor:] in the search buffer.
 * Matches may overlap (extend past cursor) for run-length encoding.
 */
function findLongestMatch(
  data: Uint8Array,
  cursor: number,
  winStart: number,
  maxMatch: number
): [number, number] {
  let bestLen = 0;
  let bestOff = 0;
  const lookaheadEnd = Math.min(cursor + maxMatch, data.length);

  for (let pos = winStart; pos < cursor; pos++) {
    let len = 0;
    while (
      cursor + len < lookaheadEnd &&
      data[pos + len] === data[cursor + len]
    ) {
      len++;
    }
    if (len > bestLen) {
      bestLen = len;
      bestOff = cursor - pos;
    }
  }

  return [bestOff, bestLen];
}

/**
 * Encode bytes into an LZSS token stream.
 *
 * At each cursor position, searches the last `windowSize` bytes for the
 * longest match. If the match is at least `minMatch` bytes, emits a Match
 * and advances by that length. Otherwise emits a Literal and advances by 1.
 *
 * @example
 * encode(new TextEncoder().encode("ABABAB"))
 * // [literal(65), literal(66), match(2, 4)]
 */
export function encode(
  data: Uint8Array,
  windowSize = DEFAULT_WINDOW_SIZE,
  maxMatch = DEFAULT_MAX_MATCH,
  minMatch = DEFAULT_MIN_MATCH
): Token[] {
  const tokens: Token[] = [];
  let cursor = 0;

  while (cursor < data.length) {
    const winStart = Math.max(0, cursor - windowSize);
    const [offset, length] = findLongestMatch(data, cursor, winStart, maxMatch);

    if (length >= minMatch) {
      tokens.push(match(offset, length));
      cursor += length;
    } else {
      tokens.push(literal(data[cursor]));
      cursor++;
    }
  }

  return tokens;
}

// ─── Decoder ─────────────────────────────────────────────────────────────────

/**
 * Decode an LZSS token stream back into the original bytes.
 *
 * For each Literal, appends the byte. For each Match, copies `length` bytes
 * from `offset` positions back, byte-by-byte for overlap safety.
 *
 * @param originalLength - if >= 0, truncates output to this length
 *
 * @example
 * decode([literal(65), match(1, 6)], 7)
 * // Uint8Array for "AAAAAAA"
 */
export function decode(tokens: Token[], originalLength = -1): Uint8Array {
  const output: number[] = [];

  for (const tok of tokens) {
    if (tok.kind === "literal") {
      output.push(tok.byte);
    } else {
      const start = output.length - tok.offset;
      for (let i = 0; i < tok.length; i++) {
        output.push(output[start + i]); // byte-by-byte for overlap
      }
    }
  }

  const result = new Uint8Array(output);
  if (originalLength >= 0 && result.length > originalLength) {
    return result.subarray(0, originalLength);
  }
  return result;
}

// ─── Serialisation ───────────────────────────────────────────────────────────

/**
 * Serialise an LZSS token list to the CMP02 wire format.
 *
 * Header: original_length (BE uint32) + block_count (BE uint32).
 * Then block_count blocks of: [1-byte flag] + symbol data.
 */
export function serialiseTokens(
  tokens: Token[],
  originalLength: number
): Uint8Array {
  const blocks: Uint8Array[] = [];

  for (let i = 0; i < tokens.length; i += 8) {
    const chunk = tokens.slice(i, i + 8);
    let flag = 0;
    const symbolParts: number[] = [];

    for (let bit = 0; bit < chunk.length; bit++) {
      const tok = chunk[bit];
      if (tok.kind === "match") {
        flag |= 1 << bit;
        symbolParts.push(
          (tok.offset >> 8) & 0xff,
          tok.offset & 0xff,
          tok.length & 0xff
        );
      } else {
        symbolParts.push(tok.byte);
      }
    }

    const block = new Uint8Array(1 + symbolParts.length);
    block[0] = flag;
    block.set(symbolParts, 1);
    blocks.push(block);
  }

  const totalBodySize = blocks.reduce((sum, b) => sum + b.length, 0);
  const buf = new Uint8Array(8 + totalBodySize);
  const view = new DataView(buf.buffer);

  view.setUint32(0, originalLength, false); // big-endian
  view.setUint32(4, blocks.length, false);

  let pos = 8;
  for (const block of blocks) {
    buf.set(block, pos);
    pos += block.length;
  }

  return buf;
}

/**
 * Deserialise CMP02 wire-format bytes into tokens and original length.
 *
 * Security: block_count is capped against actual payload size to prevent DoS
 * from a crafted header claiming more blocks than data can hold.
 */
export function deserialiseTokens(data: Uint8Array): [Token[], number] {
  if (data.length < 8) return [[], 0];

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const originalLength = view.getUint32(0, false);
  let blockCount = view.getUint32(4, false);

  // 1 byte minimum per block — cap to prevent DoS.
  const maxPossible = data.length - 8;
  if (blockCount > maxPossible) blockCount = maxPossible;

  const tokens: Token[] = [];
  let pos = 8;

  for (let b = 0; b < blockCount; b++) {
    if (pos >= data.length) break;

    const flag = data[pos++];

    for (let bit = 0; bit < 8; bit++) {
      if (pos >= data.length) break;

      if (flag & (1 << bit)) {
        // Match: 3 bytes
        if (pos + 3 > data.length) break;
        const offset = (data[pos] << 8) | data[pos + 1];
        const length = data[pos + 2];
        tokens.push(match(offset, length));
        pos += 3;
      } else {
        // Literal: 1 byte
        tokens.push(literal(data[pos++]));
      }
    }
  }

  return [tokens, originalLength];
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

/**
 * Compress bytes using LZSS, returning the CMP02 wire format.
 *
 * @example
 * const compressed = compress(new TextEncoder().encode("hello hello hello"));
 * const original   = decompress(compressed);
 */
export function compress(
  data: Uint8Array,
  windowSize = DEFAULT_WINDOW_SIZE,
  maxMatch = DEFAULT_MAX_MATCH,
  minMatch = DEFAULT_MIN_MATCH
): Uint8Array {
  const tokens = encode(data, windowSize, maxMatch, minMatch);
  return serialiseTokens(tokens, data.length);
}

/**
 * Decompress bytes produced by compress().
 */
export function decompress(data: Uint8Array): Uint8Array {
  const [tokens, originalLength] = deserialiseTokens(data);
  return decode(tokens, originalLength);
}
