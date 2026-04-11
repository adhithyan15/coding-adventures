// Package lz77 implements the LZ77 lossless compression algorithm (1977).
//
// # What Is LZ77?
//
// LZ77 is the foundational sliding-window compression algorithm published by
// Abraham Lempel and Jacob Ziv in 1977. It is the ancestor of LZSS, LZW,
// DEFLATE, zstd, LZ4, and virtually every modern compressor used in ZIP, gzip,
// PNG, and zlib.
//
// The core idea: instead of storing every byte verbatim, notice when a sequence
// of bytes has appeared recently. Replace that sequence with a cheap reference
// to where it was — the "offset" (how far back) and "length" (how many bytes).
// This exploits the locality of real data: a repeated word in a document, a
// copied instruction in a binary, an adjacent colour run in an image — all
// trigger compression.
//
// # The Sliding Window Model
//
// LZ77 processes input left-to-right, maintaining two conceptual buffers:
//
//	┌─────────────────────────────────┬──────────────────┐
//	│         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
//	│  (already processed — the       │  (not yet seen —  │
//	│   last windowSize bytes)        │  next maxMatch)   │
//	└─────────────────────────────────┴──────────────────┘
//	                                   ↑
//	                               cursor (current position)
//
// At each step, the encoder searches the search buffer for the longest sequence
// that matches the start of the lookahead buffer. If found and long enough
// (length ≥ minMatch), emit a backreference token. Otherwise, emit a literal
// token for the current byte and advance.
//
// # The Token: (Offset, Length, NextChar)
//
// The encoder outputs a stream of tokens. Each token is a triple:
//
//	(Offset, Length, NextChar)
//
//   - Offset:   Distance back the match starts (1..windowSize), or 0 for no match.
//   - Length:   How many bytes the match covers (0 = no match), up to maxMatch.
//   - NextChar: The literal byte immediately after the match.
//
// The NextChar is always emitted to advance the stream by Length+1 bytes.
//
// # Overlapping Matches (Why Byte-by-Byte Copy Matters)
//
// A match is allowed to extend into bytes that haven't been written yet. This
// happens when Offset < Length. For example:
//
//	If output so far is [A, B] and the token is (2, 5, 'Z'):
//	    Offset=2 means "go back 2 bytes" → position 0 (byte A)
//	    Length=5 means "copy 5 bytes starting there"
//	    But we only have 2 bytes in the buffer!
//
//	The decoder must copy byte-by-byte, not all at once:
//	    1. Copy output[0] (A) → [A, B, A]
//	    2. Copy output[1] (B) → [A, B, A, B]
//	    3. Copy output[2] (A, just written) → [A, B, A, B, A]
//	    4. Copy output[3] (B, just written) → [A, B, A, B, A, B]
//	    5. Copy output[4] (A, just written) → [A, B, A, B, A, B, A]
//	    Finally, append NextChar 'Z' → [A, B, A, B, A, B, A, Z]
//
// This byte-by-byte copy automatically handles the self-referential match and
// acts as run-length encoding for repeating patterns.
//
// # The Series: CMP00 → CMP05
//
//   - CMP00 (LZ77, 1977) — Sliding-window backreferences. This package.
//   - CMP01 (LZ78, 1978) — Explicit dictionary (trie), no sliding window.
//   - CMP02 (LZSS, 1982) — LZ77 + flag bits; eliminates wasted next_char.
//   - CMP03 (LZW,  1984) — Pre-initialized dictionary; powers GIF.
//   - CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   - CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//
// Understanding LZ77 builds the mental model for all of them.
package lz77

import "encoding/binary"

// Token is a single LZ77 token: (Offset, Length, NextChar).
//
// Represents one unit of the compressed stream.
//
//   - Offset:   Distance back the match starts (1..windowSize), or 0.
//   - Length:   Number of bytes the match covers (0 = no match).
//   - NextChar: Literal byte immediately after the match (0..255).
type Token struct {
	Offset   uint16
	Length   uint8
	NextChar byte
}

// findLongestMatch scans the search buffer for the longest match.
//
// Searches the last windowSize bytes before cursor for the longest substring
// that matches the start of the lookahead buffer (starting at cursor).
//
// Returns (bestOffset, bestLength) where bestOffset is the distance back from
// cursor (1-indexed). Returns (0, 0) if no match is found or length < 1.
func findLongestMatch(data []byte, cursor, windowSize, maxMatch int) (bestOffset, bestLength int) {
	// The search buffer starts at most windowSize bytes back.
	searchStart := cursor - windowSize
	if searchStart < 0 {
		searchStart = 0
	}

	// The lookahead cannot extend past the end of input.
	// We must reserve 1 byte for NextChar, so the match can cover at most
	// len(data)-1 as its last position.
	lookaheadEnd := cursor + maxMatch
	if lookaheadEnd > len(data)-1 {
		lookaheadEnd = len(data) - 1
	}

	// Try every possible match start in the search buffer.
	for pos := searchStart; pos < cursor; pos++ {
		length := 0
		// Match byte by byte. Matches may overlap (extend past cursor).
		for cursor+length < lookaheadEnd && data[pos+length] == data[cursor+length] {
			length++
		}

		if length > bestLength {
			bestLength = length
			bestOffset = cursor - pos // Distance back from cursor.
		}
	}
	return
}

