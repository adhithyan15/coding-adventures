// Package brotli implements CMP06: Brotli lossless compression (2013, RFC 7932).
//
// Brotli was developed at Google by Jyrki Alakuijärvi and Zoltán Szabadka,
// initially as a web-font compression format (.woff2) and later extended for
// general-purpose HTTP response compression. It became the standard for HTTP
// "Content-Encoding: br" and is typically 15–25% smaller than gzip on HTML.
//
// # Three Key Innovations over DEFLATE (CMP05)
//
//  1. Context-dependent literal trees — Instead of one Huffman tree for all
//     literals, Brotli assigns each literal to one of 4 context buckets based
//     on the preceding byte. Each bucket gets its own Huffman tree, exploiting
//     the fact that "h after t" is far more probable than "x after t".
//
//  2. Insert-and-copy commands — Instead of DEFLATE's flat stream of literal
//     and back-reference tokens, Brotli uses commands that bundle an insert
//     run (raw literals) with a copy operation (back-reference). Both lengths
//     are encoded together as a single ICC Huffman symbol, saving overhead.
//
//  3. Larger sliding window — 65535 bytes instead of DEFLATE's 4096, allowing
//     matches across much longer distances in large documents.
//
// # Context Buckets (CodingAdventures 4-bucket model)
//
// Full RFC 7932 uses 64 context buckets. CMP06 uses 4 buckets based on the
// last emitted byte:
//
//	Bucket 0 — last byte is space or punctuation (or no previous byte)
//	Bucket 1 — last byte is a digit ('0'–'9')
//	Bucket 2 — last byte is uppercase ('A'–'Z')
//	Bucket 3 — last byte is lowercase ('a'–'z')
//
// # Insert-and-Copy Commands
//
// Every command has three parts:
//
//	Command { insert_length, copy_length, copy_distance, literals[] }
//
// The pair (insert_length, copy_length) is encoded as a single ICC symbol
// (0–62) chosen from the ICC table. ICC code 63 is the end-of-data sentinel.
//
// # Encoding Order
//
// Each non-flush command in the bit stream:
//
//	[ICC code] [insert_extras (LSB-first)] [copy_extras (LSB-first)]
//	[insert_length literal bytes, each via per-context Huffman tree]
//	[distance code] [dist_extras (LSB-first)]
//
// The bit stream ends with the sentinel ICC code (63), which may be followed
// by "flush literal" bytes if the input did not end on a copy boundary:
//
//	[... last real command ...] [ICC=63] [flush literal bytes, if any]
//
// This design allows pure-literal inputs (no LZ matches) to be encoded
// correctly: the sentinel terminates the command loop, then the decompressor
// reads any remaining literals up to original_length.
//
// # Wire Format
//
//	Header (10 bytes):
//	  [4B] original_length    big-endian uint32
//	  [1B] icc_entry_count    uint8 (1–64)
//	  [1B] dist_entry_count   uint8 (0–32)
//	  [1B] ctx0_entry_count   uint8
//	  [1B] ctx1_entry_count   uint8
//	  [1B] ctx2_entry_count   uint8
//	  [1B] ctx3_entry_count   uint8
//	ICC code-length table  (icc_entry_count × 2 bytes: symbol uint8, len uint8)
//	Dist code-length table (dist_entry_count × 2 bytes: symbol uint8, len uint8)
//	Literal tree 0 table   (ctx0_entry_count × 3 bytes: symbol uint16 BE, len uint8)
//	Literal tree 1 table   (ctx1_entry_count × 3 bytes)
//	Literal tree 2 table   (ctx2_entry_count × 3 bytes)
//	Literal tree 3 table   (ctx3_entry_count × 3 bytes)
//	Bit stream (LSB-first packed bits, zero-padded to byte boundary)
//
// # Series
//
//	CMP00 (LZ77,    1977) — Sliding-window backreferences.
//	CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//	CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//	CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//	CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//	CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//	CMP06 (Brotli,  2013) — Context modeling + insert-copy + large window. (this package)
package brotli

import (
	"encoding/binary"
	"fmt"
	"sort"

	huffmantree "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree"
)

// ---------------------------------------------------------------------------
// ICC (Insert-Copy Code) table
// ---------------------------------------------------------------------------
//
// Each Brotli command encodes (insert_length, copy_length) together as a
// single ICC symbol.  The exact lengths are further refined by extra bits.
//
// Reading the table:
//   insert_length = insert_base + extra_bits_value  (insert_extra bits)
//   copy_length   = copy_base   + extra_bits_value  (copy_extra bits)
//
// ICC code 63 is special: insert_base=0, copy_base=0 → end-of-data sentinel.
//
// Illustration — ICC code 22:
//   insert_base=1, insert_extra=0 → insert_length = 1  (exact)
//   copy_base=18,  copy_extra=2   → copy_length ∈ {18,19,20,21}

type iccEntry struct {
	insertBase  int
	insertExtra int
	copyBase    int
	copyExtra   int
}

