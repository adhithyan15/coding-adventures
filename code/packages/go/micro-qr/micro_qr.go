// Package microqr encodes strings as Micro QR Code symbols (ISO/IEC 18004:2015 Annex E).
//
// # What is Micro QR Code?
//
// Micro QR Code is the compact sibling of regular QR Code, designed for applications
// where space is so tight that even the smallest standard QR (21×21 at version 1) is
// too large. Common uses include surface-mount component labels on circuit boards,
// miniature product markings, and tiny industrial tags scanned in controlled environments.
//
// The defining structural difference: Micro QR uses a SINGLE finder pattern in the
// top-left corner, rather than regular QR's three corner finders. This saves dramatic
// space at the cost of some scanning robustness — Micro QR targets factory-floor
// environments, not consumer smartphones.
//
// # Symbol sizes
//
//	M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
//	formula: size = 2 × version_number + 9
//
// # Key differences from regular QR Code
//
//   - Single finder pattern at top-left only (one 7×7 square, not three).
//   - Timing patterns at row 0 / col 0 (not row 6 / col 6).
//   - Only 4 mask patterns (not 8).
//   - Format XOR mask 0x4445 (not 0x5412).
//   - Single copy of format info (not two).
//   - 2-module quiet zone (not 4).
//   - Narrower mode indicators (0–3 bits instead of 4).
//   - Single block RS encoding (no interleaving).
//
// # Encoding pipeline
//
//	input string
//	  → auto-select smallest symbol (M1..M4) and encoding mode
//	  → build bit stream (mode indicator + char count + data + terminator + padding)
//	  → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
//	  → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
//	  → zigzag data placement (two-column snake from bottom-right)
//	  → evaluate 4 mask patterns, pick lowest penalty score
//	  → write format information (15 bits, single copy, XOR 0x4445)
//	  → ModuleGrid
//
// # Quick start
//
//	// Encode "HELLO" — auto-selects M2 (13×13)
//	grid, err := microqr.Encode("HELLO", nil, nil)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Printf("Symbol is %d×%d\n", grid.Rows, grid.Cols)
//
//	// Encode at a specific version and ECC level
//	v := microqr.VersionM4
//	e := microqr.EccL
//	grid, err = microqr.EncodeAt("https://a.b", v, e)
package microqr

import (
	"errors"
	"fmt"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
)

// Version is the semantic version of this package.
const Version = "0.1.0"

// ============================================================================
// Public types — MicroQRVersion, MicroQREccLevel
// ============================================================================

// MicroQRVersion identifies the Micro QR symbol size (M1..M4).
//
// Each step up adds two rows and two columns:
//
//	M1 = 11×11, M2 = 13×13, M3 = 15×15, M4 = 17×17
//	size formula: 2 × version_number + 9
//
// Higher versions hold more data but use more physical space. Choose the
// smallest version that fits the input.
type MicroQRVersion int

const (
	// VersionM1 is the smallest Micro QR symbol: 11×11 modules.
	// Supports numeric encoding only. Error detection only (no correction).
	VersionM1 MicroQRVersion = 1
	// VersionM2 is a 13×13 symbol. Supports numeric and alphanumeric modes.
	VersionM2 MicroQRVersion = 2
	// VersionM3 is a 15×15 symbol. Supports numeric, alphanumeric, and byte modes.
	VersionM3 MicroQRVersion = 3
	// VersionM4 is the largest Micro QR symbol: 17×17 modules.
	// Supports all modes including byte. Supports L, M, and Q ECC levels.
	VersionM4 MicroQRVersion = 4
)

// MicroQREccLevel is the error correction level.
//
// Unlike regular QR Code which supports L/M/Q/H, Micro QR has a restricted set:
//
//	| Level     | Available in | Recovery capacity |
//	|-----------|-------------|-------------------|
//	| Detection | M1 only     | detects errors only (no correction) |
//	| L (Low)   | M2, M3, M4  | ~7% of codewords recoverable |
//	| M (Medium)| M2, M3, M4  | ~15% of codewords recoverable |
//	| Q (Quartile)| M4 only   | ~25% of codewords recoverable |
//
// Level H (High) is not available in any Micro QR symbol — the symbols are
// so small that 30% redundancy would leave almost no room for data.
type MicroQREccLevel int

const (
	// EccDetection provides error detection only (M1 exclusively).
	EccDetection MicroQREccLevel = iota
	// EccL is Low error correction (~7% recovery). Available in M2, M3, M4.
	EccL
	// EccM is Medium error correction (~15% recovery). Available in M2, M3, M4.
	EccM
	// EccQ is Quartile error correction (~25% recovery). M4 only.
	EccQ
)

// ============================================================================
// Error types
// ============================================================================

// MicroQRError is the base error type for all micro-qr errors.
type MicroQRError struct {
	Kind    string
	Message string
}

func (e *MicroQRError) Error() string {
	return fmt.Sprintf("micro-qr [%s]: %s", e.Kind, e.Message)
}

func errInputTooLong(msg string) error {
	return &MicroQRError{Kind: "InputTooLong", Message: msg}
}

func errECCNotAvailable(msg string) error {
	return &MicroQRError{Kind: "ECCNotAvailable", Message: msg}
}

func errUnsupportedMode(msg string) error {
	return &MicroQRError{Kind: "UnsupportedMode", Message: msg}
}

// IsInputTooLong returns true if the error is an InputTooLong error.
func IsInputTooLong(err error) bool {
	var e *MicroQRError
	return errors.As(err, &e) && e.Kind == "InputTooLong"
}

// IsECCNotAvailable returns true if the error is an ECCNotAvailable error.
func IsECCNotAvailable(err error) bool {
	var e *MicroQRError
	return errors.As(err, &e) && e.Kind == "ECCNotAvailable"
}

// IsUnsupportedMode returns true if the error is an UnsupportedMode error.
func IsUnsupportedMode(err error) bool {
	var e *MicroQRError
	return errors.As(err, &e) && e.Kind == "UnsupportedMode"
}

// ============================================================================
// Symbol configurations — one struct per valid (version, ECC) combination
// ============================================================================

