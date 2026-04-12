// Package lzss implements the LZSS lossless compression algorithm (1982).
//
// LZSS (Lempel-Ziv-Storer-Szymanski) is a refinement of LZ77 (CMP00) that
// eliminates a systematic waste: in LZ77 every token emits a trailing literal
// byte (next_char) even when a long match was just found. LZSS uses flag bits
// to distinguish literals from back-references, so each symbol is exactly one
// or the other — never both.
//
// # Tokens
//
// LZSS uses two distinct token kinds:
//
//   - KindLiteral — a single byte (no match).
//   - KindMatch   — a (Offset, Length) back-reference into the sliding window.
//
// Encoding costs:
//
//	Literal: 1 byte  (vs. LZ77's 4 bytes per token)
//	Match:   3 bytes (vs. LZ77's 4 bytes per token)
//
// # Flag-Byte Scheme
//
// Tokens are grouped into blocks of up to 8. Each block is preceded by a
// flag byte; bit i (LSB = bit 0) describes token i in the block:
//
//	0 = Literal (1 byte follows)
//	1 = Match   (3 bytes follow: offset uint16 BE + length uint8)
//
// # Wire Format
//
//	Bytes 0–3:  original_length (big-endian uint32)
//	Bytes 4–7:  block_count     (big-endian uint32)
//	Bytes 8+:   blocks
//
//	Each block:
//	  [1 byte]  flag_byte
//	  [variable] 1 or 3 bytes per symbol depending on flag bit
//
// # Series
//
//	CMP00 (LZ77, 1977) — Sliding-window backreferences.
//	CMP01 (LZ78, 1978) — Explicit dictionary (trie).
//	CMP02 (LZSS, 1982) — LZ77 + flag bits. ← this package
//	CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
//	CMP04 (Huffman, 1952) — Entropy coding.
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
package lzss

import (
	"encoding/binary"
)

// TokenKind distinguishes the two LZSS token types.
type TokenKind uint8

const (
	// KindLiteral indicates a single-byte literal token.
	KindLiteral TokenKind = iota
	// KindMatch indicates a back-reference match token.
	KindMatch
)

// Token is one LZSS output unit.
//
// When Kind == KindLiteral, Byte holds the literal value.
// When Kind == KindMatch, Offset and Length hold the back-reference.
type Token struct {
	Kind   TokenKind
	Byte   byte   // valid when Kind == KindLiteral
	Offset uint16 // valid when Kind == KindMatch; distance back (1..window_size)
	Length uint8  // valid when Kind == KindMatch; bytes to copy (min_match..max_match)
}

// Literal returns a new literal token for byte b.
func Literal(b byte) Token {
	return Token{Kind: KindLiteral, Byte: b}
}

// Match returns a new match token for the given offset and length.
func Match(offset uint16, length uint8) Token {
	return Token{Kind: KindMatch, Offset: offset, Length: length}
}

// ─── Encoder ─────────────────────────────────────────────────────────────────

// findLongestMatch searches data[winStart:cursor] for the longest prefix match
// of data[cursor:]. Returns (offset, length) where offset is the distance back
// from cursor and length is the match length. Returns (0, 0) if no match.
//
// Matches may overlap (extend past cursor) — this enables run-length encoding
// as a degenerate case (e.g., "AAAAAAA" → one literal + one self-referential match).
func findLongestMatch(data []byte, cursor, winStart, maxMatch int) (uint16, uint8) {
	bestLen := 0
	bestOff := 0

	lookaheadEnd := cursor + maxMatch
	if lookaheadEnd > len(data) {
		lookaheadEnd = len(data)
	}

	for pos := winStart; pos < cursor; pos++ {
		length := 0
		for cursor+length < lookaheadEnd && data[pos+length] == data[cursor+length] {
			length++
		}
		if length > bestLen {
			bestLen = length
			bestOff = cursor - pos
		}
	}

	return uint16(bestOff), uint8(bestLen)
}

// Encode encodes data into an LZSS token stream.
//
// At each cursor position, searches the last windowSize bytes for the longest
// match. If the match is at least minMatch bytes, emits a Match token and
// advances cursor by that length. Otherwise emits a Literal and advances by 1.
//
// Key difference from LZ77: no next_char byte is emitted after a match.
// The cursor advances by exactly length bytes (not length+1).
func Encode(data []byte, windowSize, maxMatch, minMatch int) []Token {
	tokens := make([]Token, 0, len(data))
	cursor := 0

	for cursor < len(data) {
		winStart := cursor - windowSize
		if winStart < 0 {
			winStart = 0
		}

		offset, length := findLongestMatch(data, cursor, winStart, maxMatch)

		if int(length) >= minMatch {
			tokens = append(tokens, Match(offset, length))
			cursor += int(length)
		} else {
			tokens = append(tokens, Literal(data[cursor]))
			cursor++
		}
	}

	return tokens
}

