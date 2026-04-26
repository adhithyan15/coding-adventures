// Package zstd implements the Zstandard (ZStd) lossless compression algorithm (CMP07).
//
// Zstandard (RFC 8878) is a high-ratio, fast compression format created by
// Yann Collet at Facebook (2015). It combines two powerful techniques:
//
//   - LZ77 back-references (via LZSS token generation) to exploit repetition
//     in the data — the same "copy from earlier in the output" trick as DEFLATE,
//     but with a larger 32 KB window for better ratio.
//   - FSE (Finite State Entropy) coding instead of Huffman for the sequence
//     descriptor symbols. FSE is an asymmetric numeral system that approaches the
//     Shannon entropy limit in a single pass with no precision loss.
//
// # Frame layout (RFC 8878 §3)
//
//	┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
//	│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
//	│ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
//	└────────┴─────┴──────────────────────┴────────┴──────────────────┘
//
// Each block has a 3-byte header:
//
//	bit 0      = Last_Block flag
//	bits [2:1] = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
//	bits [23:3] = Block_Size
//
// # Compression strategy (this implementation)
//
//  1. Split data into 128 KB blocks (maxBlockSize).
//  2. For each block, try:
//     a. RLE — all bytes identical → 5 bytes total (3 header + 1 byte).
//     b. Compressed (LZ77 + FSE) — if output < input length.
//     c. Raw — verbatim copy as fallback.
//
// # Series
//
//	CMP00 (LZ77)     — Sliding-window back-references
//	CMP01 (LZ78)     — Explicit dictionary (trie)
//	CMP02 (LZSS)     — LZ77 + flag bits
//	CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
//	CMP04 (Huffman)  — Entropy coding
//	CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
//	CMP06 (Brotli)   — DEFLATE + context modelling + static dict
//	CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed ← this package
//
// # Examples
//
//	data := []byte("the quick brown fox jumps over the lazy dog")
//	compressed := zstd.Compress(data)
//	original, err := zstd.Decompress(compressed)
package zstd

import (
	"encoding/binary"
	"fmt"
	"math/bits"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lzss"
)

// ─── Constants ────────────────────────────────────────────────────────────────

// magic is the ZStd frame magic number: 0xFD2FB528 (little-endian: 28 B5 2F FD).
//
// Every valid ZStd frame starts with these 4 bytes. The value was chosen to
// be unlikely to appear at the start of plaintext files.
const magic uint32 = 0xFD2FB528

// maxBlockSize is the maximum size of one ZStd block: 128 KB.
//
// ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
// blocks. The spec maximum is actually min(WindowSize, 128 KB).
const maxBlockSize = 128 * 1024

// maxOutput caps the total decompressed output at 256 MB.
//
// This prevents "decompression bomb" attacks where a tiny compressed input
// expands to gigabytes, exhausting memory. We reject any frame that would
// produce more than 256 MB of output.
const maxOutput = 256 * 1024 * 1024

// ─── LL / ML code tables (RFC 8878 §3.1.1.3) ─────────────────────────────────
//
// These tables map a *code number* to a (baseline, extraBits) pair.
//
// For example, LL code 17 means literal_length = 18 + read(1 extra bit),
// so it covers literal lengths 18 and 19.
//
// The FSE state machine tracks one code number per field; extra bits are
// read directly from the bitstream after state transitions.

// llCodes maps LL code numbers 0..35 to (baseline, extraBits).
//
// Literal length 0..15 each have their own code (0 extra bits).
// Larger lengths are grouped with increasing ranges.
var llCodes = [36][2]uint32{
	// code: value = baseline + read(extraBits)
	{0, 0}, {1, 0}, {2, 0}, {3, 0}, {4, 0}, {5, 0},
	{6, 0}, {7, 0}, {8, 0}, {9, 0}, {10, 0}, {11, 0},
	{12, 0}, {13, 0}, {14, 0}, {15, 0},
	// Grouped ranges start at code 16.
	{16, 1}, {18, 1}, {20, 1}, {22, 1},
	{24, 2}, {28, 2},
	{32, 3}, {40, 3},
	{48, 4}, {64, 6},
	{128, 7}, {256, 8}, {512, 9}, {1024, 10}, {2048, 11}, {4096, 12},
	{8192, 13}, {16384, 14}, {32768, 15}, {65536, 16},
}

// mlCodes maps ML code numbers 0..52 to (baseline, extraBits).
//
// Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
var mlCodes = [53][2]uint32{
	// codes 0..31: individual values 3..34
	{3, 0}, {4, 0}, {5, 0}, {6, 0}, {7, 0}, {8, 0},
	{9, 0}, {10, 0}, {11, 0}, {12, 0}, {13, 0}, {14, 0},
	{15, 0}, {16, 0}, {17, 0}, {18, 0}, {19, 0}, {20, 0},
	{21, 0}, {22, 0}, {23, 0}, {24, 0}, {25, 0}, {26, 0},
	{27, 0}, {28, 0}, {29, 0}, {30, 0}, {31, 0}, {32, 0},
	{33, 0}, {34, 0},
	// codes 32+: grouped ranges
	{35, 1}, {37, 1}, {39, 1}, {41, 1},
	{43, 2}, {47, 2},
	{51, 3}, {59, 3},
	{67, 4}, {83, 4},
	{99, 5}, {131, 7},
	{259, 8}, {515, 9}, {1027, 10}, {2051, 11},
	{4099, 12}, {8195, 13}, {16387, 14}, {32771, 15}, {65539, 16},
}

// ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted.
// The decoder builds the same table from these fixed distributions.
//
// Entries of -1 mean "probability 1/table_size" — these symbols get one slot
// in the decode table and their encoder state never needs extra bits.

// llNorm is the predefined normalised distribution for Literal Length FSE.
// Table accuracy log = 6 → 64 slots.
var llNorm = [36]int16{
	4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
	2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
	-1, -1, -1, -1,
}

// llAccLog is the accuracy (log2 of table size) for the LL FSE table: 2^6 = 64 slots.
const llAccLog uint8 = 6

// mlNorm is the predefined normalised distribution for Match Length FSE.
// Table accuracy log = 6 → 64 slots.
var mlNorm = [53]int16{
	1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
	1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
	1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
	-1, -1, -1, -1, -1,
}

// mlAccLog is the accuracy (log2 of table size) for the ML FSE table: 2^6 = 64 slots.
const mlAccLog uint8 = 6

