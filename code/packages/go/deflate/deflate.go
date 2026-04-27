// Package deflate implements CMP05: DEFLATE lossless compression (1996).
//
// DEFLATE is the dominant general-purpose lossless compression algorithm,
// powering ZIP, gzip, PNG, and HTTP/2 header compression. It combines two
// complementary techniques from earlier in this series:
//
//  1. LZSS tokenization (CMP02) — replaces repeated substrings with
//     back-references into a 4096-byte sliding window.
//  2. Dual canonical Huffman coding (DT27/CMP04) — entropy-codes the
//     resulting token stream using two separate Huffman trees.
//
// # The Two-Pass Design
//
// Pass 1 (LZSS): scan input left-to-right, emitting Literal or Match tokens.
//
//	"ABCABCABC" → Lit('A') Lit('B') Lit('C') Match(offset=3, length=6)
//
// Pass 2 (Huffman): count token frequencies, build two canonical trees,
// then encode the token stream:
//
//   - LL tree: covers symbols 0–284 (literals, end-of-data, length codes)
//   - Dist tree: covers distance codes 0–23
//
// # The Expanded LL Alphabet
//
// Instead of using a separate token type for lengths, DEFLATE merges literal
// bytes and length codes into one alphabet:
//
//	Symbols 0–255:   literal byte values
//	Symbol  256:     end-of-data marker
//	Symbols 257–284: length codes (each covers a range via extra bits)
//
// Length codes use "extra bits": after emitting the Huffman code for a length
// symbol, a few raw bits specify the exact length within the symbol's range.
// This shrinks the length alphabet from 253 symbols (3–255) to 28 symbols.
//
// # Distance Codes
//
// Similarly, the 4096 possible offsets are grouped into 24 distance codes,
// each with extra bits to specify the exact offset within its range.
//
// # Wire Format (CMP05)
//
//	[4B] original_length    big-endian uint32
//	[2B] ll_entry_count     big-endian uint16
//	[2B] dist_entry_count   big-endian uint16 (0 if no matches)
//	[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8), sorted
//	[dist_entry_count × 3B] same format
//	[remaining bytes]       LSB-first packed bit stream
//
// # Series
//
//	CMP00 (LZ77,    1977) — Sliding-window backreferences.
//	CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//	CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//	CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//	CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.  (this package)
package deflate

import (
	"encoding/binary"
	"fmt"
	"sort"

	huffmantree "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lzss"
)

// ---------------------------------------------------------------------------
// Length code table (LL symbols 257–284)
// ---------------------------------------------------------------------------
//
// Each length symbol covers a range of match lengths. The exact length within
// the range is encoded as extra_bits raw bits after the Huffman code.
//
// Example:
//   length=13 → symbol 266 (base=13, extra=1, extra_value=0 → bit "0")
//   length=14 → symbol 266 (base=13, extra=1, extra_value=1 → bit "1")

type lengthEntry struct {
	symbol    int
	base      int
	extraBits int
}

var lengthTable = []lengthEntry{
	{257, 3, 0}, {258, 4, 0}, {259, 5, 0}, {260, 6, 0},
	{261, 7, 0}, {262, 8, 0}, {263, 9, 0}, {264, 10, 0},
	{265, 11, 1}, {266, 13, 1}, {267, 15, 1}, {268, 17, 1},
	{269, 19, 2}, {270, 23, 2}, {271, 27, 2}, {272, 31, 2},
	{273, 35, 3}, {274, 43, 3}, {275, 51, 3}, {276, 59, 3},
	{277, 67, 4}, {278, 83, 4}, {279, 99, 4}, {280, 115, 4},
	{281, 131, 5}, {282, 163, 5}, {283, 195, 5}, {284, 227, 5},
}

// lengthBase[symbol] and lengthExtra[symbol] for fast O(1) lookup.
var lengthBase = map[int]int{}
var lengthExtra = map[int]int{}

// ---------------------------------------------------------------------------
// Distance code table (codes 0–23)
// ---------------------------------------------------------------------------

type distEntry struct {
	code      int
	base      int
	extraBits int
}

var distTable = []distEntry{
	{0, 1, 0}, {1, 2, 0}, {2, 3, 0}, {3, 4, 0},
	{4, 5, 1}, {5, 7, 1}, {6, 9, 2}, {7, 13, 2},
	{8, 17, 3}, {9, 25, 3}, {10, 33, 4}, {11, 49, 4},
	{12, 65, 5}, {13, 97, 5}, {14, 129, 6}, {15, 193, 6},
	{16, 257, 7}, {17, 385, 7}, {18, 513, 8}, {19, 769, 8},
	{20, 1025, 9}, {21, 1537, 9}, {22, 2049, 10}, {23, 3073, 10},
}

var distBase = map[int]int{}
var distExtra = map[int]int{}

