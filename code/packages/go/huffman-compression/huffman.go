// Package huffman implements CMP04: Huffman lossless compression and decompression.
//
// # What Is Huffman Coding?
//
// Huffman coding (1952) is an entropy-coding algorithm. "Entropy" here means the
// theoretical minimum number of bits needed to represent data given its symbol
// frequencies. Huffman achieves this minimum with a prefix-free code: no code is
// a prefix of another, so bits can be decoded unambiguously without delimiters.
//
// Think of it like an optimised Morse code. In Morse, the most common English
// letter "E" is a single dot, while rare "Z" is dash-dash-dot-dot. Huffman's
// algorithm constructs this optimally: the most frequent byte gets the shortest
// code, the rarest gets the longest.
//
// # Canonical Codes
//
// This package uses canonical Huffman codes (DEFLATE-style). Two implementations
// with the same byte frequencies produce identical codes, because canonicalisation
// depends only on code lengths — not on which particular tree shape was chosen.
//
// Algorithm for canonical codes (given sorted (symbol, length) pairs):
//
//	code[0] = 0                              (padded to length[0] bits)
//	code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
//
// This means the receiver only needs the code-length table, not the tree. That is
// exactly what the CMP04 wire format stores.
//
// # Wire Format (CMP04)
//
//	Bytes 0–3:    original_length  (big-endian uint32)
//	Bytes 4–7:    symbol_count     (big-endian uint32)
//	Bytes 8–8+2N: code-length table — N entries × 2 bytes:
//	                [0] symbol value  (uint8)
//	                [1] code length   (uint8)
//	              Sorted by (code_length, symbol_value) ascending.
//	Bytes 8+2N+:  bit stream — LSB-first packed, zero-padded to byte boundary.
//
// # How Bits Are Packed
//
// Bits are packed LSB-first: the first bit of the stream goes into bit 0 of
// the first byte. If the stream's total bit count is not a multiple of 8, the
// final byte is zero-padded in its high bits.
//
//	Bit stream: 0 1 0 1 1 (5 bits)
//	Byte:       0b00010110 → 0x16  (bits 0-4 used, bits 5-7 zero)
//
// # The Series
//
//	CMP00 (LZ77,    1977) — Sliding-window backreferences.
//	CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//	CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//	CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//	CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.  (this package)
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
package huffman

import (
	"encoding/binary"
	"fmt"
	"sort"

	huffmantree "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree"
)

// ---------------------------------------------------------------------------
// Wire-format constants
// ---------------------------------------------------------------------------

const (
	// headerSize is the fixed size of the CMP04 header: 4 bytes for
	// original_length + 4 bytes for symbol_count.
	headerSize = 8

	// tableEntrySize is the size of each code-length table entry in bytes.
	// Each entry is (symbol uint8, code_length uint8) — 2 bytes total.
	tableEntrySize = 2

	// maxSymbols is the most entries a valid table can hold: symbols are bytes.
	maxSymbols = 256

	// maxCodeLen is the longest code length the decoder accepts. Real codes
	// over 256 symbols are far shorter; 56 keeps every 64-bit shift safe.
	maxCodeLen = 56
)

// ---------------------------------------------------------------------------
// Internal helpers: bit I/O
// ---------------------------------------------------------------------------

// bitBuilder accumulates individual bit strings into a packed byte slice,
// LSB-first.
//
// "LSB-first" means bit 0 of the first bit string occupies bit 0 (the
// least-significant bit) of the first byte. Bits fill the byte from low to
// high before moving to the next byte.
//
// This matches the convention used by gzip and DEFLATE.
type bitBuilder struct {
	buf    uint64 // accumulator for up to 64 bits
	bitPos uint   // how many bits are currently in buf
	out    []byte // fully completed bytes
}

// writeBitString appends the bit characters in s (e.g. "10110") to the
// builder, LSB-first.
//
// Each character must be '0' or '1'. The first character in s occupies the
// next available LSB position.
func (b *bitBuilder) writeBitString(s string) {
	for i := 0; i < len(s); i++ {
		if s[i] == '1' {
			b.buf |= uint64(1) << b.bitPos
		}
		b.bitPos++
		if b.bitPos == 64 {
			// Flush 8 bytes worth of bits into out.
			for j := 0; j < 8; j++ {
				b.out = append(b.out, byte(b.buf&0xFF))
				b.buf >>= 8
			}
			b.bitPos = 0
		}
	}
}