// ofNorm is the predefined normalised distribution for Offset FSE.
// Table accuracy log = 5 → 32 slots.
var ofNorm = [29]int16{
	1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
	1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
}

// ofAccLog is the accuracy (log2 of table size) for the OF FSE table: 2^5 = 32 slots.
const ofAccLog uint8 = 5

// ─── FSE decode table entry ───────────────────────────────────────────────────

// fseDe is one cell in the FSE decode table.
//
// To decode a symbol from state S:
//  1. sym is the output symbol.
//  2. Read nb bits from the bitstream as bits.
//  3. New state = base + bits.
type fseDe struct {
	sym  uint8
	nb   uint8
	base uint16
}

// buildDecodeTable builds an FSE decode table from a normalised probability distribution.
//
// The algorithm:
//  1. Place symbols with probability -1 (very rare) at the top of the table.
//     These symbols each get exactly 1 slot.
//  2. Spread remaining symbols using a deterministic step function derived from
//     the table size. This ensures each symbol occupies the correct fraction of slots.
//  3. Assign nb (number of state bits to read) and base to each slot so that the
//     decoder can reconstruct the next state.
//
// The step function step = (sz >> 1) + (sz >> 3) + 3 is co-prime to sz when sz
// is a power of two (which it always is in ZStd), ensuring the walk visits every
// slot exactly once.
func buildDecodeTable(norm []int16, accLog uint8) []fseDe {
	sz := 1 << accLog // table size = 2^accLog
	step := (sz >> 1) + (sz >> 3) + 3
	tbl := make([]fseDe, sz)
	symNext := make([]uint16, len(norm))

	// Phase 1: symbols with probability -1 go at the top (high indices).
	// These symbols each get exactly 1 slot, and their state transition uses
	// the full accLog bits (they can go to any state).
	high := sz - 1
	for s, c := range norm {
		if c == -1 {
			tbl[high].sym = uint8(s)
			if high > 0 {
				high--
			}
			symNext[s] = 1
		}
	}

	// Phase 2: spread remaining symbols into the lower portion of the table.
	// Two-pass approach: first symbols with count > 1, then count == 1.
	// This matches the reference implementation's deterministic ordering.
	pos := 0
	for pass := 0; pass < 2; pass++ {
		for s, c := range norm {
			if c <= 0 {
				continue
			}
			cnt := int(c)
			if (pass == 0) != (cnt > 1) {
				continue
			}
			symNext[s] = uint16(cnt)
			for i := 0; i < cnt; i++ {
				tbl[pos].sym = uint8(s)
				pos = (pos + step) & (sz - 1)
				for pos > high {
					pos = (pos + step) & (sz - 1)
				}
			}
		}
	}

	// Phase 3: assign nb (number of state bits to read) and base.
	//
	// For a symbol with count cnt occupying slots i₀, i₁, ...:
	//   The next_state counter starts at cnt and increments.
	//   nb = accLog - floor(log2(next_state))
	//   base = next_state * (1 << nb) - sz
	//
	// This ensures that when we reconstruct state = base + read(nb bits),
	// we land in the range [sz, 2*sz), which is the valid encoder state range.
	sn := make([]uint16, len(symNext))
	copy(sn, symNext)
	for i := 0; i < sz; i++ {
		s := int(tbl[i].sym)
		ns := uint32(sn[s])
		sn[s]++
		// floor(log2(ns)) = 31 - bits.LeadingZeros32(ns)
		nb := accLog - uint8(31-bits.LeadingZeros32(ns))
		// base = ns * (1 << nb) - sz
		base := uint16((ns<<nb) - uint32(sz))
		tbl[i].nb = nb
		tbl[i].base = base
	}

	return tbl
}

// ─── FSE encode symbol table entry ───────────────────────────────────────────

// fseEe is the encode transform for one symbol.
//
// Given encoder state S for symbol s:
//
//	nbOut = (S + deltaNb) >> 16   (number of bits to emit)
//	emit low nbOut bits of S
//	new_S = st[(S >> nbOut) + deltaFs]
//
// The deltaNb and deltaFs values are precomputed from the distribution so
// the hot-path encode loop needs only arithmetic and a table lookup.
type fseEe struct {
	// (maxBitsOut << 16) - (count << maxBitsOut)
	// Used to derive nbOut: nbOut = (state + deltaNb) >> 16
	deltaNb uint32
	// cumulative_count_before_sym - count (may be negative, hence int32)
	// Used to index st: new_S = st[(S >> nbOut) + deltaFs]
	deltaFs int32
}

