// LZ78.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// LZ78 Lossless Compression Algorithm (1978) — CMP01
// ============================================================================
//
// LZ78 (Lempel & Ziv, 1978) builds an explicit trie-based dictionary of byte
// sequences as it encodes. Both encoder and decoder build the same dictionary
// independently — no dictionary is transmitted on the wire.
//
// How It Differs from LZ77 (CMP00)
// ---------------------------------
//
// LZ77 uses a *sliding window*: it forgets bytes that fall off the back.
// LZ78 grows a *global dictionary* that never forgets, which makes it better
// for repetitive data spread throughout a file.
//
// Token: (dictIndex, nextChar)
// ----------------------------
//
// - dictIndex: ID of the longest dictionary prefix matched (0 = literal).
// - nextChar:  The byte following the match. 0 = flush sentinel at end.
//
// Wire Format (CMP01)
// --------------------
//
//   Bytes 0–3:  original length (big-endian UInt32)
//   Bytes 4–7:  token count    (big-endian UInt32)
//   Bytes 8+:   N × 4 bytes each:
//                 [0..1]  dictIndex (big-endian UInt16)
//                 [2]     nextChar  (UInt8)
//                 [3]     reserved  (0x00)
//
// Series
// ------
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie). ← this file
//   CMP02 (LZSS,    1982) — LZ77 + flag bits.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
//   CMP04 (Huffman, 1952) — Entropy coding.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG.
//
// ============================================================================

// ─── Token ────────────────────────────────────────────────────────────────────

/// One LZ78 output unit: a (dictIndex, nextChar) pair.
///
/// - `dictIndex`: ID of the longest dictionary prefix that matches current
///   input. 0 means no match (pure literal).
/// - `nextChar`:  Byte following the matched sequence. Also used as the flush
///   sentinel (value 0) when input ends mid-match.
public struct Token: Equatable {
    public let dictIndex: UInt16
    public let nextChar:  UInt8

    public init(dictIndex: UInt16, nextChar: UInt8) {
        self.dictIndex = dictIndex
        self.nextChar  = nextChar
    }
}

// ─── TrieCursor ───────────────────────────────────────────────────────────────

/// A step-by-step cursor for navigating a byte-keyed trie.
///
/// Unlike a full trie API (which operates on complete keys), `TrieCursor`
/// maintains a current position and advances one byte at a time. This is
/// the core abstraction for streaming dictionary algorithms:
///
/// - **LZ78** (CMP01): `step(_:)` → emit token on miss, `insert(_:dictID:)` new entry
/// - **LZW**  (CMP03): same pattern with a pre-seeded 256-entry alphabet
///
/// ## Usage
///
/// ```swift
/// var cursor = TrieCursor()
/// for byte in data {
///     if !cursor.step(byte) {
///         emit(Token(dictIndex: cursor.dictID, nextChar: byte))
///         cursor.insert(byte, dictID: nextID)
///         nextID += 1
///         cursor.reset()
///     }
/// }
/// if !cursor.atRoot { emit flush token }
/// ```
public struct TrieCursor {

    // Arena-based trie: nodes stored in a flat array, referenced by index.
    // Node 0 = root (dictID: 0, children: [:]).
    private var arena:   [CursorNode]
    private var current: Int

    /// Create a new `TrieCursor` with an empty trie. Cursor starts at root.
    public init() {
        arena   = [CursorNode(dictID: 0)]
        current = 0
    }

    /// Try to follow the child edge for `byte` from the current position.
    ///
    /// Returns `true` and advances if the edge exists; `false` otherwise
    /// (cursor stays at current position).
    @discardableResult
    public mutating func step(_ byte: UInt8) -> Bool {
        if let childIdx = arena[current].children[byte] {
            current = childIdx
            return true
        }
        return false
    }

    /// Add a child edge for `byte` at the current position with the given `dictID`.
    ///
    /// Does not advance the cursor — call `reset()` to return to root.
    public mutating func insert(_ byte: UInt8, dictID: UInt16) {
        let newIdx = arena.count
        arena.append(CursorNode(dictID: dictID))
        arena[current].children[byte] = newIdx
    }

    /// Reset the cursor to the trie root.
    public mutating func reset() {
        current = 0
    }

    /// Dictionary ID at the current cursor position.
    ///
    /// Returns `0` when cursor is at root (representing the empty sequence).
    public var dictID: UInt16 {
        arena[current].dictID
    }

    /// `true` if the cursor is at the root node.
    public var atRoot: Bool {
        current == 0
    }

    /// All `(path, dictID)` pairs in the trie (DFS, sorted by dictID).
    public func entries() -> [([UInt8], UInt16)] {
        var results: [([UInt8], UInt16)] = []
        collectEntries(nodeIdx: 0, path: [], results: &results)
        return results.sorted { $0.1 < $1.1 }
    }

    private func collectEntries(nodeIdx: Int, path: [UInt8], results: inout [([UInt8], UInt16)]) {
        let node = arena[nodeIdx]
        if node.dictID > 0 {
            results.append((path, node.dictID))
        }
        for (byte, childIdx) in node.children {
            var p = path
            p.append(byte)
            collectEntries(nodeIdx: childIdx, path: p, results: &results)
        }
    }
}

private struct CursorNode {
    var dictID:   UInt16
    var children: [UInt8: Int] = [:]

    init(dictID: UInt16) {
        self.dictID = dictID
    }
}

// ─── Encoder ──────────────────────────────────────────────────────────────────

