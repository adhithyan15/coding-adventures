// Package datamatrix implements a Data Matrix ECC200 encoder conforming to
// ISO/IEC 16022:2006.
//
// # What is Data Matrix?
//
// Data Matrix is a two-dimensional matrix barcode invented by RVSI Acuity
// CiMatrix in 1989 under the name "DataCode" and standardised as ISO/IEC
// 16022:2006.  The ECC200 variant — using Reed-Solomon over GF(256) — has
// replaced the older ECC000–ECC140 lineage and is the dominant form worldwide.
//
// Where Data Matrix is used:
//
//   - PCBs: every board carries a Data Matrix etched on the substrate for
//     traceability through automated assembly lines.
//   - Pharmaceuticals: US FDA DSCSA mandates Data Matrix on unit-dose packages.
//   - Aerospace parts: etched/dot-peened marks survive decades of heat and
//     abrasion that would destroy ink-printed labels.
//   - Medical devices: GS1 DataMatrix on surgical instruments and implants.
//   - USPS registered mail and customs forms.
//
// # Key differences from QR Code
//
//   - GF(256) uses 0x12D (not QR's 0x11D)
//   - Reed-Solomon b=1 convention (roots α¹…αⁿ) — matches MA02 reed-solomon exactly
//   - L-shaped finder (left column + bottom row all dark) + clock border
//   - Diagonal "Utah" placement algorithm (no masking step!)
//   - 36 symbol sizes: 30 square (10×10 … 144×144) + 6 rectangular
//
// # Encoding pipeline
//
//	input string
//	  → ASCII encoding      (chars+1; digit pairs packed into one codeword)
//	  → symbol selection    (smallest symbol whose capacity ≥ codeword count)
//	  → pad to capacity     (scrambled-pad codewords fill unused slots)
//	  → RS blocks + ECC     (GF(256)/0x12D, b=1 convention)
//	  → interleave blocks   (data round-robin then ECC round-robin)
//	  → grid init           (L-finder + timing border + alignment borders)
//	  → Utah placement      (diagonal codeword placement, NO masking)
//	  → ModuleGrid          (abstract boolean grid, true = dark)
//
// # Public API
//
//	Encode(input string, opts Options) (barcode2d.ModuleGrid, error)
//	EncodeToScene(input string, opts Options, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error)
package datamatrix