// symbolConfig holds all compile-time constants for a single (version, ECC)
// combination.
//
// There are exactly 8 valid combinations for Micro QR:
//
//	M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q
//
// Think of this like a pre-compiled look-up table: rather than computing
// codeword counts and field widths at runtime, the encoder reads from these
// structs which were derived from the ISO/IEC 18004:2015 Annex E tables.
type symbolConfig struct {
	version MicroQRVersion
	ecc     MicroQREccLevel
	// symbolIndicator is the 3-bit value placed in format information (0..7).
	// It encodes both version and ECC level together — 8 combinations fit in 3 bits.
	symbolIndicator uint8
	// size is the symbol side length in modules (11, 13, 15, or 17).
	size int
	// dataCW is the number of data codewords (8-bit bytes), except M1 which
	// uses 2 full bytes + one 4-bit nibble = 2.5 bytes = "3 codewords" where
	// the last byte's lower nibble is forced to zero.
	dataCW int
	// eccCW is the number of ECC (Reed-Solomon) codewords.
	eccCW int
	// numericCap is the maximum numeric characters. 0 = mode not supported.
	numericCap int
	// alphaCap is the maximum alphanumeric characters. 0 = mode not supported.
	alphaCap int
	// byteCap is the maximum byte-mode characters. 0 = mode not supported.
	byteCap int
	// terminatorBits is the number of zero-bits appended after data (3/5/7/9).
	// Longer terminators for larger symbols ensure the last codeword is padded
	// to a byte boundary more cleanly.
	terminatorBits int
	// modeIndicatorBits is how many bits the mode indicator uses (0=M1, 1=M2,
	// 2=M3, 3=M4). M1 has no indicator because it only supports one mode.
	modeIndicatorBits int
	// ccBitsNumeric is character count field width for numeric mode.
	ccBitsNumeric int
	// ccBitsAlpha is character count field width for alphanumeric mode.
	ccBitsAlpha int
	// ccBitsByte is character count field width for byte mode.
	ccBitsByte int
	// m1HalfCW is true only for M1: last data codeword is 4 bits, total = 20 bits.
	m1HalfCW bool
}

// symbolConfigs is the complete table of all 8 valid Micro QR configurations
// from ISO 18004:2015 Annex E, in smallest-first order (M1..M4, L before M before Q).
//
// Data capacities, codeword counts, and field widths are all from the standard.
// The encoder iterates this slice to find the first config that can hold the input.
var symbolConfigs = []symbolConfig{
	// ── M1 / Detection ────────────────────────────────────────────────────────
	// The smallest possible Micro QR. Only numeric encoding. 5 digits maximum.
	// Error DETECTION only — no correction capability. 2 ECC codewords (just
	// enough for a polynomial remainder check).
	{
		version: VersionM1, ecc: EccDetection,
		symbolIndicator:  0, size: 11,
		dataCW: 3, eccCW: 2,
		numericCap: 5, alphaCap: 0, byteCap: 0,
		terminatorBits: 3, modeIndicatorBits: 0,
		ccBitsNumeric: 3, ccBitsAlpha: 0, ccBitsByte: 0,
		m1HalfCW: true,
	},
	// ── M2 / L ────────────────────────────────────────────────────────────────
	// 13×13. Adds alphanumeric and byte modes. 5 data codewords, 5 ECC.
	{
		version: VersionM2, ecc: EccL,
		symbolIndicator:  1, size: 13,
		dataCW: 5, eccCW: 5,
		numericCap: 10, alphaCap: 6, byteCap: 4,
		terminatorBits: 5, modeIndicatorBits: 1,
		ccBitsNumeric: 4, ccBitsAlpha: 3, ccBitsByte: 4,
		m1HalfCW: false,
	},
	// ── M2 / M ────────────────────────────────────────────────────────────────
	// Same 13×13 grid, but 4 data codewords + 6 ECC = more redundancy, less data.
	{
		version: VersionM2, ecc: EccM,
		symbolIndicator:  2, size: 13,
		dataCW: 4, eccCW: 6,
		numericCap: 8, alphaCap: 5, byteCap: 3,
		terminatorBits: 5, modeIndicatorBits: 1,
		ccBitsNumeric: 4, ccBitsAlpha: 3, ccBitsByte: 4,
		m1HalfCW: false,
	},
	// ── M3 / L ────────────────────────────────────────────────────────────────
	// 15×15. 11 data codewords + 6 ECC.
	{
		version: VersionM3, ecc: EccL,
		symbolIndicator:  3, size: 15,
		dataCW: 11, eccCW: 6,
		numericCap: 23, alphaCap: 14, byteCap: 9,
		terminatorBits: 7, modeIndicatorBits: 2,
		ccBitsNumeric: 5, ccBitsAlpha: 4, ccBitsByte: 4,
		m1HalfCW: false,
	},
	// ── M3 / M ────────────────────────────────────────────────────────────────
	// Same 15×15 grid, 9 data + 8 ECC.
	{
		version: VersionM3, ecc: EccM,
		symbolIndicator:  4, size: 15,
		dataCW: 9, eccCW: 8,
		numericCap: 18, alphaCap: 11, byteCap: 7,
		terminatorBits: 7, modeIndicatorBits: 2,
		ccBitsNumeric: 5, ccBitsAlpha: 4, ccBitsByte: 4,
		m1HalfCW: false,
	},
	// ── M4 / L ────────────────────────────────────────────────────────────────
	// 17×17. 16 data codewords + 8 ECC.
	{
		version: VersionM4, ecc: EccL,
		symbolIndicator:  5, size: 17,
		dataCW: 16, eccCW: 8,
		numericCap: 35, alphaCap: 21, byteCap: 15,
		terminatorBits: 9, modeIndicatorBits: 3,
		ccBitsNumeric: 6, ccBitsAlpha: 5, ccBitsByte: 5,
		m1HalfCW: false,
	},
	// ── M4 / M ────────────────────────────────────────────────────────────────
	// Same 17×17 grid, 14 data + 10 ECC.
	{
		version: VersionM4, ecc: EccM,
		symbolIndicator:  6, size: 17,
		dataCW: 14, eccCW: 10,
		numericCap: 30, alphaCap: 18, byteCap: 13,
		terminatorBits: 9, modeIndicatorBits: 3,
		ccBitsNumeric: 6, ccBitsAlpha: 5, ccBitsByte: 5,
		m1HalfCW: false,
	},
	// ── M4 / Q ────────────────────────────────────────────────────────────────
	// Same 17×17 grid, 10 data + 14 ECC. Highest redundancy in Micro QR.
	{
		version: VersionM4, ecc: EccQ,
		symbolIndicator:  7, size: 17,
		dataCW: 10, eccCW: 14,
		numericCap: 21, alphaCap: 13, byteCap: 9,
		terminatorBits: 9, modeIndicatorBits: 3,
		ccBitsNumeric: 6, ccBitsAlpha: 5, ccBitsByte: 5,
		m1HalfCW: false,
	},
}