// iccTable holds all 64 ICC codes as defined in CMP06.
//
// Codes 0–15:   insert=0, copy lengths 4–514+
// Codes 16–23:  insert=1, copy lengths 4–26+
// Codes 24–31:  insert=2, copy lengths 4–26+
// Codes 32–39:  insert=3±1, copy lengths 4–26+
// Codes 40–47:  insert=5±2, copy lengths 4–26+
// Codes 48–55:  insert=9±3, copy lengths 4–26+
// Codes 56–62:  insert=17±4, copy lengths 4–26+
// Code  63:     end-of-data sentinel (insert=0, copy=0)
var iccTable = [64]iccEntry{
	// codes 0–15: insert_base=0, insert_extra=0
	{0, 0, 4, 0},   // 0
	{0, 0, 5, 0},   // 1
	{0, 0, 6, 0},   // 2
	{0, 0, 8, 1},   // 3
	{0, 0, 10, 1},  // 4
	{0, 0, 14, 2},  // 5
	{0, 0, 18, 2},  // 6
	{0, 0, 26, 3},  // 7
	{0, 0, 34, 3},  // 8
	{0, 0, 50, 4},  // 9
	{0, 0, 66, 4},  // 10
	{0, 0, 98, 5},  // 11
	{0, 0, 130, 5}, // 12
	{0, 0, 194, 6}, // 13
	{0, 0, 258, 7}, // 14
	{0, 0, 514, 8}, // 15

	// codes 16–23: insert_base=1, insert_extra=0
	{1, 0, 4, 0},  // 16
	{1, 0, 5, 0},  // 17
	{1, 0, 6, 0},  // 18
	{1, 0, 8, 1},  // 19
	{1, 0, 10, 1}, // 20
	{1, 0, 14, 2}, // 21
	{1, 0, 18, 2}, // 22
	{1, 0, 26, 3}, // 23

	// codes 24–31: insert_base=2, insert_extra=0
	{2, 0, 4, 0},  // 24
	{2, 0, 5, 0},  // 25
	{2, 0, 6, 0},  // 26
	{2, 0, 8, 1},  // 27
	{2, 0, 10, 1}, // 28
	{2, 0, 14, 2}, // 29
	{2, 0, 18, 2}, // 30
	{2, 0, 26, 3}, // 31

	// codes 32–39: insert_base=3, insert_extra=1
	{3, 1, 4, 0},  // 32
	{3, 1, 5, 0},  // 33
	{3, 1, 6, 0},  // 34
	{3, 1, 8, 1},  // 35
	{3, 1, 10, 1}, // 36
	{3, 1, 14, 2}, // 37
	{3, 1, 18, 2}, // 38
	{3, 1, 26, 3}, // 39

	// codes 40–47: insert_base=5, insert_extra=2
	{5, 2, 4, 0},  // 40
	{5, 2, 5, 0},  // 41
	{5, 2, 6, 0},  // 42
	{5, 2, 8, 1},  // 43
	{5, 2, 10, 1}, // 44
	{5, 2, 14, 2}, // 45
	{5, 2, 18, 2}, // 46
	{5, 2, 26, 3}, // 47

	// codes 48–55: insert_base=9, insert_extra=3
	{9, 3, 4, 0},  // 48
	{9, 3, 5, 0},  // 49
	{9, 3, 6, 0},  // 50
	{9, 3, 8, 1},  // 51
	{9, 3, 10, 1}, // 52
	{9, 3, 14, 2}, // 53
	{9, 3, 18, 2}, // 54
	{9, 3, 26, 3}, // 55

	// codes 56–62: insert_base=17, insert_extra=4
	{17, 4, 4, 0},  // 56
	{17, 4, 5, 0},  // 57
	{17, 4, 6, 0},  // 58
	{17, 4, 8, 1},  // 59
	{17, 4, 10, 1}, // 60
	{17, 4, 14, 2}, // 61
	{17, 4, 18, 2}, // 62

	// code 63: end-of-data sentinel
	{0, 0, 0, 0}, // 63
}

// ---------------------------------------------------------------------------
// Distance code table (codes 0–31)
// ---------------------------------------------------------------------------
//
// Distance codes group the 65535 possible offsets into 32 ranges.
// Codes 0–23 are identical to CMP05/DEFLATE. Codes 24–31 extend to 65535.
//
// offset = dist_base[code] + read(extra_bits) raw LSB-first bits

type distEntry struct {
	code      int
	base      int
	extraBits int
}

// brotliDistTable covers offsets 1–65535 using 32 codes.
//
// Codes 0–23 mirror DEFLATE's distance table (offsets 1–4096).
// Codes 24–31 are new in CMP06, extending the window to 65535.
var brotliDistTable = []distEntry{
	{0, 1, 0}, {1, 2, 0}, {2, 3, 0}, {3, 4, 0},
	{4, 5, 1}, {5, 7, 1}, {6, 9, 2}, {7, 13, 2},
	{8, 17, 3}, {9, 25, 3}, {10, 33, 4}, {11, 49, 4},
	{12, 65, 5}, {13, 97, 5}, {14, 129, 6}, {15, 193, 6},
	{16, 257, 7}, {17, 385, 7}, {18, 513, 8}, {19, 769, 8},
	{20, 1025, 9}, {21, 1537, 9}, {22, 2049, 10}, {23, 3073, 10},
	{24, 4097, 11}, {25, 6145, 11}, {26, 8193, 12}, {27, 12289, 12},
	{28, 16385, 13}, {29, 24577, 13}, {30, 32769, 14}, {31, 49153, 14},
}

