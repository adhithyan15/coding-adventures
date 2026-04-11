// Package lz78 implements the LZ78 lossless compression algorithm (1978).
//
// LZ78 (Lempel & Ziv, 1978) builds an explicit dictionary of byte sequences
// encountered during encoding. Both encoder and decoder build the same
// dictionary independently — no dictionary is transmitted on the wire.
//
// # The Sliding Window vs. the Explicit Dictionary
//
// LZ77 stores references as (offset, length) into a fixed-size lookback window.
// LZ78 stores references as (dict_index) into a growing trie:
//
//	root ──'A'──> node1 ("A")
//	     ──'B'──> node2 ("B")
//	node1 ──'B'──> node3 ("AB")
//
// Each dictionary entry is defined by a (parent_id, byte) pair. The decoder
// reconstructs any sequence by walking the parent chain upward from the entry,
// then reversing — without needing the dictionary transmitted.
//
// # Token
//
// Each token is a (DictIndex, NextChar) pair:
//   - DictIndex: ID of the longest matching dictionary prefix (0 = literal).
//   - NextChar:  The byte immediately following the match.
//
// # End-of-Stream Handling
//
// If the input ends mid-match, a flush token is emitted with NextChar=0.
// Compress stores the original length in the wire format so Decompress can
// truncate and discard the sentinel byte.
//
// # Wire Format
//
//	Bytes 0–3:  original length (big-endian uint32)
//	Bytes 4–7:  token count (big-endian uint32)
//	Bytes 8+:   token_count × 4 bytes each:
//	              [0..1]  DictIndex (big-endian uint16)
//	              [2]     NextChar (uint8)
//	              [3]     reserved (0x00)
//
// # Series
//
//	CMP00 (LZ77, 1977) — Sliding-window backreferences.
//	CMP01 (LZ78, 1978) — Explicit dictionary (trie). ← this package
//	CMP02 (LZSS, 1982) — LZ77 + flag bits.
//	CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
//	CMP04 (Huffman, 1952) — Entropy coding.
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
package lz78

import (
	"encoding/binary"
)

// Token is one LZ78 output unit: a (DictIndex, NextChar) pair.
//
//   - DictIndex: ID of the longest dictionary prefix that matches current input.
//     0 means no match (pure literal).
//   - NextChar: Byte following the matched sequence. Also used as the flush
//     sentinel (value 0) when input ends mid-match.
type Token struct {
	DictIndex uint16
	NextChar  byte
}

// ─── TrieCursor ───────────────────────────────────────────────────────────────

// TrieCursor is a step-by-step cursor for navigating a byte-keyed trie.
//
// Unlike a full trie API (which operates on complete keys), TrieCursor
// maintains a current position and advances one byte at a time. This is the
// core abstraction for streaming dictionary algorithms:
//
//   - LZ78 (CMP01): Step(byte) → emit token on miss, Insert new entry
//   - LZW  (CMP03): same pattern with a pre-seeded 256-entry alphabet
//
// Usage:
//
//	cursor := NewTrieCursor()
//	for _, b := range data {
//	    if !cursor.Step(b) {
//	        emit Token(cursor.DictID(), b)
//	        cursor.Insert(b, nextID)
//	        cursor.Reset()
//	    }
//	}
//	if !cursor.AtRoot() {
//	    emit flush token
//	}
type TrieCursor struct {
	root    *cursorNode
	current *cursorNode
}

// cursorNode is one node in a TrieCursor's internal trie.
// children maps a byte value to the child node for that byte.
type cursorNode struct {
	dictID   uint16
	children map[byte]*cursorNode
}

// NewTrieCursor creates an empty TrieCursor positioned at root.
func NewTrieCursor() *TrieCursor {
	root := &cursorNode{dictID: 0, children: make(map[byte]*cursorNode)}
	return &TrieCursor{root: root, current: root}
}

// Step tries to follow the child edge for b from the current position.
// Returns true if the edge exists and the cursor advanced; false otherwise
// (cursor stays at current position).
func (c *TrieCursor) Step(b byte) bool {
	if child, ok := c.current.children[b]; ok {
		c.current = child
		return true
	}
	return false
}

// Insert adds a child edge for b at the current position with the given
// dictionary ID. Does not advance the cursor — call Reset() to return to root.
func (c *TrieCursor) Insert(b byte, dictID uint16) {
	c.current.children[b] = &cursorNode{dictID: dictID, children: make(map[byte]*cursorNode)}
}

// Reset returns the cursor to the trie root.
func (c *TrieCursor) Reset() {
	c.current = c.root
}

// DictID returns the dictionary ID at the current cursor position.
// Returns 0 when cursor is at root (representing the empty sequence).
func (c *TrieCursor) DictID() uint16 {
	return c.current.dictID
}

// AtRoot returns true if the cursor is at the root node.
func (c *TrieCursor) AtRoot() bool {
	return c.current == c.root
}

// ─── Encoder ──────────────────────────────────────────────────────────────────