// buildEncodeTable builds FSE encode tables from a normalised distribution.
//
// Returns:
//   - ee[sym]: the fseEe transform for each symbol.
//   - st[slot]: the encoder state table (slot → output state in [sz, 2*sz)).
//
// The FSE decoder assigns (sym, nb, base) to each table cell in INDEX ORDER.
// For symbol s, the j-th cell (in ascending index order) has:
//
//	ns = count[s] + j
//	nb = accLog - floor(log2(ns))
//	base = ns * (1<<nb) - sz
//
// The FSE encoder must use the SAME indexing: slot cumul[s]+j maps to the j-th
// table cell for symbol s (in ascending index order).
func buildEncodeTable(norm []int16, accLog uint8) ([]fseEe, []uint16) {
	sz := uint32(1) << accLog

	// Step 1: compute cumulative sums (where each symbol starts in the state table).
	cumul := make([]uint32, len(norm))
	total := uint32(0)
	for s, c := range norm {
		cumul[s] = total
		var cnt uint32
		if c == -1 {
			cnt = 1
		} else if c > 0 {
			cnt = uint32(c)
		}
		total += cnt
	}

	// Step 2: build the spread table (which symbol occupies each table slot).
	//
	// This uses the same spreading algorithm as buildDecodeTable, producing
	// a mapping from table index to symbol.
	step := (sz >> 1) + (sz >> 3) + 3
	spread := make([]uint8, sz)
	idxHigh := int(sz) - 1

	// Phase 1: probability -1 symbols at the high end.
	for s, c := range norm {
		if c == -1 {
			spread[idxHigh] = uint8(s)
			if idxHigh > 0 {
				idxHigh--
			}
		}
	}
	idxLimit := idxHigh // highest free slot for phase-2 spread

	// Phase 2: spread remaining symbols using the step function.
	pos := 0
	for pass := 0; pass < 2; pass++ {
		for s, c := range norm {
			if c <= 0 {
				continue
			}
			cnt := int(c)
			if (pass == 0) != (cnt > 1) {
				continue
			}
			for i := 0; i < cnt; i++ {
				spread[pos] = uint8(s)
				pos = (pos + int(step)) & (int(sz) - 1)
				for pos > idxLimit {
					pos = (pos + int(step)) & (int(sz) - 1)
				}
			}
		}
	}

	// Step 3: build the state table by iterating spread in INDEX ORDER.
	//
	// For each table index i (in ascending order), determine which occurrence of
	// symbol s = spread[i] this is (j = 0, 1, 2, ...).
	// The encode slot is cumul[s] + j, and the encoder output state is i + sz
	// (so the decoder, in state i, will decode symbol s).
	symOcc := make([]uint32, len(norm))
	st := make([]uint16, sz)

	for i := uint32(0); i < sz; i++ {
		s := int(spread[i])
		j := symOcc[s]
		symOcc[s]++
		slot := cumul[s] + j
		// Encoder output state = decode table index + sz
		st[slot] = uint16(i + sz)
	}

	// Step 4: build fseEe entries.
	//
	// For symbol s with count c and maxBitsOut mbo:
	//   deltaNb = (mbo << 16) - (c << mbo)
	//   deltaFs = cumul[s] - c
	//
	// Encode step: given current encoder state E ∈ [sz, 2*sz):
	//   nb = (E + deltaNb) >> 16     (number of state bits to emit)
	//   emit low nb bits of E
	//   new_E = st[(E >> nb) + deltaFs]
	ee := make([]fseEe, len(norm))
	for s, c := range norm {
		var cnt uint32
		if c == -1 {
			cnt = 1
		} else if c > 0 {
			cnt = uint32(c)
		}
		if cnt == 0 {
			continue
		}
		var mbo uint32
		if cnt == 1 {
			mbo = uint32(accLog)
		} else {
			// maxBitsOut = accLog - floor(log2(cnt))
			mbo = uint32(accLog) - uint32(31-bits.LeadingZeros32(cnt))
		}
		ee[s].deltaNb = (mbo << 16) - (cnt << mbo)
		ee[s].deltaFs = int32(cumul[s]) - int32(cnt)
	}

	return ee, st
}

// ─── Reverse bit-writer ───────────────────────────────────────────────────────
//
// ZStd's sequence bitstream is written *backwards* relative to the data flow:
// the encoder writes bits that the decoder will read last, first. This allows
// the decoder to read a forward-only stream while decoding sequences in order.
//
// Byte layout: [byte0, byte1, ..., byteN] where byteN is the last byte written,
// and it contains a sentinel bit (the highest set bit) that marks the end of
// meaningful data. The decoder initialises by finding this sentinel.
//
// Bit layout within each byte: LSB = first bit written.
//
// Example: write bits 1, 0, 1, 1 (4 bits) then flush:
//
//	reg = 0b1011, bits = 4
//	flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
//	buf = [0x1B]
//
// The decoder reads this as: find MSB (bit 4 = sentinel), then read
// bits 3..0 = 0b1011 = the original 4 bits.

type revBitWriter struct {
	buf  []byte
	reg  uint64 // accumulation register (bits fill from LSB)
	bits uint8  // number of valid bits in reg
}

// addBits adds the nb low-order bits of val to the stream.
func (w *revBitWriter) addBits(val uint64, nb uint8) {
	if nb == 0 {
		return
	}
	var mask uint64
	if nb == 64 {
		mask = ^uint64(0)
	} else {
		mask = (uint64(1) << nb) - 1
	}
	w.reg |= (val & mask) << w.bits
	w.bits += nb
	for w.bits >= 8 {
		w.buf = append(w.buf, byte(w.reg))
		w.reg >>= 8
		w.bits -= 8
	}
}

// flush writes remaining bits with a sentinel and closes the stream.
//
// The sentinel is a 1 bit placed at position w.bits in the last byte.
// The decoder locates it with leading_zeros arithmetic.
func (w *revBitWriter) flush() {
	sentinel := byte(1) << w.bits // bit above all remaining data bits
	lastByte := byte(w.reg) | sentinel
	w.buf = append(w.buf, lastByte)
	w.reg = 0
	w.bits = 0
}

// finish returns the completed buffer.
func (w *revBitWriter) finish() []byte {
	return w.buf
}

// ─── Reverse bit-reader ───────────────────────────────────────────────────────
//
// Mirrors revBitWriter: reads bits from the END of the buffer going backwards.
// The stream is laid out so that the LAST bits written by the encoder are at the
// END of the byte buffer (in the sentinel-containing last byte). The reader
// initialises at the last byte and reads backward toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// readBits(n) extracts the top n bits and shifts the register left by n.
//
// Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
// byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
// bits first (which were in the highest byte positions and in the high bits of
// each byte), we need a left-aligned register so that reading from the top
// gives the highest-position bits first.

type revBitReader struct {
	data []byte
	reg  uint64 // shift register, valid bits packed at the TOP (MSB side)
	bits uint8  // how many valid bits are loaded (count from MSB)
	pos  int    // index of the next byte to load (decrements toward 0)
}

// newRevBitReader initialises a revBitReader from a sentinel-terminated byte slice.
func newRevBitReader(data []byte) (*revBitReader, error) {
	if len(data) == 0 {
		return nil, fmt.Errorf("empty bitstream")
	}

	// Find the sentinel bit in the last byte.
	// The sentinel is the highest set bit; valid data bits are below it.
	last := data[len(data)-1]
	if last == 0 {
		return nil, fmt.Errorf("bitstream last byte is zero (no sentinel)")
	}

	// sentinelPos = bit index (0 = LSB) of the sentinel in the last byte.
	// bits.Len(x) returns the number of bits required to represent x,
	// so bits.Len(last) - 1 is the position of the highest set bit.
	sentinelPos := uint8(bits.Len8(last)) - 1 // 0-indexed from LSB
	// validBits = number of data bits below the sentinel
	validBits := sentinelPos

	// Place the valid bits of the sentinel byte at the TOP of the register.
	// Example: last=0b00011110, sentinel at bit4, validBits=4,
	//   data bits = last & 0b1111 = 0b1110.
	//   After shifting to top: reg bit63=1, bit62=1, bit61=1, bit60=0.
	var reg uint64
	if validBits > 0 {
		mask := (uint64(1) << validBits) - 1
		reg = (uint64(last) & mask) << (64 - validBits)
	}

	r := &revBitReader{
		data: data,
		reg:  reg,
		bits: validBits,
		pos:  len(data) - 1, // sentinel byte already consumed; load from here-1
	}

	// Fill the register from earlier bytes.
	r.reload()
	return r, nil
}

