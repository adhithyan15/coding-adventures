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

// ─── Internal trie ────────────────────────────────────────────────────────────

// trieNode is one node in the encoding trie.
// Each node represents a dictionary entry (a byte sequence reachable from
// the root). The sequence is implicit in the path from root to this node.
type trieNode struct {
	dictID   uint16
	children map[byte]*trieNode
}

func newTrieNode(id uint16) *trieNode {
	return &trieNode{dictID: id, children: make(map[byte]*trieNode)}
}

// ─── Encoder ──────────────────────────────────────────────────────────────────

// Encode encodes data into an LZ78 token stream.
//
// Scans the input left-to-right, following trie edges for each byte. When a
// byte has no child edge from the current node, emits a token and resets to the
// root. Adds a new trie node for the matched sequence plus the new byte.
//
// If the input ends mid-match, a flush token with NextChar=0 is emitted.
// Use Compress/Decompress to round-trip data without dealing with the flush
// sentinel manually.
//
//	tokens := Encode([]byte("ABCDE"))
//	// tokens.count == 5: all literals (no repeated sequences)
func Encode(data []byte, maxDictSize int) []Token {
	root := newTrieNode(0)
	nextID := uint16(1)
	current := root
	var tokens []Token

	for _, b := range data {
		if child, ok := current.children[b]; ok {
			// Edge exists — extend the current match.
			current = child
		} else {
			// No edge — emit token and expand dictionary.
			tokens = append(tokens, Token{DictIndex: current.dictID, NextChar: b})

			if int(nextID) < maxDictSize {
				node := newTrieNode(nextID)
				current.children[b] = node
				nextID++
			}

			current = root
		}
	}

	// Flush partial match at end of stream.
	if current != root {
		tokens = append(tokens, Token{DictIndex: current.dictID, NextChar: 0})
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