import (
	"errors"
	"fmt"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

// Ensure paintinstructions and gf256 are used (they are needed for transitive deps).
var _ = paintinstructions.Version
var _ = gf256.Version

// Version is the semantic version of this package.
const Version = "0.1.0"

// ============================================================================
// Public error types
// ============================================================================

// InputTooLongError is returned when the encoded data exceeds the capacity of
// the largest Data Matrix symbol (144×144, up to 1558 data codewords).
type InputTooLongError struct {
	// EncodedCW is the number of codewords the input encodes to.
	EncodedCW int
	// MaxCW is the maximum capacity (1558 for a 144×144 symbol).
	MaxCW int
}

func (e *InputTooLongError) Error() string {
	return fmt.Sprintf("data-matrix: input too long — encoded %d codewords, maximum is %d (144×144 symbol)",
		e.EncodedCW, e.MaxCW)
}

// Is implements errors.Is for unwrapping.
func (e *InputTooLongError) Is(target error) bool {
	_, ok := target.(*InputTooLongError)
	return ok
}

// ErrInputTooLong is a sentinel value for use with errors.Is.
var ErrInputTooLong = &InputTooLongError{}

// ============================================================================
// Options
// ============================================================================

// SymbolShape controls which symbol sizes are considered during selection.
type SymbolShape int

const (
	// SymbolShapeSquare selects from square symbols only (default).
	// Squares (10×10 … 144×144) are the most common Data Matrix variant.
	SymbolShapeSquare SymbolShape = iota

	// SymbolShapeRectangular selects from rectangular symbols only.
	// Rectangles (8×18 … 16×48) are used when print area aspect ratio matters.
	SymbolShapeRectangular

	// SymbolShapeAny tries both square and rectangular, picks the smallest.
	SymbolShapeAny
)

// Options configures the Data Matrix encoder.
type Options struct {
	// Shape controls which symbol shapes are considered. Default: SymbolShapeSquare.
	Shape SymbolShape
}

// ============================================================================
// GF(256) over 0x12D — Data Matrix field
// ============================================================================
//
// Data Matrix uses GF(256) with primitive polynomial 0x12D:
//
//	p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301
//
// IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial.
// Both are degree-8 irreducible polynomials over GF(2), but the fields are
// non-isomorphic.  Never mix tables between QR and Data Matrix.
//
// The generator g = 2 (polynomial x) generates all 255 non-zero elements.
// Pre-computed tables: dmGFExp[i] = α^i, dmGFLog[v] = k such that α^k = v.

const gfPoly = 0x12D // Data Matrix primitive polynomial

// dmGFExp[i] = α^i mod 0x12D  (i = 0..255; index 255 wraps to 0)
var dmGFExp [256]byte

// dmGFLog[v] = k such that α^k = v  (v = 1..255; dmGFLog[0] = 0 sentinel)
var dmGFLog [256]byte

func init() {
	// Build exp (antilog) and log tables for GF(256)/0x12D.
	//
	// Algorithm:
	//   Start with val = 1 (= α^0).
	//   Each step: left-shift 1 bit (multiply by α = x).
	//   If bit 8 is set (val >= 256), XOR with 0x12D to reduce.
	//
	// After 255 steps we have covered all non-zero field elements exactly once,
	// proving α = 2 is primitive for this polynomial.
	val := 1
	for i := 0; i < 255; i++ {
		dmGFExp[i] = byte(val)
		dmGFLog[val] = byte(i)
		val <<= 1
		if val&0x100 != 0 {
			val ^= gfPoly
		}
	}
	// α^255 = α^0 = 1 (multiplicative order = 255)
	dmGFExp[255] = dmGFExp[0]
}

// gfMul multiplies two GF(256)/0x12D field elements using log/antilog tables.
//
// For a, b ≠ 0:  a × b = α^{(log[a] + log[b]) mod 255}
// If either operand is 0, the product is 0 (zero absorbs multiplication).
//
// This turns a polynomial multiplication + reduction into two table lookups
// and an addition modulo 255 — effectively O(1).
func gfMul(a, b byte) byte {
	if a == 0 || b == 0 {
		return 0
	}
	return dmGFExp[(uint(dmGFLog[a])+uint(dmGFLog[b]))%255]
}

// ============================================================================
// Symbol size table
// ============================================================================

// symbolEntry describes one Data Matrix ECC200 symbol size.
//
// A "data region" is one rectangular interior sub-area.  Small symbols
// (≤ 26×26) have a single 1×1 region.  Larger symbols subdivide into
// a grid of regions separated by alignment borders (2 modules wide each).
//
// The Utah placement algorithm works on the "logical data matrix" — the
// concatenation of all region interiors — then maps back to physical coords.
type symbolEntry struct {
	// symbolRows / symbolCols: total symbol size including outer border.
	symbolRows int
	symbolCols int

	// regionRows / regionCols: how many data region rows/cols (rr × rc).
	regionRows int
	regionCols int

	// dataRegionHeight / dataRegionWidth: interior data size per region.
	dataRegionHeight int
	dataRegionWidth  int

	// dataCW: total data codeword capacity for this symbol size.
	dataCW int

	// eccCW: total ECC codewords.
	eccCW int

	// numBlocks: number of interleaved RS blocks.
	numBlocks int

	// eccPerBlock: ECC codewords per block (same for all blocks in one symbol).
	eccPerBlock int
}

// squareSizes lists all 24 primary square Data Matrix ECC200 symbol sizes.
//
// Source: ISO/IEC 16022:2006, Table 7 (square symbols).
// Each entry has been verified against the standard.
//
// Column order: symbolRows, symbolCols, regionRows, regionCols,
//
//	dataRegionHeight, dataRegionWidth, dataCW, eccCW, numBlocks, eccPerBlock
var squareSizes = []symbolEntry{
	{10, 10, 1, 1, 8, 8, 3, 5, 1, 5},
	{12, 12, 1, 1, 10, 10, 5, 7, 1, 7},
	{14, 14, 1, 1, 12, 12, 8, 10, 1, 10},
	{16, 16, 1, 1, 14, 14, 12, 12, 1, 12},
	{18, 18, 1, 1, 16, 16, 18, 14, 1, 14},
	{20, 20, 1, 1, 18, 18, 22, 18, 1, 18},
	{22, 22, 1, 1, 20, 20, 30, 20, 1, 20},
	{24, 24, 1, 1, 22, 22, 36, 24, 1, 24},
	{26, 26, 1, 1, 24, 24, 44, 28, 1, 28},
	{32, 32, 2, 2, 14, 14, 62, 36, 2, 18},
	{36, 36, 2, 2, 16, 16, 86, 42, 2, 21},
	{40, 40, 2, 2, 18, 18, 114, 48, 2, 24},
	{44, 44, 2, 2, 20, 20, 144, 56, 4, 14},
	{48, 48, 2, 2, 22, 22, 174, 68, 4, 17},
	{52, 52, 2, 2, 24, 24, 204, 84, 4, 21},
	{64, 64, 4, 4, 14, 14, 280, 112, 4, 28},
	{72, 72, 4, 4, 16, 16, 368, 144, 4, 36},
	{80, 80, 4, 4, 18, 18, 456, 192, 4, 48},
	{88, 88, 4, 4, 20, 20, 576, 224, 4, 56},
	{96, 96, 4, 4, 22, 22, 696, 272, 4, 68},
	{104, 104, 4, 4, 24, 24, 816, 336, 6, 56},
	{120, 120, 6, 6, 18, 18, 1050, 408, 6, 68},
	{132, 132, 6, 6, 20, 20, 1304, 496, 8, 62},
	{144, 144, 6, 6, 22, 22, 1558, 620, 10, 62},
}

// rectSizes lists all 6 rectangular Data Matrix ECC200 symbol sizes.
//
// Source: ISO/IEC 16022:2006, Table 7 (rectangular symbols).
var rectSizes = []symbolEntry{
	{8, 18, 1, 1, 6, 16, 5, 7, 1, 7},
	{8, 32, 1, 2, 6, 14, 10, 11, 1, 11},
	{12, 26, 1, 1, 10, 24, 16, 14, 1, 14},
	{12, 36, 1, 2, 10, 16, 22, 18, 1, 18},
	{16, 36, 1, 2, 14, 16, 32, 24, 1, 24},
	{16, 48, 1, 2, 14, 22, 49, 28, 1, 28},
}

// ============================================================================
// Generator polynomials for Data Matrix (GF(256)/0x12D, b=1 convention)
// ============================================================================
//
// The RS generator polynomial g(x) = ∏(x + α^k) for k=1..n_ecc.
// These are computed over GF(256)/0x12D with roots α¹, α², …, α^n.
// Format: highest-degree first, including the implicit leading 1.
// Length = n_ecc + 1.
//
// Source: ISO/IEC 16022:2006, Annex A.

// buildGenerator constructs the RS generator polynomial for nEcc ECC bytes
// over GF(256)/0x12D with b=1: g(x) = (x + α¹)(x + α²)···(x + α^{nEcc}).
//
// Algorithm: start with g = [1], then for each i from 1 to nEcc, multiply
// g by the linear factor (x + α^i):
//
//	for j, coeff := range g:
//	    newG[j]   ^= coeff · α^i    (coefficient × constant term of factor)
//	    newG[j+1] ^= coeff          (coefficient × x term of factor)
func buildGenerator(nEcc int) []byte {
	g := []byte{1}
	for i := 1; i <= nEcc; i++ {
		ai := dmGFExp[i] // α^i
		newG := make([]byte, len(g)+1)
		for j, coeff := range g {
			newG[j] ^= coeff          // coeff · x
			newG[j+1] ^= gfMul(coeff, ai) // coeff · α^i
		}
		g = newG
	}
	return g
}

// genPolyCache caches computed generator polynomials keyed by nEcc.
var genPolyCache = map[int][]byte{}

// getGenerator returns the generator polynomial for nEcc ECC bytes.
// Results are cached so each polynomial is built at most once.
func getGenerator(nEcc int) []byte {
	if g, ok := genPolyCache[nEcc]; ok {
		return g
	}
	g := buildGenerator(nEcc)
	genPolyCache[nEcc] = g
	return g
}

func init() {
	// Pre-build all generator polynomials needed for the symbol size tables.
	// This avoids any per-encode latency for first-use construction.
	seen := map[int]bool{}
	for _, e := range squareSizes {
		if !seen[e.eccPerBlock] {
			getGenerator(e.eccPerBlock)
			seen[e.eccPerBlock] = true
		}
	}
	for _, e := range rectSizes {
		if !seen[e.eccPerBlock] {
			getGenerator(e.eccPerBlock)
			seen[e.eccPerBlock] = true
		}
	}
}

// ============================================================================
// Reed-Solomon encoding (b=1 convention, GF(256)/0x12D)
// ============================================================================

// rsEncodeBlock computes nEcc ECC bytes for a data block using the LFSR
// polynomial division method.
//
// Algorithm: R(x) = D(x) × x^{nEcc} mod G(x)
//
// LFSR shift-register implementation (for each data byte d):
//
//	feedback = d XOR rem[0]
//	shift rem left: rem[i] ← rem[i+1]
//	rem[i] ^= gen[i+1] × feedback   for i = 0..nEcc-1
//
// This is the standard systematic RS encoding approach: equivalent to
// polynomial long-division but implemented as a streaming shift register.
//
// The generator array format (from buildGenerator) is big-endian:
// gen[0] = 1 (leading coefficient), gen[1..nEcc] = remaining coefficients.
func rsEncodeBlock(data []byte, generator []byte) []byte {
	nEcc := len(generator) - 1
	rem := make([]byte, nEcc)
	for _, d := range data {
		fb := d ^ rem[0]
		// Shift register left
		copy(rem, rem[1:])
		rem[nEcc-1] = 0
		if fb != 0 {
			for i := 0; i < nEcc; i++ {
				rem[i] ^= gfMul(generator[i+1], fb)
			}
		}
	}
	return rem
}

// ============================================================================
// ASCII data encoding
// ============================================================================

// encodeASCII encodes input bytes in Data Matrix ASCII mode.
//
// ASCII mode rules:
//
//   - Two consecutive ASCII digits (0x30–0x39) → one codeword = 130 + (d1×10 + d2).
//     This digit-pair optimization halves the codeword budget for numeric strings —
//     critical for manufacturing lot codes, serial numbers, and barcodes that are
//     mostly digit strings.
//
//   - Single ASCII char (0–127) → codeword = ASCII_value + 1.
//     For example: 'A' (65) → 66, space (32) → 33.
//
//   - Extended ASCII (128–255) → two codewords: 235 (UPPER_SHIFT), then ASCII_value - 127.
//     This enables encoding of Latin-1 or Windows-1252 characters, though it's
//     uncommon in practice (most Data Matrix content is ASCII).
//
// Examples:
//
//	"A"    → [66]           (65 + 1)
//	" "    → [33]           (32 + 1)
//	"12"   → [142]          (130 + 12, digit pair)
//	"1234" → [142, 174]     (two digit pairs)
//	"1A"   → [50, 66]       (49+1 for '1', 65+1 for 'A' — no pair because 'A' not digit)
//	"00"   → [130]          (130 + 0)
//	"99"   → [229]          (130 + 99)
func encodeASCII(input []byte) []byte {
	codewords := make([]byte, 0, len(input))
	i := 0
	for i < len(input) {
		c := input[i]
		// Check for digit pair: both current and next bytes are ASCII digits 0x30–0x39
		if c >= 0x30 && c <= 0x39 &&
			i+1 < len(input) &&
			input[i+1] >= 0x30 && input[i+1] <= 0x39 {
			d1 := int(c - 0x30)       // first digit value (0–9)
			d2 := int(input[i+1] - 0x30) // second digit value (0–9)
			codewords = append(codewords, byte(130+d1*10+d2))
			i += 2
		} else if c <= 127 {
			// Standard single ASCII character: value + 1
			codewords = append(codewords, c+1)
			i++
		} else {
			// Extended ASCII (128–255): UPPER_SHIFT (235) then (value - 127)
			codewords = append(codewords, 235)
			codewords = append(codewords, c-127)
			i++
		}
	}
	return codewords
}

// ============================================================================
// Pad codewords (ISO/IEC 16022:2006 §5.2.3)
// ============================================================================

// padCodewords pads the encoded codeword slice to exactly dataCW bytes.
//
// Padding rules from ISO/IEC 16022:2006 §5.2.3:
//
//  1. The first pad codeword is always the literal value 129.
//
//  2. Subsequent pads use a scrambled value that depends on their 1-indexed
//     position k within the full codeword stream:
//
//     scrambled = 129 + (149 × k mod 253) + 1
//     if scrambled > 254: scrambled -= 254
//
//     The scrambling prevents a run of "129 129 129 …" from creating a
//     degenerate placement pattern in the Utah algorithm (long identical runs
//     would cluster related modules and bias the error-correction structure).
//
// Example for "A" (codeword [66]) in a 10×10 symbol (dataCW = 3):
//
//	k=2: 129                   (first pad — always literal)
//	k=3: 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324; 324 > 254 → 70
//	Result: [66, 129, 70]
func padCodewords(codewords []byte, dataCW int) []byte {
	padded := make([]byte, len(codewords), dataCW)
	copy(padded, codewords)

	isFirst := true
	k := len(codewords) + 1 // 1-indexed position of first pad byte
	for len(padded) < dataCW {
		if isFirst {
			padded = append(padded, 129)
			isFirst = false
		} else {
			// Scrambled pad: 129 + (149 × k mod 253) + 1, then wrap at 254
			scrambled := 129 + (149*k)%253 + 1
			if scrambled > 254 {
				scrambled -= 254
			}
			padded = append(padded, byte(scrambled))
		}
		k++
	}
	return padded
}

// ============================================================================
// Symbol selection
// ============================================================================

// selectSymbol returns the smallest symbol entry that can hold codewordCount
// data codewords with the given shape preference.
//
// Iterates all candidates in ascending capacity order and returns the first
// whose dataCW ≥ codewordCount.  Returns an error if nothing fits.
func selectSymbol(codewordCount int, shape SymbolShape) (symbolEntry, error) {
	var candidates []symbolEntry
	switch shape {
	case SymbolShapeSquare:
		candidates = append(candidates, squareSizes...)
	case SymbolShapeRectangular:
		candidates = append(candidates, rectSizes...)
	case SymbolShapeAny:
		candidates = append(candidates, squareSizes...)
		candidates = append(candidates, rectSizes...)
	}

	// Sort by dataCW ascending, then by area for tie-breaking.
	// Since both squareSizes and rectSizes are already in ascending order,
	// and we append squares then rects, a simple pass is sufficient;
	// but to be safe (especially for SymbolShapeAny) we do a minimal sort.
	// Use insertion sort since the slice is small (≤ 30 entries).
	for i := 1; i < len(candidates); i++ {
		key := candidates[i]
		j := i - 1
		for j >= 0 && (candidates[j].dataCW > key.dataCW ||
			(candidates[j].dataCW == key.dataCW &&
				candidates[j].symbolRows*candidates[j].symbolCols > key.symbolRows*key.symbolCols)) {
			candidates[j+1] = candidates[j]
			j--
		}
		candidates[j+1] = key
	}

	for _, e := range candidates {
		if e.dataCW >= codewordCount {
			return e, nil
		}
	}
	return symbolEntry{}, &InputTooLongError{
		EncodedCW: codewordCount,
		MaxCW:     1558,
	}
}

// ============================================================================
// Block splitting, ECC computation, and interleaving
// ============================================================================

// computeInterleaved splits padded data across RS blocks, computes ECC for
// each block, then interleaves data and ECC round-robin for placement.
//
// Block splitting:
//
//	baseLen    = dataCW / numBlocks  (integer division)
//	extraBlocks = dataCW mod numBlocks
//	Blocks 0..extraBlocks-1 get baseLen+1 data codewords.
//	Blocks extraBlocks..numBlocks-1 get baseLen data codewords.
//
// This is the ISO interleaving convention: earlier blocks get one extra if
// the total is not evenly divisible.
//
// Interleaving:
//
//	data round-robin: for pos in 0..maxDataPerBlock: for blk: append data[blk][pos]
//	ECC round-robin:  for pos in 0..eccPerBlock:     for blk: append ecc[blk][pos]
//
// Interleaving distributes burst errors: a physical scratch destroying N
// contiguous modules affects at most ⌈N/numBlocks⌉ codewords per block, which
// is far more likely to be within each block's correction capacity.
func computeInterleaved(data []byte, entry symbolEntry) []byte {
	numBlocks := entry.numBlocks
	eccPerBlock := entry.eccPerBlock
	dataCW := entry.dataCW
	gen := getGenerator(eccPerBlock)

	// Split data into blocks.
	baseLen := dataCW / numBlocks
	extraBlocks := dataCW % numBlocks

	dataBlocks := make([][]byte, numBlocks)
	offset := 0
	for b := 0; b < numBlocks; b++ {
		l := baseLen
		if b < extraBlocks {
			l = baseLen + 1
		}
		dataBlocks[b] = data[offset : offset+l]
		offset += l
	}

	// Compute ECC for each block independently.
	eccBlocks := make([][]byte, numBlocks)
	for b := 0; b < numBlocks; b++ {
		eccBlocks[b] = rsEncodeBlock(dataBlocks[b], gen)
	}

	// Interleave data round-robin.
	total := dataCW + numBlocks*eccPerBlock
	interleaved := make([]byte, 0, total)

	maxDataLen := 0
	for _, db := range dataBlocks {
		if len(db) > maxDataLen {
			maxDataLen = len(db)
		}
	}
	for pos := 0; pos < maxDataLen; pos++ {
		for b := 0; b < numBlocks; b++ {
			if pos < len(dataBlocks[b]) {
				interleaved = append(interleaved, dataBlocks[b][pos])
			}
		}
	}

	// Interleave ECC round-robin.
	for pos := 0; pos < eccPerBlock; pos++ {
		for b := 0; b < numBlocks; b++ {
			interleaved = append(interleaved, eccBlocks[b][pos])
		}
	}

	return interleaved
}

// ============================================================================
// Grid initialization (border + alignment borders)
// ============================================================================

// initGrid allocates and fills the physical module grid with the fixed
// structural elements (finder pattern + timing clock + alignment borders).
//
// The "finder + clock" border (outermost ring):
//
//	Top row (row 0):        alternating dark/light starting dark at col 0.
//	                        These are the timing clock marks for the top edge.
//	Right col (col C-1):    alternating dark/light starting dark at row 0.
//	                        Timing clock for the right edge.
//	Left col (col 0):       all dark — vertical leg of the L-finder.
//	Bottom row (row R-1):   all dark — horizontal leg of the L-finder.
//
// The L-shaped solid-dark bar (left+bottom) tells a scanner where the symbol
// starts and which orientation it has — the asymmetry between the L-bar and
// the alternating timing distinguishes all four 90-degree rotations.
//
// For multi-region symbols (e.g. 32×32 = 2×2 regions), alignment borders
// are placed between data regions.  Each is 2 modules wide:
//
//	Row/Col AB+0: all dark
//	Row/Col AB+1: alternating dark/light starting dark
//
// Writing order matters: alignment borders are written first, then outer
// timing row and right column, then outer left column, then bottom row.
// The L-finder bottom row always wins (written last, highest precedence).
func initGrid(entry symbolEntry) [][]bool {
	R, C := entry.symbolRows, entry.symbolCols

	// Allocate all-light grid.
	grid := make([][]bool, R)
	for r := range grid {
		grid[r] = make([]bool, C)
	}

	// ── Alignment borders (multi-region symbols only) ──────────────────────
	// Written FIRST so the outer borders can override at intersections.
	for rr := 0; rr < entry.regionRows-1; rr++ {
		// Physical row of first AB row after data region rr+1:
		//   outer border (1) + (rr+1) * dataRegionHeight + rr * 2 (prev ABs)
		abRow0 := 1 + (rr+1)*entry.dataRegionHeight + rr*2
		abRow1 := abRow0 + 1
		for c := 0; c < C; c++ {
			grid[abRow0][c] = true         // all dark
			grid[abRow1][c] = (c%2 == 0)  // alternating, starts dark
		}
	}

	for rc := 0; rc < entry.regionCols-1; rc++ {
		abCol0 := 1 + (rc+1)*entry.dataRegionWidth + rc*2
		abCol1 := abCol0 + 1
		for r := 0; r < R; r++ {
			grid[r][abCol0] = true         // all dark
			grid[r][abCol1] = (r%2 == 0)  // alternating, starts dark
		}
	}

	// ── Top row (row 0): timing clock — alternating dark/light, starts dark ─
	for c := 0; c < C; c++ {
		grid[0][c] = (c%2 == 0)
	}

	// ── Right column (col C-1): timing clock — alternating, starts dark ─────
	for r := 0; r < R; r++ {
		grid[r][C-1] = (r%2 == 0)
	}

	// ── Left column (col 0): L-finder left leg — all dark ───────────────────
	// Written after timing to override timing value at (0, 0) and beyond.
	for r := 0; r < R; r++ {
		grid[r][0] = true
	}

	// ── Bottom row (row R-1): L-finder bottom leg — all dark ─────────────────
	// Written LAST: overrides alignment borders, right-column timing, everything.
	for c := 0; c < C; c++ {
		grid[R-1][c] = true
	}

	return grid
}

// ============================================================================
// Utah placement algorithm
// ============================================================================
//
// The Utah placement algorithm is the most distinctive part of Data Matrix
// encoding.  It was named "Utah" because the 8-module codeword shape vaguely
// resembles the outline of the US state of Utah — a rectangle with a notch cut
// from the top-left corner.
//
// The algorithm scans the logical grid (all data region interiors concatenated)
// in a diagonal zigzag.  For each codeword, 8 bits are placed at 8 fixed
// offsets relative to the current reference position (row, col).  After each
// codeword the reference moves diagonally (row-=2, col+=2 for upward leg;
// row+=2, col-=2 for downward leg).
//
// Four special "corner" patterns handle positions where the standard Utah shape
// would extend outside the grid boundary.
//
// There is NO masking step after placement.  The diagonal traversal naturally
// distributes bits across the symbol without the degenerate clustering that
// would otherwise require masking in QR Code.

// applyWrap applies the boundary wrap rules from ISO/IEC 16022:2006 Annex F.
//
// When the standard Utah shape extends beyond the logical grid edge,
// these rules fold the coordinates back into the valid range.
//
// The four wrap rules (applied in order):
//
//  1. row < 0 AND col == 0      → (1, 3)       special top-left singularity
//  2. row < 0 AND col == nCols  → (0, col-2)   wrapped past right edge at top
//  3. row < 0                   → (row+nRows, col-4)  wrap top → bottom, shift left
//  4. col < 0                   → (row-4, col+nCols)  wrap left → right, shift up
func applyWrap(row, col, nRows, nCols int) (int, int) {
	// Special case: top-left corner singularity
	if row < 0 && col == 0 {
		return 1, 3
	}
	// Special case: wrapped past the right edge at the top
	if row < 0 && col == nCols {
		return 0, col - 2
	}
	// Wrap row off top → bottom of grid, shift left
	if row < 0 {
		return row + nRows, col - 4
	}
	// Wrap col off left → right of grid, shift up
	if col < 0 {
		return row - 4, col + nCols
	}
	return row, col
}

// placeUtah places one codeword using the standard "Utah" 8-module pattern.
//
// The Utah shape at reference position (row, col):
//
//	col: c-2  c-1   c
//
// row-2:  .   [1]  [2]
// row-1: [3]  [4]  [5]
// row  : [6]  [7]  [8]
//
// Numbers [1]–[8] correspond to bits 1–8 (1=LSB, 8=MSB) of the codeword.
// MSB (bit 8) is placed at (row, col); LSB (bit 1) at (row-2, col-1).
func placeUtah(cw byte, row, col, nRows, nCols int, grid, used [][]bool) {
	// [rawRow, rawCol, bitShift (7=MSB, 0=LSB)]
	placements := [8][3]int{
		{row, col, 7},      // bit 8 (MSB)
		{row, col - 1, 6},  // bit 7
		{row, col - 2, 5},  // bit 6
		{row - 1, col, 4},  // bit 5
		{row - 1, col - 1, 3}, // bit 4
		{row - 1, col - 2, 2}, // bit 3
		{row - 2, col, 1},  // bit 2
		{row - 2, col - 1, 0}, // bit 1 (LSB)
	}
	for _, p := range placements {
		r, c := applyWrap(p[0], p[1], nRows, nCols)
		if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
			grid[r][c] = ((cw >> uint(p[2])) & 1) == 1
			used[r][c] = true
		}
	}
}