// reload loads more bytes into the register from the stream going backward.
//
// Each new byte is placed just BELOW the currently loaded bits (in the
// left-aligned register, that means at position 64 - bits - 8).
func (r *revBitReader) reload() {
	for r.bits <= 56 && r.pos > 0 {
		r.pos--
		// Place this byte just below existing bits (MSB-aligned packing).
		// Current top bits bits are occupied; new byte goes just below.
		shift := uint(64) - uint(r.bits) - 8
		r.reg |= uint64(r.data[r.pos]) << shift
		r.bits += 8
	}
}

// readBits reads nb bits from the top of the register (returns 0 if nb == 0).
//
// This returns the most recently written bits first (highest stream
// positions first), mirroring the encoder's backward order.
func (r *revBitReader) readBits(nb uint8) uint64 {
	if nb == 0 {
		return 0
	}
	// Extract the top nb bits.
	val := r.reg >> (64 - nb)
	// Shift the register left to consume those bits.
	if nb == 64 {
		r.reg = 0
	} else {
		r.reg <<= nb
	}
	if r.bits >= nb {
		r.bits -= nb
	} else {
		r.bits = 0
	}
	if r.bits < 24 {
		r.reload()
	}
	return val
}

// ─── FSE encode/decode helpers ────────────────────────────────────────────────

// fseEncodeSym encodes one symbol into the backward bitstream, updating the FSE state.
//
// The encoder maintains state in [sz, 2*sz). To emit symbol sym:
//  1. Compute how many bits to flush: nb = (state + deltaNb) >> 16
//  2. Write the low nb bits of state to the bitstream.
//  3. New state = st[(state >> nb) + deltaFs]
//
// Note: after all symbols are encoded, the final state (minus sz) is written
// as accLog bits to allow the decoder to initialise.
func fseEncodeSym(state *uint32, sym uint8, ee []fseEe, st []uint16, bw *revBitWriter) {
	e := ee[sym]
	nb := uint8((*state + e.deltaNb) >> 16)
	bw.addBits(uint64(*state), nb)
	slotI := int32(*state>>nb) + e.deltaFs
	if slotI < 0 {
		slotI = 0
	} else if int(slotI) >= len(st) {
		slotI = int32(len(st) - 1)
	}
	slot := int(slotI)
	*state = uint32(st[slot])
}

// fseDecodeSym decodes one symbol from the backward bitstream, updating the FSE state.
//
//  1. Look up de[state] to get sym, nb, and base.
//  2. New state = base + read(nb bits).
func fseDecodeSym(state *uint16, de []fseDe, br *revBitReader) uint8 {
	e := de[*state]
	sym := e.sym
	next := e.base + uint16(br.readBits(e.nb))
	*state = next
	return sym
}

// ─── LL/ML/OF code number computation ────────────────────────────────────────

// llToCode maps a literal length value to its LL code number (0..35).
//
// Codes 0..15 are identity; codes 16+ cover ranges via lookup.
// We do a linear scan because the table is only 36 entries.
func llToCode(ll uint32) int {
	code := 0
	for i, entry := range llCodes {
		if entry[0] <= ll {
			code = i
		} else {
			break
		}
	}
	return code
}

// mlToCode maps a match length value to its ML code number (0..52).
func mlToCode(ml uint32) int {
	code := 0
	for i, entry := range mlCodes {
		if entry[0] <= ml {
			code = i
		} else {
			break
		}
	}
	return code
}

// ─── Sequence struct ──────────────────────────────────────────────────────────

// seq is one ZStd sequence: (literal_length, match_length, match_offset).
//
// A sequence means: emit ll literal bytes from the literals section,
// then copy ml bytes starting off positions back in the output buffer.
// After all sequences, any remaining literals are appended.
type seq struct {
	ll  uint32 // literal length (bytes to copy from literal section before this match)
	ml  uint32 // match length (bytes to copy from output history)
	off uint32 // match offset (1-indexed: 1 = last byte written)
}

// tokensToSeqs converts LZSS tokens into ZStd sequences + a flat literals buffer.
//
// LZSS produces a stream of Literal(byte) and Match{offset, length}.
// ZStd groups consecutive literals before each match into a single sequence.
// Any trailing literals (after the last match) go into the literals buffer
// without a corresponding sequence entry.
func tokensToSeqs(tokens []lzss.Token) ([]byte, []seq) {
	lits := make([]byte, 0)
	seqs := make([]seq, 0)
	litRun := uint32(0)

	for _, tok := range tokens {
		if tok.Kind == lzss.KindLiteral {
			lits = append(lits, tok.Byte)
			litRun++
		} else {
			// KindMatch
			seqs = append(seqs, seq{
				ll:  litRun,
				ml:  uint32(tok.Length),
				off: uint32(tok.Offset),
			})
			litRun = 0
		}
	}
	// Trailing literals stay in lits; no sequence for them.
	return lits, seqs
}

// ─── Literals section encoding ────────────────────────────────────────────────
//
// ZStd literals can be Huffman-coded or raw. We use Raw_Literals (type=0),
// which is the simplest: no Huffman table, bytes are stored verbatim.
//
// Header format depends on literal count:
//   ≤ 31 bytes:   1-byte header  = (lit_len << 3) | 0b000
//   ≤ 4095 bytes: 2-byte header  = (lit_len << 4) | 0b0100
//   else:         3-byte header  = (lit_len << 4) | 0b1000
//
// The bottom 2 bits = Literals_Block_Type (0 = Raw).
// The next 2 bits = Size_Format.