/// Encode bytes into an LZ78 token stream.
///
/// Uses `TrieCursor` to walk the dictionary one byte at a time.
/// When `step(_:)` returns `false` (no child edge), emits a token for the
/// current `dictID` plus `byte`, records the new sequence, and resets to root.
///
/// If the input ends mid-match, a flush token with `nextChar: 0` is emitted.
///
/// - Parameters:
///   - data:        Input bytes.
///   - maxDictSize: Maximum dictionary entries (default 65536).
/// - Returns: Array of `Token` in emission order.
///
/// ```swift
/// let tokens = encode([65, 66, 67, 68, 69])
/// // All 5 tokens have dictIndex == 0 (all literals)
/// ```
public func encode(_ data: [UInt8], maxDictSize: Int = 65536) -> [Token] {
    var cursor  = TrieCursor()
    var nextID  = UInt16(1)
    var tokens  = [Token]()

    for byte in data {
        if !cursor.step(byte) {
            tokens.append(Token(dictIndex: cursor.dictID, nextChar: byte))
            if Int(nextID) < maxDictSize {
                cursor.insert(byte, dictID: nextID)
                nextID += 1
            }
            cursor.reset()
        }
    }

    // Flush partial match at end of stream.
    if !cursor.atRoot {
        tokens.append(Token(dictIndex: cursor.dictID, nextChar: 0))
    }

    return tokens
}

// ─── Decoder ──────────────────────────────────────────────────────────────────

/// Decode an LZ78 token stream back into the original bytes.
///
/// Mirrors `encode`: maintains a parallel dictionary as an array of
/// `(parentID, byte)` pairs. For each token, reconstructs the sequence for
/// `dictIndex`, emits it, emits `nextChar`, then adds a new dictionary entry.
///
/// - Parameters:
///   - tokens:         Token stream from `encode`.
///   - originalLength: If >= 0, truncates output to that length (discards flush
///     sentinel). Pass -1 to return all output bytes.
/// - Returns: Reconstructed bytes.
public func decode(_ tokens: [Token], originalLength: Int = -1) -> [UInt8] {
    // table[0] = root sentinel (unused in reconstruction).
    var table: [(parentID: UInt16, byte: UInt8)] = [(0, 0)]
    var output = [UInt8]()

    for tok in tokens {
        let seq = reconstruct(table: table, index: tok.dictIndex)
        output.append(contentsOf: seq)

        if originalLength < 0 || output.count < originalLength {
            output.append(tok.nextChar)
        }

        table.append((parentID: tok.dictIndex, byte: tok.nextChar))
    }

    if originalLength >= 0 && output.count > originalLength {
        return Array(output.prefix(originalLength))
    }
    return output
}

private func reconstruct(table: [(parentID: UInt16, byte: UInt8)], index: UInt16) -> [UInt8] {
    guard index != 0 else { return [] }
    var rev = [UInt8]()
    var idx = Int(index)
    while idx != 0 {
        let entry = table[idx]
        rev.append(entry.byte)
        idx = Int(entry.parentID)
    }
    return rev.reversed()
}

// ─── Serialisation ────────────────────────────────────────────────────────────

/// Serialise tokens to the CMP01 wire format.
public func serialiseTokens(_ tokens: [Token], originalLength: Int) -> [UInt8] {
    var buf = [UInt8]()
    buf.reserveCapacity(8 + tokens.count * 4)

    func appendU32(_ n: UInt32) {
        buf.append(UInt8((n >> 24) & 0xff))
        buf.append(UInt8((n >> 16) & 0xff))
        buf.append(UInt8((n >>  8) & 0xff))
        buf.append(UInt8( n        & 0xff))
    }
    func appendU16(_ n: UInt16) {
        buf.append(UInt8((n >> 8) & 0xff))
        buf.append(UInt8( n       & 0xff))
    }

    appendU32(UInt32(originalLength))
    appendU32(UInt32(tokens.count))
    for tok in tokens {
        appendU16(tok.dictIndex)
        buf.append(tok.nextChar)
        buf.append(0x00)
    }
    return buf
}

/// Deserialise CMP01 wire-format bytes back into tokens and original length.
public func deserialiseTokens(_ data: [UInt8]) -> ([Token], Int) {
    guard data.count >= 8 else { return ([], 0) }

    func readU32(at pos: Int) -> UInt32 {
        UInt32(data[pos]) << 24 | UInt32(data[pos+1]) << 16 |
        UInt32(data[pos+2]) <<  8 | UInt32(data[pos+3])
    }
    func readU16(at pos: Int) -> UInt16 {
        UInt16(data[pos]) << 8 | UInt16(data[pos+1])
    }

    let originalLength = Int(readU32(at: 0))
    let tokenCount     = Int(readU32(at: 4))
    var tokens         = [Token]()
    tokens.reserveCapacity(tokenCount)

    for i in 0 ..< tokenCount {
        let base = 8 + i * 4
        guard base + 4 <= data.count else { break }
        let dictIndex = readU16(at: base)
        let nextChar  = data[base + 2]
        tokens.append(Token(dictIndex: dictIndex, nextChar: nextChar))
    }
    return (tokens, originalLength)
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

/// Compress bytes using LZ78 and serialise to the CMP01 wire format.
///
/// ```swift
/// let compressed = compress([UInt8]("hello".utf8))
/// let original   = decompress(compressed)
/// ```
public func compress(_ data: [UInt8], maxDictSize: Int = 65536) -> [UInt8] {
    let tokens = encode(data, maxDictSize: maxDictSize)
    return serialiseTokens(tokens, originalLength: data.count)
}

/// Decompress bytes that were compressed with `compress`.
public func decompress(_ data: [UInt8]) -> [UInt8] {
    let (tokens, originalLength) = deserialiseTokens(data)
    return decode(tokens, originalLength: originalLength)
}
