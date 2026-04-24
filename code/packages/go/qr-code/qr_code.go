// Package qrcode implements a complete QR Code encoder for ISO/IEC 18004:2015.
//
// # What is a QR Code?
//
// QR Code (Quick Response code) was invented by Masahiro Hara at Denso Wave
// in 1994 to track automotive parts on assembly lines. It is a two-dimensional
// matrix barcode that can store URLs, text, contact information, or any binary
// data up to about 3 KB. Crucially, it is designed to be read 10× faster than
// a 1D barcode and to survive up to 30% physical damage.
//
// # How a QR code is built (pipeline overview)
//
//	input string
//	  → mode selection     (numeric / alphanumeric / byte)
//	  → version selection  (smallest symbol that holds the data at the chosen ECC level)
//	  → bit stream         (mode indicator + char count + data + padding bytes)
//	  → blocks + RS ECC   (GF(256) Reed-Solomon, b=0 root convention)
//	  → interleave         (weave codewords from all blocks together)
//	  → grid init          (finder patterns, timing strips, alignment patterns, dark module)
//	  → zigzag placement   (fill data modules bottom-right to top-left, snake pattern)
//	  → mask evaluation    (try all 8 masks, pick the one with the lowest penalty score)
//	  → finalize           (write format info + version info for v7+)
//	  → ModuleGrid         (abstract boolean grid: true = dark module)
//
// # Dependency stack
//
//	barcode-2d (ModuleGrid + Layout) ← produces PaintScene
//	gf256      (GF(2^8) field arithmetic: multiply, power, etc.)
//
// Note: this package does NOT depend on MA02 reed-solomon. QR uses a b=0 RS
// convention (first root is α^0 = 1) while MA02 uses b=1. We embed our own
// lightweight RS encoder here that matches the QR spec exactly.
//
// # Public API
//
//	Encode(data string, opts EncodeOptions) (barcode2d.ModuleGrid, error)
//	EncodeToScene(data string, opts EncodeOptions, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error)
package qrcode