// encodeLiteralsSection encodes a raw literals buffer with a size header.
func encodeLiteralsSection(litBytes []byte) []byte {
	n := len(litBytes)
	out := make([]byte, 0, n+3)

	// Raw_Literals header format (RFC 8878 §3.1.1.2.1):
	// bits [1:0] = Literals_Block_Type = 00 (Raw)
	// bits [3:2] = Size_Format: 00 or 10 = 1-byte, 01 = 2-byte, 11 = 3-byte
	//
	// 1-byte:  size in bits [7:3] (5 bits) — header = (size << 3) | 0b000
	// 2-byte:  size in bits [11:4] (12 bits) — header = (size << 4) | 0b0100
	// 3-byte:  size in bits [19:4] (16 bits) — header = (size << 4) | 0b1100
	if n <= 31 {
		// 1-byte header: size_format=00, type=00
		out = append(out, byte(n<<3))
	} else if n <= 4095 {
		// 2-byte header: size_format=01, type=00 → 0b0100
		hdr := uint16(n<<4) | 0b0100
		out = append(out, byte(hdr), byte(hdr>>8))
	} else {
		// 3-byte header: size_format=11, type=00 → 0b1100
		hdr := uint32(n<<4) | 0b1100
		out = append(out, byte(hdr), byte(hdr>>8), byte(hdr>>16))
	}

	out = append(out, litBytes...)
	return out
}

// decodeLiteralsSection decodes a literals section, returning (literals, bytesConsumed).
func decodeLiteralsSection(data []byte) ([]byte, int, error) {
	if len(data) == 0 {
		return nil, 0, fmt.Errorf("empty literals section")
	}

	b0 := data[0]
	ltype := b0 & 0b11 // bottom 2 bits = Literals_Block_Type

	if ltype != 0 {
		// Only Raw_Literals (type=0) is implemented in this package.
		// Huffman-coded literals (type=2,3) are not emitted by our encoder,
		// so if we see them here the input came from another encoder.
		return nil, 0, fmt.Errorf("unsupported literals type %d (only Raw=0 supported)", ltype)
	}

	// Decode size_format from bits [3:2] of b0
	sizeFormat := (b0 >> 2) & 0b11

	// Raw_Literals size_format encoding (RFC 8878 §3.1.1.2.1):
	//   0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0..31)
	//   0b01          → 2-byte LE header: size in bits [11:4] (12 bits, values 0..4095)
	//   0b11          → 3-byte LE header: size in bits [19:4] (20 bits, values 0..1MB)
	var n, headerBytes int
	switch sizeFormat {
	case 0, 2:
		// 1-byte header: size in bits [7:3] (5 bits = values 0..31)
		n = int(b0 >> 3)
		headerBytes = 1
	case 1:
		// 2-byte header: 12-bit size
		if len(data) < 2 {
			return nil, 0, fmt.Errorf("truncated literals header (2-byte)")
		}
		n = (int(b0) >> 4) | (int(data[1]) << 4)
		headerBytes = 2
	case 3:
		// 3-byte header: 20-bit size (enough for blocks up to 1 MB)
		if len(data) < 3 {
			return nil, 0, fmt.Errorf("truncated literals header (3-byte)")
		}
		n = (int(b0) >> 4) | (int(data[1]) << 4) | (int(data[2]) << 12)
		headerBytes = 3
	}

	start := headerBytes
	end := start + n
	if end > len(data) {
		return nil, 0, fmt.Errorf("literals data truncated: need %d, have %d", end, len(data))
	}

	return data[start:end], end, nil
}

// ─── Sequences section encoding ───────────────────────────────────────────────
//
// Layout:
//   [sequence_count: 1-3 bytes]
//   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
//   [FSE bitstream: variable]
//
// Symbol compression modes byte:
//   bits [7:6] = LL mode
//   bits [5:4] = OF mode
//   bits [3:2] = ML mode
//   bits [1:0] = reserved (0)
// Mode 0 = Predefined, Mode 1 = RLE, Mode 2 = FSE_Compressed, Mode 3 = Repeat.
// We always write 0x00 (all Predefined).
//
// The FSE bitstream is a backward bit-stream (reverse bit writer):
//   - Sequences are encoded in REVERSE ORDER (last first).
//   - For each sequence:
//       OF extra bits, ML extra bits, LL extra bits  (in this order)
//       then FSE symbol for OF, ML, LL              (in this order)
//   - After all sequences, flush the final FSE states:
//       (state_of - sz_of) as OF_ACC_LOG bits
//       (state_ml - sz_ml) as ML_ACC_LOG bits
//       (state_ll - sz_ll) as LL_ACC_LOG bits
//   - Add sentinel and flush.
//
// The decoder does the mirror:
//  1. Read LL_ACC_LOG bits → initial state_ll
//  2. Read ML_ACC_LOG bits → initial state_ml
//  3. Read OF_ACC_LOG bits → initial state_of
//  4. For each sequence:
//     decode LL symbol (state transition)
//     decode OF symbol
//     decode ML symbol
//     read LL extra bits
//     read ML extra bits
//     read OF extra bits
//  5. Apply sequence to output buffer.

// encodeSeqCount encodes the sequence count in 1-3 bytes per RFC 8878 §3.1.1.3.1.
//
// Layout — byte0 is the FORMAT MARKER:
//
//	byte0 < 128            → 1-byte form, count = byte0
//	byte0 ∈ [128, 254]     → 2-byte form, count = ((byte0 - 128) << 8) | byte1
//	byte0 == 0xFF          → 3-byte form, count = byte1 + (byte2 << 8) + 0x7F00
//
// The decoder branches on byte0 alone, so the encoder MUST place the byte
// that determines the form first. The previous implementation used
// `binary.LittleEndian.AppendUint16(nil, count|0x8000)`, which writes the
// LOW byte first. For any count ≥ 128 whose low byte happened to be < 128
// (e.g. count = 515 → byte0 = 0x03), the decoder mis-took the 1-byte
// path and returned a tiny garbage count, mis-aligning every byte downstream
// (including the symbol-modes byte). Because the bug was silent for any
// count whose low byte was ≥ 128 — roughly half the range — most tests
// still passed.
func encodeSeqCount(count int) []byte {
	if count == 0 {
		return []byte{0}
	}
	if count < 128 {
		return []byte{byte(count)}
	}
	if count < 0x7F00 {
		// 2-byte form: byte0 = (count >> 8) | 0x80, byte1 = count & 0xFF.
		// count < 0x7F00 keeps byte0 in [0x80, 0xFE]; counts at or above 0x7F00
		// fall through to the 3-byte form (byte0 = 0xFF).
		return []byte{byte((count >> 8) | 0x80), byte(count & 0xFF)}
	}
	// 3-byte encoding: first byte = 0xFF, next 2 bytes = (count - 0x7F00) LE
	r := count - 0x7F00
	return []byte{0xFF, byte(r & 0xFF), byte((r >> 8) & 0xFF)}
}