// Encode encodes data into an LZ77 token stream.
//
// Scans the input left-to-right, finding the longest match in the search buffer
// for each position. If a match is long enough (≥ minMatch), emits a
// backreference token; otherwise, emits a literal token.
//
// Parameters:
//   - windowSize: Maximum offset (default 4096). Larger = better compression.
//   - maxMatch:   Maximum match length (default 255). Limited by serialization.
//   - minMatch:   Minimum length for a backreference (default 3). A match of
//     length < 3 does not save space over a literal token.
//
// Example:
//
//	tokens := Encode([]byte("ABABABAB"), 4096, 255, 3)
//	// Returns 3 tokens: [A literal, B literal, backreference to AB]
//	decoded := Decode(tokens, nil)
//	// decoded == []byte("ABABABAB")
func Encode(data []byte, windowSize, maxMatch, minMatch int) []Token {
	tokens := make([]Token, 0, len(data))
	cursor := 0

	for cursor < len(data) {
		// Edge case: last byte has no room for NextChar after a match, so emit literal.
		if cursor == len(data)-1 {
			tokens = append(tokens, Token{0, 0, data[cursor]})
			cursor++
			continue
		}

		offset, length := findLongestMatch(data, cursor, windowSize, maxMatch)

		if length >= minMatch {
			// Emit a backreference token.
			nextChar := data[cursor+length]
			tokens = append(tokens, Token{uint16(offset), uint8(length), nextChar})
			cursor += length + 1
		} else {
			// Emit a literal token (no match or too short).
			tokens = append(tokens, Token{0, 0, data[cursor]})
			cursor++
		}
	}

	return tokens
}

// Decode decodes a token stream back into the original data.
//
// Processes each token: if Length > 0, copies Length bytes from the search
// buffer (initialBuffer + output so far) starting at position
// (current_output_length - Offset). Then appends NextChar.
//
// Parameters:
//   - tokens:        The token stream (output of Encode).
//   - initialBuffer: Optional seed for the search buffer (useful for streaming
//     decompression). Pass nil for normal use.
//
// Example:
//
//	tokens := []Token{{0, 0, 65}, {1, 3, 68}}
//	Decode(tokens, nil) // returns []byte("AAAD")
func Decode(tokens []Token, initialBuffer []byte) []byte {
	output := make([]byte, 0, len(tokens)*2)
	output = append(output, initialBuffer...)

	for _, token := range tokens {
		if token.Length > 0 {
			// Copy Length bytes from position (current_output_length - Offset).
			start := len(output) - int(token.Offset)
			// Copy byte-by-byte to handle overlapping matches (Offset < Length).
			// A bulk copy (copy/append) would read stale data for overlapping cases.
			for i := 0; i < int(token.Length); i++ {
				output = append(output, output[start+i])
			}
		}

		// Always append NextChar — it advances the stream by 1.
		output = append(output, token.NextChar)
	}

	return output
}

// serialiseTokens serialises a token list to bytes using a fixed-width format.
//
// Format:
//
//	[4 bytes: token count (big-endian uint32)]
//	[N × 4 bytes: each token as (Offset, Length, NextChar, padding)]
//	    - 2 bytes: Offset   (big-endian uint16)
//	    - 1 byte:  Length   (uint8)
//	    - 1 byte:  NextChar (uint8)
//
// This is a teaching format, not an industry one. Production compressors use
// variable-width bit-packing (see DEFLATE, zstd).
func serialiseTokens(tokens []Token) []byte {
	// 4-byte header + 4 bytes per token.
	buf := make([]byte, 4+len(tokens)*4)

	// Write token count as big-endian uint32.
	binary.BigEndian.PutUint32(buf[0:4], uint32(len(tokens)))

	for i, t := range tokens {
		base := 4 + i*4
		binary.BigEndian.PutUint16(buf[base:base+2], t.Offset)
		buf[base+2] = t.Length
		buf[base+3] = t.NextChar
	}

	return buf
}

// deserialiseTokens deserialises bytes back into a token list.
//
// Inverse of serialiseTokens.
func deserialiseTokens(data []byte) []Token {
	if len(data) < 4 {
		return nil
	}

	count := int(binary.BigEndian.Uint32(data[0:4]))
	tokens := make([]Token, 0, count)

	for i := range count {
		base := 4 + i*4
		if base+4 > len(data) {
			break
		}
		tokens = append(tokens, Token{
			Offset:   binary.BigEndian.Uint16(data[base : base+2]),
			Length:   data[base+2],
			NextChar: data[base+3],
		})
	}

	return tokens
}

// Compress compresses data using LZ77.
//
// One-shot API: Encode then serialise the token stream to bytes.
//
// Parameters:
//   - windowSize: Maximum offset (default 4096).
//   - maxMatch:   Maximum match length (default 255).
//   - minMatch:   Minimum match length for backreferences (default 3).
//
// Example:
//
//	compressed := Compress([]byte("AAAAAAA"), 4096, 255, 3)
//	Decompress(compressed) // returns []byte("AAAAAAA")
func Compress(data []byte, windowSize, maxMatch, minMatch int) []byte {
	tokens := Encode(data, windowSize, maxMatch, minMatch)
	return serialiseTokens(tokens)
}

// Decompress decompresses data that was compressed with Compress.
//
// Deserialises the byte stream into tokens, then decodes.
//
// Example:
//
//	original := []byte("hello world")
//	compressed := Compress(original, 4096, 255, 3)
//	Decompress(compressed) // returns []byte("hello world")
func Decompress(data []byte) []byte {
	tokens := deserialiseTokens(data)
	return Decode(tokens, nil)
}