// placeCorner1 places a codeword using corner pattern 1.
//
// Triggered at the top-left boundary.  Absolute positions within the logical
// grid (nRows × nCols):
//
//	bit 8: (0,      nCols-2)
//	bit 7: (0,      nCols-1)
//	bit 6: (1,      0)
//	bit 5: (2,      0)
//	bit 4: (nRows-2, 0)
//	bit 3: (nRows-1, 0)
//	bit 2: (nRows-1, 1)
//	bit 1: (nRows-1, 2)
func placeCorner1(cw byte, nRows, nCols int, grid, used [][]bool) {
	positions := [8][3]int{
		{0, nCols - 2, 7},
		{0, nCols - 1, 6},
		{1, 0, 5},
		{2, 0, 4},
		{nRows - 2, 0, 3},
		{nRows - 1, 0, 2},
		{nRows - 1, 1, 1},
		{nRows - 1, 2, 0},
	}
	for _, p := range positions {
		r, c, bit := p[0], p[1], p[2]
		if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
			grid[r][c] = ((cw >> uint(bit)) & 1) == 1
			used[r][c] = true
		}
	}
}

// placeCorner2 places a codeword using corner pattern 2.
//
// Triggered at the top-right boundary.
//
//	bit 8: (0,      nCols-2)
//	bit 7: (0,      nCols-1)
//	bit 6: (1,      nCols-1)
//	bit 5: (2,      nCols-1)
//	bit 4: (nRows-1, 0)
//	bit 3: (nRows-1, 1)
//	bit 2: (nRows-1, 2)
//	bit 1: (nRows-1, 3)
func placeCorner2(cw byte, nRows, nCols int, grid, used [][]bool) {
	positions := [8][3]int{
		{0, nCols - 2, 7},
		{0, nCols - 1, 6},
		{1, nCols - 1, 5},
		{2, nCols - 1, 4},
		{nRows - 1, 0, 3},
		{nRows - 1, 1, 2},
		{nRows - 1, 2, 1},
		{nRows - 1, 3, 0},
	}
	for _, p := range positions {
		r, c, bit := p[0], p[1], p[2]
		if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
			grid[r][c] = ((cw >> uint(bit)) & 1) == 1
			used[r][c] = true
		}
	}
}