// distBase and distExtra are fast O(1) lookups from code → (base, extraBits).
var distBase = map[int]int{}
var distExtra = map[int]int{}

func init() {
	for _, e := range brotliDistTable {
		distBase[e.code] = e.base
		distExtra[e.code] = e.extraBits
	}
}

// distCodeFor maps an offset (1–65535) to the smallest distance code whose
// range contains the offset.
//
// Distance codes use ranges:
//
//	code 0 → offset 1           (exact)
//	code 1 → offset 2           (exact)
//	code 4 → offsets 5–6        (2 values, 1 extra bit)
//	code 24 → offsets 4097–6144 (2048 values, 11 extra bits)
func distCodeFor(offset int) int {
	for _, e := range brotliDistTable {
		maxDist := e.base + (1<<e.extraBits) - 1
		if offset <= maxDist {
			return e.code
		}
	}
	return 31 // fallback: largest code
}

// ---------------------------------------------------------------------------
// ICC code lookup helpers
// ---------------------------------------------------------------------------

// findBestICCCopy finds the largest copy_length ≤ requested that has a valid
// ICC code for the given insert_length.
//
// The ICC table has gaps in copy-length coverage (e.g., copy=7 is not
// representable for any code). This returns the largest encodable copy ≤
// requested for the given insert length.
//
// Examples:
//
//	findBestICCCopy(0, 4)   → 4   (exact match)
//	findBestICCCopy(0, 7)   → 6   (best below the gap at 7)
//	findBestICCCopy(0, 258) → 258 (exact match)
func findBestICCCopy(insertLen, copyLen int) int {
	best := 0
	for code := 0; code < 63; code++ {
		e := iccTable[code]
		maxIns := e.insertBase + (1<<e.insertExtra) - 1
		if insertLen < e.insertBase || insertLen > maxIns {
			continue
		}
		copyMax := e.copyBase + (1<<e.copyExtra) - 1
		if e.copyBase <= copyLen && copyLen <= copyMax {
			return copyLen // exact match
		}
		if copyMax <= copyLen && copyMax > best {
			best = copyMax
		}
	}
	if best < minMatch {
		return minMatch
	}
	return best
}

// iccCodeFor finds the ICC code (0–62) that covers the given
// (insertLength, copyLength) pair.
//
// Precondition: caller must ensure (insertLength, copyLength) fits in some
// ICC code — use findBestICCCopy() to clamp the copy length first.
func iccCodeFor(insertLen, copyLen int) int {
	for code := 0; code < 63; code++ {
		e := iccTable[code]
		maxInsert := e.insertBase + (1<<e.insertExtra) - 1
		maxCopy := e.copyBase + (1<<e.copyExtra) - 1
		if insertLen >= e.insertBase && insertLen <= maxInsert &&
			copyLen >= e.copyBase && copyLen <= maxCopy {
			return code
		}
	}
	// Fallback: copy-only code (insert=0) for this copy_length.
	for code := 0; code < 16; code++ {
		e := iccTable[code]
		maxCopy := e.copyBase + (1<<e.copyExtra) - 1
		if copyLen >= e.copyBase && copyLen <= maxCopy {
			return code
		}
	}
	return 0
}

// ---------------------------------------------------------------------------
// Literal context function
// ---------------------------------------------------------------------------

// literalContext maps the last emitted byte (or -1 for start-of-stream) to a
// context bucket (0–3).
//
// This is the core of Brotli's context modeling. By routing each literal
// through a different Huffman tree depending on what came before it, the
// encoder can assign short codes to statistically likely continuations.
//
//	Bucket 0 — after space/punctuation (most common starter for new words)
//	Bucket 1 — after a digit (numbers usually follow numbers)
//	Bucket 2 — after uppercase (PascalCase, ACRONYMS)
//	Bucket 3 — after lowercase (the most common case in prose)
//
// At the start of the stream (p1 < 0), bucket 0 is used.
func literalContext(p1 int) int {
	if p1 >= 0x61 && p1 <= 0x7A { // 'a'–'z'
		return 3
	}
	if p1 >= 0x41 && p1 <= 0x5A { // 'A'–'Z'
		return 2
	}
	if p1 >= 0x30 && p1 <= 0x39 { // '0'–'9'
		return 1
	}
	return 0 // space/punct or p1 < 0
}

// ---------------------------------------------------------------------------
// LZ matching
// ---------------------------------------------------------------------------

const (
	windowSize  = 65535 // maximum sliding window size (bytes)
	minMatch    = 4     // minimum copy length (bytes)
	maxMatchLen = 258   // maximum copy length (bytes)
)

// maxInsertPerICC is the maximum insert length encodable by any ICC code.
// Codes 56–62: insert_base=17, insert_extra=4 → max = 17 + (1<<4) - 1 = 32.
const maxInsertPerICC = 32