// decodeSeqCount decodes the sequence count, returning (count, bytesConsumed).
func decodeSeqCount(data []byte) (int, int, error) {
	if len(data) == 0 {
		return 0, 0, fmt.Errorf("empty sequence count")
	}
	b0 := data[0]
	if b0 < 128 {
		// 1-byte encoding: value is in [0, 127]
		return int(b0), 1, nil
	}
	if b0 < 0xFF {
		// 2-byte encoding: count = ((b0 - 128) << 8) | b1
		// Equivalent to ((b0 & 0x7F) << 8) | b1 since b0's bit 7 is set.
		if len(data) < 2 {
			return 0, 0, fmt.Errorf("truncated sequence count")
		}
		count := (int(b0&0x7F) << 8) | int(data[1])
		return count, 2, nil
	}
	// 3-byte encoding: byte0=0xFF, then (count - 0x7F00) as LE u16
	if len(data) < 3 {
		return 0, 0, fmt.Errorf("truncated sequence count (3-byte)")
	}
	count := 0x7F00 + int(data[1]) + (int(data[2]) << 8)
	return count, 3, nil
}

// encodeSequencesSection encodes the sequences section using predefined FSE tables.
func encodeSequencesSection(seqs []seq) []byte {
	// Build encode tables (precomputed from the predefined distributions).
	eeLL, stLL := buildEncodeTable(llNorm[:], llAccLog)
	eeML, stML := buildEncodeTable(mlNorm[:], mlAccLog)
	eeOF, stOF := buildEncodeTable(ofNorm[:], ofAccLog)

	szLL := uint32(1) << llAccLog
	szML := uint32(1) << mlAccLog
	szOF := uint32(1) << ofAccLog

	// FSE encoder states start at table_size (= sz).
	// The state range [sz, 2*sz) maps to slot range [0, sz).
	stateLL := szLL
	stateML := szML
	stateOF := szOF

	bw := &revBitWriter{}

	// Encode sequences in reverse order.
	for i := len(seqs) - 1; i >= 0; i-- {
		s := seqs[i]
		llCode := llToCode(s.ll)
		mlCode := mlToCode(s.ml)

		// Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
		// code = floor(log2(raw)); extra = raw - (1 << code)
		rawOff := s.off + 3
		var ofCode uint8
		if rawOff <= 1 {
			ofCode = 0
		} else {
			ofCode = uint8(31 - bits.LeadingZeros32(rawOff))
		}
		ofExtra := rawOff - (uint32(1) << ofCode)

		// Write extra bits (OF, ML, LL in this order for backward stream).
		bw.addBits(uint64(ofExtra), ofCode)
		mlExtra := s.ml - mlCodes[mlCode][0]
		bw.addBits(uint64(mlExtra), uint8(mlCodes[mlCode][1]))
		llExtra := s.ll - llCodes[llCode][0]
		bw.addBits(uint64(llExtra), uint8(llCodes[llCode][1]))

		// FSE encode symbols in the order that the backward bitstream reverses
		// to match the decoder's read order (LL first, OF second, ML third).
		//
		// Since the backward stream reverses write order, we write the REVERSE
		// of the decode order: ML → OF → LL (LL is written last = at the top
		// of the bitstream = read first by the decoder).
		//
		// Decode order: LL, OF, ML
		// Encode order (reversed): ML, OF, LL
		fseEncodeSym(&stateML, uint8(mlCode), eeML, stML, bw)
		fseEncodeSym(&stateOF, ofCode, eeOF, stOF, bw)
		fseEncodeSym(&stateLL, uint8(llCode), eeLL, stLL, bw)
	}

	// Flush final states (low accLog bits of state - sz).
	bw.addBits(uint64(stateOF-szOF), ofAccLog)
	bw.addBits(uint64(stateML-szML), mlAccLog)
	bw.addBits(uint64(stateLL-szLL), llAccLog)
	bw.flush()

	return bw.finish()
}

// ─── Block-level compress ─────────────────────────────────────────────────────

// compressBlock tries to compress one block into ZStd compressed block format.
//
// Returns nil if the compressed form is larger than the input (in which
// case the caller should use a Raw block instead).
func compressBlock(block []byte) []byte {
	// Use LZSS to generate LZ77 tokens.
	// Window = 32 KB, max match = 255, min match = 3 (ZStd defaults with
	// a bigger window than LZSS default to improve compression ratio).
	tokens := lzss.Encode(block, 32768, 255, 3)

	// Convert tokens to ZStd sequences.
	litBytes, seqs := tokensToSeqs(tokens)

	// If no sequences were found, LZ77 had nothing to compress.
	// A compressed block with 0 sequences still has overhead, so fall back.
	if len(seqs) == 0 {
		return nil
	}

	out := make([]byte, 0)

	// Encode literals section (Raw_Literals).
	out = append(out, encodeLiteralsSection(litBytes)...)

	// Encode sequences section.
	out = append(out, encodeSeqCount(len(seqs))...)
	out = append(out, 0x00) // Symbol_Compression_Modes = all Predefined

	bitstream := encodeSequencesSection(seqs)
	out = append(out, bitstream...)

	if len(out) >= len(block) {
		return nil // Not beneficial
	}
	return out
}