func init() {
	// Populate fast-lookup maps from the tables.
	for _, e := range lengthTable {
		lengthBase[e.symbol] = e.base
		lengthExtra[e.symbol] = e.extraBits
	}
	for _, e := range distTable {
		distBase[e.code] = e.base
		distExtra[e.code] = e.extraBits
	}
}

// ---------------------------------------------------------------------------
// Helper: length symbol lookup
// ---------------------------------------------------------------------------

// lengthSymbol maps a match length (3–255) to the LL alphabet symbol (257–284).
//
// We scan the table and return the first symbol whose range [base, base+2^extra-1]
// contains the target length.
func lengthSymbol(length int) int {
	for _, e := range lengthTable {
		maxLen := e.base + (1<<e.extraBits) - 1
		if length <= maxLen {
			return e.symbol
		}
	}
	return 284 // maximum symbol
}

// distCode maps an offset (1–4096) to a distance code (0–23).
func distCodeFor(offset int) int {
	for _, e := range distTable {
		maxDist := e.base + (1<<e.extraBits) - 1
		if offset <= maxDist {
			return e.code
		}
	}
	return 23 // maximum code
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

// bitBuilder accumulates bits into a packed byte slice, LSB-first.
type bitBuilder struct {
	buf    uint64
	bitPos uint
	out    []byte
}

// writeBitString appends the bit characters in s to the builder, LSB-first.
func (b *bitBuilder) writeBitString(s string) {
	for i := 0; i < len(s); i++ {
		if s[i] == '1' {
			b.buf |= uint64(1) << b.bitPos
		}
		b.bitPos++
		if b.bitPos == 64 {
			for j := 0; j < 8; j++ {
				b.out = append(b.out, byte(b.buf&0xFF))
				b.buf >>= 8
			}
			b.bitPos = 0
		}
	}
}

// writeRawBitsLSB writes n raw bits from val, LSB-first.
// Bit 0 of val is emitted first, then bit 1, etc.
func (b *bitBuilder) writeRawBitsLSB(val, n int) {
	for i := 0; i < n; i++ {
		if (val>>i)&1 == 1 {
			b.buf |= uint64(1) << b.bitPos
		}
		b.bitPos++
		if b.bitPos == 64 {
			for j := 0; j < 8; j++ {
				b.out = append(b.out, byte(b.buf&0xFF))
				b.buf >>= 8
			}
			b.bitPos = 0
		}
	}
}

// flush writes any remaining bits in buf into out, zero-padding the final byte.
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
func (b *bitBuilder) bytes() []byte { return b.out }

// unpackBits reads the packed byte slice and returns '0'/'1' chars, LSB-first.
func unpackBits(data []byte) string {
	buf := make([]byte, len(data)*8)
	pos := 0
	for _, by := range data {
		for bit := uint(0); bit < 8; bit++ {
			if (by>>bit)&1 == 1 {
				buf[pos] = '1'
			} else {
				buf[pos] = '0'
			}
			pos++
		}
	}
	return string(buf)
}

// ---------------------------------------------------------------------------
// Canonical code reconstruction
// ---------------------------------------------------------------------------

type symbolLen struct {
	symbol  int
	codeLen int
}

// buildCanonicalCodes reconstructs a symbol→bitString map from sorted pairs.
func buildCanonicalCodes(pairs []symbolLen) map[int]string {
	result := make(map[int]string, len(pairs))
	if len(pairs) == 0 {
		return result
	}
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

// reverseMap inverts a symbol→bitString map to bitString→symbol.
func reverseMap(m map[int]string) map[string]int {
	r := make(map[string]int, len(m))
	for sym, bits := range m {
		r[bits] = sym
	}
	return r
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Compress compresses data using DEFLATE and returns CMP05 wire-format bytes.
//
// Algorithm:
//  1. LZSS tokenization (window=4096, max_match=255, min_match=3).
//  2. Count LL and distance symbol frequencies.
//  3. Build canonical Huffman trees via DT27.
//  4. Encode token stream to LSB-first bit stream.
//  5. Assemble header + LL table + dist table + bit stream.
//
// Edge cases:
//   - Empty input: returns 8-byte header with a single end-of-data symbol.
//   - No matches: dist_entry_count=0, no dist table emitted.
func Compress(data []byte) ([]byte, error) {
	originalLength := len(data)

	if originalLength == 0 {
		// Empty input: LL tree has only symbol 256 (end-of-data).
		out := make([]byte, 0, 12)
		hdr := make([]byte, 8)
		binary.BigEndian.PutUint32(hdr[0:4], 0)
		binary.BigEndian.PutUint16(hdr[4:6], 1) // ll_entry_count = 1
		binary.BigEndian.PutUint16(hdr[6:8], 0) // dist_entry_count = 0
		out = append(out, hdr...)
		// LL table: symbol=256, code_length=1
		out = append(out, 0x01, 0x00, 0x01)
		// Bit stream: code "0" → 1 bit → 0x00
		out = append(out, 0x00)
		return out, nil
	}

	// ── Pass 1: LZSS tokenization ────────────────────────────────────────────
	tokens := lzss.Encode(data, 4096, 255, 3)

	// ── Pass 2a: Tally frequencies ───────────────────────────────────────────
	llFreq := map[int]int{}
	distFreq := map[int]int{}

	for _, tok := range tokens {
		if tok.Kind == lzss.KindLiteral {
			llFreq[int(tok.Byte)]++
		} else {
			sym := lengthSymbol(int(tok.Length))
			llFreq[sym]++
			dc := distCodeFor(int(tok.Offset))
			distFreq[dc]++
		}
	}
	llFreq[256]++ // end-of-data marker

	// ── Pass 2b: Build canonical Huffman trees ───────────────────────────────
	llWeights := make([]huffmantree.WeightPair, 0, len(llFreq))
	for sym, freq := range llFreq {
		llWeights = append(llWeights, huffmantree.WeightPair{Symbol: sym, Frequency: freq})
	}
	llTree, err := huffmantree.Build(llWeights)
	if err != nil {
		return nil, fmt.Errorf("deflate.Compress: build LL tree: %w", err)
	}
	llCodeTable := huffmantree.CanonicalCodeTable(llTree) // map[int]string

	distCodeTable := map[int]string{}
	if len(distFreq) > 0 {
		distWeights := make([]huffmantree.WeightPair, 0, len(distFreq))
		for sym, freq := range distFreq {
			distWeights = append(distWeights, huffmantree.WeightPair{Symbol: sym, Frequency: freq})
		}
		distTree, err := huffmantree.Build(distWeights)
		if err != nil {
			return nil, fmt.Errorf("deflate.Compress: build dist tree: %w", err)
		}
		distCodeTable = huffmantree.CanonicalCodeTable(distTree)
	}

	// ── Pass 2c: Encode token stream ─────────────────────────────────────────
	bb := &bitBuilder{}
	for _, tok := range tokens {
		if tok.Kind == lzss.KindLiteral {
			code, ok := llCodeTable[int(tok.Byte)]
			if !ok {
				return nil, fmt.Errorf("deflate.Compress: no LL code for literal %d", tok.Byte)
			}
			bb.writeBitString(code)
		} else {
			sym := lengthSymbol(int(tok.Length))
			code, ok := llCodeTable[sym]
			if !ok {
				return nil, fmt.Errorf("deflate.Compress: no LL code for length symbol %d", sym)
			}
			bb.writeBitString(code)
			// Extra bits for exact length (LSB-first).
			extra := lengthExtra[sym]
			extraVal := int(tok.Length) - lengthBase[sym]
			bb.writeRawBitsLSB(extraVal, extra)

			dc := distCodeFor(int(tok.Offset))
			dcode, ok := distCodeTable[dc]
			if !ok {
				return nil, fmt.Errorf("deflate.Compress: no dist code for code %d", dc)
			}
			bb.writeBitString(dcode)
			// Extra bits for exact distance (LSB-first).
			dextra := distExtra[dc]
			dextraVal := int(tok.Offset) - distBase[dc]
			bb.writeRawBitsLSB(dextraVal, dextra)
		}
	}
	// End-of-data symbol.
	eodCode, ok := llCodeTable[256]
	if !ok {
		return nil, fmt.Errorf("deflate.Compress: no LL code for end-of-data (256)")
	}
	bb.writeBitString(eodCode)
	bb.flush()
	packedBits := bb.bytes()

	// ── Assemble wire format ─────────────────────────────────────────────────
	// Build sorted (symbol, code_length) lists.
	llPairs := make([]symbolLen, 0, len(llCodeTable))
	for sym, code := range llCodeTable {
		llPairs = append(llPairs, symbolLen{symbol: sym, codeLen: len(code)})
	}
	sort.Slice(llPairs, func(i, j int) bool {
		if llPairs[i].codeLen != llPairs[j].codeLen {
			return llPairs[i].codeLen < llPairs[j].codeLen
		}
		return llPairs[i].symbol < llPairs[j].symbol
	})

	distPairs := make([]symbolLen, 0, len(distCodeTable))
	for sym, code := range distCodeTable {
		distPairs = append(distPairs, symbolLen{symbol: sym, codeLen: len(code)})
	}
	sort.Slice(distPairs, func(i, j int) bool {
		if distPairs[i].codeLen != distPairs[j].codeLen {
			return distPairs[i].codeLen < distPairs[j].codeLen
		}
		return distPairs[i].symbol < distPairs[j].symbol
	})

	// Header: original_length (4B) + ll_entry_count (2B) + dist_entry_count (2B).
	out := make([]byte, 0, 8+3*len(llPairs)+3*len(distPairs)+len(packedBits))
	hdr := make([]byte, 8)
	binary.BigEndian.PutUint32(hdr[0:4], uint32(originalLength))
	binary.BigEndian.PutUint16(hdr[4:6], uint16(len(llPairs)))
	binary.BigEndian.PutUint16(hdr[6:8], uint16(len(distPairs)))
	out = append(out, hdr...)

	for _, p := range llPairs {
		entry := make([]byte, 3)
		binary.BigEndian.PutUint16(entry[0:2], uint16(p.symbol))
		entry[2] = byte(p.codeLen)
		out = append(out, entry...)
	}
	for _, p := range distPairs {
		entry := make([]byte, 3)
		binary.BigEndian.PutUint16(entry[0:2], uint16(p.symbol))
		entry[2] = byte(p.codeLen)
		out = append(out, entry...)
	}
	out = append(out, packedBits...)
	return out, nil
}

// Decompress decompresses CMP05 wire-format data and returns the original bytes.
//
// Algorithm:
//  1. Parse 8-byte header.
//  2. Parse LL and dist code-length tables.
//  3. Reconstruct canonical codes.
//  4. Unpack LSB-first bit stream.
//  5. Decode: literals go to output; length symbols trigger a copy from
//     output[-offset] for length bytes (byte-by-byte for overlap safety).
//     Stop at end-of-data symbol (256).
func Decompress(data []byte) ([]byte, error) {
	if len(data) < 8 {
		return nil, fmt.Errorf("deflate.Decompress: data too short (%d bytes)", len(data))
	}

	originalLength := int(binary.BigEndian.Uint32(data[0:4]))
	llEntryCount := int(binary.BigEndian.Uint16(data[4:6]))
	distEntryCount := int(binary.BigEndian.Uint16(data[6:8]))

	if originalLength == 0 {
		return []byte{}, nil
	}

	off := 8

	// Parse LL code-length table.
	llPairs := make([]symbolLen, llEntryCount)
	for i := 0; i < llEntryCount; i++ {
		if off+3 > len(data) {
			return nil, fmt.Errorf("deflate.Decompress: LL table truncated at entry %d", i)
		}
		sym := int(binary.BigEndian.Uint16(data[off : off+2]))
		clen := int(data[off+2])
		llPairs[i] = symbolLen{symbol: sym, codeLen: clen}
		off += 3
	}

	// Parse dist code-length table.
	distPairs := make([]symbolLen, distEntryCount)
	for i := 0; i < distEntryCount; i++ {
		if off+3 > len(data) {
			return nil, fmt.Errorf("deflate.Decompress: dist table truncated at entry %d", i)
		}
		sym := int(binary.BigEndian.Uint16(data[off : off+2]))
		clen := int(data[off+2])
		distPairs[i] = symbolLen{symbol: sym, codeLen: clen}
		off += 3
	}

	// Reconstruct canonical codes.
	llCodeMap := buildCanonicalCodes(llPairs)
	distCodeMap := buildCanonicalCodes(distPairs)
	llRevMap := reverseMap(llCodeMap)
	distRevMap := reverseMap(distCodeMap)

	// Unpack bit stream.
	bits := unpackBits(data[off:])
	bitPos := 0

	readBits := func(n int) int {
		val := 0
		for i := 0; i < n; i++ {
			if bits[bitPos+i] == '1' {
				val |= 1 << i
			}
		}
		bitPos += n
		return val
	}

	nextHuffmanSymbol := func(revMap map[string]int) (int, error) {
		acc := ""
		for {
			if bitPos >= len(bits) {
				return 0, fmt.Errorf("deflate.Decompress: bit stream exhausted")
			}
			acc += string(bits[bitPos])
			bitPos++
			if sym, ok := revMap[acc]; ok {
				return sym, nil
			}
		}
	}

	// Decode token stream.
	output := make([]byte, 0, originalLength)
	for {
		llSym, err := nextHuffmanSymbol(llRevMap)
		if err != nil {
			return nil, err
		}

		if llSym == 256 {
			break // end-of-data
		} else if llSym < 256 {
			output = append(output, byte(llSym))
		} else {
			// Length code.
			extra := lengthExtra[llSym]
			length := lengthBase[llSym] + readBits(extra)

			distSym, err := nextHuffmanSymbol(distRevMap)
			if err != nil {
				return nil, err
			}
			dextra := distExtra[distSym]
			distOffset := distBase[distSym] + readBits(dextra)

			// Copy byte-by-byte (supports overlapping matches).
			start := len(output) - distOffset
			for i := 0; i < length; i++ {
				output = append(output, output[start+i])
			}
		}
	}

	return output, nil
}