// ============================================================================
// RS generator polynomials (compile-time constants)
// ============================================================================

// rsGenerators is a map from ECC codeword count to the monic RS generator
// polynomial for GF(256)/0x11D with b=0 convention.
//
// The polynomial g(x) = (x+α⁰)(x+α¹)···(x+α^{n-1}) where α is the primitive
// root 2 of GF(256). For n ECC codewords, the generator has degree n and n+1
// coefficients (including the leading 1).
//
// These constants are the same polynomials used in regular QR Code for blocks
// with the same ECC codeword count. Micro QR needs only the counts: 2, 5, 6, 8, 10, 14.
//
// Embedding as constants avoids any possibility of computation error at runtime.
var rsGenerators = map[int][]byte{
	// g(x) = (x+α⁰)(x+α¹) = x² + 3x + 2
	2: {0x01, 0x03, 0x02},
	// 5 ECC codewords
	5: {0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68},
	// 6 ECC codewords
	6: {0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37},
	// 8 ECC codewords
	8: {0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3},
	// 10 ECC codewords
	10: {0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45},
	// 14 ECC codewords
	14: {0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac},
}

// ============================================================================
// Pre-computed format information table
// ============================================================================

// formatTable holds all 32 pre-computed 15-bit format words (after XOR with 0x4445).
//
// Indexed as formatTable[symbolIndicator][maskPattern].
//
// # Format word structure (15 bits total)
//
//	[symbol_indicator (3b)][mask_pattern (2b)][BCH-10 remainder (10b)]
//
// The 10-bit BCH remainder is computed using the generator polynomial:
//
//	G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1  (0x537)
//
// After computing the 15-bit word, it is XOR-masked with 0x4445
// (Micro QR specific — regular QR uses 0x5412). This masking ensures that a
// Micro QR symbol cannot be misidentified as a regular QR symbol by a decoder.
//
// Rather than computing these at runtime, they are embedded as constants
// derived from the ISO standard worked examples.
var formatTable = [8][4]uint16{
	{0x4445, 0x4172, 0x4E2B, 0x4B1C}, // M1 / Detection
	{0x5528, 0x501F, 0x5F46, 0x5A71}, // M2-L
	{0x6649, 0x637E, 0x6C27, 0x6910}, // M2-M
	{0x7764, 0x7253, 0x7D0A, 0x783D}, // M3-L
	{0x06DE, 0x03E9, 0x0CB0, 0x0987}, // M3-M
	{0x17F3, 0x12C4, 0x1D9D, 0x18AA}, // M4-L
	{0x24B2, 0x2185, 0x2EDC, 0x2BEB}, // M4-M
	{0x359F, 0x30A8, 0x3FF1, 0x3AC6}, // M4-Q
}

// ============================================================================
// Encoding mode
// ============================================================================

// encodingMode describes how input characters are packed into bits.
//
// The mode is chosen to minimize the number of bits needed:
//
//	numeric     — only digits 0-9, most compact (3 digits = 10 bits)
//	alphanumeric — digits + A-Z + 7 symbols, medium compact (2 chars = 11 bits)
//	byte        — arbitrary raw bytes, least compact (1 byte = 8 bits)
type encodingMode int

const (
	modeNumeric     encodingMode = iota
	modeAlphanumeric
	modeByte
)

// alphanumChars is the 45-character set recognized by alphanumeric mode.
// Identical to regular QR Code's alphanumeric set.
//
// The position of each character in this string IS its numeric index value
// used during encoding: pair encoding = firstIndex × 45 + secondIndex.
const alphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

// selectMode picks the most compact encoding mode supported by the given config.
//
// Priority (most compact to least): numeric > alphanumeric > byte.
//
// This implements the ISO standard's "minimum codeword" principle: use the
// mode that produces the fewest bits for the full input. Since we encode the
// entire string in a single mode, we pick the tightest mode whose character
// set covers all input characters.
func selectMode(input string, cfg *symbolConfig) (encodingMode, error) {
	// Test numeric: every character must be a decimal digit (0–9).
	isNumeric := true
	for _, ch := range input {
		if ch < '0' || ch > '9' {
			isNumeric = false
			break
		}
	}
	if isNumeric && cfg.ccBitsNumeric > 0 {
		return modeNumeric, nil
	}

	// Test alphanumeric: every character must appear in the 45-char set.
	isAlpha := true
	for _, ch := range input {
		found := false
		for _, ac := range alphanumChars {
			if ch == ac {
				found = true
				break
			}
		}
		if !found {
			isAlpha = false
			break
		}
	}
	if isAlpha && cfg.alphaCap > 0 {
		return modeAlphanumeric, nil
	}

	// Byte mode: any string is encodable as raw bytes.
	if cfg.byteCap > 0 {
		return modeByte, nil
	}

	return 0, errUnsupportedMode(fmt.Sprintf(
		"input cannot be encoded in any mode supported by M%d/%v",
		int(cfg.version), cfg.ecc,
	))
}

// modeIndicatorValue returns the mode indicator bits for the given mode and config.
//
// M1 has no mode indicator (0 bits) because it supports only numeric mode —
// there is nothing to indicate. M2 uses 1 bit (0=numeric, 1=alpha).
// M3 uses 2 bits. M4 uses 3 bits. This matches ISO Annex E Table E.3.
func modeIndicatorValue(mode encodingMode, cfg *symbolConfig) uint32 {
	switch cfg.modeIndicatorBits {
	case 0:
		return 0 // M1: no indicator
	case 1:
		if mode == modeNumeric {
			return 0
		}
		return 1
	case 2:
		switch mode {
		case modeNumeric:
			return 0b00
		case modeAlphanumeric:
			return 0b01
		default: // byte
			return 0b10
		}
	case 3:
		switch mode {
		case modeNumeric:
			return 0b000
		case modeAlphanumeric:
			return 0b001
		default: // byte
			return 0b010
		}
	}
	return 0
}