// findLongestMatch scans backwards from pos in data to find the longest
// match of length ≥ minMatch within a window of windowSize bytes.
//
// Returns (distance, length) where:
//   - distance is the backwards offset (1 = immediately preceding byte)
//   - length is the match length in bytes
//   - Returns (0, 0) if no match of length ≥ minMatch was found.
//
// The algorithm is O(n²) in the worst case but easy to follow:
// for each candidate start position in the window, extend the match as far
// as possible, then keep the longest one found.
func findLongestMatch(data []byte, pos int) (distance, length int) {
	windowStart := pos - windowSize
	if windowStart < 0 {
		windowStart = 0
	}

	bestLen := 0
	bestOff := 0

	for start := pos - 1; start >= windowStart; start-- {
		off := pos - start

		matchLen := 0
		maxLen := len(data) - pos
		if maxLen > maxMatchLen {
			maxLen = maxMatchLen
		}
		for matchLen < maxLen && data[start+matchLen] == data[pos+matchLen] {
			matchLen++
		}

		if matchLen > bestLen {
			bestLen = matchLen
			bestOff = off
		}
	}

	if bestLen < minMatch {
		return 0, 0
	}
	return bestOff, bestLen
}

// ---------------------------------------------------------------------------
// Brotli command
// ---------------------------------------------------------------------------