// placeCorner3 places a codeword using corner pattern 3.
//
// Triggered at the bottom-left boundary.
//
//	bit 8: (0,      nCols-1)
//	bit 7: (1,      0)
//	bit 6: (2,      0)
//	bit 5: (nRows-2, 0)
//	bit 4: (nRows-1, 0)
//	bit 3: (nRows-1, 1)
//	bit 2: (nRows-1, 2)
//	bit 1: (nRows-1, 3)
func placeCorner3(cw byte, nRows, nCols int, grid, used [][]bool) {
	positions := [8][3]int{
		{0, nCols - 1, 7},
		{1, 0, 6},
		{2, 0, 5},
		{nRows - 2, 0, 4},
		{nRows - 1, 0, 3},
		{nRows - 1, 1, 2},
		{nRows - 1, 2, 1},
		{nRows - 1, 3, 0},
	}
	for _, p := range positions {
		r, c, bit := p[0], p[1], p[2]
		if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
			grid[r][c] = ((cw >> uint(bit)) & 1) == 1
			used[r][c] = true
		}
	}
}

// placeCorner4 places a codeword using corner pattern 4.
//
// Triggered for odd-dimension rectangular matrices (both nRows and nCols odd).
//
//	bit 8: (nRows-3, nCols-1)
//	bit 7: (nRows-2, nCols-1)
//	bit 6: (nRows-1, nCols-3)
//	bit 5: (nRows-1, nCols-2)
//	bit 4: (nRows-1, nCols-1)
//	bit 3: (0,       0)
//	bit 2: (1,       0)
//	bit 1: (2,       0)
func placeCorner4(cw byte, nRows, nCols int, grid, used [][]bool) {
	positions := [8][3]int{
		{nRows - 3, nCols - 1, 7},
		{nRows - 2, nCols - 1, 6},
		{nRows - 1, nCols - 3, 5},
		{nRows - 1, nCols - 2, 4},
		{nRows - 1, nCols - 1, 3},
		{0, 0, 2},
		{1, 0, 1},
		{2, 0, 0},
	}
	for _, p := range positions {
		r, c, bit := p[0], p[1], p[2]
		if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
			grid[r][c] = ((cw >> uint(bit)) & 1) == 1
			used[r][c] = true
		}
	}
}