import (
	"errors"
	"fmt"
	"unicode/utf8"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

// Version is the semantic version of this package.
const Version = "0.1.0"

// ============================================================================
// Public types
// ============================================================================

// EccLevel represents the error correction capability of the QR symbol.
//
// Higher levels add more redundancy, allowing more of the symbol to be damaged
// or obscured while still being decoded correctly. The trade-off: more ECC
// means fewer bytes available for data, so the same message requires a larger
// (higher version) symbol.
//
//	L  ~7%  recovery  — maximum data density, least redundancy
//	M  ~15% recovery  — good general-purpose default
//	Q  ~25% recovery  — recommended when some damage is expected
//	H  ~30% recovery  — maximum error correction (logos, watermarks, outdoor)
type EccLevel int

const (
	EccL EccLevel = iota // L — Low       (~7%  recovery)
	EccM                 // M — Medium    (~15% recovery)
	EccQ                 // Q — Quartile  (~25% recovery)
	EccH                 // H — High      (~30% recovery)
)

// EncodingMode controls which subset of characters a data segment uses.
//
// Choosing the most compact mode that covers your input shrinks the bit stream,
// allowing a smaller (lower-version) symbol.
//
//	ModeAuto        — let the encoder pick the best mode automatically
//	ModeNumeric     — digits 0–9 only; ~3× denser than byte mode
//	ModeAlphanumeric — digits, A–Z, space, $ % * + - . / :; ~1.6× denser
//	ModeByte        — arbitrary UTF-8 bytes; always valid, universal fallback
//	ModeKanji       — Shift-JIS kanji (future; not yet implemented)
type EncodingMode int

const (
	ModeAuto        EncodingMode = iota // pick best mode automatically
	ModeNumeric                         // digits 0–9 only
	ModeAlphanumeric                    // uppercase + digits + $%*+-./: + space
	ModeByte                            // raw bytes (UTF-8)
	ModeKanji                           // Shift-JIS kanji (not implemented)
)

// EncodeOptions holds all configuration for the encoder.
//
// Zero value (EncodeOptions{}) is a valid configuration:
//
//	Level   = EccM  (medium error correction, sensible default)
//	Version = 0     (auto-select smallest version that fits)
//	Mode    = 0     (auto-select most compact mode)
type EncodeOptions struct {
	// Level is the error correction level. Default EccM.
	Level EccLevel
	// Version is the QR symbol version (1–40). 0 means auto-select.
	Version int
	// Mode is the encoding mode. ModeAuto (0) means auto-select.
	Mode EncodingMode
}

// ============================================================================
// Error types
// ============================================================================

// QRCodeError is the base type for all errors returned by this package.
// Use errors.As to test for this type or its subtypes.
type QRCodeError struct {
	msg string
}

func (e *QRCodeError) Error() string { return "qr-code: " + e.msg }

// InputTooLongError is returned when the input cannot fit in any version 1–40
// symbol at the requested ECC level.
type InputTooLongError struct{ QRCodeError }

// IsInputTooLongError reports whether err is an InputTooLongError.
func IsInputTooLongError(err error) bool {
	var t *InputTooLongError
	return errors.As(err, &t)
}

// InvalidInputError is returned when the input contains characters that cannot
// be encoded in the chosen mode (e.g. lowercase letters in Alphanumeric mode).
type InvalidInputError struct{ QRCodeError }

// IsInvalidInputError reports whether err is an InvalidInputError.
func IsInvalidInputError(err error) bool {
	var t *InvalidInputError
	return errors.As(err, &t)
}

func errTooLong(msg string) *InputTooLongError {
	return &InputTooLongError{QRCodeError{msg}}
}

func errInvalid(msg string) *InvalidInputError {
	return &InvalidInputError{QRCodeError{msg}}
}

// ============================================================================
// ISO 18004:2015 — ECC indicator bits (Table 12)
// ============================================================================

// eccIndicator is the 2-bit ECC level indicator placed in format information.
//
// Notably, these are NOT in the intuitive order:
//
//	L → 01   (not 00)
//	M → 00   (not 01)
//	Q → 11   (not 10)
//	H → 10   (not 11)
//
// This mapping comes directly from the ISO standard. Historically the ordering
// was chosen to satisfy certain format-info constraints.
var eccIndicator = [4]int{0b01, 0b00, 0b11, 0b10} // indexed by EccLevel

// ============================================================================
// ISO 18004:2015 — Capacity tables (Table 9)
// ============================================================================

// eccCodewordsPerBlock holds the number of ECC codewords per RS block.
// Indexed as [eccIdx][version]; version index 0 is unused (placeholder -1).
//
// These numbers come from the ISO standard. They determine how many RS check
// bytes are appended to each data block. More ECC codewords = more redundancy
// = more damage tolerance.
var eccCodewordsPerBlock = [4][41]int{
	// L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30},
	// M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28},
	// Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30},
	// H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30},
}

// numBlocks holds the total number of RS blocks per version/ECC combination.
// Indexed as [eccIdx][version]; version index 0 is unused (placeholder -1).
//
// When numBlocks > 1 the data is split across multiple RS blocks and then
// interleaved before placement. This limits the damage any single burst error
// (contiguous scratch, fold, or shadow) can do: a burst destroys only a few
// codewords in each block, leaving each block's RS decoder well within budget.
var numBlocks = [4][41]int{
	// L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25},
	// M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49},
	// Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68},
	// H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
	{-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80},
}

// ============================================================================
// ISO 18004:2015 Annex E — Alignment pattern center coordinates
// ============================================================================

// alignmentPositions gives the list of center coordinates for alignment
// patterns per version. Alignment patterns are placed at every combination
// (crossproduct) of positions in the list, except those that would overlap
// the three finder patterns.
//
// Version 1 has no alignment patterns (the list is empty).
// Version 2 has one: center at (18, 18).
// Larger versions have grids: e.g. v7 has positions {6,22,38} giving
// a 3×3 grid of possible centers (minus the three finder-corner overlaps),
// resulting in up to 5 alignment patterns.
var alignmentPositions = [41][]int{
	nil,              // v0 (unused)
	{},               // v1  — none
	{6, 18},          // v2
	{6, 22},          // v3
	{6, 26},          // v4
	{6, 30},          // v5
	{6, 34},          // v6
	{6, 22, 38},      // v7
	{6, 24, 42},      // v8
	{6, 26, 46},      // v9
	{6, 28, 50},      // v10
	{6, 30, 54},      // v11
	{6, 32, 58},      // v12
	{6, 34, 62},      // v13
	{6, 26, 46, 66},  // v14
	{6, 26, 48, 70},  // v15
	{6, 26, 50, 74},  // v16
	{6, 30, 54, 78},  // v17
	{6, 30, 56, 82},  // v18
	{6, 30, 58, 86},  // v19
	{6, 34, 62, 90},  // v20
	{6, 28, 50, 72, 94},   // v21
	{6, 26, 50, 74, 98},   // v22
	{6, 30, 54, 78, 102},  // v23
	{6, 28, 54, 80, 106},  // v24
	{6, 32, 58, 84, 110},  // v25
	{6, 30, 58, 86, 114},  // v26
	{6, 34, 62, 90, 118},  // v27
	{6, 26, 50, 74, 98, 122},   // v28
	{6, 30, 54, 78, 102, 126},  // v29
	{6, 26, 52, 78, 104, 130},  // v30
	{6, 30, 56, 82, 108, 134},  // v31
	{6, 34, 60, 86, 112, 138},  // v32
	{6, 30, 58, 86, 114, 142},  // v33
	{6, 34, 62, 90, 118, 146},  // v34
	{6, 30, 54, 78, 102, 126, 150}, // v35
	{6, 24, 50, 76, 102, 128, 154}, // v36
	{6, 28, 54, 80, 106, 132, 158}, // v37
	{6, 32, 58, 84, 110, 136, 162}, // v38
	{6, 26, 54, 82, 110, 138, 166}, // v39
	{6, 30, 58, 86, 114, 142, 170}, // v40
}

// ============================================================================
// Grid geometry helpers
// ============================================================================

// symbolSize returns the side length (in modules) of a QR symbol for a given
// version. This is the formula from the ISO standard:
//
//	size = 4 × version + 17
//
// Version 1 → 21×21. Version 40 → 177×177.
func symbolSize(version int) int {
	return 4*version + 17
}

// numRawDataModules returns the total number of bits available in a QR symbol
// after subtracting all function module areas (finders, separators, timing,
// alignment, format info, version info, dark module).
//
// This formula is derived from counting all module types and is reproduced from
// Nayuki's reference implementation (public domain, MIT license). It avoids
// having to actually build a grid just to count available data modules.
//
//	rawBits = (16×v + 128)×v + 64
//	         − alignment overhead (versions 2+)
//	         − version info overhead (versions 7+)
func numRawDataModules(version int) int {
	result := (16*version+128)*version + 64
	if version >= 2 {
		numAlign := version/7 + 2
		result -= (25*numAlign-10)*numAlign - 55
		if version >= 7 {
			result -= 36
		}
	}
	return result
}

// numDataCodewords returns how many 8-bit data codewords a QR symbol can hold
// (excluding ECC codewords) for the given version and ECC level.
//
// Total codeword capacity = numRawDataModules / 8
// ECC codewords = numBlocks × eccCodewordsPerBlock
// Data codewords = total − ECC
func numDataCodewords(version int, ecc EccLevel) int {
	e := int(ecc)
	return numRawDataModules(version)/8 -
		numBlocks[e][version]*eccCodewordsPerBlock[e][version]
}

// numRemainderBits returns the number of padding zero bits appended after the
// last interleaved codeword. These "remainder bits" fill the trailing modules
// of the grid when numRawDataModules is not a multiple of 8.
//
// Only a few versions require remainder bits:
//
//	versions 2–6, 14–20, 28–34 need 7 remainder bits
//	versions 21–27             need 4 remainder bits
//	versions 7–13, 35–40       need 0 remainder bits
//
// In practice this is simply numRawDataModules(version) % 8.
func numRemainderBits(version int) int {
	return numRawDataModules(version) % 8
}

// ============================================================================
// Reed-Solomon encoder (QR-specific, b=0 convention)
// ============================================================================

// The QR Code RS encoder uses the b=0 convention: the generator polynomial is
//
//	g(x) = (x + α^0)(x + α^1)(x + α^2)···(x + α^{n-1})
//
// where α = 2 is the primitive element of GF(256) (per our gf256 package).
// This is different from MA02's b=1 convention, so we implement our own RS
// encoder here to avoid importing reedsolomon just to miss the root shift.
//
// The ALOG table (α^i for i=0..254) comes from the gf256 package.
var alogTable [256]int

func init() {
	// Copy the antilog table from the gf256 package into a local array.
	// This is safe: gf256.ALOG() returns the same values every time (it's
	// computed in gf256's init() using the QR/RS primitive polynomial 0x11D).
	t := gf256.ALOG()
	for i := range alogTable {
		alogTable[i] = t[i]
	}
}

// buildQRGenerator builds the RS generator polynomial for n ECC codewords
// using the b=0 convention: g(x) = ∏(x + α^i) for i = 0 to n-1.
//
// The result is stored as a big-endian coefficient slice of length n+1, where
// result[0] is the leading (monic) coefficient and result[n] is the constant term.
//
// Algorithm:
//  1. Start with g = [1] (degree-0 monic polynomial).
//  2. For each i from 0 to n-1, multiply the current g by (x + α^i):
//     new[j]   = old[j]       (the x term contributes to degree j+1 output)
//     new[j+1] ^= old[j] * α^i  (the constant term)
//  3. Shift g to include the new degree.
func buildQRGenerator(n int) []byte {
	// Start with the trivial polynomial p(x) = 1.
	g := []byte{1}

	for i := 0; i < n; i++ {
		// α^i from the antilog table (same primitive polynomial as gf256 uses).
		ai := byte(alogTable[i])

		// Multiply g(x) by (x + α^i):
		//   new_g[j]   += old_g[j-1]   (x term — shift all coefficients right)
		//   new_g[j+1] += old_g[j] * α^i  (constant term)
		next := make([]byte, len(g)+1)
		for j := 0; j < len(g); j++ {
			next[j] ^= g[j]
			next[j+1] ^= gf256.Multiply(g[j], ai)
		}
		g = next
	}

	return g
}

// rsEncode computes n ECC bytes for the given data slice using the provided
// generator polynomial (big-endian, length n+1, monic leading coefficient).
//
// Algorithm — LFSR shift register:
//
//	rem = [0, 0, ..., 0]  (n zeros)
//	for each byte b in data:
//	    feedback = b XOR rem[0]
//	    shift rem left one position (rem[i] ← rem[i+1], rem[n-1] ← 0)
//	    for i in 0..n-1:
//	        rem[i] ^= generator[i+1] * feedback
//
// The result rem is exactly the ECC bytes (the remainder of data·x^n mod g(x)).
//
// Why this works: polynomial long division in GF(256). Each step processes one
// codeword coefficient from the most-significant end. The LFSR simulates the
// division without materialising the full D(x)·x^n polynomial.
func rsEncode(data []byte, generator []byte) []byte {
	n := len(generator) - 1 // number of ECC bytes = degree of generator
	rem := make([]byte, n)

	for _, b := range data {
		// Compute the feedback: the next data byte XOR the current leading remainder.
		fb := b ^ rem[0]

		// Shift the register left: rem[i] ← rem[i+1].
		copy(rem, rem[1:])
		rem[n-1] = 0

		// Add feedback * generator[i+1] to each remainder position.
		// (generator[0] = 1 = monic coefficient; we skip it.)
		if fb != 0 {
			for i := 0; i < n; i++ {
				rem[i] ^= gf256.Multiply(generator[i+1], fb)
			}
		}
	}

	return rem
}

// generatorCache caches pre-built generators by ECC count to avoid rebuilding
// the same polynomial repeatedly (each generator for a given n is always the same).
var generatorCache = map[int][]byte{}

func getGenerator(n int) []byte {
	if g, ok := generatorCache[n]; ok {
		return g
	}
	g := buildQRGenerator(n)
	generatorCache[n] = g
	return g
}

// ============================================================================
// Data encoding modes
// ============================================================================

// alphanumChars is the 45-character set for QR alphanumeric mode, in the
// order defined by ISO 18004 Table 5. The position of each character in this
// string is its numeric value for encoding.
//
//	Indices 0–9:  digits 0–9
//	Indices 10–35: uppercase A–Z
//	Index 36: space
//	Indices 37–44: $ % * + - . / :
const alphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./"

// alphanumIndex returns the QR alphanumeric index (0–44) for a byte,
// or -1 if the byte is not in the alphanumeric set.
func alphanumIndex(b byte) int {
	switch {
	case b >= '0' && b <= '9':
		return int(b - '0')
	case b >= 'A' && b <= 'Z':
		return int(b-'A') + 10
	case b == ' ':
		return 36
	case b == '$':
		return 37
	case b == '%':
		return 38
	case b == '*':
		return 39
	case b == '+':
		return 40
	case b == '-':
		return 41
	case b == '.':
		return 42
	case b == '/':
		return 43
	case b == ':':
		return 44
	}
	return -1
}

// selectMode picks the most compact QR encoding mode for the entire input.
//
// Priority order (most to least compact):
//  1. Numeric — input contains only ASCII digits 0–9.
//  2. Alphanumeric — all bytes are in the 45-character QR alphanumeric set.
//  3. Byte — fallback; any UTF-8 string can be encoded in byte mode.
//
// Using a more compact mode reduces the bit count and may allow a smaller
// (lower-version) symbol. The trade-off is that alphanumeric mode forces
// the input to be uppercase, and numeric requires only digits.
func selectMode(input string) EncodingMode {
	bytes := []byte(input)
	numeric := true
	alphanum := true

	for _, b := range bytes {
		if b < '0' || b > '9' {
			numeric = false
		}
		if alphanumIndex(b) < 0 {
			alphanum = false
		}
		if !numeric && !alphanum {
			break // can't improve on byte mode
		}
	}

	if numeric {
		return ModeNumeric
	}
	if alphanum {
		return ModeAlphanumeric
	}
	return ModeByte
}

// charCountBits returns the number of bits used for the character count field
// in the bit stream. This width depends on both mode and version group:
//
//	Mode        | v1–9 | v10–26 | v27–40
//	------------|------|--------|-------
//	Numeric     |  10  |   12   |  14
//	Alphanumeric|   9  |   11   |  13
//	Byte        |   8  |   16   |  16
//	Kanji       |   8  |   10   |  12
//
// The width increases for larger versions because larger symbols can hold more
// characters, and a wider count field is needed to represent that.
func charCountBits(mode EncodingMode, version int) int {
	switch mode {
	case ModeNumeric:
		if version <= 9 {
			return 10
		} else if version <= 26 {
			return 12
		}
		return 14
	case ModeAlphanumeric:
		if version <= 9 {
			return 9
		} else if version <= 26 {
			return 11
		}
		return 13
	case ModeByte:
		if version <= 9 {
			return 8
		}
		return 16
	case ModeKanji:
		if version <= 9 {
			return 8
		} else if version <= 26 {
			return 10
		}
		return 12
	}
	return 8
}

// modeIndicator returns the 4-bit mode indicator for a QR encoding mode.
//
//	Numeric      → 0001 (= 1)
//	Alphanumeric → 0010 (= 2)
//	Byte         → 0100 (= 4)
//	Kanji        → 1000 (= 8)
//	Terminator   → 0000 (= 0)  (not an encoding mode; used to end the stream)
func modeIndicator(mode EncodingMode) int {
	switch mode {
	case ModeNumeric:
		return 0b0001
	case ModeAlphanumeric:
		return 0b0010
	case ModeByte:
		return 0b0100
	case ModeKanji:
		return 0b1000
	}
	return 0
}

// ============================================================================
// Bit writer — accumulates bits and converts to bytes
// ============================================================================

// bitWriter is a simple bit accumulator. It appends bits MSB-first into an
// internal byte slice. When flush() is called the trailing partial byte is
// zero-padded to a full byte (this is the byte-boundary padding step in the
// QR bit stream).
type bitWriter struct {
	buf []byte // accumulated complete bytes
	cur byte   // current partial byte being built
	nBits int  // how many bits are in cur (0..7)
	total int  // total number of bits written (for bookkeeping)
}

// writeBits appends value (MSB first) using count bits. count must be 1–32.
func (w *bitWriter) writeBits(value int, count int) {
	w.total += count
	for i := count - 1; i >= 0; i-- {
		bit := (value >> i) & 1
		w.cur = (w.cur << 1) | byte(bit)
		w.nBits++
		if w.nBits == 8 {
			w.buf = append(w.buf, w.cur)
			w.cur = 0
			w.nBits = 0
		}
	}
}

// bitLen returns the total number of bits written so far.
func (w *bitWriter) bitLen() int { return w.total }

// toBytes flushes any partial byte (zero-padding the LSBs) and returns the
// complete byte slice.
func (w *bitWriter) toBytes() []byte {
	if w.nBits > 0 {
		// Zero-pad the last byte to 8 bits.
		out := append(w.buf, w.cur<<(8-w.nBits))
		return out
	}
	out := make([]byte, len(w.buf))
	copy(out, w.buf)
	return out
}

// ============================================================================
// Bit stream assembly
// ============================================================================

// buildDataCodewords encodes the input string into the data codeword sequence.
//
// The output is exactly numDataCodewords(version, ecc) bytes. The format is:
//
//	[4-bit mode indicator]
//	[character count field (width depends on mode and version)]
//	[encoded data]
//	[terminator: up to 4 zero bits, fewer if we're already at capacity]
//	[zero bits to reach byte boundary]
//	[padding bytes 0xEC, 0x11, 0xEC, 0x11, ... to fill remaining capacity]
//
// The alternating 0xEC/0x11 padding is a QR standard convention chosen to
// produce a balanced dark/light ratio even in the padding area.
func buildDataCodewords(input string, version int, ecc EccLevel) ([]byte, error) {
	mode := selectMode(input)
	capacity := numDataCodewords(version, ecc)
	w := &bitWriter{}

	// Write mode indicator (4 bits).
	w.writeBits(modeIndicator(mode), 4)

	// Write character count.
	// For byte mode, the count is the number of UTF-8 bytes, not rune count.
	// For other modes, the count is the number of input characters.
	var charCount int
	inputBytes := []byte(input)
	if mode == ModeByte {
		charCount = len(inputBytes)
	} else {
		charCount = utf8.RuneCountInString(input)
	}
	w.writeBits(charCount, charCountBits(mode, version))

	// Encode data bits.
	switch mode {
	case ModeNumeric:
		if err := encodeNumeric(input, w); err != nil {
			return nil, err
		}
	case ModeAlphanumeric:
		if err := encodeAlphanumeric(input, w); err != nil {
			return nil, err
		}
	case ModeByte:
		encodeByte(inputBytes, w)
	}

	// Terminator: at most 4 zero bits, but stop early if already at capacity.
	remaining := capacity*8 - w.bitLen()
	termLen := remaining
	if termLen > 4 {
		termLen = 4
	}
	if termLen > 0 {
		w.writeBits(0, termLen)
	}

	// Pad to byte boundary.
	rem := w.bitLen() % 8
	if rem != 0 {
		w.writeBits(0, 8-rem)
	}

	// Convert to bytes (no more bits will be written after this).
	bytes := w.toBytes()

	// Fill remaining capacity with alternating 0xEC / 0x11.
	// These values were chosen by the QR standard to avoid all-dark or all-light
	// padding regions. (0xEC = 11101100, 0x11 = 00010001 — complementary bit
	// distributions.)
	pad := byte(0xEC)
	for len(bytes) < capacity {
		bytes = append(bytes, pad)
		if pad == 0xEC {
			pad = 0x11
		} else {
			pad = 0xEC
		}
	}

	return bytes, nil
}

// encodeNumeric encodes a digit string into the bit writer in numeric mode.
//
// The encoding groups digits into triples, pairs, and singles:
//
//	3 digits → 10 bits (represents the integer value 0–999)
//	2 digits →  7 bits (represents the integer value 0–99)
//	1 digit  →  4 bits (represents the integer value 0–9)
//
// Example: "01234567"
//
//	"012" → integer 12 → 10 bits: 0000001100
//	"345" → integer 345 → 10 bits: 0101011001
//	"67"  → integer 67 → 7 bits: 1000011
func encodeNumeric(input string, w *bitWriter) error {
	i := 0
	for i+2 < len(input) {
		val := int(input[i]-'0')*100 + int(input[i+1]-'0')*10 + int(input[i+2]-'0')
		w.writeBits(val, 10)
		i += 3
	}
	if i+1 < len(input) {
		val := int(input[i]-'0')*10 + int(input[i+1]-'0')
		w.writeBits(val, 7)
		i += 2
	}
	if i < len(input) {
		val := int(input[i] - '0')
		w.writeBits(val, 4)
	}
	return nil
}

// encodeAlphanumeric encodes a string in QR alphanumeric mode.
//
// Pairs of characters are packed into 11 bits using the formula:
//
//	combined = first_index * 45 + second_index
//
// A trailing single character uses 6 bits for its index directly.
//
// Example: "HELLO WORLD"
//
//	"HE" → H=17, E=14 → 17*45+14 = 779 → 11 bits: 01100001011
//	"LL" → L=21, L=21 → 21*45+21 = 966 → 11 bits: 01111000110
//	"O " → O=24, ' '=36 → 24*45+36 = 1116 → 11 bits: 10001011100
//	"WO" → W=32, O=24 → 32*45+24 = 1464 → 11 bits: 10110111000
//	"RL" → R=27, L=21 → 27*45+21 = 1236 → 11 bits: 10011010100
//	"D"  → D=13 → 6 bits: 001101
func encodeAlphanumeric(input string, w *bitWriter) error {
	bytes := []byte(input)
	i := 0
	for i+1 < len(bytes) {
		idx0 := alphanumIndex(bytes[i])
		idx1 := alphanumIndex(bytes[i+1])
		if idx0 < 0 || idx1 < 0 {
			return errInvalid(fmt.Sprintf("character not in QR alphanumeric set: %q", input[i:i+2]))
		}
		w.writeBits(idx0*45+idx1, 11)
		i += 2
	}
	if i < len(bytes) {
		idx := alphanumIndex(bytes[i])
		if idx < 0 {
			return errInvalid(fmt.Sprintf("character not in QR alphanumeric set: %q", input[i:i+1]))
		}
		w.writeBits(idx, 6)
	}
	return nil
}

// encodeByte writes each byte of the input verbatim as 8 bits.
// For UTF-8 encoded strings this writes the raw UTF-8 bytes, which all modern
// QR scanners will interpret as UTF-8.
func encodeByte(data []byte, w *bitWriter) {
	for _, b := range data {
		w.writeBits(int(b), 8)
	}
}

// ============================================================================
// Block splitting and RS encoding
// ============================================================================

// block is one RS block: the raw data bytes and the computed ECC bytes.
type block struct {
	data []byte
	ecc  []byte
}

// computeBlocks splits the data codeword sequence into RS blocks, then computes
// ECC bytes for each block using the QR RS encoder.
//
// The block split follows the ISO standard's "group 1 / group 2" structure:
//
//	total blocks    = numBlocks[ecc][version]
//	ECC per block   = eccCodewordsPerBlock[ecc][version]
//	data per block  = floor(totalData / totalBlocks) for group 1
//	               = floor(totalData / totalBlocks) + 1 for group 2 (longer blocks)
//	group 1 count   = totalBlocks - (totalData % totalBlocks)
//	group 2 count   = totalData % totalBlocks
//
// For example, Version 5 Q:
//
//	Total data CW = 64, totalBlocks = 4, ECC/block = 18
//	shortLen = 64/4 = 16, numLong = 64%4 = 0 → all 4 blocks are 16 bytes
//
// Version 5 M:
//
//	Total data CW = 86, totalBlocks = 2, ECC/block = 24
//	shortLen = 43, numLong = 0 → 2 blocks of 43 bytes
func computeBlocks(data []byte, version int, ecc EccLevel) []block {
	e := int(ecc)
	totalBlocks := numBlocks[e][version]
	eccLen := eccCodewordsPerBlock[e][version]
	totalData := len(data)
	shortLen := totalData / totalBlocks // data bytes in a "short" block
	numLong := totalData % totalBlocks  // number of "long" blocks (one extra byte)

	gen := getGenerator(eccLen)
	blocks := make([]block, 0, totalBlocks)
	offset := 0

	// Group 1: (totalBlocks - numLong) blocks of shortLen bytes each.
	g1Count := totalBlocks - numLong
	for i := 0; i < g1Count; i++ {
		d := data[offset : offset+shortLen]
		blocks = append(blocks, block{data: d, ecc: rsEncode(d, gen)})
		offset += shortLen
	}

	// Group 2: numLong blocks of (shortLen+1) bytes each.
	for i := 0; i < numLong; i++ {
		d := data[offset : offset+shortLen+1]
		blocks = append(blocks, block{data: d, ecc: rsEncode(d, gen)})
		offset += shortLen + 1
	}

	return blocks
}

// interleaveBlocks weaves codewords from all blocks together.
//
// The interleaving order is:
//  1. Round-robin over data codewords: first from block 0, then block 1, ...
//     Continue until all data codewords are placed (shorter blocks stop early).
//  2. Round-robin over ECC codewords in the same way.
//
// Why interleave? A QR code printed on a surface can suffer a "burst error" — a
// contiguous damaged area. Without interleaving, a burst could wipe out a whole
// block. With interleaving, a burst only destroys one codeword per block, and
// each block's RS decoder can recover from that.
func interleaveBlocks(blocks []block) []byte {
	// Find the maximum data and ECC lengths across all blocks.
	maxData := 0
	maxEcc := 0
	for _, b := range blocks {
		if len(b.data) > maxData {
			maxData = len(b.data)
		}
		if len(b.ecc) > maxEcc {
			maxEcc = len(b.ecc)
		}
	}

	result := make([]byte, 0, maxData*len(blocks)+maxEcc*len(blocks))

	// Interleave data codewords.
	for i := 0; i < maxData; i++ {
		for _, b := range blocks {
			if i < len(b.data) {
				result = append(result, b.data[i])
			}
		}
	}

	// Interleave ECC codewords.
	for i := 0; i < maxEcc; i++ {
		for _, b := range blocks {
			if i < len(b.ecc) {
				result = append(result, b.ecc[i])
			}
		}
	}

	return result
}

// ============================================================================
// Work grid — mutable grid used during construction
// ============================================================================

// workGrid is the internal mutable representation used during encoding.
// We need two parallel boolean matrices: modules (the actual dark/light values)
// and reserved (which modules are structural and must not be touched during
// data placement or masking).
//
// This is separate from barcode2d.ModuleGrid which is immutable. We convert
// to a ModuleGrid at the very end of encoding.
type workGrid struct {
	size     int
	modules  [][]bool // true = dark
	reserved [][]bool // true = structural (finder/timing/format/etc.)
}

func newWorkGrid(size int) *workGrid {
	modules := make([][]bool, size)
	reserved := make([][]bool, size)
	for i := range modules {
		modules[i] = make([]bool, size)
		reserved[i] = make([]bool, size)
	}
	return &workGrid{size: size, modules: modules, reserved: reserved}
}

// set sets the module at (r, c) to dark. If reserve is true, marks the module
// as reserved (structural) so it will be skipped during data placement and masking.
func (g *workGrid) set(r, c int, dark, reserve bool) {
	g.modules[r][c] = dark
	if reserve {
		g.reserved[r][c] = true
	}
}

// copyModules returns a deep copy of the modules slice.
func (g *workGrid) copyModules() [][]bool {
	cp := make([][]bool, g.size)
	for i, row := range g.modules {
		cp[i] = make([]bool, g.size)
		copy(cp[i], row)
	}
	return cp
}

// ============================================================================
// Finder patterns
// ============================================================================

// placeFinder places a 7×7 finder pattern with top-left corner at (topRow, topCol).
//
// The finder pattern looks like:
//
//	■ ■ ■ ■ ■ ■ ■
//	■ □ □ □ □ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ □ □ □ □ ■
//	■ ■ ■ ■ ■ ■ ■
//
// The 1:1:3:1:1 ratio of dark:light:dark:light:dark in any scan direction is
// distinctive enough for a scanner to find all three corner patterns even when
// the QR code is tilted, partially covered, or very small. The three-corner
// placement immediately tells the scanner which corner is missing (bottom-right)
// and therefore which way the code is oriented.
func (g *workGrid) placeFinder(topRow, topCol int) {
	for dr := 0; dr < 7; dr++ {
		for dc := 0; dc < 7; dc++ {
			// A module in the finder is dark if it's on the outer border (dr=0/6 or dc=0/6)
			// or in the 3×3 inner core (dr=2..4, dc=2..4).
			// The 1-module-wide white ring between them is light.
			onBorder := dr == 0 || dr == 6 || dc == 0 || dc == 6
			inCore := dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
			g.set(topRow+dr, topCol+dc, onBorder || inCore, true)
		}
	}
}

// placeAlignment places a 5×5 alignment pattern centred at (row, col).
//
// The alignment pattern looks like:
//
//	■ ■ ■ ■ ■
//	■ □ □ □ ■
//	■ □ ■ □ ■
//	■ □ □ □ ■
//	■ ■ ■ ■ ■
//
// Alignment patterns appear in versions 2+ and help decoders correct for
// perspective distortion (e.g. when photographing a curved or tilted QR code).
// The scanner uses them as additional registration points beyond the three
// finder patterns.
func (g *workGrid) placeAlignment(row, col int) {
	for dr := -2; dr <= 2; dr++ {
		for dc := -2; dc <= 2; dc++ {
			onBorder := absInt(dr) == 2 || absInt(dc) == 2
			isCenter := dr == 0 && dc == 0
			g.set(row+dr, col+dc, onBorder || isCenter, true)
		}
	}
}

// placeAllAlignments places all alignment patterns for the given version.
//
// The positions are the full crossproduct of the position list from
// alignmentPositions[version]. Any position whose center falls on an already-
// reserved module (finder or timing) is skipped automatically.
func (g *workGrid) placeAllAlignments(version int) {
	positions := alignmentPositions[version]
	for _, row := range positions {
		for _, col := range positions {
			// Skip if the center overlaps an already-reserved module.
			// This handles the three finder-corner overlaps without needing to
			// hardcode which combinations to skip.
			if g.reserved[row][col] {
				continue
			}
			g.placeAlignment(row, col)
		}
	}
}

// placeTimingStrips places the horizontal and vertical timing strips.
//
// Timing strips are alternating dark/light modules running between the finder
// patterns. They occupy row 6 (horizontal) and column 6 (vertical), spanning
// from module 8 to size-9 in each direction.
//
// They always start and end dark (even index = dark). Scanners read them to
// determine the module grid size and compensate for skew or scaling.
func (g *workGrid) placeTimingStrips() {
	sz := g.size
	for c := 8; c <= sz-9; c++ {
		g.set(6, c, c%2 == 0, true)
	}
	for r := 8; r <= sz-9; r++ {
		g.set(r, 6, r%2 == 0, true)
	}
}

// ============================================================================
// Format information
// ============================================================================

// reserveFormatInfo marks the 15 format information module positions (× 2 copies)
// as reserved. These positions will be filled in after the best mask is chosen.
// We reserve them now so data placement skips them.
//
// Copy 1 is an L-shaped strip adjacent to the top-left finder:
//   - Row 8, columns 0–8 (skipping column 6 which is timing)
//   - Column 8, rows 0–8 (skipping row 6 which is timing)
//
// Copy 2:
//   - Column 8, rows size-7 to size-1 (bottom-left strip)
//   - Row 8, columns size-8 to size-1 (top-right strip)
func (g *workGrid) reserveFormatInfo() {
	sz := g.size

	// Copy 1: row 8, cols 0..8 (skip col 6 = timing)
	for c := 0; c <= 8; c++ {
		if c != 6 {
			g.reserved[8][c] = true
		}
	}
	// Copy 1: col 8, rows 0..8 (skip row 6 = timing)
	for r := 0; r <= 8; r++ {
		if r != 6 {
			g.reserved[r][8] = true
		}
	}

	// Copy 2: col 8, rows size-7 to size-1
	for r := sz - 7; r < sz; r++ {
		g.reserved[r][8] = true
	}
	// Copy 2: row 8, cols size-8 to size-1
	for c := sz - 8; c < sz; c++ {
		g.reserved[8][c] = true
	}
}

// computeFormatBits computes the 15-bit format information string for a given
// ECC level and mask pattern index.
//
// The format information encodes two things the decoder needs to know BEFORE
// it can read the data: the ECC level and the mask pattern. It is heavily
// protected by a BCH error correction code so that even if the format info is
// partially damaged, the decoder can recover it.
//
// Construction:
//  1. Form a 5-bit data word: [ECC indicator (2 bits)] [mask pattern (3 bits)]
//  2. Left-shift by 10 to make room for the 10-bit BCH remainder.
//  3. Compute BCH(15,5) remainder using the generator G(x) = 0x537:
//     G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1
//  4. Append the remainder: 15-bit word = data<<10 | remainder
//  5. XOR with 0x5412 to prevent all-zero format info in the all-zero case.
//
// The XOR mask 0x5412 = 0101 0100 0001 0010 is a constant defined in the ISO
// standard, chosen to ensure the format info is never all zeros (which would
// look like a very light region that some scanners might misread).
func computeFormatBits(ecc EccLevel, mask int) int {
	data := (eccIndicator[int(ecc)] << 3) | mask

	// BCH remainder of (data << 10) mod G(x) where G(x) = 0x537.
	rem := data << 10
	for i := 14; i >= 10; i-- {
		if (rem>>i)&1 != 0 {
			rem ^= 0x537 << (i - 10)
		}
	}

	return ((data << 10) | (rem & 0x3FF)) ^ 0x5412
}

// writeFormatInfo writes the 15-bit format string into both copy locations.
//
// The precise bit positions are from ISO 18004 Annex C. The key lesson from
// lessons.md: bit ordering is MSB-first in specific directions and LSB-first
// in others. The exact placement:
//
// Copy 1 (adjacent to top-left finder):
//   - Bits f0–f5 → row 8, cols 0–5 (LSB first, left to right)
//   - Bit  f6   → row 8, col 7  (col 6 is timing, skip it)
//   - Bit  f7   → row 8, col 8  (corner)
//   - Bit  f8   → row 7, col 8
//   - Bits f9–f14 → rows 5..0, col 8 (going upward)
//
// Copy 2 (bottom-left and top-right):
//   - Bits f0–f6 → rows size-1..size-7, col 8 (bottom strip, LSB at bottom)
//   - Bits f7–f14 → row 8, cols size-8..size-1 (top-right strip, MSB at right)
func writeFormatInfo(modules [][]bool, sz, fmtBits int) {
	// Copy 1 — horizontal strip (row 8, left side)
	// f0 at col 0, f1 at col 1, ..., f5 at col 5 (skip col 6), f6 at col 7, f7 at col 8
	for i := 0; i <= 5; i++ {
		modules[8][i] = (fmtBits>>i)&1 == 1
	}
	modules[8][7] = (fmtBits>>6)&1 == 1 // f6 at col 7 (col 6 = timing, skipped)
	modules[8][8] = (fmtBits>>7)&1 == 1 // f7 at corner

	// Copy 1 — vertical strip (col 8, top side)
	// f8 at row 7, f9 at row 5, ..., f14 at row 0
	modules[7][8] = (fmtBits>>8)&1 == 1  // f8 at row 7 (row 6 = timing, skipped)
	for i := 9; i <= 14; i++ {
		// f9 → row 5, f10 → row 4, ..., f14 → row 0
		modules[14-i][8] = (fmtBits>>i)&1 == 1
	}

	// Copy 2 — vertical strip (col 8, bottom-left)
	// f0 at row size-1, f1 at row size-2, ..., f6 at row size-7
	for i := 0; i <= 6; i++ {
		modules[sz-1-i][8] = (fmtBits>>i)&1 == 1
	}

	// Copy 2 — horizontal strip (row 8, top-right)
	// f7 at col size-8, f8 at col size-7, ..., f14 at col size-1
	for i := 7; i <= 14; i++ {
		modules[8][sz-15+i] = (fmtBits>>i)&1 == 1
	}
}

// ============================================================================
// Version information (versions 7–40)
// ============================================================================

// reserveVersionInfo marks the 6×3 version information blocks as reserved (v7+).
//
// Two copies:
//   - Near top-right finder: rows 0–5, cols size-11 to size-9
//   - Near bottom-left finder: rows size-11 to size-9, cols 0–5
func (g *workGrid) reserveVersionInfo(version int) {
	if version < 7 {
		return
	}
	sz := g.size
	for r := 0; r < 6; r++ {
		for dc := 0; dc < 3; dc++ {
			g.reserved[r][sz-11+dc] = true
		}
	}
	for dr := 0; dr < 3; dr++ {
		for c := 0; c < 6; c++ {
			g.reserved[sz-11+dr][c] = true
		}
	}
}

// computeVersionBits computes the 18-bit version information for versions 7–40.
//
// Construction:
//  1. Take the 6-bit version number.
//  2. Left-shift by 12 to make room for a 12-bit BCH remainder.
//  3. Compute BCH(18,6) remainder using G(x) = 0x1F25:
//     G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1
//  4. Append: 18 bits = version<<12 | remainder
//
// Scanners read version info on two copies to withstand damage near the top-right
// or bottom-left finder.
func computeVersionBits(version int) int {
	rem := version << 12
	for i := 17; i >= 12; i-- {
		if (rem>>i)&1 != 0 {
			rem ^= 0x1F25 << (i - 12)
		}
	}
	return (version << 12) | (rem & 0xFFF)
}

// writeVersionInfo writes version information into both 6×3 blocks.
//
// Bit layout for the 18-bit version word:
//
//	Top-right block:    bit i → (5 - i/3, size - 9 - i%3)
//	Bottom-left block:  bit i → (size - 9 - i%3, 5 - i/3)
//
// In other words the top-right block reads column-by-column from left to right,
// and the bottom-left block is the transpose.
func writeVersionInfo(modules [][]bool, sz, version int) {
	if version < 7 {
		return
	}
	bits := computeVersionBits(version)
	for i := 0; i < 18; i++ {
		dark := (bits>>i)&1 == 1
		a := 5 - i/3
		b := sz - 9 - i%3
		modules[a][b] = dark  // top-right block
		modules[b][a] = dark  // bottom-left block (transposed)
	}
}

// placeDarkModule places the always-dark module at position (4V+9, 8).
//
// This module is always dark, never masked, and not part of the data or ECC.
// It exists to prevent certain degenerate format information patterns.
// It is adjacent to the bottom of the format info strip and the left edge
// of the bottom-left finder separator.
func (g *workGrid) placeDarkModule(version int) {
	g.set(4*version+9, 8, true, true)
}

// ============================================================================
// Zigzag data placement
// ============================================================================

// placeBits places the interleaved codeword stream into the grid using the
// two-column zigzag scan.
//
// The scan starts from the bottom-right corner and proceeds leftward in 2-column
// strips, snaking upward and downward alternately:
//
//	→ bottom-right corner (col = size-1)
//	↑ scan upward in columns (size-1, size-2)
//	→ shift left to (size-3, size-4)
//	↓ scan downward
//	→ shift left...
//	  (skip col 6 entirely — it's the vertical timing strip)
//
// Reserved modules are skipped; only data/ECC modules are filled.
// Any bits left over (remainder bits) are zero (these are the remainder bits
// appended after the interleaved codewords).
func (g *workGrid) placeBits(codewords []byte, version int) {
	sz := g.size

	// Flatten codewords to individual bits (MSB first for each byte).
	bits := make([]bool, 0, len(codewords)*8+7)
	for _, cw := range codewords {
		for b := 7; b >= 0; b-- {
			bits = append(bits, (cw>>b)&1 == 1)
		}
	}
	// Append remainder bits (always zero).
	for i := 0; i < numRemainderBits(version); i++ {
		bits = append(bits, false)
	}

	bitIdx := 0
	up := true      // current scan direction: true = upward (bottom to top)
	col := sz - 1   // leading column of the current 2-column strip

	for col >= 1 {
		for vi := 0; vi < sz; vi++ {
			// Determine the row based on scan direction.
			row := sz - 1 - vi
			if !up {
				row = vi
			}

			// Process both columns in this strip: col and col-1.
			for _, dc := range [2]int{0, 1} {
				c := col - dc
				if c == 6 {
					// Column 6 is the vertical timing strip; skip it always.
					continue
				}
				if g.reserved[row][c] {
					// Reserved module (finder, timing, format, alignment, etc.) — skip.
					continue
				}
				// Place the next bit (or 0 if we've consumed all bits).
				if bitIdx < len(bits) {
					g.modules[row][c] = bits[bitIdx]
					bitIdx++
				}
			}
		}

		// Flip direction for the next strip.
		up = !up
		col -= 2

		// After col goes to 6 (the timing column), jump to 5 to skip it.
		if col == 6 {
			col = 5
		}
	}
}

// ============================================================================
// Mask patterns
// ============================================================================

// maskCondition returns true when the mask pattern should flip this module.
//
// ISO 18004 Table 10 defines 8 mask patterns. Each is a mathematical condition
// on (row, col). When the condition is true for a non-reserved module, that
// module is flipped (dark ↔ light).
//
// The purpose of masking is to prevent degenerate patterns:
//   - Large solid areas confuse scanners that look for contrast transitions.
//   - Finder-pattern lookalikes trigger false detections.
//   - Long runs cause clock recovery to fail in some decoders.
//
// Each of the 8 patterns distributes bits differently across the grid, and
// the encoder evaluates all 8, keeping the one with the best (lowest) penalty.
func maskCondition(mask, row, col int) bool {
	switch mask {
	case 0:
		return (row+col)%2 == 0
	case 1:
		return row%2 == 0
	case 2:
		return col%3 == 0
	case 3:
		return (row+col)%3 == 0
	case 4:
		return (row/2+col/3)%2 == 0
	case 5:
		return (row*col)%2+(row*col)%3 == 0
	case 6:
		return ((row*col)%2+(row*col)%3)%2 == 0
	case 7:
		return ((row+col)%2+(row*col)%3)%2 == 0
	}
	return false
}

// applyMask returns a new copy of the modules with mask pattern applied.
// Only non-reserved modules are flipped; structural modules are left as-is.
func applyMask(modules, reserved [][]bool, sz, maskIdx int) [][]bool {
	result := make([][]bool, sz)
	for r := 0; r < sz; r++ {
		result[r] = make([]bool, sz)
		for c := 0; c < sz; c++ {
			if reserved[r][c] {
				result[r][c] = modules[r][c]
			} else {
				result[r][c] = modules[r][c] != maskCondition(maskIdx, r, c)
			}
		}
	}
	return result
}

// ============================================================================
// Penalty scoring (ISO 18004 Section 7.8.3)
// ============================================================================

// computePenalty computes the 4-rule penalty score for a masked module array.
//
// The encoder evaluates all 8 mask patterns and picks the one with the lowest
// total penalty. A lower penalty means the masked grid is more "balanced" and
// less likely to confuse a scanner.
func computePenalty(modules [][]bool, sz int) int {
	penalty := 0

	// ── Rule 1: Runs of ≥5 same-colour modules in any row or column ──────────
	//
	// Score += (run_length - 2) for each run of length ≥ 5.
	//   run of 5 → +3
	//   run of 6 → +4
	//   run of 7 → +5, etc.
	//
	// Scanned for each row (horizontal) and each column (vertical).
	for r := 0; r < sz; r++ {
		// Horizontal scan
		run := 1
		prev := modules[r][0]
		for c := 1; c < sz; c++ {
			cur := modules[r][c]
			if cur == prev {
				run++
			} else {
				if run >= 5 {
					penalty += run - 2
				}
				run = 1
				prev = cur
			}
		}
		if run >= 5 {
			penalty += run - 2
		}

		// Vertical scan (reuse row index r as column index)
		run = 1
		prev = modules[0][r]
		for c := 1; c < sz; c++ {
			cur := modules[c][r]
			if cur == prev {
				run++
			} else {
				if run >= 5 {
					penalty += run - 2
				}
				run = 1
				prev = cur
			}
		}
		if run >= 5 {
			penalty += run - 2
		}
	}

	// ── Rule 2: 2×2 blocks of same-colour modules ─────────────────────────────
	//
	// Score += 3 for each 2×2 square where all four modules are the same colour.
	for r := 0; r < sz-1; r++ {
		for c := 0; c < sz-1; c++ {
			d := modules[r][c]
			if d == modules[r][c+1] && d == modules[r+1][c] && d == modules[r+1][c+1] {
				penalty += 3
			}
		}
	}

	// ── Rule 3: Finder-pattern-like sequences ─────────────────────────────────
	//
	// Any occurrence of the 11-module pattern 1 0 1 1 1 0 1 0 0 0 0
	// or its reverse in any row or column adds 40 to the penalty.
	//
	// These patterns resemble the QR finder patterns in their 1:1:3:1:1 ratio
	// and could trigger a false positive in some decoders.
	p1 := [11]bool{true, false, true, true, true, false, true, false, false, false, false}
	p2 := [11]bool{false, false, false, false, true, false, true, true, true, false, true}

	for a := 0; a < sz; a++ {
		for b := 0; b <= sz-11; b++ {
			mH1, mH2, mV1, mV2 := true, true, true, true
			for k := 0; k < 11; k++ {
				bH := modules[a][b+k]
				bV := modules[b+k][a]
				if bH != p1[k] {
					mH1 = false
				}
				if bH != p2[k] {
					mH2 = false
				}
				if bV != p1[k] {
					mV1 = false
				}
				if bV != p2[k] {
					mV2 = false
				}
			}
			if mH1 {
				penalty += 40
			}
			if mH2 {
				penalty += 40
			}
			if mV1 {
				penalty += 40
			}
			if mV2 {
				penalty += 40
			}
		}
	}

	// ── Rule 4: Dark module proportion ────────────────────────────────────────
	//
	// Count dark modules. The closer the ratio is to 50%, the lower the penalty.
	//
	//	dark_ratio = dark / total * 100
	//	prev5 = floor(dark_ratio / 5) * 5   (nearest lower multiple of 5)
	//	penalty += min(|prev5 - 50|, |prev5 + 5 - 50|) / 5 * 10
	//
	// Zero penalty at exactly 50%. +10 at 45% or 55%. +20 at 40% or 60%, etc.
	dark := 0
	for r := 0; r < sz; r++ {
		for c := 0; c < sz; c++ {
			if modules[r][c] {
				dark++
			}
		}
	}
	total := sz * sz
	// Multiply by 100 first and use integer arithmetic to avoid floating-point.
	// dark_ratio_pct_x100 = dark * 10000 / total
	ratio := dark * 100 / total
	prev5 := (ratio / 5) * 5
	next5 := prev5 + 5
	d1 := prev5 - 50
	if d1 < 0 {
		d1 = -d1
	}
	d2 := next5 - 50
	if d2 < 0 {
		d2 = -d2
	}
	minD := d1
	if d2 < minD {
		minD = d2
	}
	penalty += (minD / 5) * 10

	return penalty
}

// ============================================================================
// Version selection
// ============================================================================

// bitsNeededForInput computes the number of bits needed to encode the input
// in the given mode at the given version, including mode indicator and char count.
//
// This is used to find the minimum version that fits. We must check at each
// version because charCountBits changes at version boundaries.
func bitsNeededForInput(inputBytes []byte, mode EncodingMode, version int) int {
	bits := 4 // mode indicator

	var charCount int
	switch mode {
	case ModeNumeric:
		n := len(inputBytes)
		// Groups of 3 → 10 bits each. Pairs → 7 bits. Singles → 4 bits.
		charCount = n
		bits += charCountBits(mode, version)
		triples := n / 3
		remainder := n % 3
		bits += triples * 10
		if remainder == 2 {
			bits += 7
		} else if remainder == 1 {
			bits += 4
		}
		return bits
	case ModeAlphanumeric:
		n := len(inputBytes)
		charCount = n
		bits += charCountBits(mode, version)
		pairs := n / 2
		bits += pairs * 11
		if n%2 == 1 {
			bits += 6
		}
		return bits
	default: // byte mode
		charCount = len(inputBytes)
		bits += charCountBits(mode, version)
		bits += charCount * 8
		return bits
	}
}

// selectVersion returns the minimum QR version (1–40) that can hold the input
// string at the given ECC level.
//
// We iterate from version 1 upward and compute the minimum bits needed for the
// chosen mode at each version (mode and char-count field size both matter).
// The first version where numDataCodewords*8 >= bitsNeeded is our answer.
func selectVersion(input string, ecc EccLevel, forcedVersion int) (int, error) {
	if forcedVersion > 0 {
		// Caller specified a version; validate it can hold the data.
		mode := selectMode(input)
		inputBytes := []byte(input)
		needed := bitsNeededForInput(inputBytes, mode, forcedVersion)
		capacity := numDataCodewords(forcedVersion, ecc) * 8
		if needed > capacity {
			return 0, errTooLong(fmt.Sprintf(
				"input (%d chars) does not fit in version %d at ECC %v (need %d bits, have %d)",
				len(input), forcedVersion, ecc, needed, capacity,
			))
		}
		return forcedVersion, nil
	}

	// Quick bounds check: if even the maximum mode/version can't fit, fail fast.
	// QR v40 byte mode at L holds 2953 bytes.
	if len(input) > 7089 {
		return 0, errTooLong(fmt.Sprintf(
			"input length %d exceeds QR Code v40 maximum capacity (~7089 chars)",
			len(input),
		))
	}

	mode := selectMode(input)
	inputBytes := []byte(input)

	for v := 1; v <= 40; v++ {
		needed := bitsNeededForInput(inputBytes, mode, v)
		capacity := numDataCodewords(v, ecc) * 8
		if needed <= capacity {
			return v, nil
		}
	}

	return 0, errTooLong(fmt.Sprintf(
		"input (%d chars, ECC level %v) exceeds version 40 capacity",
		len(input), ecc,
	))
}

// ============================================================================
// Grid initialisation
// ============================================================================

// buildGrid constructs the initial work grid with all structural elements placed.
// Data modules are left as false (light); they will be filled by placeBits.
func buildGrid(version int) *workGrid {
	sz := symbolSize(version)
	g := newWorkGrid(sz)

	// ── Finder patterns at three corners ────────────────────────────────────
	g.placeFinder(0, 0)       // top-left
	g.placeFinder(0, sz-7)    // top-right
	g.placeFinder(sz-7, 0)    // bottom-left

	// ── Separators: 1-module light border just outside each finder ───────────
	//
	// The separator isolates the finder from the data area. Without it, the
	// strongly-contrasting finder edges could bleed into data modules and cause
	// read errors. Separators are always light (false).
	//
	// Top-left finder: row 7 (horizontal) and col 7 (vertical)
	for i := 0; i <= 7; i++ {
		g.set(7, i, false, true)   // row 7, cols 0..7
		g.set(i, 7, false, true)   // col 7, rows 0..7
	}
	// Top-right finder: row 7 (cols sz-8..sz-1) and col sz-8 (rows 0..7)
	for i := 0; i <= 7; i++ {
		g.set(7, sz-1-i, false, true)  // row 7, cols sz-1..sz-8
		g.set(i, sz-8, false, true)    // col sz-8, rows 0..7
	}
	// Bottom-left finder: row sz-8 (cols 0..7) and col 7 (rows sz-7..sz-1)
	for i := 0; i <= 7; i++ {
		g.set(sz-8, i, false, true)    // row sz-8, cols 0..7
		g.set(sz-1-i, 7, false, true)  // col 7, rows sz-7..sz-1
	}

	// Timing strips must come before alignments (they mark row/col 6 as reserved).
	g.placeTimingStrips()

	// Alignment patterns (versions 2+). Overlaps with finders/timing auto-skipped.
	g.placeAllAlignments(version)

	// Reserve format and version info modules.
	g.reserveFormatInfo()
	g.reserveVersionInfo(version)

	// Always-dark module.
	g.placeDarkModule(version)

	return g
}

// ============================================================================
// Public API
// ============================================================================

// Encode encodes a UTF-8 string into a QR Code ModuleGrid.
//
// The returned ModuleGrid is (4V+17) × (4V+17) where V is the auto-selected
// (or forced) version. Every true module is dark; every false module is light.
//
// Pass the result to barcode2d.Layout() to get pixel coordinates, or to
// EncodeToScene() for a one-step encode+layout.
//
// Error cases:
//   - InputTooLongError: data does not fit in any version 1–40 at the chosen ECC.
//   - InvalidInputError: data contains characters incompatible with the chosen mode.
func Encode(data string, opts EncodeOptions) (barcode2d.ModuleGrid, error) {
	ecc := opts.Level

	// Resolve version (auto-select if not forced).
	version, err := selectVersion(data, ecc, opts.Version)
	if err != nil {
		return barcode2d.ModuleGrid{}, err
	}

	sz := symbolSize(version)

	// Build data codewords (bit stream assembly + padding).
	dataCW, err := buildDataCodewords(data, version, ecc)
	if err != nil {
		return barcode2d.ModuleGrid{}, err
	}

	// Compute RS ECC for each block.
	blocks := computeBlocks(dataCW, version, ecc)

	// Interleave codewords from all blocks.
	interleaved := interleaveBlocks(blocks)

	// Build the grid with all structural elements placed.
	grid := buildGrid(version)

	// Fill data modules with the interleaved codeword stream.
	grid.placeBits(interleaved, version)

	// ── Evaluate all 8 mask patterns ────────────────────────────────────────
	//
	// For each candidate mask:
	//   1. Apply the mask to produce a new module array.
	//   2. Write format info for this ECC/mask combination into the masked array.
	//   3. Compute the 4-rule penalty score.
	// Keep the mask with the lowest penalty.
	//
	// We write format info before scoring because the format info bits themselves
	// contribute to the penalty (they are not reserved in the penalty scan).
	bestMask := 0
	bestPenalty := -1 // -1 means "not yet set"

	for m := 0; m < 8; m++ {
		masked := applyMask(grid.modules, grid.reserved, sz, m)
		fmtBits := computeFormatBits(ecc, m)
		writeFormatInfo(masked, sz, fmtBits)
		p := computePenalty(masked, sz)
		if bestPenalty < 0 || p < bestPenalty {
			bestPenalty = p
			bestMask = m
		}
	}

	// ── Finalize with the best mask ──────────────────────────────────────────
	finalModules := applyMask(grid.modules, grid.reserved, sz, bestMask)
	writeFormatInfo(finalModules, sz, computeFormatBits(ecc, bestMask))
	writeVersionInfo(finalModules, sz, version)

	// ── Convert to immutable ModuleGrid ──────────────────────────────────────
	return barcode2d.ModuleGrid{
		Rows:        uint32(sz),
		Cols:        uint32(sz),
		Modules:     finalModules,
		ModuleShape: barcode2d.ModuleShapeSquare,
	}, nil
}

// EncodeToScene encodes a UTF-8 string directly to a PaintScene.
//
// This is a convenience function that calls Encode() and then barcode2d.Layout()
// in one step. The config parameter controls module size, quiet zone, and colours.
func EncodeToScene(
	data string,
	opts EncodeOptions,
	config barcode2d.Barcode2DLayoutConfig,
) (paintinstructions.PaintScene, error) {
	grid, err := Encode(data, opts)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return barcode2d.Layout(grid, &config)
}

// ============================================================================
// Utility
// ============================================================================

func absInt(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

// Ensure the paint-instructions import is used (it flows through barcode2d.Layout).
var _ paintinstructions.PaintScene