// charCountBits returns the width of the character count field for the given mode/config.
func charCountBits(mode encodingMode, cfg *symbolConfig) uint32 {
	switch mode {
	case modeNumeric:
		return uint32(cfg.ccBitsNumeric)
	case modeAlphanumeric:
		return uint32(cfg.ccBitsAlpha)
	default:
		return uint32(cfg.ccBitsByte)
	}
}

// ============================================================================
// Bit writer — accumulates bits MSB-first, flushes to bytes
// ============================================================================

// bitWriter accumulates individual bits in MSB-first order and converts them
// to byte slices on demand.
//
// Think of it like writing a long binary number one digit at a time, left to right.
// Each call to write() appends bits to the right end of the growing number.
//
// All QR and Micro QR encoding uses MSB-first (big-endian) bit ordering within
// each codeword. For example, the value 123 in 10-bit numeric encoding becomes:
//
//	123 = 0b00_0111_1011  →  bits: 0 0 0 1 1 1 1 0 1 1
type bitWriter struct {
	bits []uint8 // Each element is 0 or 1 (one bit per byte for simplicity)
}

// write appends count bits from value to the stream, MSB first.
//
// Only the `count` least-significant bits of `value` are used.
// Example: write(0b10110, 4) appends bits 1 0 1 1 (the lower 4 bits, MSB first).
func (bw *bitWriter) write(value uint32, count uint32) {
	for i := int(count) - 1; i >= 0; i-- {
		bw.bits = append(bw.bits, uint8((value>>uint(i))&1))
	}
}

// bitLen returns the current number of bits in the stream.
func (bw *bitWriter) bitLen() int {
	return len(bw.bits)
}

// toBytes packs the accumulated bits into bytes, MSB-first.
// Incomplete trailing byte is zero-padded on the right.
func (bw *bitWriter) toBytes() []byte {
	result := make([]byte, (len(bw.bits)+7)/8)
	for i, b := range bw.bits {
		result[i/8] |= b << uint(7-i%8)
	}
	return result
}

// ============================================================================
// Data encoding helpers
// ============================================================================

// encodeNumeric packs decimal digits into the bit stream.
//
// Groups of 3 digits are converted to their decimal value (0–999) and stored
// in 10 bits. A remaining pair uses 7 bits (0–99). A single trailing digit
// uses 4 bits (0–9). This exploits the fact that 10 bits can hold up to 1023,
// much more than 999, so we never run out of range.
//
// Example:
//
//	"12345" → groups "123" and "45"
//	          123 in 10 bits: 0001111011
//	           45 in  7 bits:    0101101
func encodeNumeric(input string, w *bitWriter) {
	digits := make([]uint32, len(input))
	for i, ch := range input {
		digits[i] = uint32(ch - '0')
	}
	i := 0
	for i+2 < len(digits) {
		w.write(digits[i]*100+digits[i+1]*10+digits[i+2], 10)
		i += 3
	}
	if i+1 < len(digits) {
		w.write(digits[i]*10+digits[i+1], 7)
		i += 2
	}
	if i < len(digits) {
		w.write(digits[i], 4)
	}
}

// encodeAlphanumeric packs characters from the 45-char set into the bit stream.
//
// Pairs of characters are encoded as firstIndex*45+secondIndex in 11 bits.
// A trailing single character uses 6 bits. This is more efficient than 8
// bits/char (the byte-mode approach) because 45²-1 = 2024 < 2^11 = 2048.
//
// The alphanumeric set index order is critical: the lookup position in
// alphanumChars IS the numeric index used in encoding.
func encodeAlphanumeric(input string, w *bitWriter) {
	indices := make([]uint32, 0, len(input))
	for _, ch := range input {
		for idx, ac := range alphanumChars {
			if ch == ac {
				indices = append(indices, uint32(idx))
				break
			}
		}
	}
	i := 0
	for i+1 < len(indices) {
		w.write(indices[i]*45+indices[i+1], 11)
		i += 2
	}
	if i < len(indices) {
		w.write(indices[i], 6)
	}
}

// encodeByteMode writes each byte of the input directly as 8 bits.
//
// UTF-8 strings are encoded byte by byte — a multi-byte UTF-8 sequence
// occupies multiple entries in the character count and data stream.
func encodeByteMode(input string, w *bitWriter) {
	for _, b := range []byte(input) {
		w.write(uint32(b), 8)
	}
}

// ============================================================================
// Reed-Solomon encoder
// ============================================================================

// rsEncode computes the ECC (error correction) codewords for the given data
// using Reed-Solomon over GF(256) with the 0x11D primitive polynomial and
// the b=0 convention (first root is α^0 = 1).
//
// # How Reed-Solomon works (in brief)
//
// Think of the data codewords as the coefficients of a polynomial D(x).
// We want to find a remainder polynomial E(x) such that D(x)·x^n + E(x) is
// divisible by the generator polynomial G(x) over GF(256). When a scanner
// reads the transmitted polynomial T(x) = D(x)·x^n + E(x) and computes
// T(x) mod G(x), a zero result means no errors detected.
//
// The algorithm is an LFSR (linear feedback shift register) implementation
// of polynomial long division:
//
//	ecc = [0] × n
//	for each data byte b:
//	    feedback = b XOR ecc[0]
//	    shift ecc left by one (drop ecc[0], append 0)
//	    for each position i:
//	        ecc[i] ^= gf_mul(generator[i+1], feedback)
//
// This is the same algorithm used in regular QR Code.
func rsEncode(data []byte, generator []byte) []byte {
	n := len(generator) - 1 // number of ECC codewords = degree of generator poly
	rem := make([]byte, n)
	for _, b := range data {
		fb := b ^ rem[0]
		// Shift the register left by one (drop rem[0], add a 0 at the end)
		copy(rem, rem[1:])
		rem[n-1] = 0
		if fb != 0 {
			for i := 0; i < n; i++ {
				rem[i] ^= gf256.Multiply(generator[i+1], fb)
			}
		}
	}
	return rem
}