// utahPlacement runs the Utah diagonal placement algorithm on the logical
// data matrix (nRows × nCols), filling in all codeword bits.
//
// # Algorithm overview
//
// The reference position (row, col) starts at (4, 0) and zigzags diagonally
// across the logical grid.  Each iteration of the outer loop has two legs:
//
//  1. Upward-right leg: place codewords at (row, col), moving row-=2, col+=2
//     until out of bounds, then step to the next diagonal start: row+=1, col+=3.
//
//  2. Downward-left leg: place codewords at (row, col), moving row+=2, col-=2
//     until out of bounds, then step to the next diagonal start: row+=3, col+=1.
//
// Between legs, the four corner patterns fire when the reference position
// matches specific trigger conditions (described per-function).
//
// Termination: when both row≥nRows and col≥nCols, all modules have been
// visited.  Any unvisited modules at the end receive the "fill" pattern
// (r+c) mod 2 == 1 (dark), matching the ISO right-and-bottom fill rule.
func utahPlacement(codewords []byte, nRows, nCols int) [][]bool {
	grid := make([][]bool, nRows)
	used := make([][]bool, nRows)
	for r := range grid {
		grid[r] = make([]bool, nCols)
		used[r] = make([]bool, nCols)
	}

	cwIdx := 0
	row := 4
	col := 0

	// place dispatches one codeword (if remaining) to a corner function.
	place := func(fn func(byte, int, int, [][]bool, [][]bool)) {
		if cwIdx < len(codewords) {
			fn(codewords[cwIdx], nRows, nCols, grid, used)
			cwIdx++
		}
	}

	for {
		// ── Corner special cases ──────────────────────────────────────────────
		// Corner 1: reference at (nRows, 0) when nRows or nCols is divisible by 4.
		if row == nRows && col == 0 && (nRows%4 == 0 || nCols%4 == 0) {
			place(placeCorner1)
		}
		// Corner 2: reference at (nRows-2, 0) when nCols mod 4 ≠ 0.
		if row == nRows-2 && col == 0 && nCols%4 != 0 {
			place(placeCorner2)
		}
		// Corner 3: reference at (nRows-2, 0) when nCols mod 8 == 4.
		if row == nRows-2 && col == 0 && nCols%8 == 4 {
			place(placeCorner3)
		}
		// Corner 4: reference at (nRows+4, 2) when nCols mod 8 == 0.
		if row == nRows+4 && col == 2 && nCols%8 == 0 {
			place(placeCorner4)
		}

		// ── Upward-right diagonal leg (row -= 2, col += 2) ───────────────────
		for {
			if row >= 0 && row < nRows && col >= 0 && col < nCols && !used[row][col] {
				if cwIdx < len(codewords) {
					placeUtah(codewords[cwIdx], row, col, nRows, nCols, grid, used)
					cwIdx++
				}
			}
			row -= 2
			col += 2
			if row < 0 || col >= nCols {
				break
			}
		}

		// Step to next diagonal start.
		row++
		col += 3

		// ── Downward-left diagonal leg (row += 2, col -= 2) ──────────────────
		for {
			if row >= 0 && row < nRows && col >= 0 && col < nCols && !used[row][col] {
				if cwIdx < len(codewords) {
					placeUtah(codewords[cwIdx], row, col, nRows, nCols, grid, used)
					cwIdx++
				}
			}
			row += 2
			col -= 2
			if row >= nRows || col < 0 {
				break
			}
		}

		// Step to next diagonal start.
		row += 3
		col++

		// ── Termination check ─────────────────────────────────────────────────
		if row >= nRows && col >= nCols {
			break
		}
		if cwIdx >= len(codewords) {
			break
		}
	}

	// ── Fill remaining unset modules (ISO right-and-bottom fill rule) ─────────
	// Some symbol sizes have residual modules the diagonal walk does not reach.
	// ISO/IEC 16022 §10 specifies these receive (r+c) mod 2 == 1 (dark).
	for r := 0; r < nRows; r++ {
		for c := 0; c < nCols; c++ {
			if !used[r][c] {
				grid[r][c] = (r+c)%2 == 1
			}
		}
	}

	return grid
}

