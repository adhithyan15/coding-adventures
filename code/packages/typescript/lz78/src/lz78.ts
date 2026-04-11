/**
 * lz78.ts — LZ78 Lossless Compression Algorithm (1978)
 *
 * LZ78 builds an explicit dictionary of byte sequences as it encodes.
 * Both encoder and decoder build the same dictionary independently — no
 * dictionary is transmitted. Each token is a (dictIndex, nextChar) pair.
 *
 * @module
 */

// ─── Token ────────────────────────────────────────────────────────────────────

/**
 * One LZ78 token: a (dictIndex, nextChar) pair.
 *
 * - `dictIndex`: ID of the longest dictionary prefix that matches current input.
 *   0 = pure literal (no match).
 * - `nextChar`:  Byte following the match. Also used as flush sentinel (0) when
 *   input ends mid-match.
 */
export interface Token {
  readonly dictIndex: number;
  readonly nextChar: number;
}

// ─── Internal trie ────────────────────────────────────────────────────────────

/** One node in the encoding trie. */
class TrieNode {
  readonly dictId: number;
  readonly children: Map<number, TrieNode> = new Map();

  constructor(dictId: number) {
    this.dictId = dictId;
  }
}

// ─── Encoder ──────────────────────────────────────────────────────────────────

/**
 * Encode bytes into an LZ78 token stream.
 *
 * @param data        - Input bytes.
 * @param maxDictSize - Maximum dictionary entries (default 65536).
 * @returns Array of Token in emission order.
 *
 * @example
 * encode(new Uint8Array([65, 66, 67, 68, 69]))
 * // → [{dictIndex:0,nextChar:65}, {dictIndex:0,nextChar:66}, ...]
 */
export function encode(
  data: Uint8Array,
  maxDictSize = 65536,
): Token[] {
  const root = new TrieNode(0);
  let nextId = 1;
  let current = root;
  const tokens: Token[] = [];

  for (const byte of data) {
    const child = current.children.get(byte);
    if (child !== undefined) {
      current = child;
    } else {
      tokens.push({ dictIndex: current.dictId, nextChar: byte });
      if (nextId < maxDictSize) {
        current.children.set(byte, new TrieNode(nextId));
        nextId++;
      }
      current = root;
    }
  }

  // Flush partial match at end of stream.
  if (current !== root) {
    tokens.push({ dictIndex: current.dictId, nextChar: 0 });
  }

  return tokens;
}

// ─── Decoder ──────────────────────────────────────────────────────────────────

/** A dictionary entry: (parentId, byte). */
interface DictEntry {
  parentId: number;
  b: number;
}

function reconstruct(table: DictEntry[], index: number): number[] {
  if (index === 0) return [];
  const rev: number[] = [];
  let idx = index;
  while (idx !== 0) {
    const entry = table[idx];
    rev.push(entry.b);
    idx = entry.parentId;
  }
  rev.reverse();
  return rev;
}

/**
 * Decode an LZ78 token stream back into the original bytes.
 *
 * @param tokens         - Token stream from encode().
 * @param originalLength - If >= 0, truncate output to this length (strips flush
 *   sentinel). Pass -1 for all bytes.
 * @returns Reconstructed bytes.
 */
export function decode(tokens: Token[], originalLength = -1): Uint8Array {
  const table: DictEntry[] = [{ parentId: 0, b: 0 }]; // index 0 = root sentinel
  const output: number[] = [];

  for (const tok of tokens) {
    const seq = reconstruct(table, tok.dictIndex);
    output.push(...seq);

    if (originalLength < 0 || output.length < originalLength) {
      output.push(tok.nextChar);
    }

    table.push({ parentId: tok.dictIndex, b: tok.nextChar });
  }

  const result = originalLength >= 0
    ? output.slice(0, originalLength)
    : output;
  return new Uint8Array(result);
}

// ─── Serialisation ────────────────────────────────────────────────────────────

/**
 * Serialise tokens to the CMP01 wire format.
 *
 * Wire format:
 * - 4 bytes: original length (big-endian uint32)
 * - 4 bytes: token count (big-endian uint32)
 * - N × 4:   tokens: uint16 dictIndex (BE) + uint8 nextChar + uint8 0x00
 */
export function serialiseTokens(tokens: Token[], originalLength: number): Uint8Array {
  const buf = new Uint8Array(8 + tokens.length * 4);
  const view = new DataView(buf.buffer);
  view.setUint32(0, originalLength, false); // big-endian
  view.setUint32(4, tokens.length, false);
  for (let i = 0; i < tokens.length; i++) {
    const base = 8 + i * 4;
    view.setUint16(base, tokens[i].dictIndex, false);
    buf[base + 2] = tokens[i].nextChar;
    buf[base + 3] = 0x00;
  }
  return buf;
}

/**
 * Deserialise CMP01 wire format back into tokens and original length.
 */
export function deserialiseTokens(data: Uint8Array): [Token[], number] {
  if (data.length < 8) return [[], 0];
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const originalLength = view.getUint32(0, false);
  const tokenCount = view.getUint32(4, false);
  const tokens: Token[] = [];
  for (let i = 0; i < tokenCount; i++) {
    const base = 8 + i * 4;
    if (base + 4 > data.length) break;
    tokens.push({
      dictIndex: view.getUint16(base, false),
      nextChar: data[base + 2],
    });
  }
  return [tokens, originalLength];
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

/**
 * Compress bytes using LZ78, returning the CMP01 wire format.
 *
 * @example
 * const compressed = compress(new TextEncoder().encode("hello"));
 * const original   = decompress(compressed);
 */
export function compress(data: Uint8Array, maxDictSize = 65536): Uint8Array {
  const tokens = encode(data, maxDictSize);
  return serialiseTokens(tokens, data.length);
}

/**
 * Decompress bytes that were compressed with compress().
 */
export function decompress(data: Uint8Array): Uint8Array {
  const [tokens, originalLength] = deserialiseTokens(data);
  return decode(tokens, originalLength);
}