// Encode encodes data into an LZ78 token stream.
//
// Uses a TrieCursor to walk the dictionary one byte at a time.
// When Step(b) returns false (no child edge), emits a token for the current
// dict ID plus b, records the new sequence, and resets to root.
//
// If the input ends mid-match, a flush token with NextChar=0 is emitted.
//
//	tokens := Encode([]byte("ABCDE"), 65536)
//	// tokens.count == 5: all literals (no repeated sequences)
func Encode(data []byte, maxDictSize int) []Token {
	cursor := NewTrieCursor()
	nextID := uint16(1)
	var tokens []Token

	for _, b := range data {
		if !cursor.Step(b) {
			tokens = append(tokens, Token{DictIndex: cursor.DictID(), NextChar: b})
			if int(nextID) < maxDictSize {
				cursor.Insert(b, nextID)
				nextID++
			}
			cursor.Reset()
		}
	}

	// Flush partial match at end of stream.
	if !cursor.AtRoot() {
		tokens = append(tokens, Token{DictIndex: cursor.DictID(), NextChar: 0})
	}

	return tokens
}

// ─── Decoder ──────────────────────────────────────────────────────────────────

// dictEntry stores one decoded dictionary entry as (parentID, byte).
type dictEntry struct {
	parentID uint16
	b        byte
}

// reconstruct walks the parent chain for a dictionary entry and returns the
// corresponding byte sequence (in correct order).
func reconstruct(table []dictEntry, index uint16) []byte {
	if index == 0 {
		return nil
	}
	var rev []byte
	idx := index
	for idx != 0 {
		e := table[idx]
		rev = append(rev, e.b)
		idx = e.parentID
	}
	// Reverse in place.
	for i, j := 0, len(rev)-1; i < j; i, j = i+1, j-1 {
		rev[i], rev[j] = rev[j], rev[i]
	}
	return rev
}

// Decode decodes an LZ78 token stream back into the original bytes.
//
// Mirrors Encode: maintains a parallel dictionary as a slice of (parentID, byte)
// pairs. For each token, reconstructs the sequence for DictIndex, emits it,
// emits NextChar, then adds a new dictionary entry.
//
// originalLength, if >= 0, truncates the output to that length, discarding any
// flush sentinel byte. Pass -1 to return all output bytes.
func Decode(tokens []Token, originalLength int) []byte {
	// table[0] is the root sentinel (unused in reconstruction).
	table := []dictEntry{{0, 0}}
	var output []byte

	for _, tok := range tokens {
		seq := reconstruct(table, tok.DictIndex)
		output = append(output, seq...)

		if originalLength < 0 || len(output) < originalLength {
			output = append(output, tok.NextChar)
		}

		table = append(table, dictEntry{parentID: tok.DictIndex, b: tok.NextChar})
	}

	if originalLength >= 0 && len(output) > originalLength {
		return output[:originalLength]
	}
	return output
}

// ─── Serialisation ────────────────────────────────────────────────────────────

// serialiseTokens encodes tokens to the CMP01 wire format.
//
// Wire format:
//
//	4 bytes  original length (big-endian uint32)
//	4 bytes  token count (big-endian uint32)
//	N × 4    tokens: uint16 DictIndex (BE) + uint8 NextChar + uint8 0x00
func serialiseTokens(tokens []Token, originalLength int) []byte {
	buf := make([]byte, 8+len(tokens)*4)
	binary.BigEndian.PutUint32(buf[0:4], uint32(originalLength))
	binary.BigEndian.PutUint32(buf[4:8], uint32(len(tokens)))
	for i, tok := range tokens {
		base := 8 + i*4
		binary.BigEndian.PutUint16(buf[base:base+2], tok.DictIndex)
		buf[base+2] = tok.NextChar
		buf[base+3] = 0x00
	}
	return buf
}

// deserialiseTokens decodes bytes back into a token list and original length.
func deserialiseTokens(data []byte) ([]Token, int) {
	if len(data) < 8 {
		return nil, 0
	}
	originalLength := int(binary.BigEndian.Uint32(data[0:4]))
	tokenCount := int(binary.BigEndian.Uint32(data[4:8]))
	tokens := make([]Token, 0, tokenCount)
	for i := 0; i < tokenCount; i++ {
		base := 8 + i*4
		if base+4 > len(data) {
			break
		}
		di := binary.BigEndian.Uint16(data[base : base+2])
		nc := data[base+2]
		tokens = append(tokens, Token{DictIndex: di, NextChar: nc})
	}
	return tokens, originalLength
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

// Compress compresses data using LZ78 and serialises to the CMP01 wire format.
//
// The wire format stores the original length so Decompress can exactly
// reconstruct the input even when the last token is a flush token with sentinel
// NextChar=0.
//
//	compressed := Compress([]byte("hello hello hello"), 65536)
//	original   := Decompress(compressed)
//	// bytes.Equal(original, []byte("hello hello hello")) == true
func Compress(data []byte, maxDictSize int) []byte {
	tokens := Encode(data, maxDictSize)
	return serialiseTokens(tokens, len(data))
}

// Decompress decompresses data that was compressed with Compress.
//
// Deserialises the wire format into tokens and original length, then decodes,
// truncating to the original length to discard any flush sentinel.
func Decompress(data []byte) []byte {
	tokens, originalLength := deserialiseTokens(data)
	return Decode(tokens, originalLength)
}