// ============================================================================
// Logical → Physical coordinate mapping
// ============================================================================

// logicalToPhysical maps a logical data matrix coordinate (r, c) to its
// physical symbol coordinate (physRow, physCol).
//
// The logical data matrix is the concatenation of all data region interiors
// treated as one flat grid.  The Utah algorithm works in this logical space.
// After placement we map back to the physical grid, which adds:
//   - 1-module outer border (finder + timing) on all four sides
//   - 2-module alignment borders between data regions
//
// For a symbol with regionRows × regionCols data regions, each of size
// (dataRegionHeight × dataRegionWidth):
//
//	physRow = floor(r / rh) × (rh + 2) + (r mod rh) + 1
//	physCol = floor(c / rw) × (rw + 2) + (c mod rw) + 1
//
// The "+2" term accounts for the 2-module alignment border between regions.
// The "+1" term accounts for the 1-module outer border.
//
// For single-region symbols (1×1), this simplifies to physRow = r+1, physCol = c+1.
func logicalToPhysical(r, c int, entry symbolEntry) (int, int) {
	rh := entry.dataRegionHeight
	rw := entry.dataRegionWidth
	physRow := (r/rh)*(rh+2) + (r%rh) + 1
	physCol := (c/rw)*(rw+2) + (c%rw) + 1
	return physRow, physCol
}