// command represents one Brotli insert-and-copy command.
//
// A command says: "emit insertLength literal bytes from literals[], then
// copy copyLength bytes from position (current_output_pos - copyDistance)".
//
// Regular commands have copyLength ≥ 4 (the minimum ICC copy range).
// The sentinel command (ICC 63) has insertLength=0 and copyLength=0.
//
// Trailing literals that cannot be bundled into a regular command are stored
// separately as "flush literals" and emitted AFTER the sentinel in the bit
// stream. The decoder reads these after seeing ICC=63.
type command struct {
	insertLength int
	copyLength   int
	copyDistance int
	literals     []byte
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

// bitBuilder accumulates bits into a packed byte slice, LSB-first.
//
// LSB-first packing means bit 0 goes into the least significant position
// of the first output byte, bit 1 goes next, and so on.
//
// Visual example — encoding bits 1,0,1,1,0,0,1,0:
//
//	Position:  7 6 5 4 3 2 1 0   (bit positions in byte 0)
//	Value:     0 1 0 0 1 1 0 1   = 0x4D
type bitBuilder struct {
	buf    uint64
	bitPos uint
	out    []byte
}

// writeBitString appends the characters in s (each '0' or '1') to the builder
// as bits, LSB-first (leftmost character in s → bit 0 of output).
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

// writeRawBitsLSB writes n raw bits from val, emitting bit 0 of val first.
//
// Used for extra bits in ICC and distance codes.  For example,
// copy_extra=2 and extra_value=3 (binary 11):
//
//	bit 0 of 3 = 1 → emit '1'
//	bit 1 of 3 = 1 → emit '1'
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

// flush emits any remaining bits, zero-padding the final byte.
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

// bytes returns the final packed byte slice (call flush() first).
func (b *bitBuilder) bytes() []byte { return b.out }

// unpackBits converts a packed byte slice into a string of '0'/'1' chars,
// LSB-first (bit 0 of byte 0 → first character in the result string).
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
// Canonical code reconstruction (for decompressor)
// ---------------------------------------------------------------------------

// symbolLen is a (symbol, code_length) pair used to serialize and reconstruct
// Huffman trees from the wire format.
type symbolLen struct {
	symbol  int
	codeLen int
}

// buildCanonicalCodes reconstructs a symbol→bitString map from a sorted list
// of (symbol, codeLen) pairs.
//
// Canonical Huffman assignment rule:
//  1. Sort by (code_length ASC, symbol ASC).
//  2. Assign integer codes 0, 1, 2, ... in that order.
//  3. When code_length increases by k, left-shift by k (multiply by 2^k).
//  4. Format each code as a zero-padded binary string of the right length.
//
// This standard algorithm means you only need the lengths to reconstruct the
// table — no need to transmit the tree structure itself.
func buildCanonicalCodes(pairs []symbolLen) map[int]string {
	result := make(map[int]string, len(pairs))
	if len(pairs) == 0 {
		return result
	}
	// Single-symbol tree: code is "0" (code length 1), per CMP06 spec.
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
// Used by the decompressor to look up symbols while reading the bit stream.
func reverseMap(m map[int]string) map[string]int {
	r := make(map[string]int, len(m))
	for sym, bits := range m {
		r[bits] = sym
	}
	return r
}

// ---------------------------------------------------------------------------
// Pass 1: LZ matching → commands + flush literals
// ---------------------------------------------------------------------------

// buildCommands scans data and produces a list of insert-and-copy commands
// plus any trailing "flush literals" that could not be bundled into a regular
// command.
//
// Returns:
//   - cmds: regular ICC commands (each has copyLength ≥ 4), followed by the
//     sentinel command{0, 0, 0, nil}.
//   - flushLiterals: trailing literal bytes emitted AFTER the sentinel in the
//     bit stream.
//
// This design cleanly handles pure-literal inputs (no LZ matches):
//
//	Input "ABCDE" → cmds=[sentinel], flushLiterals=[A,B,C,D,E]
//
// The LZ match is only accepted when the current insert buffer fits in a
// single ICC code (len(insertBuf) ≤ maxInsertPerICC = 32).  If the buffer
// has grown larger, we keep accumulating bytes into it; they become flush
// literals once scanning is complete.
func buildCommands(data []byte) (cmds []command, flushLiterals []byte) {
	cmds = make([]command, 0, 64)
	insertBuf := make([]byte, 0, 64)
	pos := 0
	n := len(data)

	for pos < n {
		dist, length := findLongestMatch(data, pos)

		if length >= minMatch && len(insertBuf) <= maxInsertPerICC {
			// Only take the match when insertBuf is still encodable in a single
			// ICC code (≤ 32 bytes). This prevents creating commands whose
			// insert_length exceeds the ICC table's representable range.
			//
			// The ICC table has gaps in copy-length coverage (e.g., copy=7 is
			// not representable). Find the largest encodable copy ≤ length.
			actualCopy := findBestICCCopy(len(insertBuf), length)

			litsCopy := make([]byte, len(insertBuf))
			copy(litsCopy, insertBuf)
			cmds = append(cmds, command{
				insertLength: len(insertBuf),
				copyLength:   actualCopy,
				copyDistance: dist,
				literals:     litsCopy,
			})
			insertBuf = insertBuf[:0]
			pos += actualCopy
		} else {
			insertBuf = append(insertBuf, data[pos])
			pos++
		}
	}

	// Any remaining bytes become flush literals, encoded AFTER the sentinel.
	// This handles:
	//   a. Pure-literal inputs (no LZ matches found at all).
	//   b. Trailing literals after the last LZ match.
	//   c. Bytes accumulated when insertBuf exceeded maxInsertPerICC.
	flushLiterals = make([]byte, len(insertBuf))
	copy(flushLiterals, insertBuf)

	// Sentinel: marks end of regular ICC command stream.
	cmds = append(cmds, command{0, 0, 0, nil})
	return cmds, flushLiterals
}

// ---------------------------------------------------------------------------
// Public API: Compress
// ---------------------------------------------------------------------------

// Compress compresses data using the CMP06 Brotli algorithm and returns
// the wire-format byte slice.
//
// Algorithm overview:
//
//  1. Pass 1 (LZ matching): scan input, building insert-and-copy commands
//     plus any trailing flush literals (emitted after the sentinel).
//
//  2. Pass 2a (frequency counting): replay commands to tally frequencies for
//     four literal context trees, the ICC tree, and the distance tree.
//     Also tally flush literal frequencies.
//
//  3. Pass 2b (tree building): build canonical Huffman trees via DT27.
//
//  4. Pass 2c (encoding): replay commands, emit Huffman codes + extra bits
//     into an LSB-first bit stream:
//       [regular commands] [sentinel ICC=63] [flush literals]
//
//  5. Assemble: 10-byte header + code-length tables + bit stream.
//
// Edge case — empty input: returns the spec-mandated 13-byte payload.
func Compress(data []byte) ([]byte, error) {
	originalLength := len(data)

	// ── Special case: empty input ────────────────────────────────────────────
	//
	// Per spec: Header = [0x00000000][0x01][0x00][0x00][0x00][0x00][0x00],
	// ICC table = {symbol=63, code_length=1}, bit stream = 0x00.
	if originalLength == 0 {
		out := []byte{
			0x00, 0x00, 0x00, 0x00, // original_length = 0
			0x01,       // icc_entry_count = 1
			0x00,       // dist_entry_count = 0
			0x00,       // ctx0_entry_count = 0
			0x00,       // ctx1_entry_count = 0
			0x00,       // ctx2_entry_count = 0
			0x00,       // ctx3_entry_count = 0
			63, 1,      // ICC entry: sentinel 63, code_length 1
			0x00,       // bit stream: "0" padded to byte
		}
		return out, nil
	}

	// ── Pass 1: LZ matching → commands + flush literals ──────────────────────
	cmds, flushLiterals := buildCommands(data)

	// ── Pass 2a: Tally frequencies ───────────────────────────────────────────
	//
	// We simulate the output (tracking the last emitted byte for context) while
	// counting symbol frequencies in the same order the encoder will emit them.
	litFreq := [4]map[int]int{
		make(map[int]int), make(map[int]int),
		make(map[int]int), make(map[int]int),
	}
	iccFreq := make(map[int]int)
	distFreqMap := make(map[int]int)

	// history tracks the simulated output for context (p1) tracking.
	history := make([]byte, 0, originalLength+8)
	p1 := -1 // last emitted byte value (-1 = start of stream)

	for _, cmd := range cmds {
		if cmd.copyLength == 0 {
			// Sentinel — stop tallying regular commands here.
			break
		}

		// Tally ICC and distance symbols.
		icc := iccCodeFor(cmd.insertLength, cmd.copyLength)
		iccFreq[icc]++
		dc := distCodeFor(cmd.copyDistance)
		distFreqMap[dc]++

		// Tally insert literal frequencies, routing each byte to its context.
		for _, b := range cmd.literals {
			ctx := literalContext(p1)
			litFreq[ctx][int(b)]++
			history = append(history, b)
			p1 = int(b)
		}

		// Simulate copy to advance p1.
		start := len(history) - cmd.copyDistance
		for k := 0; k < cmd.copyLength; k++ {
			history = append(history, history[start+k])
			p1 = int(history[len(history)-1])
		}
	}
	iccFreq[63]++ // sentinel always appears once

	// Tally flush literal frequencies (emitted AFTER the sentinel).
	// p1 is the last byte from the regular-command phase (or -1 if no commands).
	p1flush := p1
	for _, b := range flushLiterals {
		ctx := literalContext(p1flush)
		litFreq[ctx][int(b)]++
		p1flush = int(b)
	}

	// ── Pass 2b: Build Huffman trees ─────────────────────────────────────────

	// ICC tree.
	iccWeights := make([]huffmantree.WeightPair, 0, len(iccFreq))
	for sym, freq := range iccFreq {
		iccWeights = append(iccWeights, huffmantree.WeightPair{Symbol: sym, Frequency: freq})
	}
	iccTree, err := huffmantree.Build(iccWeights)
	if err != nil {
		return nil, fmt.Errorf("brotli.Compress: build ICC tree: %w", err)
	}
	iccCodeTable := huffmantree.CanonicalCodeTable(iccTree)

	// Distance tree.
	distCodeTable := map[int]string{}
	if len(distFreqMap) > 0 {
		distWeights := make([]huffmantree.WeightPair, 0, len(distFreqMap))
		for sym, freq := range distFreqMap {
			distWeights = append(distWeights, huffmantree.WeightPair{Symbol: sym, Frequency: freq})
		}
		distTree, err := huffmantree.Build(distWeights)
		if err != nil {
			return nil, fmt.Errorf("brotli.Compress: build dist tree: %w", err)
		}
		distCodeTable = huffmantree.CanonicalCodeTable(distTree)
	}

	// Four literal context trees.
	litCodeTables := [4]map[int]string{{}, {}, {}, {}}
	for ctx := 0; ctx < 4; ctx++ {
		if len(litFreq[ctx]) == 0 {
			continue
		}
		weights := make([]huffmantree.WeightPair, 0, len(litFreq[ctx]))
		for sym, freq := range litFreq[ctx] {
			weights = append(weights, huffmantree.WeightPair{Symbol: sym, Frequency: freq})
		}
		tree, err := huffmantree.Build(weights)
		if err != nil {
			return nil, fmt.Errorf("brotli.Compress: build literal tree %d: %w", ctx, err)
		}
		litCodeTables[ctx] = huffmantree.CanonicalCodeTable(tree)
	}

	// ── Pass 2c: Encode ──────────────────────────────────────────────────────
	//
	// The bit stream order for each regular command:
	//
	//   1. ICC symbol (Huffman code for the (insert_length, copy_length) pair)
	//   2. insert_extra bits (raw LSB-first; selects exact insert_length in range)
	//   3. copy_extra bits  (raw LSB-first; selects exact copy_length in range)
	//   4. insert_length × literal symbols (Huffman-coded per context bucket)
	//   5. dist symbol
	//   6. dist_extra bits (raw LSB-first)
	//
	// End of stream:
	//   7. [ICC=63] (sentinel Huffman code)
	//   8. flush literals (if any), encoded the same way as regular literals

	bb := &bitBuilder{}
	encHistory := make([]byte, 0, originalLength+8)
	encP1 := -1 // last emitted byte for encoder context tracking

	for _, cmd := range cmds {
		if cmd.copyLength == 0 {
			// Sentinel — emit ICC=63, then flush literals.
			sentCode, ok := iccCodeTable[63]
			if !ok {
				return nil, fmt.Errorf("brotli.Compress: no code for ICC sentinel 63")
			}
			bb.writeBitString(sentCode)

			// Emit flush literals after the sentinel.
			for _, b := range flushLiterals {
				ctx := literalContext(encP1)
				code, ok := litCodeTables[ctx][int(b)]
				if !ok {
					return nil, fmt.Errorf("brotli.Compress: no literal code for byte %d in ctx %d", b, ctx)
				}
				bb.writeBitString(code)
				encP1 = int(b)
			}
			break
		}

		// 1. Encode ICC symbol.
		icc := iccCodeFor(cmd.insertLength, cmd.copyLength)
		e := iccTable[icc]

		iccCode, ok := iccCodeTable[icc]
		if !ok {
			return nil, fmt.Errorf("brotli.Compress: no code for ICC %d", icc)
		}
		bb.writeBitString(iccCode)

		// 2. Insert extra bits.
		bb.writeRawBitsLSB(cmd.insertLength-e.insertBase, e.insertExtra)

		// 3. Copy extra bits.
		bb.writeRawBitsLSB(cmd.copyLength-e.copyBase, e.copyExtra)

		// 4. Literal symbols (each routed through per-context Huffman tree).
		for _, b := range cmd.literals {
			ctx := literalContext(encP1)
			code, ok := litCodeTables[ctx][int(b)]
			if !ok {
				return nil, fmt.Errorf("brotli.Compress: no literal code for byte %d in ctx %d", b, ctx)
			}
			bb.writeBitString(code)
			encHistory = append(encHistory, b)
			encP1 = int(b)
		}

		// 5+6. Distance symbol + extra bits.
		dc := distCodeFor(cmd.copyDistance)
		dCode, ok := distCodeTable[dc]
		if !ok {
			return nil, fmt.Errorf("brotli.Compress: no code for dist code %d", dc)
		}
		bb.writeBitString(dCode)
		bb.writeRawBitsLSB(cmd.copyDistance-distBase[dc], distExtra[dc])

		// Simulate copy for context tracking.
		start := len(encHistory) - cmd.copyDistance
		for k := 0; k < cmd.copyLength; k++ {
			encHistory = append(encHistory, encHistory[start+k])
			encP1 = int(encHistory[len(encHistory)-1])
		}
	}

	bb.flush()
	packedBits := bb.bytes()

	// ── Assemble wire format ─────────────────────────────────────────────────

	iccPairs := sortedPairs(iccCodeTable)
	distPairs := sortedPairs(distCodeTable)
	litPairs := [4][]symbolLen{}
	for ctx := 0; ctx < 4; ctx++ {
		litPairs[ctx] = sortedPairs(litCodeTables[ctx])
	}

	// 10-byte header.
	out := make([]byte, 0, 10+
		2*len(iccPairs)+2*len(distPairs)+
		3*(len(litPairs[0])+len(litPairs[1])+len(litPairs[2])+len(litPairs[3]))+
		len(packedBits))

	hdr := make([]byte, 10)
	binary.BigEndian.PutUint32(hdr[0:4], uint32(originalLength))
	hdr[4] = byte(len(iccPairs))
	hdr[5] = byte(len(distPairs))
	hdr[6] = byte(len(litPairs[0]))
	hdr[7] = byte(len(litPairs[1]))
	hdr[8] = byte(len(litPairs[2]))
	hdr[9] = byte(len(litPairs[3]))
	out = append(out, hdr...)

	// ICC table: [symbol uint8][code_length uint8].
	for _, p := range iccPairs {
		out = append(out, byte(p.symbol), byte(p.codeLen))
	}
	// Dist table: [symbol uint8][code_length uint8].
	for _, p := range distPairs {
		out = append(out, byte(p.symbol), byte(p.codeLen))
	}
	// Literal tables: [symbol uint16 BE][code_length uint8].
	for ctx := 0; ctx < 4; ctx++ {
		for _, p := range litPairs[ctx] {
			out = append(out, byte(p.symbol>>8), byte(p.symbol), byte(p.codeLen))
		}
	}

	out = append(out, packedBits...)
	return out, nil
}

// sortedPairs converts a symbol→bitString map to a slice of symbolLen pairs
// sorted by (codeLen ASC, symbol ASC) — the canonical wire order.
func sortedPairs(codeTable map[int]string) []symbolLen {
	pairs := make([]symbolLen, 0, len(codeTable))
	for sym, code := range codeTable {
		pairs = append(pairs, symbolLen{symbol: sym, codeLen: len(code)})
	}
	sort.Slice(pairs, func(i, j int) bool {
		if pairs[i].codeLen != pairs[j].codeLen {
			return pairs[i].codeLen < pairs[j].codeLen
		}
		return pairs[i].symbol < pairs[j].symbol
	})
	return pairs
}

// ---------------------------------------------------------------------------
// Public API: Decompress
// ---------------------------------------------------------------------------

// Decompress decompresses CMP06 wire-format data and returns the original bytes.
//
// Algorithm:
//  1. Parse 10-byte header (original_length + table entry counts).
//  2. Parse ICC, dist, and 4 literal code-length tables.
//  3. Reconstruct canonical Huffman codes from the stored lengths.
//  4. Unpack LSB-first bit stream.
//  5. Decode commands:
//     a. Read ICC symbol.
//     b. If ICC == 63: read flush literals until output reaches original_length.
//     c. Otherwise: read insert_length and copy_length extras, decode literals,
//        decode and execute copy.
//  6. Return output trimmed to original_length.
func Decompress(data []byte) ([]byte, error) {
	if len(data) < 10 {
		return nil, fmt.Errorf("brotli.Decompress: data too short (%d bytes)", len(data))
	}

	// ── Parse header ─────────────────────────────────────────────────────────
	originalLength := int(binary.BigEndian.Uint32(data[0:4]))
	iccEntryCount := int(data[4])
	distEntryCount := int(data[5])
	ctxEntryCounts := [4]int{int(data[6]), int(data[7]), int(data[8]), int(data[9])}

	if originalLength == 0 {
		return []byte{}, nil
	}

	off := 10

	// ── Parse ICC code-length table ──────────────────────────────────────────
	iccPairs := make([]symbolLen, iccEntryCount)
	for i := 0; i < iccEntryCount; i++ {
		if off+2 > len(data) {
			return nil, fmt.Errorf("brotli.Decompress: ICC table truncated at entry %d", i)
		}
		iccPairs[i] = symbolLen{int(data[off]), int(data[off+1])}
		off += 2
	}

	// ── Parse dist code-length table ─────────────────────────────────────────
	distPairs := make([]symbolLen, distEntryCount)
	for i := 0; i < distEntryCount; i++ {
		if off+2 > len(data) {
			return nil, fmt.Errorf("brotli.Decompress: dist table truncated at entry %d", i)
		}
		distPairs[i] = symbolLen{int(data[off]), int(data[off+1])}
		off += 2
	}

	// ── Parse four literal code-length tables ────────────────────────────────
	litPairs := [4][]symbolLen{}
	for ctx := 0; ctx < 4; ctx++ {
		count := ctxEntryCounts[ctx]
		litPairs[ctx] = make([]symbolLen, count)
		for i := 0; i < count; i++ {
			if off+3 > len(data) {
				return nil, fmt.Errorf("brotli.Decompress: literal table %d truncated at entry %d", ctx, i)
			}
			sym := int(binary.BigEndian.Uint16(data[off : off+2]))
			clen := int(data[off+2])
			litPairs[ctx][i] = symbolLen{sym, clen}
			off += 3
		}
	}

	// ── Reconstruct canonical code tables ────────────────────────────────────
	iccRevMap := reverseMap(buildCanonicalCodes(iccPairs))
	distRevMap := reverseMap(buildCanonicalCodes(distPairs))
	litRevMaps := [4]map[string]int{}
	for ctx := 0; ctx < 4; ctx++ {
		litRevMaps[ctx] = reverseMap(buildCanonicalCodes(litPairs[ctx]))
	}

	// ── Unpack bit stream ────────────────────────────────────────────────────
	bits := unpackBits(data[off:])
	bitPos := 0

	// readBitsLSB reads n bits from the stream, reconstructing the value LSB-first.
	readBitsLSB := func(n int) (int, error) {
		if bitPos+n > len(bits) {
			return 0, fmt.Errorf("brotli.Decompress: bit stream exhausted reading %d bits", n)
		}
		val := 0
		for i := 0; i < n; i++ {
			if bits[bitPos+i] == '1' {
				val |= 1 << i
			}
		}
		bitPos += n
		return val, nil
	}

	// nextSym decodes the next Huffman symbol by reading one bit at a time.
	// It appends each bit to an accumulator string and checks for a match
	// in the reverse map (bitString → symbol).  This is the standard
	// prefix-free code decoder: since Huffman codes are prefix-free, no
	// valid code is a prefix of another, so the first match is correct.
	nextSym := func(revMap map[string]int) (int, error) {
		acc := ""
		for {
			if bitPos >= len(bits) {
				return 0, fmt.Errorf("brotli.Decompress: bit stream exhausted during symbol decode")
			}
			acc += string(bits[bitPos])
			bitPos++
			if sym, ok := revMap[acc]; ok {
				return sym, nil
			}
			if len(acc) > 16 {
				return 0, fmt.Errorf("brotli.Decompress: Huffman code exceeded 16 bits")
			}
		}
	}

	// ── Decode command stream ────────────────────────────────────────────────
	output := make([]byte, 0, originalLength)
	p1 := -1 // last emitted byte value (-1 = start of stream)

	for {
		// Decode ICC symbol.
		icc, err := nextSym(iccRevMap)
		if err != nil {
			return nil, err
		}

		if icc == 63 {
			// End-of-data sentinel. Decode flush literals until we reach original_length.
			// The encoder wrote these AFTER the sentinel in the bit stream.
			for len(output) < originalLength {
				ctx := literalContext(p1)
				sym, err := nextSym(litRevMaps[ctx])
				if err != nil {
					return nil, fmt.Errorf("brotli.Decompress: decode flush literal in ctx %d: %w", ctx, err)
				}
				output = append(output, byte(sym))
				p1 = sym
			}
			break
		}

		e := iccTable[icc]

		insExtra, err := readBitsLSB(e.insertExtra)
		if err != nil {
			return nil, err
		}
		insertLength := e.insertBase + insExtra

		copyExtraBits, err := readBitsLSB(e.copyExtra)
		if err != nil {
			return nil, err
		}
		copyLength := e.copyBase + copyExtraBits

		// Decode and emit insertLength literal bytes.
		for i := 0; i < insertLength; i++ {
			ctx := literalContext(p1)
			sym, err := nextSym(litRevMaps[ctx])
			if err != nil {
				return nil, fmt.Errorf("brotli.Decompress: decode literal in ctx %d: %w", ctx, err)
			}
			output = append(output, byte(sym))
			p1 = sym
		}

		// Decode and perform copy.
		if copyLength > 0 {
			dc, err := nextSym(distRevMap)
			if err != nil {
				return nil, fmt.Errorf("brotli.Decompress: decode dist code: %w", err)
			}
			dExtra, err := readBitsLSB(distExtra[dc])
			if err != nil {
				return nil, err
			}
			copyDistance := distBase[dc] + dExtra

			start := len(output) - copyDistance
			if start < 0 {
				return nil, fmt.Errorf("brotli.Decompress: copy distance %d exceeds output length %d",
					copyDistance, len(output))
			}
			// Copy byte-by-byte to support overlapping matches (length > distance).
			// Example: output="A", distance=1, length=5 → "AAAAAA" (5 copies of 'A').
			for k := 0; k < copyLength; k++ {
				b := output[start+k]
				output = append(output, b)
				p1 = int(b)
			}
		}
	}

	// Trim to original_length (defensive; flush-literal loop stops exactly at
	// original_length, but guard against off-by-one bugs in edge cases).
	if len(output) > originalLength {
		output = output[:originalLength]
	}

	return output, nil
}