// ============================================================================
// Data codeword assembly
// ============================================================================

// buildDataCodewords assembles the complete data codeword byte sequence.
//
// For all symbols except M1, the structure is:
//
//	[mode indicator (0/1/2/3 bits)] [char count] [encoded data] [terminator]
//	[zero-pad to byte boundary] [0xEC/0x11 fill to reach dataCW bytes]
//
// For M1 (m1HalfCW = true):
//
//	Total capacity = 20 bits = 2 full bytes + 4-bit nibble
//	The RS encoder receives 3 bytes where byte[2] = data in upper 4 bits, lower 4 = 0.
//	No 0xEC/0x11 padding — M1 uses a fixed 20-bit window.
//
// The alternating 0xEC/0x11 padding pattern is shared with regular QR Code.
// It avoids runs of identical bytes that could confuse error correction.
func buildDataCodewords(input string, cfg *symbolConfig, mode encodingMode) []byte {
	// Total usable data bit capacity.
	// M1 is special: 3 codewords but the last is only 4 bits → 3×8−4 = 20 bits total.
	totalBits := cfg.dataCW * 8
	if cfg.m1HalfCW {
		totalBits -= 4 // M1: 3×8 − 4 = 20 bits
	}

	var w bitWriter

	// Mode indicator (0/1/2/3 bits depending on symbol version)
	if cfg.modeIndicatorBits > 0 {
		w.write(modeIndicatorValue(mode, cfg), uint32(cfg.modeIndicatorBits))
	}

	// Character count — number of characters (not bytes) for numeric/alpha,
	// number of bytes for byte mode (since each UTF-8 byte counts separately).
	var charCount uint32
	if mode == modeByte {
		charCount = uint32(len([]byte(input)))
	} else {
		charCount = uint32(len([]rune(input)))
	}
	w.write(charCount, charCountBits(mode, cfg))

	// Encoded data bits
	switch mode {
	case modeNumeric:
		encodeNumeric(input, &w)
	case modeAlphanumeric:
		encodeAlphanumeric(input, &w)
	case modeByte:
		encodeByteMode(input, &w)
	}

	// Terminator: up to terminatorBits zero bits (truncated if capacity is already full)
	remaining := totalBits - w.bitLen()
	if remaining > 0 {
		termLen := cfg.terminatorBits
		if termLen > remaining {
			termLen = remaining
		}
		w.write(0, uint32(termLen))
	}

	// M1 special case: pack into exactly 20 bits → 3 bytes
	if cfg.m1HalfCW {
		bits := make([]uint8, 20)
		copy(bits, w.bits)
		// Pack bits 0–7 → byte 0, bits 8–15 → byte 1, bits 16–19 → upper nibble of byte 2
		b0 := bits[0]<<7 | bits[1]<<6 | bits[2]<<5 | bits[3]<<4 |
			bits[4]<<3 | bits[5]<<2 | bits[6]<<1 | bits[7]
		b1 := bits[8]<<7 | bits[9]<<6 | bits[10]<<5 | bits[11]<<4 |
			bits[12]<<3 | bits[13]<<2 | bits[14]<<1 | bits[15]
		b2 := bits[16]<<7 | bits[17]<<6 | bits[18]<<5 | bits[19]<<4
		return []byte{b0, b1, b2}
	}

	// Pad to byte boundary with zero bits
	rem := w.bitLen() % 8
	if rem != 0 {
		w.write(0, uint32(8-rem))
	}

	// Fill remaining codewords with alternating 0xEC / 0x11
	// (These bytes were chosen because their patterns avoid common degenerate sequences.)
	bytes := w.toBytes()
	pad := byte(0xEC)
	for len(bytes) < cfg.dataCW {
		bytes = append(bytes, pad)
		if pad == 0xEC {
			pad = 0x11
		} else {
			pad = 0xEC
		}
	}
	return bytes
}

// ============================================================================
// Symbol selection
// ============================================================================

// selectConfig finds the smallest (version, ECC) combination that can hold the input.
//
// If version and/or ecc are non-nil, only configurations matching those constraints
// are considered. This enables force-selecting a specific symbol size.
//
// The iteration order of symbolConfigs (M1→M4, L before M before Q) ensures
// we always pick the smallest symbol with the least redundancy that can still
// hold the input.
func selectConfig(input string, version *MicroQRVersion, ecc *MicroQREccLevel) (*symbolConfig, error) {
	// Build the filtered candidate list
	candidates := make([]*symbolConfig, 0, len(symbolConfigs))
	for i := range symbolConfigs {
		cfg := &symbolConfigs[i]
		if version != nil && cfg.version != *version {
			continue
		}
		if ecc != nil && cfg.ecc != *ecc {
			continue
		}
		candidates = append(candidates, cfg)
	}

	if len(candidates) == 0 {
		return nil, errECCNotAvailable(fmt.Sprintf(
			"no symbol configuration matches version=%v ecc=%v",
			version, ecc,
		))
	}

	for _, cfg := range candidates {
		mode, err := selectMode(input, cfg)
		if err != nil {
			continue
		}
		var inputLen int
		if mode == modeByte {
			inputLen = len([]byte(input))
		} else {
			inputLen = len([]rune(input))
		}
		var cap int
		switch mode {
		case modeNumeric:
			cap = cfg.numericCap
		case modeAlphanumeric:
			cap = cfg.alphaCap
		case modeByte:
			cap = cfg.byteCap
		}
		if cap > 0 && inputLen <= cap {
			return cfg, nil
		}
	}

	return nil, errInputTooLong(fmt.Sprintf(
		"input (length %d) does not fit in any Micro QR symbol (version=%v, ecc=%v). "+
			"Maximum is 35 numeric chars in M4-L.",
		len(input), version, ecc,
	))
}

// ============================================================================
// Working grid — mutable grid during encoding
// ============================================================================

