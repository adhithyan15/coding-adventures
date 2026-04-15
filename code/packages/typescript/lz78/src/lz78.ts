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

// ─── TrieCursor ───────────────────────────────────────────────────────────────

/** Internal node used by TrieCursor. */
interface CursorNode {
  dictId: number;
  children: Map<number, CursorNode>;
}

function makeCursorNode(dictId: number): CursorNode {
  return { dictId, children: new Map() };
}

/**
 * A step-by-step cursor for navigating a byte-keyed trie.
 *
 * Unlike a full trie API (which operates on complete keys), `TrieCursor`
 * maintains a current position and advances one byte at a time. This is the
 * core abstraction for streaming dictionary algorithms:
 *
 * - **LZ78** (CMP01): `step(byte)` → emit token on miss, `insert` new entry
 * - **LZW**  (CMP03): same pattern with a pre-seeded 256-entry alphabet
 *
 * @example
 * const cursor = new TrieCursor();
 * for (const byte of data) {
 *   if (!cursor.step(byte)) {
 *     tokens.push({ dictIndex: cursor.dictId, nextChar: byte });
 *     cursor.insert(byte, nextId++);
 *     cursor.reset();
 *   }
 * }
 */
export class TrieCursor {
  private readonly root: CursorNode;
  private current: CursorNode;

  constructor() {
    this.root = makeCursorNode(0);
    this.current = this.root;
  }

  /**
   * Try to follow the child edge for `byte` from the current position.
   * Returns `true` and advances if the child exists; `false` otherwise (cursor
   * stays at current position).
   */
  step(byte: number): boolean {
    const child = this.current.children.get(byte);
    if (child !== undefined) {
      this.current = child;
      return true;
    }
    return false;
  }

  /**
   * Add a child edge for `byte` at the current position with `dictId`.
   * Does not advance the cursor — call `reset()` to return to root.
   */
  insert(byte: number, dictId: number): void {
    this.current.children.set(byte, makeCursorNode(dictId));
  }

  /** Reset the cursor to the trie root. */
  reset(): void {
    this.current = this.root;
  }

  /** Dictionary ID at the current cursor position. 0 when at root. */
  get dictId(): number {
    return this.current.dictId;
  }

  /** `true` if the cursor is at the root node. */
  get atRoot(): boolean {
    return this.current === this.root;
  }

  /** Iterate all [path, dictId] pairs stored in the trie (DFS). */
  *[Symbol.iterator](): Iterator<[number[], number]> {
    yield* iterNode(this.root, []);
  }
}

function* iterNode(
  node: CursorNode,
  path: number[],
): Generator<[number[], number]> {
  if (node.dictId > 0) yield [[...path], node.dictId];
  for (const [byte, child] of node.children) {
    path.push(byte);
    yield* iterNode(child, path);
    path.pop();
  }
}

// ─── Encoder ──────────────────────────────────────────────────────────────────

/**
 * Encode bytes into an LZ78 token stream.
 *
 * Uses `TrieCursor` to walk the dictionary one byte at a time.
 * When `cursor.step(byte)` returns `false`, emits a token for the current
 * `dictId` plus `byte`, records the new sequence, resets to root.
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
  const cursor = new TrieCursor();
  let nextId = 1;
  const tokens: Token[] = [];

  for (const byte of data) {
    if (!cursor.step(byte)) {
      tokens.push({ dictIndex: cursor.dictId, nextChar: byte });
      if (nextId < maxDictSize) {
        cursor.insert(byte, nextId);
        nextId++;
      }
      cursor.reset();
    }
  }

  // Flush partial match at end of stream.
  if (!cursor.atRoot) {
    tokens.push({ dictIndex: cursor.dictId, nextChar: 0 });
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