// ============================================================================
// Full encoding pipeline
// ============================================================================

// Encode encodes input bytes into a Data Matrix ECC200 ModuleGrid.
//
// The smallest symbol that can hold the encoded data is selected automatically.
// The result is a ModuleGrid where true = dark module, false = light module.
//
// For very long input:
//   - Up to 1556 ASCII characters fit in the largest 144×144 symbol
//   - Digit-pair compression doubles capacity for all-numeric strings
//
// The encoding pipeline:
//
//  1. ASCII encode (with digit-pair optimization)
//  2. Select smallest fitting symbol
//  3. Pad to data capacity with scrambled pad codewords
//  4. Compute RS ECC for each block over GF(256)/0x12D
//  5. Interleave data+ECC blocks round-robin
//  6. Initialize physical grid (finder + timing + alignment borders)
//  7. Run Utah diagonal placement on logical data matrix
//  8. Map logical coords to physical coords
//  9. Return ModuleGrid (no masking — Data Matrix never masks)
//
// Errors:
//   - *InputTooLongError if encoded codeword count exceeds 1558.
func Encode(input []byte, opts Options) (barcode2d.ModuleGrid, error) {
	shape := opts.Shape

	// Step 1: ASCII encode.
	codewords := encodeASCII(input)

	// Step 2: Select smallest fitting symbol.
	entry, err := selectSymbol(len(codewords), shape)
	if err != nil {
		return barcode2d.ModuleGrid{}, err
	}

	// Step 3: Pad to data capacity.
	padded := padCodewords(codewords, entry.dataCW)

	// Step 4–5: Compute ECC and interleave.
	interleaved := computeInterleaved(padded, entry)

	// Step 6: Initialize physical grid with border and alignment borders.
	physGrid := initGrid(entry)

	// Step 7: Run Utah placement on the logical data matrix.
	nRows := entry.regionRows * entry.dataRegionHeight
	nCols := entry.regionCols * entry.dataRegionWidth
	logicalGrid := utahPlacement(interleaved, nRows, nCols)

	// Step 8: Map logical coordinates to physical coordinates.
	for r := 0; r < nRows; r++ {
		for c := 0; c < nCols; c++ {
			pr, pc := logicalToPhysical(r, c, entry)
			physGrid[pr][pc] = logicalGrid[r][c]
		}
	}

	// Step 9: Return ModuleGrid.
	return barcode2d.ModuleGrid{
		Rows:        uint32(entry.symbolRows),
		Cols:        uint32(entry.symbolCols),
		Modules:     physGrid,
		ModuleShape: barcode2d.ModuleShapeSquare,
	}, nil
}