// workGrid is the mutable grid used during encoding.
//
// We use a flat mutable approach during construction because the encoder needs
// to: (1) place structural modules, (2) place data modules, (3) try all 4 masks
// and score each. Using a mutable grid avoids the overhead of immutable copies
// for each mask trial.
//
// The reserved flag marks modules that belong to the finder pattern, separator,
// timing strips, or format information area. These must NOT be flipped by masking.
type workGrid struct {
	size     int
	modules  [][]bool
	reserved [][]bool
}

// newWorkGrid creates a fresh size×size grid with all modules light (false).
func newWorkGrid(size int) *workGrid {
	mods := make([][]bool, size)
	res := make([][]bool, size)
	for r := 0; r < size; r++ {
		mods[r] = make([]bool, size)
		res[r] = make([]bool, size)
	}
	return &workGrid{size: size, modules: mods, reserved: res}
}

// set assigns a module value and optionally marks it as reserved.
func (g *workGrid) set(row, col int, dark, reserve bool) {
	g.modules[row][col] = dark
	if reserve {
		g.reserved[row][col] = true
	}
}

// ============================================================================
// Structural module placement
// ============================================================================

// placeFinder places the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
//
// The finder pattern is the same "bull's-eye" used in regular QR Code:
//
//	■ ■ ■ ■ ■ ■ ■
//	■ □ □ □ □ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ □ □ □ □ ■
//	■ ■ ■ ■ ■ ■ ■
//
// Dark modules: outer border (r=0, r=6, c=0, c=6) and 3×3 core (rows 2-4, cols 2-4).
// Light modules: the ring between border and core.
//
// The 1:1:3:1:1 dark/light/dark ratio is what scanners detect as a finder pattern.
// Because Micro QR has only one finder pattern, a scanner immediately knows
// which corner is top-left — the data area is always to the bottom-right.
func placeFinder(g *workGrid) {
	for dr := 0; dr < 7; dr++ {
		for dc := 0; dc < 7; dc++ {
			onBorder := dr == 0 || dr == 6 || dc == 0 || dc == 6
			inCore := dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
			g.set(dr, dc, onBorder || inCore, true)
		}
	}
}

// placeSeparator places the L-shaped separator between the finder and data area.
//
// Regular QR Code surrounds all three finder patterns with separators on all four
// sides. Micro QR has only one finder pattern, in the top-left corner. Its top
// and left edges are the symbol boundary itself — no separator needed there.
// Only the bottom (row 7) and right (col 7) edges need separators.
//
// The separator is always light (value 0), forming a white border that visually
// and logically separates the finder from the data area.
//
//	Row 7, cols 0–7: light  (bottom of finder)
//	Col 7, rows 0–7: light  (right of finder)
func placeSeparator(g *workGrid) {
	for i := 0; i <= 7; i++ {
		g.set(7, i, false, true) // row 7 (bottom of finder)
		g.set(i, 7, false, true) // col 7 (right of finder)
	}
}

// placeTiming places the timing pattern extensions beyond the finder area.
//
// Micro QR's timing patterns run along row 0 and col 0 (NOT row 6 / col 6 as in
// regular QR). The timing pattern tells scanners how many modules the symbol has
// and where the module boundaries are — it's like a ruler printed right on the
// barcode.
//
// The finder pattern already occupies positions 0–6 on row 0 and col 0 (and
// the separator occupies position 7). The timing *extension* starts at position 8
// and continues to the opposite edge:
//
//	Row 0, cols 8 to size-1: dark if col is even, light if odd
//	Col 0, rows 8 to size-1: dark if row is even, light if odd
func placeTiming(g *workGrid) {
	for c := 8; c < g.size; c++ {
		g.set(0, c, c%2 == 0, true)
	}
	for r := 8; r < g.size; r++ {
		g.set(r, 0, r%2 == 0, true)
	}
}

// reserveFormatInfo marks the 15 format information positions as reserved.
//
// Unlike regular QR Code which places format information in two copies,
// Micro QR has only ONE copy. The 15 modules form an L-shape:
//
//	Row 8, cols 1–8  →  8 modules (holds bits f14..f7, MSB first)
//	Col 8, rows 7–1  →  7 modules (holds bits f6..f0, f6 at row 7)
//
// These modules are reserved as light (0) initially. After the best mask is
// chosen, writeFormatInfo() fills them with the actual format word.
func reserveFormatInfo(g *workGrid) {
	for c := 1; c <= 8; c++ {
		g.set(8, c, false, true)
	}
	for r := 1; r <= 7; r++ {
		g.set(r, 8, false, true)
	}
}

// writeFormatInfo writes the 15-bit format word into the reserved positions.
//
// Bit placement (f14 = MSB, f0 = LSB):
//
//	Row 8, col 1  ← f14 (MSB)
//	Row 8, col 2  ← f13
//	...
//	Row 8, col 8  ← f7
//	Col 8, row 7  ← f6
//	Col 8, row 6  ← f5
//	...
//	Col 8, row 1  ← f0 (LSB)
//
// Note the "upward" direction on the column: f6 is at row 7 (nearer the
// separator) and f0 is at row 1 (nearer the finder corner).
func writeFormatInfo(modules [][]bool, fmt uint16) {
	// Row 8, cols 1–8: bits f14 down to f7
	for i := uint16(0); i < 8; i++ {
		modules[8][1+i] = ((fmt >> (14 - i)) & 1) == 1
	}
	// Col 8, rows 7 down to 1: bits f6 down to f0
	for i := uint16(0); i < 7; i++ {
		modules[7-i][8] = ((fmt >> (6 - i)) & 1) == 1
	}
}

// buildGrid creates and populates the initial grid with all structural modules.
func buildGrid(cfg *symbolConfig) *workGrid {
	g := newWorkGrid(cfg.size)
	placeFinder(g)
	placeSeparator(g)
	placeTiming(g)
	reserveFormatInfo(g)
	return g
}

// ============================================================================
// Data placement — two-column zigzag
// ============================================================================

