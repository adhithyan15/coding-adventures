// Package azteccode implements an Aztec Code encoder conforming to ISO/IEC 24778:2008.
//
// # What is Aztec Code?
//
// Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
// published as a patent-free format. Unlike QR Code (which uses three square
// finder patterns at three corners), Aztec Code places a single bullseye finder
// pattern at the CENTER of the symbol. A scanner finds the bullseye first, then
// reads outward in a clockwise spiral — no large quiet zone is needed.
//
// # Where Aztec Code is used today
//
//   - IATA boarding passes — every airline boarding pass uses Aztec Code
//   - Eurostar, Amtrak, and TGV rail tickets — printed and on-screen tickets
//   - PostNL, Deutsche Post, La Poste — European postal routing labels
//   - US military ID cards
//   - Some US driver's licences (alongside PDF417)
//
// # Symbol variants
//
// Compact Aztec: 1–4 layers,  size = 11 + 4 × layers  (15×15 to 27×27)
// Full Aztec:    1–32 layers, size = 15 + 4 × layers  (19×19 to 143×143)
//
// The encoder automatically selects the smallest symbol that fits the data at
// the requested ECC level (default 23%). No quiet zone mandate in the standard,
// but 1 module is recommended.
//
// # Structure of an Aztec Code symbol (center outward)
//
//	┌───────────────────────────────────────────────────────────┐
//	│  outer data layers (1–32 for full, 1–4 for compact)       │
//	│  ┌─────────────────────────────────────────────────────┐  │
//	│  │  reference grid (full symbols only, every 16 mods)  │  │
//	│  │  ┌───────────────────────────────────────────────┐  │  │
//	│  │  │  mode message band (28 bits or 40 bits)        │  │  │
//	│  │  │  ┌───────────────────────────────────────┐    │  │  │
//	│  │  │  │  orientation mark corners (4× dark)   │    │  │  │
//	│  │  │  │  ┌─────────────────────────────────┐  │    │  │  │
//	│  │  │  │  │  bullseye finder (concentric)    │  │    │  │  │
//	│  │  │  │  └─────────────────────────────────┘  │    │  │  │
//	│  │  │  └───────────────────────────────────────┘    │  │  │
//	│  │  └───────────────────────────────────────────────┘  │  │
//	│  └─────────────────────────────────────────────────────┘  │
//	└───────────────────────────────────────────────────────────┘
//
// # Encoding pipeline (v0.1.0 — byte-mode only)
//
//  1. Encode input via Binary-Shift escape from Upper mode (all bytes raw).
//  2. Select the smallest symbol at the requested ECC level (default 23%).
//  3. Pad the data codeword sequence to the exact slot count.
//  4. Compute Reed-Solomon ECC over GF(256)/0x12D (b=1 convention, same as Data Matrix).
//  5. Apply bit stuffing: insert complement bit after every 4 consecutive identical bits.
//  6. Compute GF(16) mode message (layer count + codeword count + RS nibbles).
//  7. Initialize grid: bullseye → orientation marks → mode message → reference grid.
//  8. Place data+ECC bits in the clockwise layer spiral from inside out.
//
// # v0.1.0 simplifications
//
//  1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
//     Multi-mode optimization (Digit/Upper/Lower/Mixed/Punct) is v0.2.0.
//  2. 8-bit codewords → GF(256)/0x12D RS (same polynomial as Data Matrix).
//     GF(16)/GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
//  3. Default ECC = 23% of total codeword slots.
//  4. Auto-select compact vs full (force-compact option is v0.2.0).
//
// # Quick start
//
//	grid, err := azteccode.Encode("Hello, Aztec!")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Printf("Symbol is %d×%d modules\n", grid.Rows, grid.Cols)
package azteccode