// decompressBlock decompresses one ZStd compressed block.
//
// Reads the literals section, sequences section, and applies the sequences
// to the output buffer to reconstruct the original data.
func decompressBlock(data []byte, out *[]byte) error {
	// ── Literals section ─────────────────────────────────────────────────
	litBytes, litConsumed, err := decodeLiteralsSection(data)
	if err != nil {
		return err
	}
	pos := litConsumed

	// ── Sequences count ──────────────────────────────────────────────────
	if pos >= len(data) {
		// Block has only literals, no sequences.
		*out = append(*out, litBytes...)
		return nil
	}

	nSeqs, scBytes, err := decodeSeqCount(data[pos:])
	if err != nil {
		return err
	}
	pos += scBytes

	if nSeqs == 0 {
		// No sequences — all content is in literals.
		*out = append(*out, litBytes...)
		return nil
	}

	// ── Symbol compression modes ─────────────────────────────────────────
	if pos >= len(data) {
		return fmt.Errorf("missing symbol compression modes byte")
	}
	modesByte := data[pos]
	pos++

	// Check that all modes are Predefined (0).
	llMode := (modesByte >> 6) & 3
	ofMode := (modesByte >> 4) & 3
	mlMode := (modesByte >> 2) & 3
	if llMode != 0 || ofMode != 0 || mlMode != 0 {
		return fmt.Errorf(
			"unsupported FSE modes: LL=%d OF=%d ML=%d (only Predefined=0 supported)",
			llMode, ofMode, mlMode,
		)
	}

	// ── FSE bitstream ────────────────────────────────────────────────────
	bitstream := data[pos:]
	br, err := newRevBitReader(bitstream)
	if err != nil {
		return err
	}

	// Build decode tables from predefined distributions.
	dtLL := buildDecodeTable(llNorm[:], llAccLog)
	dtML := buildDecodeTable(mlNorm[:], mlAccLog)
	dtOF := buildDecodeTable(ofNorm[:], ofAccLog)

	// Initialise FSE states from the bitstream.
	// The encoder wrote: state_ll, state_ml, state_of (each as accLog bits),
	// then sentinel-flushed. The decoder reads them in the same order.
	stateLL := uint16(br.readBits(llAccLog))
	stateML := uint16(br.readBits(mlAccLog))
	stateOF := uint16(br.readBits(ofAccLog))

	// Track position in the literals buffer.
	litPos := 0

	// Apply each sequence.
	for i := 0; i < nSeqs; i++ {
		// Decode symbols (state transitions) — order: LL, OF, ML.
		llCode := fseDecodeSym(&stateLL, dtLL, br)
		ofCode := fseDecodeSym(&stateOF, dtOF, br)
		mlCode := fseDecodeSym(&stateML, dtML, br)

		// Validate code indices before table lookups (security check).
		if int(llCode) >= len(llCodes) {
			return fmt.Errorf("invalid LL code %d", llCode)
		}
		if int(mlCode) >= len(mlCodes) {
			return fmt.Errorf("invalid ML code %d", mlCode)
		}

		llInfo := llCodes[llCode]
		mlInfo := mlCodes[mlCode]

		ll := llInfo[0] + uint32(br.readBits(uint8(llInfo[1])))
		ml := mlInfo[0] + uint32(br.readBits(uint8(mlInfo[1])))
		// Offset: raw = (1 << ofCode) | extra_bits; offset = raw - 3
		ofRaw := (uint32(1) << ofCode) | uint32(br.readBits(ofCode))
		if ofRaw < 3 {
			return fmt.Errorf("decoded offset underflow: ofRaw=%d", ofRaw)
		}
		offset := ofRaw - 3

		// Emit ll literal bytes from the literals buffer.
		litEnd := litPos + int(ll)
		if litEnd > len(litBytes) {
			return fmt.Errorf(
				"literal run %d overflows literals buffer (pos=%d len=%d)",
				ll, litPos, len(litBytes),
			)
		}
		if len(*out)+int(ll) > maxOutput {
			return fmt.Errorf("decompressed size exceeds limit of %d bytes", maxOutput)
		}
		*out = append(*out, litBytes[litPos:litEnd]...)
		litPos = litEnd

		// Copy ml bytes from offset back in the output buffer.
		// offset = 0 would be a back-reference to (out.len() - 0), which is
		// past the end. The minimum valid offset here is 1.
		if offset == 0 || int(offset) > len(*out) {
			return fmt.Errorf("bad match offset %d (output len %d)", offset, len(*out))
		}
		if len(*out)+int(ml) > maxOutput {
			return fmt.Errorf("decompressed size exceeds limit of %d bytes", maxOutput)
		}
		copyStart := len(*out) - int(offset)
		for j := 0; j < int(ml); j++ {
			*out = append(*out, (*out)[copyStart+j])
		}
	}

	// Any remaining literals after the last sequence.
	if len(*out)+len(litBytes[litPos:]) > maxOutput {
		return fmt.Errorf("decompressed size exceeds limit of %d bytes", maxOutput)
	}
	*out = append(*out, litBytes[litPos:]...)
	return nil
}

// ─── Public API ───────────────────────────────────────────────────────────────

// Compress compresses data to ZStd format (RFC 8878).
//
// The output is a valid ZStd frame that can be decompressed by the zstd CLI
// tool or any conforming implementation.
//
// Example:
//
//	text := []byte("the quick brown fox jumps over the lazy dog")
//	compressed := zstd.Compress(text)
//	original, err := zstd.Decompress(compressed)
func Compress(data []byte) []byte {
	out := make([]byte, 0, len(data)+16)

	// ── ZStd frame header ────────────────────────────────────────────────
	// Magic number (4 bytes LE).
	out = binary.LittleEndian.AppendUint32(out, magic)

	// Frame Header Descriptor (FHD):
	//   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
	//   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
	//   bit 4:   Content_Checksum_Flag = 0
	//   bit 3-2: reserved = 0
	//   bit 1-0: Dict_ID_Flag = 0
	// = 0b1110_0000 = 0xE0
	out = append(out, 0xE0)

	// Frame_Content_Size (8 bytes LE) — the uncompressed size.
	// A decoder can use this to pre-allocate the output buffer.
	out = binary.LittleEndian.AppendUint64(out, uint64(len(data)))

	// ── Blocks ───────────────────────────────────────────────────────────
	// Handle the special case of completely empty input: emit one empty raw block.
	if len(data) == 0 {
		// Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
		hdr := uint32(0b001) // last=1, type=00, size=0
		out = append(out, byte(hdr), byte(hdr>>8), byte(hdr>>16))
		return out
	}

	offset := 0
	for offset < len(data) {
		end := offset + maxBlockSize
		if end > len(data) {
			end = len(data)
		}
		block := data[offset:end]
		last := end == len(data)
		lastBit := uint32(0)
		if last {
			lastBit = 1
		}

		// ── Try RLE block ─────────────────────────────────────────────
		// If all bytes in the block are identical, a single-byte RLE block
		// encodes it in just 1 byte (plus 3-byte header = 4 bytes total).
		if len(block) > 0 && isAllSame(block) {
			// Type=RLE(01): bits [2:1] = 01 → shift left 1 = 0b10
			hdr := (uint32(len(block)) << 3) | (0b01 << 1) | lastBit
			out = append(out, byte(hdr), byte(hdr>>8), byte(hdr>>16))
			out = append(out, block[0])
		} else {
			// ── Try compressed block ──────────────────────────────────
			compressed := compressBlock(block)
			if compressed != nil {
				// Type=Compressed(10): bits [2:1] = 10 → shift left 1 = 0b100
				hdr := (uint32(len(compressed)) << 3) | (0b10 << 1) | lastBit
				out = append(out, byte(hdr), byte(hdr>>8), byte(hdr>>16))
				out = append(out, compressed...)
			} else {
				// ── Raw block (fallback) ──────────────────────────────
				// Type=Raw(00): bits [2:1] = 00
				hdr := (uint32(len(block)) << 3) | (0b00 << 1) | lastBit
				out = append(out, byte(hdr), byte(hdr>>8), byte(hdr>>16))
				out = append(out, block...)
			}
		}

		offset = end
	}

	return out
}