// placeBits places the final codeword bit stream into the grid using the
// standard Micro QR two-column zigzag scan.
//
// The zigzag works like a snake weaving through the grid:
//   - Start at the bottom-right corner.
//   - Scan upward through a two-column strip (columns col and col-1).
//   - At the top, move left two columns and scan downward.
//   - Repeat until all data bits are placed.
//
// Reserved modules (finder, separator, timing, format info) are skipped
// automatically — the bit index only advances when a non-reserved module is found.
//
// Unlike regular QR Code, there is no timing column at col 6 to hop around.
// Micro QR's timing is at col 0, which is reserved and auto-skipped.
//
// After all data+ECC bits are placed, any remaining unreserved modules
// receive 0 (remainder bits). M1 has 4 remainder bits; all others have 0.
func placeBits(g *workGrid, bits []bool) {
	bitIdx := 0
	up := true
	sz := g.size

	for col := sz - 1; col >= 1; col -= 2 {
		for vi := 0; vi < sz; vi++ {
			row := sz - 1 - vi
			if !up {
				row = vi
			}
			// Try both columns in this two-column strip (right then left)
			for dc := 0; dc <= 1; dc++ {
				c := col - dc
				if g.reserved[row][c] {
					continue
				}
				if bitIdx < len(bits) {
					g.modules[row][c] = bits[bitIdx]
					bitIdx++
				} else {
					g.modules[row][c] = false // remainder bit
				}
			}
		}
		up = !up
	}
}

// ============================================================================
// Masking
// ============================================================================

// maskCondition returns true if mask pattern maskIdx should flip module (row, col).
//
// Micro QR uses only 4 mask patterns (the first 4 of regular QR's 8):
//
//	| Pattern | Condition |
//	|---------|-----------|
//	| 0       | (row + col) mod 2 == 0 |
//	| 1       | row mod 2 == 0 |
//	| 2       | col mod 3 == 0 |
//	| 3       | (row + col) mod 3 == 0 |
//
// These are simpler than the higher-numbered QR patterns (which involve products)
// because the small symbol size means simpler patterns are sufficient to break
// up problematic degenerate sequences.
func maskCondition(maskIdx, row, col int) bool {
	switch maskIdx {
	case 0:
		return (row+col)%2 == 0
	case 1:
		return row%2 == 0
	case 2:
		return col%3 == 0
	case 3:
		return (row+col)%3 == 0
	}
	return false
}

// applyMask applies mask pattern maskIdx to all non-reserved modules and
// returns the resulting module grid as a new 2D slice.
//
// Masking flips module values at positions where the mask condition is true.
// This breaks up problematic visual patterns (long runs of same-color modules,
// 2×2 blocks, finder-like sequences) that could confuse scanners.
//
// Only data and ECC modules are flipped — never structural modules (finder,
// separator, timing, format info). The reserved grid tracks which positions
// are structural.
func applyMask(modules [][]bool, reserved [][]bool, sz, maskIdx int) [][]bool {
	result := make([][]bool, sz)
	for r := 0; r < sz; r++ {
		result[r] = make([]bool, sz)
		for c := 0; c < sz; c++ {
			if !reserved[r][c] {
				result[r][c] = modules[r][c] != maskCondition(maskIdx, r, c)
			} else {
				result[r][c] = modules[r][c]
			}
		}
	}
	return result
}

// ============================================================================
// Penalty scoring — 4 rules (same as regular QR Code)
// ============================================================================