// ─── Decoder ─────────────────────────────────────────────────────────────────

// Decode decodes an LZSS token stream back into the original bytes.
//
// For each Literal token, appends the byte to output.
// For each Match token, copies length bytes starting from offset positions
// back in the output, byte-by-byte to handle overlapping matches correctly.
//
// originalLength trims the output to the exact uncompressed size.
// Pass -1 to return all bytes.
func Decode(tokens []Token, originalLength int) []byte {
	output := make([]byte, 0, originalLength)

	for _, tok := range tokens {
		switch tok.Kind {
		case KindLiteral:
			output = append(output, tok.Byte)
		case KindMatch:
			start := len(output) - int(tok.Offset)
			for i := 0; i < int(tok.Length); i++ {
				output = append(output, output[start+i]) // byte-by-byte for overlap
			}
		}
	}

	if originalLength >= 0 && len(output) > originalLength {
		return output[:originalLength]
	}
	return output
}

// ─── Serialisation ───────────────────────────────────────────────────────────

// SerialiseTokens serialises an LZSS token list to the CMP02 wire format.
//
// Header: original_length (BE uint32) + block_count (BE uint32).
// Then block_count blocks, each: 1 flag byte + symbol data.
// Flag bit i (LSB=0): 0 = Literal (1 byte), 1 = Match (3 bytes).
func SerialiseTokens(tokens []Token, originalLength int) []byte {
	// Estimate capacity: header + blocks.
	buf := make([]byte, 8, 8+len(tokens)*2)
	binary.BigEndian.PutUint32(buf[0:4], uint32(originalLength))
	// We'll write block_count at bytes 4–7 after counting.

	blockCount := 0
	for i := 0; i < len(tokens); i += 8 {
		chunk := tokens[i:]
		if len(chunk) > 8 {
			chunk = chunk[:8]
		}

		flag := byte(0)
		var symbolData []byte
		for bit, tok := range chunk {
			if tok.Kind == KindMatch {
				flag |= 1 << bit
				symbolData = append(symbolData, byte(tok.Offset>>8), byte(tok.Offset), tok.Length)
			} else {
				symbolData = append(symbolData, tok.Byte)
			}
		}

		buf = append(buf, flag)
		buf = append(buf, symbolData...)
		blockCount++
	}

	binary.BigEndian.PutUint32(buf[4:8], uint32(blockCount))
	return buf
}

// DeserialiseTokens deserialises CMP02 wire-format bytes into tokens and original length.
//
// Security: block_count is capped against actual payload size to prevent DoS
// from a crafted header claiming more blocks than the data can hold.
func DeserialiseTokens(data []byte) ([]Token, int) {
	if len(data) < 8 {
		return nil, 0
	}

	originalLength := int(binary.BigEndian.Uint32(data[0:4]))
	blockCount := int(binary.BigEndian.Uint32(data[4:8]))

	// 1 byte minimum per block (flag only). Cap to prevent DoS.
	maxPossibleBlocks := len(data) - 8
	if blockCount > maxPossibleBlocks {
		blockCount = maxPossibleBlocks
	}

	tokens := make([]Token, 0)
	pos := 8

	for b := 0; b < blockCount; b++ {
		if pos >= len(data) {
			break
		}
		flag := data[pos]
		pos++

		for bit := 0; bit < 8; bit++ {
			if pos >= len(data) {
				break
			}
			if flag&(1<<bit) != 0 {
				// Match: 3 bytes
				if pos+3 > len(data) {
					break
				}
				offset := binary.BigEndian.Uint16(data[pos : pos+2])
				length := data[pos+2]
				tokens = append(tokens, Match(offset, length))
				pos += 3
			} else {
				// Literal: 1 byte
				tokens = append(tokens, Literal(data[pos]))
				pos++
			}
		}
	}

	return tokens, originalLength
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

// DefaultWindowSize is the default sliding-window lookback distance.
const DefaultWindowSize = 4096

// DefaultMaxMatch is the default maximum match length.
const DefaultMaxMatch = 255

// DefaultMinMatch is the minimum match length to emit a Match token.
const DefaultMinMatch = 3

// Compress encodes data with LZSS and serialises to the CMP02 wire format.
//
//	compressed := lzss.Compress(data)
//	original   := lzss.Decompress(compressed)
func Compress(data []byte) []byte {
	tokens := Encode(data, DefaultWindowSize, DefaultMaxMatch, DefaultMinMatch)
	return SerialiseTokens(tokens, len(data))
}

// Decompress decodes data that was produced by Compress.
func Decompress(data []byte) []byte {
	tokens, originalLength := DeserialiseTokens(data)
	return Decode(tokens, originalLength)
}