// isAllSame returns true if all bytes in b are identical.
func isAllSame(b []byte) bool {
	if len(b) == 0 {
		return true
	}
	first := b[0]
	for _, v := range b[1:] {
		if v != first {
			return false
		}
	}
	return true
}

// Decompress decompresses a ZStd frame, returning the original data.
//
// Accepts any valid ZStd frame with:
//   - Single-segment or multi-segment layout
//   - Raw, RLE, or Compressed blocks
//   - Predefined FSE modes (no per-frame table description)
//
// Returns an error if the input is truncated, has a bad magic number,
// or contains unsupported features (non-predefined FSE tables, Huffman
// literals, reserved block types, or output exceeding 256 MB).
//
// Example:
//
//	original, err := zstd.Decompress(compressed)
//	if err != nil {
//	    log.Fatal(err)
//	}
func Decompress(data []byte) ([]byte, error) {
	if len(data) < 5 {
		return nil, fmt.Errorf("frame too short")
	}

	// ── Validate magic ───────────────────────────────────────────────────
	m := binary.LittleEndian.Uint32(data[0:4])
	if m != magic {
		return nil, fmt.Errorf("bad magic: %#010x (expected %#010x)", m, magic)
	}

	pos := 4

	// ── Parse Frame Header Descriptor ───────────────────────────────────
	// FHD encodes several flags that control the header layout.
	fhd := data[pos]
	pos++

	// FCS_Field_Size: bits [7:6] of FHD.
	//   00 → 0 bytes if Single_Segment=0, else 1 byte
	//   01 → 2 bytes (value + 256)
	//   10 → 4 bytes
	//   11 → 8 bytes
	fcsFlag := (fhd >> 6) & 3

	// Single_Segment_Flag: bit 5. When set, the window descriptor is omitted.
	singleSeg := (fhd >> 5) & 1

	// Content_Checksum_Flag: bit 4. When set, a 4-byte checksum follows the
	// last block. We don't validate it, but we need to know it exists.
	// (unused in this implementation, but parsed for correctness)
	_ = (fhd >> 4) & 1

	// Dict_ID_Flag: bits [1:0]. Indicates how many bytes the dict ID occupies.
	dictFlag := fhd & 3

	// ── Window Descriptor ────────────────────────────────────────────────
	// Present only if Single_Segment_Flag = 0. We skip it (we don't enforce
	// window size limits in this implementation).
	if singleSeg == 0 {
		pos++ // skip Window_Descriptor byte
	}

	// ── Dict ID ──────────────────────────────────────────────────────────
	dictIDBytes := [4]int{0, 1, 2, 4}
	pos += dictIDBytes[dictFlag] // skip dict ID (we don't support custom dicts)
	if pos > len(data) {
		return nil, fmt.Errorf("zstd: frame header truncated (dict ID field)")
	}

	// ── Frame Content Size ───────────────────────────────────────────────
	// We read but don't validate FCS (we trust the blocks to be correct).
	var fcsBytes int
	switch fcsFlag {
	case 0:
		if singleSeg == 1 {
			fcsBytes = 1
		} else {
			fcsBytes = 0
		}
	case 1:
		fcsBytes = 2
	case 2:
		fcsBytes = 4
	case 3:
		fcsBytes = 8
	}
	pos += fcsBytes // skip FCS
	if pos > len(data) {
		return nil, fmt.Errorf("zstd: frame header truncated (FCS field)")
	}

	// ── Blocks ───────────────────────────────────────────────────────────
	// Guard against decompression bombs: cap total output at maxOutput (256 MB).
	out := make([]byte, 0)

	for {
		if pos+3 > len(data) {
			return nil, fmt.Errorf("truncated block header")
		}

		// 3-byte little-endian block header.
		hdr := uint32(data[pos]) | (uint32(data[pos+1]) << 8) | (uint32(data[pos+2]) << 16)
		pos += 3

		last := (hdr & 1) != 0
		btype := (hdr >> 1) & 3
		bsize := int(hdr >> 3)

		switch btype {
		case 0:
			// Raw block: bsize bytes of verbatim content.
			if pos+bsize > len(data) {
				return nil, fmt.Errorf("raw block truncated: need %d bytes at pos %d", bsize, pos)
			}
			if len(out)+bsize > maxOutput {
				return nil, fmt.Errorf("decompressed size exceeds limit of %d bytes", maxOutput)
			}
			out = append(out, data[pos:pos+bsize]...)
			pos += bsize

		case 1:
			// RLE block: 1 byte repeated bsize times.
			if pos >= len(data) {
				return nil, fmt.Errorf("RLE block missing byte")
			}
			if len(out)+bsize > maxOutput {
				return nil, fmt.Errorf("decompressed size exceeds limit of %d bytes", maxOutput)
			}
			b := data[pos]
			pos++
			for i := 0; i < bsize; i++ {
				out = append(out, b)
			}

		case 2:
			// Compressed block.
			if pos+bsize > len(data) {
				return nil, fmt.Errorf("compressed block truncated: need %d bytes", bsize)
			}
			blockData := data[pos : pos+bsize]
			pos += bsize
			if err := decompressBlock(blockData, &out); err != nil {
				return nil, err
			}
			if len(out) > maxOutput {
				return nil, fmt.Errorf("decompressed size exceeds limit of %d bytes", maxOutput)
			}

		case 3:
			return nil, fmt.Errorf("reserved block type 3")
		}

		if last {
			break
		}
	}

	return out, nil
}