// computePenalty calculates the penalty score for a given masked module grid.
//
// Four penalty rules are evaluated. The mask pattern with the LOWEST total
// penalty is selected. This minimizes visual artifacts that would confuse scanners.
//
// # Rule 1 — Adjacent run penalty
//
// For every row and every column, find runs of ≥5 consecutive same-color modules.
// Add (run_length − 2) to the penalty for each qualifying run.
//
//	run of 5 → +3, run of 6 → +4, run of 7 → +5, ...
//
// Long runs of the same color can look like a finder pattern or timing strip to
// a scanner.
//
// # Rule 2 — 2×2 block penalty
//
// For each 2×2 square with all four modules the same color, add 3.
// Such blocks create "smear" that degrades corner detection and module isolation.
//
// # Rule 3 — Finder-pattern-like sequences
//
// Scan each row and column for the 11-module sequences:
//
//	1 0 1 1 1 0 1 0 0 0 0  (looks like a finder pattern with leading quiet zone)
//	0 0 0 0 1 0 1 1 1 0 1  (reverse of the above)
//
// Each occurrence adds 40. These patterns are so similar to the actual finder
// pattern that a scanner might mistake them for a second or third finder.
//
// # Rule 4 — Dark-module proportion
//
// Deviation from 50% dark modules is penalized. Compute dark_pct = (dark_count
// × 100) / total, find the nearest multiples of 5, and add (min_distance / 5) × 10.
// A 50% balanced symbol is ideal; extreme imbalance degrades readability.
func computePenalty(modules [][]bool, sz int) uint32 {
	var penalty uint32

	// Rule 1 — runs of ≥5 same-color modules in rows and columns
	for a := 0; a < sz; a++ {
		for _, horiz := range []bool{true, false} {
			run := uint32(1)
			var prev bool
			if horiz {
				prev = modules[a][0]
			} else {
				prev = modules[0][a]
			}
			for i := 1; i < sz; i++ {
				var cur bool
				if horiz {
					cur = modules[a][i]
				} else {
					cur = modules[i][a]
				}
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
	}

	// Rule 2 — 2×2 same-color blocks
	for r := 0; r < sz-1; r++ {
		for c := 0; c < sz-1; c++ {
			d := modules[r][c]
			if d == modules[r][c+1] && d == modules[r+1][c] && d == modules[r+1][c+1] {
				penalty += 3
			}
		}
	}

	// Rule 3 — finder-pattern-like 11-module sequences
	p1 := [11]bool{true, false, true, true, true, false, true, false, false, false, false}
	p2 := [11]bool{false, false, false, false, true, false, true, true, true, false, true}
	for a := 0; a < sz; a++ {
		limit := sz - 11
		for b := 0; b <= limit; b++ {
			mh1, mh2, mv1, mv2 := true, true, true, true
			for k := 0; k < 11; k++ {
				bh := modules[a][b+k]
				bv := modules[b+k][a]
				if bh != p1[k] {
					mh1 = false
				}
				if bh != p2[k] {
					mh2 = false
				}
				if bv != p1[k] {
					mv1 = false
				}
				if bv != p2[k] {
					mv2 = false
				}
			}
			if mh1 {
				penalty += 40
			}
			if mh2 {
				penalty += 40
			}
			if mv1 {
				penalty += 40
			}
			if mv2 {
				penalty += 40
			}
		}
	}

	// Rule 4 — dark-module proportion penalty
	darkCount := 0
	for r := 0; r < sz; r++ {
		for c := 0; c < sz; c++ {
			if modules[r][c] {
				darkCount++
			}
		}
	}
	total := sz * sz
	darkPct := (darkCount * 100) / total
	prev5 := (darkPct / 5) * 5
	next5 := prev5 + 5
	d1 := prev5 - 50
	if d1 < 0 {
		d1 = -d1
	}
	d2 := next5 - 50
	if d2 < 0 {
		d2 = -d2
	}
	minDist := d1
	if d2 < minDist {
		minDist = d2
	}
	penalty += uint32((minDist / 5) * 10)

	return penalty
}

// ============================================================================
// Public API
// ============================================================================

// Encode encodes a string to a Micro QR Code ModuleGrid.
//
// Automatically selects the smallest symbol (M1..M4) and ECC level that can
// hold the input. Pass non-nil version and/or ecc to override auto-selection.
//
// If both version and ecc are nil, the encoder defaults to auto-selection with
// preference for lowest ECC level (Detection for M1, L for M2-M4).
//
// # Errors
//
//   - InputTooLong if the input exceeds M4-L capacity (35 numeric chars).
//   - ECCNotAvailable if the requested version+ECC combination does not exist.
//   - UnsupportedMode if no encoding mode can represent the input in the
//     selected symbol version.
//
// # Examples
//
//	// Auto-select: "HELLO" fits in M2 alphanumeric
//	grid, err := microqr.Encode("HELLO", nil, nil)
//	// grid.Rows == 13, grid.Cols == 13
//
//	// Force version M4, auto-select ECC
//	v := microqr.VersionM4
//	grid, err := microqr.Encode("hello world", &v, nil)
func Encode(input string, version *MicroQRVersion, ecc *MicroQREccLevel) (barcode2d.ModuleGrid, error) {
	cfg, err := selectConfig(input, version, ecc)
	if err != nil {
		return barcode2d.ModuleGrid{}, err
	}
	mode, err := selectMode(input, cfg)
	if err != nil {
		return barcode2d.ModuleGrid{}, err
	}

	// Step 1: Build data codewords
	dataCW := buildDataCodewords(input, cfg, mode)

	// Step 2: Compute Reed-Solomon ECC codewords
	gen, ok := rsGenerators[cfg.eccCW]
	if !ok {
		return barcode2d.ModuleGrid{}, fmt.Errorf("micro-qr: no RS generator for eccCW=%d", cfg.eccCW)
	}
	eccCW := rsEncode(dataCW, gen)

	// Step 3: Flatten codewords to a bit stream.
	// For M1: data[2] (the half-codeword) contributes only its upper 4 bits.
	finalCW := append(dataCW, eccCW...)
	bits := make([]bool, 0, len(finalCW)*8)
	for cwIdx, cw := range finalCW {
		bitsInCW := 8
		if cfg.m1HalfCW && cwIdx == cfg.dataCW-1 {
			bitsInCW = 4
		}
		for b := bitsInCW - 1; b >= 0; b-- {
			bits = append(bits, ((cw>>(uint(b)+(uint(8-bitsInCW))))&1) == 1)
		}
	}

	// Step 4: Build initial grid with structural modules
	grid := buildGrid(cfg)

	// Step 5: Place data/ECC bits via zigzag
	placeBits(grid, bits)

	// Step 6: Evaluate all 4 mask patterns and pick the one with lowest penalty
	bestMask := 0
	bestPenalty := uint32(0xFFFFFFFF)
	for m := 0; m < 4; m++ {
		masked := applyMask(grid.modules, grid.reserved, cfg.size, m)
		fmt := formatTable[cfg.symbolIndicator][m]
		// Write format info into a temporary copy to include in penalty scoring
		tmpModules := make([][]bool, cfg.size)
		for r := 0; r < cfg.size; r++ {
			tmpModules[r] = make([]bool, cfg.size)
			copy(tmpModules[r], masked[r])
		}
		writeFormatInfo(tmpModules, fmt)
		p := computePenalty(tmpModules, cfg.size)
		if p < bestPenalty {
			bestPenalty = p
			bestMask = m
		}
	}

	// Step 7: Apply best mask and write final format information
	finalModules := applyMask(grid.modules, grid.reserved, cfg.size, bestMask)
	finalFmt := formatTable[cfg.symbolIndicator][bestMask]
	writeFormatInfo(finalModules, finalFmt)

	// Step 8: Build the immutable ModuleGrid
	return barcode2d.ModuleGrid{
		Rows:        uint32(cfg.size),
		Cols:        uint32(cfg.size),
		Modules:     finalModules,
		ModuleShape: barcode2d.ModuleShapeSquare,
	}, nil
}

// EncodeAt encodes to a specific symbol version and ECC level.
//
// This is a convenience wrapper around Encode that takes concrete values
// rather than pointers. Use this when you know exactly which symbol size
// and error correction level you need.
//
// Returns InputTooLong if the input does not fit in the requested
// version+ECC combination, or ECCNotAvailable if that combination is invalid.
func EncodeAt(input string, version MicroQRVersion, ecc MicroQREccLevel) (barcode2d.ModuleGrid, error) {
	return Encode(input, &version, &ecc)
}

// Layout converts a ModuleGrid to a PaintScene via barcode-2d's Layout function.
//
// Defaults to a 2-module quiet zone (the Micro QR minimum — half of regular
// QR's 4-module requirement) and 10px per module.
//
// Pass a non-nil config to override the defaults. The ModuleSizePx, Foreground,
// Background, and QuietZoneModules fields are all adjustable.
func Layout(grid barcode2d.ModuleGrid, config *barcode2d.Barcode2DLayoutConfig) (interface{}, error) {
	cfg := barcode2d.DefaultBarcode2DLayoutConfig
	cfg.QuietZoneModules = 2 // Micro QR uses 2 (not 4) module quiet zone
	if config != nil {
		cfg = *config
	}
	return barcode2d.Layout(grid, &cfg)
}