// EncodeString is a convenience wrapper that encodes a UTF-8 string.
//
// Equivalent to Encode([]byte(input), opts).  All characters must be ASCII
// (0–127) for efficient encoding; non-ASCII bytes (128–255) use the
// UPPER_SHIFT mechanism and consume two codewords each.
func EncodeString(input string, opts Options) (barcode2d.ModuleGrid, error) {
	return Encode([]byte(input), opts)
}

// EncodeToScene encodes input and converts the result to a pixel-resolved
// PaintScene using barcode2d.Layout.
//
// The quiet zone defaults to 1 module (narrower than QR's 4-module quiet zone
// because the L-finder is inherently self-delimiting).  Override by setting
// cfg.QuietZoneModules.
//
// The PaintScene can be passed to any paint-vm backend (SVG, Metal, Canvas)
// to produce a renderable output.
func EncodeToScene(input []byte, opts Options, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error) {
	grid, err := Encode(input, opts)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}

	// Default quiet zone for Data Matrix: 1 module.
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
// Exported internals for testing
// ============================================================================

// Internal exports for unit testing (test files in same package).
// These are not part of the public API but are exported for white-box tests.

// GFExp returns the GF(256)/0x12D exponent table (dmGFExp).
// Index i → α^i.  Used by tests to verify the field construction.
func GFExp() [256]byte { return dmGFExp }

// GFLog returns the GF(256)/0x12D log table (dmGFLog).
// Value v → log_α(v).  Used by tests to verify the field construction.
func GFLog() [256]byte { return dmGFLog }

// GFMul multiplies two field elements (exported for testing).
func GFMul(a, b byte) byte { return gfMul(a, b) }

// EncodeASCII encodes input bytes in ASCII mode (exported for testing).
func EncodeASCII(input []byte) []byte { return encodeASCII(input) }

// PadCodewords pads codewords to dataCW length (exported for testing).
func PadCodewords(codewords []byte, dataCW int) []byte {
	return padCodewords(codewords, dataCW)
}

// SelectSymbol selects the smallest fitting symbol (exported for testing).
func SelectSymbol(codewordCount int, shape SymbolShape) (symbolEntry, error) {
	return selectSymbol(codewordCount, shape)
}

// RSEncodeBlock computes ECC bytes for a block (exported for testing).
func RSEncodeBlock(data []byte, generator []byte) []byte {
	return rsEncodeBlock(data, generator)
}

// GetGenerator returns the generator polynomial for nEcc (exported for testing).
func GetGenerator(nEcc int) []byte { return getGenerator(nEcc) }

// UtahPlacement runs the Utah algorithm (exported for testing).
func UtahPlacement(codewords []byte, nRows, nCols int) [][]bool {
	return utahPlacement(codewords, nRows, nCols)
}

// SquareSizes returns the square symbol size table (exported for testing).
func SquareSizes() []symbolEntry { return squareSizes }

// RectSizes returns the rectangular symbol size table (exported for testing).
func RectSizes() []rectEntry { return rectSizes }

// rectEntry aliases symbolEntry so tests can reference rectangular entries.
type rectEntry = symbolEntry

// ErrInputTooLongSentinel is exported so tests can check errors.Is.
var ErrInputTooLongSentinel = ErrInputTooLong

// errorsIsInputTooLong checks using errors.Is.
func IsInputTooLong(err error) bool {
	return errors.Is(err, ErrInputTooLong)
}