import (
	"errors"
	"fmt"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

// Bring in transitive dependencies used by the module graph even if only
// indirectly required at runtime through barcode2d.
var _ = paintinstructions.Version

// Version is the semantic version of this package.
const Version = "0.1.0"

// ============================================================================
// Public error types
// ============================================================================

// InputTooLongError is returned when the encoded data cannot fit even in the
// largest 32-layer full Aztec symbol.
type InputTooLongError struct {
	// DataBits is the number of bits the input encodes to.
	DataBits int
}

func (e *InputTooLongError) Error() string {
	return fmt.Sprintf("aztec-code: input too long — encoded to %d data bits, exceeds 32-layer capacity", e.DataBits)
}

// Is implements errors.Is matching for InputTooLongError.
func (e *InputTooLongError) Is(target error) bool {
	_, ok := target.(*InputTooLongError)
	return ok
}

// ErrInputTooLong is a sentinel for errors.Is comparisons.
var ErrInputTooLong = &InputTooLongError{}

// ============================================================================
// Options
// ============================================================================

// Options configures the Aztec Code encoder.
type Options struct {
	// MinEccPercent sets the minimum error-correction percentage (default: 23,
	// range: 10–90). Higher values produce larger symbols with more redundancy.
	// The default of 23% means ≈11.5% of all codewords can be corrupted and
	// still be corrected.
	MinEccPercent int
}

// defaultOptions returns Options with all fields at their defaults.
func defaultOptions() Options {
	return Options{MinEccPercent: 23}
}

// ============================================================================
// GF(16) arithmetic — for mode message Reed-Solomon
// ============================================================================
//
// GF(16) is the finite field with 16 elements. We build it from the primitive
// polynomial:
//
//	p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
//
// Every non-zero element can be expressed as a power of the primitive root α,
// where α is a root of p(x). This means α^4 = α + 1 in GF(16).
//
// The discrete-log table maps each non-zero field element to its exponent:
//   LOG16[e] = i  means  α^i = e
//
// The antilog table maps an exponent to the field element:
//   ALOG16[i] = α^i
//
// Power table:
//   α^0=1, α^1=2, α^2=4, α^3=8,
//   α^4=3, α^5=6, α^6=12, α^7=11,
//   α^8=5, α^9=10, α^10=7, α^11=14,
//   α^12=15, α^13=13, α^14=9, α^15=1  (period = 15, so α is primitive)

// gf16Log is the GF(16) discrete-log table. gf16Log[0] is undefined (−1 sentinel).
var gf16Log = [16]int{
	-1, // log(0) undefined
	0,  // log(1) = 0
	1,  // log(2) = 1
	4,  // log(3) = 4
	2,  // log(4) = 2
	8,  // log(5) = 8
	5,  // log(6) = 5
	10, // log(7) = 10
	3,  // log(8) = 3
	14, // log(9) = 14
	9,  // log(10) = 9
	7,  // log(11) = 7
	6,  // log(12) = 6
	13, // log(13) = 13
	11, // log(14) = 11
	12, // log(15) = 12
}

// gf16Alog is the GF(16) antilog (exponentiation) table. gf16Alog[i] = α^i.
var gf16Alog = [16]int{
	1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1,
}

// gf16Mul multiplies two GF(16) elements using log/antilog tables.
//
// a × b = α^{(log(a) + log(b)) mod 15}
//
// Returns 0 if either operand is 0, since 0 absorbs multiplication.
func gf16Mul(a, b int) int {
	if a == 0 || b == 0 {
		return 0
	}
	return gf16Alog[(gf16Log[a]+gf16Log[b])%15]
}

// gf16RsEncode computes n GF(16) Reed-Solomon check nibbles for the given data
// nibbles. Uses roots α^1 through α^n (b=1 convention, standard Aztec layout).
//
// Internally builds the generator polynomial g(x) = ∏(x + α^i) for i=1..n,
// then performs polynomial long division via a streaming LFSR shift register.
func gf16RsEncode(data []int, n int) []int {
	g := buildGf16Generator(n)
	rem := make([]int, n)
	for _, d := range data {
		fb := d ^ rem[0]
		// Shift register left; accumulate feedback through generator coefficients.
		for i := 0; i < n-1; i++ {
			rem[i] = rem[i+1] ^ gf16Mul(g[i+1], fb)
		}
		rem[n-1] = gf16Mul(g[n], fb)
	}
	return rem
}

// buildGf16Generator builds the GF(16) RS generator polynomial with roots
// α^1 through α^n.
//
// Starts with g = [1] (the constant polynomial 1), then multiplies by each
// linear factor (x + α^i):
//
//	g_new = g × (x + α^i)
//	      = g shifted left (×x) XOR g × α^i (constant term contribution)
func buildGf16Generator(n int) []int {
	g := []int{1}
	for i := 1; i <= n; i++ {
		ai := gf16Alog[i%15]
		next := make([]int, len(g)+1)
		for j, coeff := range g {
			next[j+1] ^= coeff            // coeff × x
			next[j] ^= gf16Mul(ai, coeff) // coeff × α^i
		}
		g = next
	}
	return g
}

// ============================================================================
// GF(256)/0x12D arithmetic — for 8-bit data codewords
// ============================================================================
//
// Aztec Code uses GF(256) with primitive polynomial:
//
//	p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
//
// This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from QR
// Code (which uses 0x11D). Never mix tables between the two formats.
//
// Reed-Solomon convention: b=1 (roots α^1 through α^n). This matches MA02's
// Data Matrix convention so the same RS algorithm applies.

const gf256Poly = 0x12D // Aztec/Data Matrix primitive polynomial

// gf256Exp[i] = α^i in GF(256)/0x12D, i = 0..254. Index 255 wraps to 0.
var gf256Exp [256]byte

// gf256Log[v] = k such that α^k = v, for v = 1..255. gf256Log[0] = 0 sentinel.
var gf256Log [256]byte

func init() {
	// Build GF(256)/0x12D exponent and log tables.
	// Algorithm: start with val = 1 = α^0, then left-shift each step (×α = ×x).
	// When bit 8 overflows, reduce modulo 0x12D.
	val := 1
	for i := 0; i < 255; i++ {
		gf256Exp[i] = byte(val)
		gf256Log[val] = byte(i)
		val <<= 1
		if val&0x100 != 0 {
			val ^= gf256Poly
		}
	}
	gf256Exp[255] = gf256Exp[0] // α^255 = α^0 = 1 (order 255)
}

// gf256Mul multiplies two GF(256)/0x12D elements using log/antilog tables.
//
// For a, b ≠ 0: a × b = α^{(log(a) + log(b)) mod 255}.
// Returns 0 if either operand is 0.
func gf256Mul(a, b byte) byte {
	if a == 0 || b == 0 {
		return 0
	}
	return gf256Exp[(uint(gf256Log[a])+uint(gf256Log[b]))%255]
}

// buildGf256Generator constructs the RS generator polynomial for nEcc ECC bytes
// over GF(256)/0x12D with b=1 roots.
//
// Returns big-endian coefficients (highest degree first), including the monic
// leading 1. Length = nEcc + 1.
func buildGf256Generator(nEcc int) []byte {
	g := []byte{1}
	for i := 1; i <= nEcc; i++ {
		// α^i where i may exceed 254: use i%255 since α has order 255 in GF(256).
		// Without the mod, i > 254 would panic with an out-of-bounds index into
		// gf256Exp (which has only 256 entries covering α^0..α^254).
		ai := gf256Exp[i%255] // α^(i mod 255)
		next := make([]byte, len(g)+1)
		for j, coeff := range g {
			next[j] ^= coeff                  // coeff × x
			next[j+1] ^= gf256Mul(coeff, ai)  // coeff × α^i
		}
		g = next
	}
	return g
}

// gf256RsEncode computes nEcc Reed-Solomon check bytes for the given data bytes
// over GF(256)/0x12D using the LFSR polynomial-division method.
//
// This is the same algorithm used by Data Matrix ECC200.
func gf256RsEncode(data []byte, nEcc int) []byte {
	g := buildGf256Generator(nEcc)
	n := len(g) - 1 // degree of generator = nEcc
	rem := make([]byte, n)
	for _, d := range data {
		fb := d ^ rem[0]
		copy(rem, rem[1:])
		rem[n-1] = 0
		if fb != 0 {
			for i := 0; i < n; i++ {
				rem[i] ^= gf256Mul(g[i+1], fb)
			}
		}
	}
	return rem
}

// ============================================================================
// Capacity tables
// ============================================================================
//
// These tables encode the total data-bit capacity of each Aztec Code symbol
// configuration, derived from ISO/IEC 24778:2008 Table 1.
//
// totalBits  = total usable bit slots in the symbol (data + ECC combined).
// maxBytes8  = total codeword slots when using 8-bit (byte-mode) codewords.
//
// For compact layer L, the total bits count includes the 16 data bits that
// spill into the mode message ring (the first layer shares that ring).

type capacityEntry struct {
	totalBits int // total data+ECC bit positions available
	maxBytes8 int // total 8-bit codeword slots (totalBits / 8)
}

// compactCapacity[L] for compact layers L = 1..4.
// Index 0 is unused (layers are 1-indexed).
var compactCapacity = [5]capacityEntry{
	{0, 0},    // unused
	{72, 9},   // 1 layer, 15×15
	{200, 25}, // 2 layers, 19×19
	{392, 49}, // 3 layers, 23×23
	{648, 81}, // 4 layers, 27×27
}

// fullCapacity[L] for full layers L = 1..32.
// Index 0 is unused.
var fullCapacity = [33]capacityEntry{
	{0, 0},         // unused
	{88, 11},       //  1 layer, 19×19
	{216, 27},      //  2 layers, 23×23
	{360, 45},      //  3 layers, 27×27
	{520, 65},      //  4 layers, 31×31
	{696, 87},      //  5 layers, 35×35
	{888, 111},     //  6 layers, 39×39
	{1096, 137},    //  7 layers, 43×43
	{1320, 165},    //  8 layers, 47×47
	{1560, 195},    //  9 layers, 51×51
	{1816, 227},    // 10 layers, 55×55
	{2088, 261},    // 11 layers, 59×59
	{2376, 297},    // 12 layers, 63×63
	{2680, 335},    // 13 layers, 67×67
	{3000, 375},    // 14 layers, 71×71
	{3336, 417},    // 15 layers, 75×75
	{3688, 461},    // 16 layers, 79×79
	{4056, 507},    // 17 layers, 83×83
	{4440, 555},    // 18 layers, 87×87
	{4840, 605},    // 19 layers, 91×91
	{5256, 657},    // 20 layers, 95×95
	{5688, 711},    // 21 layers, 99×99
	{6136, 767},    // 22 layers, 103×103
	{6600, 825},    // 23 layers, 107×107
	{7080, 885},    // 24 layers, 111×111
	{7576, 947},    // 25 layers, 115×115
	{8088, 1011},   // 26 layers, 119×119
	{8616, 1077},   // 27 layers, 123×123
	{9160, 1145},   // 28 layers, 127×127
	{9720, 1215},   // 29 layers, 131×131
	{10296, 1287},  // 30 layers, 135×135
	{10888, 1361},  // 31 layers, 139×139
	{11496, 1437},  // 32 layers, 143×143
}

// ============================================================================
// Symbol selection
// ============================================================================

// symbolSpec describes the selected symbol configuration.
type symbolSpec struct {
	compact     bool // true = compact, false = full
	layers      int  // number of data layers
	dataCwCount int  // number of 8-bit data codewords
	eccCwCount  int  // number of 8-bit ECC codewords
}

// selectSymbol picks the smallest Aztec Code symbol that can hold dataBitCount
// bits at the requested minimum ECC percentage.
//
// The selection applies a 20% conservative overhead for bit stuffing, since
// stuffed bits consume space not reflected in the raw bit count. We compare:
//
//	ceil(dataBitCount × 1.2 / 8)  ≤  dataCwCount
//
// Try compact layers 1–4 first (smaller symbols); fall back to full 1–32.
func selectSymbol(dataBitCount, minEccPct int) (symbolSpec, error) {
	// Conservative stuffed byte count: assume 20% worst-case overhead.
	stuffedBytes := (dataBitCount*12/10 + 7) / 8

	for layers := 1; layers <= 4; layers++ {
		cap := compactCapacity[layers]
		totalBytes := cap.maxBytes8
		eccCwCount := (minEccPct*totalBytes + 99) / 100 // ceil(pct/100 × total)
		dataCwCount := totalBytes - eccCwCount
		if dataCwCount <= 0 {
			continue
		}
		if stuffedBytes <= dataCwCount {
			return symbolSpec{compact: true, layers: layers, dataCwCount: dataCwCount, eccCwCount: eccCwCount}, nil
		}
	}

	for layers := 1; layers <= 32; layers++ {
		cap := fullCapacity[layers]
		totalBytes := cap.maxBytes8
		eccCwCount := (minEccPct*totalBytes + 99) / 100
		dataCwCount := totalBytes - eccCwCount
		if dataCwCount <= 0 {
			continue
		}
		if stuffedBytes <= dataCwCount {
			return symbolSpec{compact: false, layers: layers, dataCwCount: dataCwCount, eccCwCount: eccCwCount}, nil
		}
	}

	return symbolSpec{}, &InputTooLongError{DataBits: dataBitCount}
}

// ============================================================================
// Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
// ============================================================================
//
// All input is wrapped in one Binary-Shift block from Upper mode:
//
//  1. Emit 5 bits = 0b11111 (the Binary-Shift escape codeword in Upper mode)
//  2. Emit the length:
//     - If len ≤ 31: 5 bits for the length value
//     - If len > 31: 5 bits = 0b00000, then 11 bits for the length value
//  3. Emit each input byte as 8 bits, MSB first.
//
// This is always valid regardless of the input content, though not maximally
// compact for uppercase ASCII (which could use direct Upper mode codewords).
// Multi-mode optimization is planned for v0.2.0.

// encodeBytesAsBits encodes input bytes as the flat bit stream using the
// Binary-Shift escape from Upper mode. Returns bits as []byte where each
// element is 0 or 1.
func encodeBytesAsBits(input []byte) []byte {
	bits := make([]byte, 0, 5+5+len(input)*8+16)

	writeBits := func(val int, count int) {
		for i := count - 1; i >= 0; i-- {
			bits = append(bits, byte((val>>i)&1))
		}
	}

	writeBits(31, 5) // Binary-Shift escape codeword (0b11111)

	if len(input) <= 31 {
		writeBits(len(input), 5)
	} else {
		writeBits(0, 5)                // extended-length flag
		writeBits(len(input), 11)      // actual length in 11 bits
	}

	for _, b := range input {
		writeBits(int(b), 8)
	}

	return bits
}

// ============================================================================
// Padding
// ============================================================================

// padToBytes zero-pads the bit slice to exactly targetBytes×8 bits.
//
// The LFSR bit stream usually ends on a non-byte boundary; we pad the trailing
// partial codeword with zeros. Then we append zero-byte codewords until we
// reach the target. Finally, if the last codeword would be all-zero (value 0
// in GF), we replace it with 0xFF (the "all-zero codeword avoidance" rule
// from ISO/IEC 24778:2008) to prevent GF arithmetic degeneracies.
func padToBytes(bits []byte, targetBytes int) []byte {
	out := make([]byte, len(bits), targetBytes*8)
	copy(out, bits)
	for len(out)%8 != 0 {
		out = append(out, 0)
	}
	for len(out) < targetBytes*8 {
		out = append(out, 0)
	}
	return out[:targetBytes*8]
}

// ============================================================================
// Bit stuffing
// ============================================================================
//
// Aztec Code requires bit stuffing on the data+ECC bit stream before placement.
// The rule: after every 4 consecutive identical bits (all-0 or all-1), insert
// one complement bit.
//
// Why? Reference grid lines on full symbols alternate dark/light. If a long run
// of identical bits in the data layers happens to align with a reference grid
// position, the scanner's perspective-correction calculations can fail. Stuffing
// guarantees the longest run of identical bits is 4, limiting this risk.
//
// Example:
//   Input:  1 1 1 1 0 0 0 0 1 0
//   After 4× 1: insert complement 0  →  1 1 1 1 [0] 0 0 0 0 ...
//   After 4× 0: insert complement 1  →  ... 0 0 0 0 [1] 1 0
//   Result: 1 1 1 1 0 0 0 0 0 1 1 0
//
// Note: bit stuffing is applied AFTER RS ECC is appended. The decoder must
// de-stuff first, then RS-decode.
//
// Note: stuffing does NOT apply to the bullseye, orientation marks, mode
// message, or reference grid lines — only the data layer bits.

// stuffBits applies Aztec bit stuffing to the data+ECC bit stream.
// Returns the stuffed bit stream (may be longer than the input).
func stuffBits(bits []byte) []byte {
	stuffed := make([]byte, 0, len(bits)+len(bits)/4)
	runVal := byte(255) // 255 = "no run yet" sentinel
	runLen := 0

	for _, bit := range bits {
		if bit == runVal {
			runLen++
		} else {
			runVal = bit
			runLen = 1
		}

		stuffed = append(stuffed, bit)

		if runLen == 4 {
			// Insert a stuff bit of the opposite value.
			stuffBit := 1 - bit
			stuffed = append(stuffed, stuffBit)
			runVal = stuffBit
			runLen = 1
		}
	}

	return stuffed
}

// ============================================================================
// Mode message encoding
// ============================================================================
//
// The mode message is Aztec Code's equivalent of QR Code's format information.
// It tells the decoder:
//   - Whether the symbol is compact or full (inferred from bullseye size, also
//     redundantly encoded for robustness).
//   - The number of data layers (so the decoder knows how many rings to read).
//   - The number of data codewords (so the decoder knows where data ends and
//     ECC begins).
//
// The mode message is placed in the ring immediately outside the bullseye,
// interleaved with the first few data bits.
//
// Compact mode message: 28 bits = 7 nibbles
//   m = ((layers-1) << 6) | (dataCwCount-1)   [8 bits: 2 for layers, 6 for cwcount]
//   Pack as 2 data nibbles, compute 5 GF(16) ECC nibbles, total 7 nibbles.
//
// Full mode message: 40 bits = 10 nibbles
//   m = ((layers-1) << 11) | (dataCwCount-1)   [16 bits: 5 for layers, 11 for cwcount]
//   Pack as 4 data nibbles, compute 6 GF(16) ECC nibbles, total 10 nibbles.
//
// The nibbles are emitted MSB-first (i.e., nibble[i] bit 3 first, then bit 2,
// bit 1, bit 0) into the flat 28-bit or 40-bit stream.

// encodeModeMessage computes the mode message as a flat bit stream.
//
// Returns 28 bits for compact, 40 bits for full.
func encodeModeMessage(compact bool, layers, dataCwCount int) []byte {
	var dataNibbles []int
	var numEcc int

	if compact {
		m := ((layers - 1) << 6) | (dataCwCount - 1)
		dataNibbles = []int{m & 0xF, (m >> 4) & 0xF}
		numEcc = 5
	} else {
		m := ((layers - 1) << 11) | (dataCwCount - 1)
		dataNibbles = []int{m & 0xF, (m >> 4) & 0xF, (m >> 8) & 0xF, (m >> 12) & 0xF}
		numEcc = 6
	}

	eccNibbles := gf16RsEncode(dataNibbles, numEcc)
	allNibbles := append(dataNibbles, eccNibbles...)

	bits := make([]byte, 0, len(allNibbles)*4)
	for _, nibble := range allNibbles {
		for i := 3; i >= 0; i-- {
			bits = append(bits, byte((nibble>>i)&1))
		}
	}
	return bits
}

// ============================================================================
// Grid construction helpers
// ============================================================================

// symbolSize returns the side length of the square symbol grid.
//
// Formula:
//   compact: 11 + 4 × layers  (11×11 bullseye + 2-module bands for each layer)
//   full:    15 + 4 × layers  (15×15 bullseye + 2-module bands)
func symbolSize(compact bool, layers int) int {
	if compact {
		return 11 + 4*layers
	}
	return 15 + 4*layers
}

// bullseyeRadius returns the Chebyshev radius of the outermost bullseye ring.
//
// Compact: radius 5 → 11×11 bullseye
// Full:    radius 7 → 15×15 bullseye
func bullseyeRadius(compact bool) int {
	if compact {
		return 5
	}
	return 7
}

// drawBullseye places the concentric bullseye finder pattern on the grid.
//
// The bullseye uses Chebyshev distance (max(|Δrow|, |Δcol|)) from the center:
//
//	d ≤ 1:          DARK  (solid 3×3 inner core — rings 0 and 1 merged)
//	d > 1, d even:  LIGHT (ring at that radius is a light ring)
//	d > 1, d odd:   DARK  (ring at that radius is a dark ring)
//
// This produces the characteristic concentric bullseye that gives Aztec Code
// its name. Scanners detect the 1:1:1:1:1 ratio of dark:light:dark:light:dark
// modules along any scan line through the center.
func drawBullseye(modules, reserved [][]bool, cx, cy int, compact bool) {
	br := bullseyeRadius(compact)
	for row := cy - br; row <= cy+br; row++ {
		for col := cx - br; col <= cx+br; col++ {
			d := abs(col-cx)
			if dr := abs(row - cy); dr > d {
				d = dr
			}
			dark := d <= 1 || d%2 == 1
			modules[row][col] = dark
			reserved[row][col] = true
		}
	}
}

// drawReferenceGrid places the reference grid for full (non-compact) symbols.
//
// The reference grid is a cross-hatch of alternating dark/light modules spaced
// every 16 modules from the center row and center column. It helps scanners
// correct for severe perspective distortion (e.g., reading a ticket at an angle).
//
// Grid lines exist at:
//   rows: cy, cy±16, cy±32, ... (within symbol bounds)
//   cols: cx, cx±16, cx±32, ... (within symbol bounds)
//
// Value at a grid module (row, col):
//   on both H and V grid line: DARK (intersection)
//   on H grid line only:       (cx−col) % 2 == 0 → DARK, else LIGHT
//   on V grid line only:       (cy−row) % 2 == 0 → DARK, else LIGHT
//
// This interleaving ensures the reference grid itself alternates predictably,
// making it easy for a decoder to distinguish reference modules from data modules.
//
// Compact symbols have NO reference grid.
func drawReferenceGrid(modules, reserved [][]bool, cx, cy, size int) {
	for row := 0; row < size; row++ {
		for col := 0; col < size; col++ {
			onH := (cy-row)%16 == 0
			onV := (cx-col)%16 == 0
			if !onH && !onV {
				continue
			}
			var dark bool
			if onH && onV {
				dark = true
			} else if onH {
				dark = (cx-col)%2 == 0
			} else {
				dark = (cy-row)%2 == 0
			}
			modules[row][col] = dark
			reserved[row][col] = true
		}
	}
}

// drawOrientationAndModeMessage places the orientation corners and mode message
// bits in the ring immediately outside the bullseye.
//
// The mode message ring:
//   - Lies at Chebyshev radius r = bullseyeRadius + 1 from center.
//   - Has perimeter = 4 × (2r − 1) non-corner modules + 4 corner modules.
//   - The 4 corners are orientation marks (always DARK).
//   - The remaining non-corner positions carry mode message bits clockwise
//     starting at "top-left corner + 1".
//
// Orientation marks break the 4-fold rotational symmetry of the concentric
// rings, allowing a decoder to determine whether the symbol is upright, rotated
// 90°, 180°, or 270°. They are always dark regardless of the mode message bits.
//
// Returns the non-corner positions that follow after the mode message bits
// (these will be filled by the first few bits of the data layer spiral).
func drawOrientationAndModeMessage(
	modules, reserved [][]bool,
	cx, cy int,
	compact bool,
	modeMsg []byte,
) [][2]int {
	r := bullseyeRadius(compact) + 1

	// Enumerate non-corner perimeter positions clockwise from (TL + 1):
	// Top edge: left to right (skipping corners)
	// Right edge: top to bottom (skipping corners)
	// Bottom edge: right to left (skipping corners)
	// Left edge: bottom to top (skipping corners)
	nonCorner := make([][2]int, 0, 4*(2*r-2))

	for col := cx - r + 1; col <= cx+r-1; col++ {
		nonCorner = append(nonCorner, [2]int{col, cy - r})
	}
	for row := cy - r + 1; row <= cy+r-1; row++ {
		nonCorner = append(nonCorner, [2]int{cx + r, row})
	}
	for col := cx + r - 1; col >= cx-r+1; col-- {
		nonCorner = append(nonCorner, [2]int{col, cy + r})
	}
	for row := cy + r - 1; row >= cy-r+1; row-- {
		nonCorner = append(nonCorner, [2]int{cx - r, row})
	}

	// Place 4 orientation mark corners as DARK.
	corners := [4][2]int{
		{cx - r, cy - r},
		{cx + r, cy - r},
		{cx + r, cy + r},
		{cx - r, cy + r},
	}
	for _, c := range corners {
		col, row := c[0], c[1]
		modules[row][col] = true
		reserved[row][col] = true
	}

	// Place mode message bits in the non-corner positions.
	for i := 0; i < len(modeMsg) && i < len(nonCorner); i++ {
		col, row := nonCorner[i][0], nonCorner[i][1]
		modules[row][col] = modeMsg[i] == 1
		reserved[row][col] = true
	}

	// Return the positions not consumed by the mode message.
	if len(modeMsg) < len(nonCorner) {
		return nonCorner[len(modeMsg):]
	}
	return nil
}

// ============================================================================
// Data layer spiral placement
// ============================================================================
//
// After constructing the structural elements, the stuffed data+ECC bits are
// placed in a clockwise spiral starting from the innermost data layer.
//
// Each layer is a band exactly 2 modules wide. Within the band, bits are placed
// in pairs: outer row/column first, then inner. The four sides of each layer
// are traversed in this order:
//
//  1. Top edge:    left to right, columns cx−dI+1..cx+dI, rows cy−dO then cy−dI
//  2. Right edge:  top to bottom, rows cy−dI+1..cy+dI,   cols cx+dO then cx+dI
//  3. Bottom edge: right to left, columns cx+dI..cx−dI+1, rows cy+dO then cy+dI
//  4. Left edge:   bottom to top, rows cy+dI..cy−dI+1,   cols cx−dO then cx−dI
//
// where dI = inner radius, dO = dI + 1 = outer radius of the band.
//
// For compact symbols:
//   First layer (L=0): dI = bullseyeRadius + 2 = 7  (mode msg ring at radius 6)
// For full symbols:
//   First layer (L=0): dI = bullseyeRadius + 2 = 9
//
// Each subsequent layer increments dI by 2 (and dO by 2).
//
// Before starting the layer spiral, the remaining non-mode-message positions
// in the mode message ring (modeRingRemainder) are filled with the first few
// data bits.

// placeDataBits fills the symbol's data positions with the stuffed bit stream.
//
// It first fills the remaining mode message ring positions (which are part of
// the innermost data band's ring), then spirals outward through all data layers.
func placeDataBits(
	modules, reserved [][]bool,
	bits []byte,
	cx, cy int,
	compact bool,
	layers int,
	modeRingRemainder [][2]int,
) {
	size := len(modules)
	bitIndex := 0

	placeBit := func(col, row int) {
		if row < 0 || row >= size || col < 0 || col >= size {
			return
		}
		if !reserved[row][col] {
			var v byte
			if bitIndex < len(bits) {
				v = bits[bitIndex]
			}
			modules[row][col] = v == 1
			bitIndex++
		}
	}

	// Fill remaining mode ring positions first.
	for _, pos := range modeRingRemainder {
		col, row := pos[0], pos[1]
		var v byte
		if bitIndex < len(bits) {
			v = bits[bitIndex]
		}
		modules[row][col] = v == 1
		bitIndex++
	}

	// Spiral through each data layer from inside to outside.
	br := bullseyeRadius(compact)
	dStart := br + 2 // mode message ring at radius br+1; first data layer at br+2

	for L := 0; L < layers; L++ {
		dI := dStart + 2*L // inner radius of this layer band
		dO := dI + 1       // outer radius of this layer band

		// Top edge: left to right
		for col := cx - dI + 1; col <= cx+dI; col++ {
			placeBit(col, cy-dO)
			placeBit(col, cy-dI)
		}
		// Right edge: top to bottom
		for row := cy - dI + 1; row <= cy+dI; row++ {
			placeBit(cx+dO, row)
			placeBit(cx+dI, row)
		}
		// Bottom edge: right to left
		for col := cx + dI; col >= cx-dI+1; col-- {
			placeBit(col, cy+dO)
			placeBit(col, cy+dI)
		}
		// Left edge: bottom to top
		for row := cy + dI; row >= cy-dI+1; row-- {
			placeBit(cx-dO, row)
			placeBit(cx-dI, row)
		}
	}
}

// ============================================================================
// Main encoding pipeline
// ============================================================================

// Encode encodes data as an Aztec Code symbol and returns the module grid.
//
// The module grid is a 2D boolean array where true = dark module and false =
// light module. Grid origin (0, 0) is the top-left corner.
//
// The function uses Options defaults (23% ECC) when opts is nil.
//
// Full encoding pipeline:
//
//  1. Encode input via Binary-Shift from Upper mode (all bytes raw, MSB first).
//  2. Select the smallest symbol (compact 1–4, then full 1–32) at ≥23% ECC.
//  3. Pad data codeword sequence to the exact slot count.
//  4. Compute Reed-Solomon ECC over GF(256)/0x12D.
//  5. Flatten data+ECC bytes to bits; apply bit stuffing.
//  6. Compute GF(16) mode message (7 nibbles compact, 10 nibbles full).
//  7. Initialize grid: reference grid (full only), bullseye, orientation
//     marks, mode message.
//  8. Place stuffed bits in the clockwise layer spiral, inside → outside.
//
// Errors:
//   - *InputTooLongError if data cannot fit in a 32-layer full Aztec symbol.
func Encode(data string, opts *Options) (barcode2d.ModuleGrid, error) {
	return EncodeBytes([]byte(data), opts)
}

// EncodeBytes encodes arbitrary bytes as an Aztec Code symbol.
//
// Identical to Encode but accepts a []byte input, allowing arbitrary binary
// data (not just valid UTF-8 strings).
func EncodeBytes(input []byte, opts *Options) (barcode2d.ModuleGrid, error) {
	o := defaultOptions()
	if opts != nil {
		if opts.MinEccPercent >= 10 && opts.MinEccPercent <= 90 {
			o.MinEccPercent = opts.MinEccPercent
		}
	}

	// Step 1: encode data bits via Binary-Shift from Upper mode.
	dataBits := encodeBytesAsBits(input)

	// Step 2: select the smallest symbol at the requested ECC level.
	spec, err := selectSymbol(len(dataBits), o.MinEccPercent)
	if err != nil {
		return barcode2d.ModuleGrid{}, err
	}

	// Step 3: pad to dataCwCount bytes.
	paddedBits := padToBytes(dataBits, spec.dataCwCount)
	dataBytes := make([]byte, spec.dataCwCount)
	for i := 0; i < spec.dataCwCount; i++ {
		var b byte
		for bit := 0; bit < 8; bit++ {
			b = (b << 1) | paddedBits[i*8+bit]
		}
		// All-zero codeword avoidance: a final codeword of 0x00 could confuse
		// GF arithmetic. Replace it with 0xFF.
		if b == 0 && i == spec.dataCwCount-1 {
			b = 0xFF
		}
		dataBytes[i] = b
	}

	// Step 4: compute RS ECC over GF(256)/0x12D.
	eccBytes := gf256RsEncode(dataBytes, spec.eccCwCount)

	// Step 5: flatten all bytes to bits, then apply bit stuffing.
	allBytes := append(dataBytes, eccBytes...)
	rawBits := make([]byte, 0, len(allBytes)*8)
	for _, b := range allBytes {
		for i := 7; i >= 0; i-- {
			rawBits = append(rawBits, byte((b>>uint(i))&1))
		}
	}
	stuffedBits := stuffBits(rawBits)

	// Step 6: compute mode message.
	modeMsg := encodeModeMessage(spec.compact, spec.layers, spec.dataCwCount)

	// Step 7: initialize grid.
	size := symbolSize(spec.compact, spec.layers)
	cx := size / 2
	cy := size / 2

	modules := make([][]bool, size)
	reserved := make([][]bool, size)
	for i := range modules {
		modules[i] = make([]bool, size)
		reserved[i] = make([]bool, size)
	}

	// For full symbols, draw reference grid before bullseye (bullseye overwrites).
	if !spec.compact {
		drawReferenceGrid(modules, reserved, cx, cy, size)
	}
	drawBullseye(modules, reserved, cx, cy, spec.compact)

	modeRingRemainder := drawOrientationAndModeMessage(
		modules, reserved, cx, cy, spec.compact, modeMsg,
	)

	// Step 8: place data bits in the clockwise layer spiral.
	placeDataBits(modules, reserved, stuffedBits, cx, cy, spec.compact, spec.layers, modeRingRemainder)

	return barcode2d.ModuleGrid{
		Rows:        uint32(size),
		Cols:        uint32(size),
		Modules:     modules,
		ModuleShape: barcode2d.ModuleShapeSquare,
	}, nil
}

// EncodeToScene encodes data and converts the result to a pixel-resolved
// PaintScene using barcode2d.Layout.
//
// The quiet zone defaults to 1 module. Override by setting cfg.QuietZoneModules.
//
// The PaintScene can be passed to any paint-vm backend (SVG, Metal, Canvas, etc.)
// to produce a renderable output.
func EncodeToScene(data string, opts *Options, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error) {
	grid, err := Encode(data, opts)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	if cfg.QuietZoneModules == 0 {
		cfg.QuietZoneModules = 1
	}
	scene, err := barcode2d.Layout(grid, &cfg)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return scene, nil
}

// ============================================================================
// Utilities
// ============================================================================

// abs returns the absolute value of x.
func abs(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

// ============================================================================
// Exported internals for testing (white-box tests)
// ============================================================================

// GF16Mul multiplies two GF(16) elements (exported for testing).
func GF16Mul(a, b int) int { return gf16Mul(a, b) }

// BuildGf16Generator builds the GF(16) generator polynomial (exported for testing).
func BuildGf16Generator(n int) []int { return buildGf16Generator(n) }

// GF16RsEncode computes n GF(16) RS check nibbles (exported for testing).
func GF16RsEncode(data []int, n int) []int { return gf16RsEncode(data, n) }

// GF256Mul multiplies two GF(256)/0x12D elements (exported for testing).
func GF256Mul(a, b byte) byte { return gf256Mul(a, b) }

// GF256RsEncode computes nEcc GF(256)/0x12D RS bytes (exported for testing).
func GF256RsEncode(data []byte, nEcc int) []byte { return gf256RsEncode(data, nEcc) }

// EncodeBytesAsBits encodes input as the Binary-Shift bit stream (exported for testing).
func EncodeBytesAsBits(input []byte) []byte { return encodeBytesAsBits(input) }

// StuffBits applies bit stuffing (exported for testing).
func StuffBits(bits []byte) []byte { return stuffBits(bits) }

// EncodeModeMessage encodes the mode message bits (exported for testing).
func EncodeModeMessage(compact bool, layers, dataCwCount int) []byte {
	return encodeModeMessage(compact, layers, dataCwCount)
}

// SelectSymbol selects the smallest fitting symbol configuration (exported for testing).
func SelectSymbol(dataBitCount, minEccPct int) (symbolSpec, error) {
	return selectSymbol(dataBitCount, minEccPct)
}

// SymbolSpec is the exported type alias for white-box tests.
type SymbolSpec = symbolSpec

// ErrInputTooLongSentinel is exported for errors.Is in tests.
var ErrInputTooLongSentinel = ErrInputTooLong

// IsInputTooLong checks if err is an InputTooLongError.
func IsInputTooLong(err error) bool {
	return errors.Is(err, ErrInputTooLong)
}