// flush writes any remaining bits in buf into out, zero-padding the final
// byte.
func (b *bitBuilder) flush() {
	for b.bitPos > 0 {
		b.out = append(b.out, byte(b.buf&0xFF))
		b.buf >>= 8
		if b.bitPos >= 8 {
			b.bitPos -= 8
		} else {
			b.bitPos = 0
		}
	}
}

// bytes returns the final packed byte slice. Call flush() first.
func (b *bitBuilder) bytes() []byte {
	return b.out
}

// ---------------------------------------------------------------------------
// Internal helper: reconstruct canonical codes from a sorted table
// ---------------------------------------------------------------------------

// symbolLen is a (symbol, code_length) pair from the wire-format table.
type symbolLen struct {
	symbol  int
	codeLen int
}

// buildCanonicalCodes reconstructs a symbol→bitString map from a slice of
// (symbol, code_length) pairs that is already sorted by (code_length, symbol).
//
// This is the inverse of CanonicalCodeTable: given only the lengths (as stored
// in the wire format), we reproduce the exact same codes without the tree.
//
// The algorithm mirrors the encoder's CanonicalCodeTable logic:
//
//	code = 0, prev_len = pairs[0].codeLen
//	for each (sym, len):
//	  if len > prev_len: code <<= (len - prev_len)
//	  assign bitString = fmt.Sprintf("%0*b", len, code)
//	  code++
func buildCanonicalCodes(pairs []symbolLen) map[int]string {
	result := make(map[int]string, len(pairs))
	if len(pairs) == 0 {
		return result
	}

	// Single-symbol edge case: the tree assigns code "0" by convention.
	if len(pairs) == 1 {
		result[pairs[0].symbol] = "0"
		return result
	}

	code := 0
	prevLen := pairs[0].codeLen

	for _, sl := range pairs {
		if sl.codeLen > prevLen {
			code <<= (sl.codeLen - prevLen)
		}
		result[sl.symbol] = fmt.Sprintf("%0*b", sl.codeLen, code)
		code++
		prevLen = sl.codeLen
	}

	return result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Compress compresses data using canonical Huffman coding and returns
// CMP04 wire-format bytes.
//
// The algorithm:
//  1. Build a byte-frequency histogram.
//  2. Build a Huffman tree (via DT27 huffman-tree).
//  3. Derive canonical codes (DEFLATE-style: only lengths transmitted).
//  4. Sort (symbol, code_length) pairs by (code_length, symbol).
//  5. Concatenate canonical bit strings for each input byte.
//  6. Pack the bit stream LSB-first, zero-padding to a byte boundary.
//  7. Assemble: 8-byte header + code-length table + packed bits.
//
// Edge cases:
//   - Empty input: returns an 8-byte header (original_length=0, symbol_count=0).
//   - Single distinct byte: the huffman-tree package assigns it code "0",
//     so every occurrence encodes as one zero bit.
func Compress(data []byte) ([]byte, error) {
	// ── Step 1: byte-frequency histogram ────────────────────────────────────
	//
	// We count how many times each byte value (0–255) appears. Only bytes
	// with count > 0 become entries in the Huffman tree.
	var freq [256]int
	for _, b := range data {
		freq[b]++
	}

	// ── Edge case: empty input ───────────────────────────────────────────────
	if len(data) == 0 {
		header := make([]byte, headerSize)
		// original_length = 0, symbol_count = 0 — both big-endian uint32.
		binary.BigEndian.PutUint32(header[0:4], 0)
		binary.BigEndian.PutUint32(header[4:8], 0)
		return header, nil
	}

	// ── Step 2: build Huffman tree ───────────────────────────────────────────
	//
	// Collect non-zero (symbol, frequency) pairs and pass them to DT27.
	weights := make([]huffmantree.WeightPair, 0, 256)
	for sym, count := range freq {
		if count > 0 {
			weights = append(weights, huffmantree.WeightPair{
				Symbol:    sym,
				Frequency: count,
			})
		}
	}

	tree, err := huffmantree.Build(weights)
	if err != nil {
		return nil, fmt.Errorf("huffman.Compress: build tree: %w", err)
	}

	// ── Step 3: get canonical code table ────────────────────────────────────
	//
	// CanonicalCodeTable returns map[symbol]bitString sorted canonically.
	// We only need the lengths for the wire format, but we need the full
	// strings for encoding the input bytes.
	codeTable := huffmantree.CanonicalCodeTable(tree)

	// ── Step 4: sort (symbol, code_length) pairs ────────────────────────────
	//
	// Sorted by (code_length, symbol_value) ascending — this is the exact
	// order that allows both encoder and decoder to reconstruct codes
	// deterministically from lengths alone.
	pairs := make([]symbolLen, 0, len(codeTable))
	for sym, bits := range codeTable {
		pairs = append(pairs, symbolLen{symbol: sym, codeLen: len(bits)})
	}
	sort.Slice(pairs, func(i, j int) bool {
		if pairs[i].codeLen != pairs[j].codeLen {
			return pairs[i].codeLen < pairs[j].codeLen
		}
		return pairs[i].symbol < pairs[j].symbol
	})

	// ── Step 5 & 6: encode input and pack bits LSB-first ────────────────────
	//
	// For each input byte, look up its canonical bit string in codeTable and
	// append it to the bit stream. The bitBuilder packs bits LSB-first.
	bb := &bitBuilder{}
	for _, b := range data {
		bits, ok := codeTable[int(b)]
		if !ok {
			return nil, fmt.Errorf("huffman.Compress: no code for symbol %d", b)
		}
		bb.writeBitString(bits)
	}
	bb.flush()
	packedBits := bb.bytes()

	// ── Step 7: assemble wire-format output ─────────────────────────────────
	//
	// Layout:
	//   [0:4]  original_length (big-endian uint32)
	//   [4:8]  symbol_count    (big-endian uint32)
	//   [8:8+2N] code-length table (N × 2 bytes: symbol, code_length)
	//   [8+2N:]  packed bit stream
	symbolCount := uint32(len(pairs))
	tableSize := int(symbolCount) * tableEntrySize

	out := make([]byte, headerSize+tableSize+len(packedBits))

	binary.BigEndian.PutUint32(out[0:4], uint32(len(data)))
	binary.BigEndian.PutUint32(out[4:8], symbolCount)

	for i, sl := range pairs {
		offset := headerSize + i*tableEntrySize
		out[offset] = byte(sl.symbol)
		out[offset+1] = byte(sl.codeLen)
	}

	copy(out[headerSize+tableSize:], packedBits)
	return out, nil
}

// Decompress decompresses CMP04 wire-format data and returns the original bytes.
//
// The algorithm:
//  1. Parse the 8-byte header (original_length, symbol_count).
//  2. Parse the code-length table (symbol_count × 2 bytes).
//  3. Reconstruct canonical codes from the sorted (symbol, length) list.
//  4. Unpack the bit stream LSB-first into a bit string.
//  5. Decode exactly original_length symbols by prefix-matching against the
//     canonical code table.
//
// Edge cases:
//   - original_length == 0: return empty slice.
//   - symbol_count == 1: every "0" bit decodes to the single symbol.
func Decompress(data []byte) ([]byte, error) {
	// ── Step 1: parse header ────────────────────────────────────────────────
	if len(data) < headerSize {
		return nil, fmt.Errorf("huffman.Decompress: data too short for header (%d bytes)", len(data))
	}
	// Keep the header fields as uint64 so nothing wraps on 32-bit platforms.
	originalLength := uint64(binary.BigEndian.Uint32(data[0:4]))
	symbolCount := uint64(binary.BigEndian.Uint32(data[4:8]))

	// ── Edge case: empty original ────────────────────────────────────────────
	if originalLength == 0 {
		return []byte{}, nil
	}

	// ── Step 2: parse code-length table ─────────────────────────────────────
	//
	// SECURITY: the header is attacker-controlled. Symbols are single bytes, so
	// a table can name at most 256 of them. Checking this first also bounds
	// the table size computation below.
	if symbolCount > maxSymbols {
		return nil, fmt.Errorf("huffman.Decompress: symbol_count %d exceeds %d", symbolCount, maxSymbols)
	}
	tableEnd := headerSize + int(symbolCount)*tableEntrySize
	if len(data) < tableEnd {
		return nil, fmt.Errorf(
			"huffman.Decompress: data too short for table (need %d bytes, have %d)",
			tableEnd, len(data),
		)
	}

	// ── Step 3: reconstruct canonical codes ─────────────────────────────────
	//
	// Given the sorted (symbol, length) list, assign codes numerically
	// starting from 0, left-shifting whenever the length grows.
	//
	// SECURITY: the table is hostile input, so we reject
	//   - a symbol that appears twice (ambiguous table),
	//   - lengths of 0 or above maxCodeLen (a zero-length code matches
	//     nothing; a huge length would overflow the 64-bit shift),
	//   - tables not sorted by length (the encoder always sorts), and
	//   - over-subscribed tables (Kraft sum > 1): before each assignment the
	//     running code must still fit in `length` bits, i.e. code>>length == 0.
	// Codes are keyed by (length, value) packed into a uint64 so decoding needs
	// no per-symbol string building.
	codeToSym := make(map[uint64]byte, symbolCount)
	var seen [256]bool
	var code uint64
	prevLen := uint(0)
	maxLen := uint(0)
	for i := 0; i < int(symbolCount); i++ {
		offset := headerSize + i*tableEntrySize
		sym := data[offset]
		length := uint(data[offset+1])
		if seen[sym] {
			return nil, fmt.Errorf("huffman.Decompress: symbol %d appears twice in table", sym)
		}
		seen[sym] = true
		if length == 0 || length > maxCodeLen {
			return nil, fmt.Errorf("huffman.Decompress: invalid code length %d in table", length)
		}
		if i == 0 {
			prevLen = length
		}
		if length < prevLen {
			return nil, fmt.Errorf("huffman.Decompress: table is not sorted by code length")
		}
		code <<= length - prevLen
		if code>>length != 0 {
			return nil, fmt.Errorf("huffman.Decompress: table over-subscribes the code space")
		}
		codeToSym[uint64(length)<<56|code] = sym
		code++
		prevLen = length
		maxLen = length
	}

	// SECURITY: every code is at least one bit, so N payload bytes hold at most
	// 8*N symbols. Reject a larger declared length before allocating anything
	// sized from it.
	payload := data[tableEnd:]
	totalBits := uint64(len(payload)) * 8
	if originalLength > totalBits {
		return nil, fmt.Errorf(
			"huffman.Decompress: declared length %d exceeds the %d bits available",
			originalLength, totalBits,
		)
	}

	// ── Steps 4+5: decode straight from the packed bits ─────────────────────
	//
	// Bits are LSB-first within each byte, so bit i is
	// (payload[i/8] >> (i%8)) & 1 -- O(1), no expanded bit string. We build the
	// candidate code MSB-first as an integer. Canonical codes are prefix-free,
	// so the first (length, value) hit is the symbol. Once the accumulated
	// length reaches the longest code without a match, no later bit can match
	// (an incomplete table fed unmatched bits), so we fail immediately.
	result := make([]byte, 0, originalLength)
	var acc uint64
	var accLen uint
	for pos := uint64(0); uint64(len(result)) < originalLength; pos++ {
		if pos >= totalBits {
			return nil, fmt.Errorf(
				"huffman.Decompress: decoded %d symbols, expected %d",
				len(result), originalLength,
			)
		}
		acc = acc<<1 | uint64((payload[pos>>3]>>(pos&7))&1)
		accLen++
		if sym, ok := codeToSym[uint64(accLen)<<56|acc]; ok {
			result = append(result, sym)
			acc, accLen = 0, 0
		} else if accLen >= maxLen {
			return nil, fmt.Errorf("huffman.Decompress: bit pattern does not match any code")
		}
	}

	return result, nil
}
