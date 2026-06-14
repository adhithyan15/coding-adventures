// Package pdf417 implements a PDF417 stacked linear barcode encoder
// conforming to ISO/IEC 15438:2015.
//
// # What is PDF417?
//
// PDF417 (Portable Data File 417) is a stacked linear barcode invented by
// Ynjiun P. Wang at Symbol Technologies in 1991. The "417" in the name is a
// piece of literate engineering trivia: every codeword has exactly 4 bars
// and 4 spaces (8 elements total), occupying exactly 17 horizontal modules.
//
// Unlike a true 2D matrix barcode (QR, Data Matrix), PDF417 is a stack of
// short 1D barcode rows. Each row is independently scannable by a moving
// laser, which is why PDF417 is the format of choice for driver's licences,
// boarding passes, and shipping labels — anywhere a one-shot laser scanner
// has to read a lot of data quickly.
//
// # Where PDF417 is deployed
//
//   - AAMVA: North American driver's licences and government IDs
//   - IATA BCBP: airline boarding passes
//   - USPS: domestic shipping labels
//   - US immigration: Form I-94, customs declarations
//   - Healthcare: patient wristbands, medication labels
//
// # Encoding pipeline
//
//	raw bytes
//	  → byte compaction      (codeword 924 latch + 6-bytes-to-5-codewords base-900)
//	  → length descriptor    (first codeword = total codewords in symbol)
//	  → RS ECC               (GF(929) Reed-Solomon, b=3 convention, α=3)
//	  → dimension selection  (auto: roughly square symbol)
//	  → padding              (codeword 900 fills unused slots)
//	  → row indicators       (LRI + RRI per row, encode R/C/ECC level)
//	  → cluster table lookup (codeword → 17-module bar/space pattern)
//	  → start/stop patterns  (fixed per row)
//	  → ModuleGrid           (abstract boolean grid)
//
// # v0.1.0 scope
//
// This release implements byte compaction only. All inputs are treated as
// raw bytes. Text and numeric compaction modes (which can pack ASCII letters
// or digit runs more densely) are planned for v0.2.0.
//
// # Public API
//
//	Encode(data string) *barcode2d.ModuleGrid
//	EncodeBytes(data []byte, opts Options) (*barcode2d.ModuleGrid, error)
//	EncodeToScene(data []byte, opts Options, cfg *barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error)
package pdf417

import (
	"errors"
	"fmt"
	"math"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

// Version is the semantic version of this package.
const Version = "0.1.0"

// ============================================================================
// Public error types
// ============================================================================

// ErrInputTooLong is returned when the encoded data exceeds the maximum
// PDF417 symbol capacity (90 rows × 30 data columns = 2700 slots minus
// ECC overhead).
var ErrInputTooLong = errors.New("pdf417: input too long for any valid PDF417 symbol")

// ErrInvalidDimensions is returned when user-supplied rows or columns are
// outside the 3–90 rows / 1–30 columns limits.
var ErrInvalidDimensions = errors.New("pdf417: invalid dimensions")

// ErrInvalidECCLevel is returned when the ECC level is outside the valid
// range 0–8.
var ErrInvalidECCLevel = errors.New("pdf417: invalid ECC level (must be 0–8)")

// ============================================================================
// Options
// ============================================================================

// Options configures the PDF417 encoder.
//
// All fields are optional. The zero-value Options{} triggers full auto-
// selection: ECC level chosen from data length, dimensions chosen for a
// roughly square symbol, and a row height of 3.
type Options struct {
	// ECCLevel sets the Reed-Solomon error correction level (0–8).
	// Higher levels use more ECC codewords, making the symbol larger but
	// more resilient to damage. Use ECCLevelAuto for auto-selection.
	ECCLevel int

	// Columns sets the number of data columns (1–30).
	// Use 0 for auto-selection (roughly square symbol).
	Columns int

	// RowHeight sets the number of module-rows per logical PDF417 row (1–10).
	// Larger values produce taller, more easily scanned symbols. Default: 3.
	// Use 0 for the default.
	RowHeight int
}

// ECCLevelAuto signals that the ECC level should be auto-selected based on
// data length. Stored as -1 so the zero value Options{} (ECCLevel = 0) still
// means "use ECC level 0", consistent with the ISO/IEC 15438 enumeration.
//
// Pass it explicitly when you want the encoder to choose:
//
//	opts := pdf417.Options{ECCLevel: pdf417.ECCLevelAuto}
const ECCLevelAuto = -1

// ============================================================================
// Constants
// ============================================================================

// PDF417 lives in GF(929): the integers modulo the prime 929.
//
// Why 929? Because the codeword alphabet contains 929 distinct values
// (0..928): 900 for data, plus 29 for control codewords like latches,
// shifts, padding, and macro headers. A prime modulus guarantees every
// non-zero element has a multiplicative inverse — a requirement for
// Reed-Solomon error correction to work.
const (
	// gf929Prime is the field modulus.
	gf929Prime = 929

	// latchByteCW is the codeword that latches the decoder into byte
	// compaction mode (alternate form, accepts any number of bytes).
	latchByteCW uint16 = 924

	// paddingCW is the neutral padding codeword (latch-to-text). Decoders
	// recognise it as a no-op tail filler.
	paddingCW uint16 = 900

	// Symbol-size limits per ISO/IEC 15438:2015.
	minRows = 3
	maxRows = 90
	minCols = 1
	maxCols = 30
)

// ============================================================================
// GF(929) arithmetic — INLINE
// ============================================================================
//
// GF(929) = the integers mod 929 with the usual + and ×.
//
// Because 929 fits comfortably in 16 bits and 929 × 929 fits in 32 bits, we
// can implement field arithmetic with plain integer ops — no tables, no
// polynomial reduction. This is much simpler than GF(2^8) used by most other
// barcode formats (QR, Data Matrix, Aztec) where the field has characteristic 2
// and multiplication is bit-shifted polynomial multiplication mod a primitive
// polynomial.
//
//   gfAdd(a, b) = (a + b) mod 929
//   gfMul(a, b) = (a × b) mod 929
//
// We keep one log/antilog pair anyway, but only because the Reed-Solomon
// generator polynomial wants α^j for j up to ~514 — building a 929-entry
// exponent table once at init time is faster than calling pow(3, j, 929)
// inside the inner loop.

// gfExp[i] = α^i mod 929, for i in 0..928. gfExp[928] = gfExp[0] = 1.
var gfExp [929]uint16

// gfLog[v] = i such that α^i = v, for v in 1..928. gfLog[0] is unused.
var gfLog [929]uint16

func init() {
	// Build α^i and its inverse log table.
	//
	// α = 3 is a primitive root mod 929 — its powers 3^0, 3^1, …, 3^927
	// cover every non-zero element of GF(929) exactly once. (One of the
	// classic facts of number theory: primes have primitive roots.)
	//
	// We tabulate to make multiplication a table lookup, but addition stays
	// inline because (a + b) mod 929 is already two cheap CPU ops.
	val := uint32(1)
	for i := 0; i < 928; i++ {
		gfExp[i] = uint16(val)
		gfLog[val] = uint16(i)
		val = (val * 3) % gf929Prime
	}
	// gfExp[928] = gfExp[0] = 1 — convenience for wrap-around in gfMul.
	gfExp[928] = gfExp[0]
}

// gfAdd returns (a + b) mod 929. Inlined for clarity; the Go compiler will
// optimise the call away.
func gfAdd(a, b uint16) uint16 {
	return uint16((uint32(a) + uint32(b)) % gf929Prime)
}

// gfMul returns (a × b) mod 929 using log/antilog tables.
//
// For non-zero a, b:  a × b = α^{(log a + log b) mod 928}
// Either operand zero → product zero (zero absorbs multiplication).
func gfMul(a, b uint16) uint16 {
	if a == 0 || b == 0 {
		return 0
	}
	return gfExp[(uint32(gfLog[a])+uint32(gfLog[b]))%928]
}

// ============================================================================
// Reed-Solomon generator polynomial
// ============================================================================
//
// For ECC level L, we need k = 2^(L+1) ECC codewords. The generator
// polynomial uses the b=3 convention from ISO/IEC 15438:
//
//   g(x) = (x − α^3)(x − α^4) ··· (x − α^{k+2})
//
// Why b=3? The standard chose roots starting at α^3 rather than α^0 to gain
// some implementation flexibility. The choice is arbitrary as long as the
// encoder and decoder agree.
//
// We build g iteratively by multiplying in each linear factor (x − α^j),
// starting from g_0(x) = 1 and ending at g_k(x) of degree k.

// buildGenerator returns the k+1 coefficients of the RS generator polynomial
// for ECC level eccLevel, ordered [g_k, g_{k-1}, …, g_1, g_0]. g_k = 1.
func buildGenerator(eccLevel int) []uint16 {
	k := 1 << uint(eccLevel+1) // 2^(eccLevel+1)
	g := []uint16{1}

	for j := 3; j <= k+2; j++ {
		root := gfExp[j%928]                     // α^j
		negRoot := uint16(gf929Prime) - root     // −α^j ≡ 929 − α^j (mod 929)
		newG := make([]uint16, len(g)+1)
		for i, coeff := range g {
			newG[i] = gfAdd(newG[i], coeff)
			newG[i+1] = gfAdd(newG[i+1], gfMul(coeff, negRoot))
		}
		g = newG
	}
	return g
}

// ============================================================================
// Reed-Solomon encoder
// ============================================================================

// rsEncode computes k = 2^(eccLevel+1) RS ECC codewords for data over
// GF(929) using the b=3 convention.
//
// Algorithm: classic shift-register (LFSR) polynomial long division. The
// register holds the partial remainder; each input symbol is added in,
// then the register is shifted and updated by the feedback term × g.
//
// PDF417 uses a single RS encoder for all data — there is no block
// interleaving as in QR Code. This makes the encoder simpler at the cost
// of slightly worse burst-error tolerance.
func rsEncode(data []uint16, eccLevel int) []uint16 {
	g := buildGenerator(eccLevel)
	k := len(g) - 1
	ecc := make([]uint16, k)

	for _, d := range data {
		feedback := gfAdd(d, ecc[0])
		// Shift the register one slot toward the leading end.
		for i := 0; i < k-1; i++ {
			ecc[i] = ecc[i+1]
		}
		ecc[k-1] = 0
		// Add feedback × generator coefficient into each cell.
		for i := 0; i < k; i++ {
			ecc[i] = gfAdd(ecc[i], gfMul(g[k-i], feedback))
		}
	}
	return ecc
}

// ============================================================================
// Byte compaction
// ============================================================================
//
// Byte compaction packs raw bytes by treating every group of 6 bytes as a
// 48-bit big-endian integer and re-expressing it in base 900 (5 base-900
// digits). The result fits in 5 codewords — 17% denser than encoding each
// byte separately.
//
//   6 bytes (48 bits) → integer N → 5 base-900 digits → 5 codewords
//
// Any leftover 1–5 bytes at the tail are emitted as one codeword each
// (byte value < 256 fits trivially in a 0–928 codeword slot).
//
// We prefix the codeword stream with the 924 latch so the decoder knows
// to switch into byte compaction mode.

// byteCompact encodes raw bytes using PDF417 byte compaction mode.
// Returns [924, c1, c2, …] where c_i are byte-compacted codewords.
func byteCompact(bytes []byte) []uint16 {
	codewords := make([]uint16, 0, 1+len(bytes))
	codewords = append(codewords, latchByteCW)

	i := 0
	n := len(bytes)

	// Process full 6-byte groups → 5 codewords each.
	//
	// The 48-bit accumulator fits in a uint64 with room to spare, so we
	// don't need big.Int (unlike the JS reference, which has 53-bit float
	// limits and needs BigInt).
	for i+6 <= n {
		var acc uint64
		for j := 0; j < 6; j++ {
			acc = acc*256 + uint64(bytes[i+j])
		}
		// Express acc in base 900, most-significant first.
		var group [5]uint16
		for j := 4; j >= 0; j-- {
			group[j] = uint16(acc % 900)
			acc /= 900
		}
		codewords = append(codewords, group[:]...)
		i += 6
	}

	// Tail: 1 codeword per remaining byte.
	for ; i < n; i++ {
		codewords = append(codewords, uint16(bytes[i]))
	}
	return codewords
}

// ============================================================================
// ECC level auto-selection
// ============================================================================

// autoECCLevel picks a recommended ECC level given the total data codeword
// count (including the length descriptor). Levels are tuned so that small
// symbols don't waste space on excessive ECC and large symbols stay readable
// even with significant damage.
func autoECCLevel(dataCount int) int {
	switch {
	case dataCount <= 40:
		return 2
	case dataCount <= 160:
		return 3
	case dataCount <= 320:
		return 4
	case dataCount <= 863:
		return 5
	default:
		return 6
	}
}

// ============================================================================
// Dimension selection
// ============================================================================

// chooseDimensions returns (cols, rows) for a roughly square symbol holding
// total codewords. The heuristic c = ceil(sqrt(total / 3)) is biased toward
// extra columns because each column contributes 17 modules of width but
// each row contributes only the row-height in modules of height — so a
// "square" pixel-aspect symbol needs roughly 3× more rows than columns.
func chooseDimensions(total int) (cols, rows int) {
	c := int(math.Ceil(math.Sqrt(float64(total) / 3.0)))
	if c < minCols {
		c = minCols
	}
	if c > maxCols {
		c = maxCols
	}
	r := (total + c - 1) / c
	if r < minRows {
		r = minRows
		c = (total + r - 1) / r
		if c < minCols {
			c = minCols
		}
		if c > maxCols {
			c = maxCols
		}
		r = (total + c - 1) / c
		if r < minRows {
			r = minRows
		}
	}
	if r > maxRows {
		r = maxRows
	}
	return c, r
}

// ============================================================================
// Row indicator computation
// ============================================================================
//
// Each row carries two row indicator codewords. Together LRI and RRI encode
// the symbol's overall shape so the decoder can recover R, C, and ECC level
// without needing to read every row in order.
//
//   R_info = (R-1) / 3        (information about total rows)
//   C_info =  C - 1           (information about total columns)
//   L_info = 3*L + (R-1) % 3  (ECC level + row count parity)
//
// Cluster (= row mod 3) determines which piece each side carries:
//
//   Cluster 0:  LRI = 30·rowGroup + R_info,  RRI = 30·rowGroup + C_info
//   Cluster 1:  LRI = 30·rowGroup + L_info,  RRI = 30·rowGroup + R_info
//   Cluster 2:  LRI = 30·rowGroup + C_info,  RRI = 30·rowGroup + L_info
//
// where rowGroup = floor(r / 3).
//
// The 30·rowGroup term gives the decoder a position fix — it can read just
// one row indicator and immediately know which row group the row belongs to.

// computeLRI returns the Left Row Indicator codeword for row r.
func computeLRI(r, rows, cols, eccLevel int) uint16 {
	rInfo := (rows - 1) / 3
	cInfo := cols - 1
	lInfo := 3*eccLevel + (rows-1)%3
	rowGroup := r / 3
	cluster := r % 3
	switch cluster {
	case 0:
		return uint16(30*rowGroup + rInfo)
	case 1:
		return uint16(30*rowGroup + lInfo)
	default:
		return uint16(30*rowGroup + cInfo)
	}
}

// computeRRI returns the Right Row Indicator codeword for row r.
func computeRRI(r, rows, cols, eccLevel int) uint16 {
	rInfo := (rows - 1) / 3
	cInfo := cols - 1
	lInfo := 3*eccLevel + (rows-1)%3
	rowGroup := r / 3
	cluster := r % 3
	switch cluster {
	case 0:
		return uint16(30*rowGroup + cInfo)
	case 1:
		return uint16(30*rowGroup + rInfo)
	default:
		return uint16(30*rowGroup + lInfo)
	}
}

// ============================================================================
// Codeword → modules expansion
// ============================================================================

// expandPattern unpacks a 32-bit packed pattern into 17 boolean module values
// and appends them to modules.
//
// Layout of the packed u32:
//
//	bits 31..28 = b1   (width of bar 1, in modules)
//	bits 27..24 = s1   (width of space 1)
//	bits 23..20 = b2
//	bits 19..16 = s2
//	bits 15..12 = b3
//	bits 11..8  = s3
//	bits 7..4   = b4
//	bits 3..0   = s4
//
// Bars are dark (true), spaces are light (false). The 8 widths always
// sum to 17 — that is the defining geometric invariant of a PDF417 codeword.
func expandPattern(packed uint32, modules *[]bool) {
	widths := [8]uint8{
		uint8((packed >> 28) & 0xf),
		uint8((packed >> 24) & 0xf),
		uint8((packed >> 20) & 0xf),
		uint8((packed >> 16) & 0xf),
		uint8((packed >> 12) & 0xf),
		uint8((packed >> 8) & 0xf),
		uint8((packed >> 4) & 0xf),
		uint8(packed & 0xf),
	}
	dark := true
	for _, w := range widths {
		for i := uint8(0); i < w; i++ {
			*modules = append(*modules, dark)
		}
		dark = !dark
	}
}

// expandWidths unpacks an explicit width list into module booleans.
// The first element is always a bar (dark = true), then alternates.
// Used for the start (8 widths) and stop (9 widths) patterns.
func expandWidths(widths []uint8, modules *[]bool) {
	dark := true
	for _, w := range widths {
		for i := uint8(0); i < w; i++ {
			*modules = append(*modules, dark)
		}
		dark = !dark
	}
}

// ============================================================================
// Encode — primary public API
// ============================================================================

// Encode is the simple-string entry point requested in the PDF417 spec.
//
// It treats data as raw UTF-8 bytes, encodes with all defaults (auto ECC,
// auto dimensions, row height 3), and returns the resulting ModuleGrid.
//
// Errors are unreachable from this signature — the only fault paths come
// from explicit Options or extreme input length, which would require the
// (caller-controlled) overload EncodeBytes. If a future change makes Encode
// fallible, prefer adding a sibling that returns (*ModuleGrid, error).
//
// Returns a pointer for symmetry with the rest of the Go barcode stack.
func Encode(data string) *barcode2d.ModuleGrid {
	grid, err := EncodeBytes([]byte(data), Options{ECCLevel: ECCLevelAuto})
	if err != nil {
		// The default-options path can only fail on inputs so large that
		// no PDF417 symbol could hold them. Surface as a panic to keep the
		// signature ergonomic — callers needing graceful errors should use
		// EncodeBytes directly.
		panic(fmt.Sprintf("pdf417.Encode: %v", err))
	}
	return grid
}

// EncodeBytes encodes data into a PDF417 ModuleGrid using the given options.
//
// Errors:
//   - ErrInvalidECCLevel — opts.ECCLevel is outside 0–8 (and not ECCLevelAuto)
//   - ErrInvalidDimensions — opts.Columns is outside 1–30 (and not 0)
//   - ErrInputTooLong — data does not fit in any valid PDF417 symbol
func EncodeBytes(data []byte, opts Options) (*barcode2d.ModuleGrid, error) {
	// ── Early input-size guard ────────────────────────────────────────────
	// The largest PDF417 symbol is 90 rows × 30 cols = 2700 codeword slots.
	// At ECC level 0 (2 ECC codewords), byte compaction can encode at most
	// ~3235 raw bytes. Cap at 3600 to reject oversized inputs before running
	// O(n·k) Reed-Solomon encoding (where k ≤ 512), preventing DoS.
	const maxInputBytes = 3600
	if len(data) > maxInputBytes {
		return nil, fmt.Errorf("%w: input length %d exceeds maximum encodable size (%d bytes)",
			ErrInputTooLong, len(data), maxInputBytes)
	}

	// ── Validate ECC level ────────────────────────────────────────────────
	eccLevel := opts.ECCLevel
	if eccLevel != ECCLevelAuto && (eccLevel < 0 || eccLevel > 8) {
		return nil, fmt.Errorf("%w: got %d", ErrInvalidECCLevel, eccLevel)
	}

	// ── Byte compaction ───────────────────────────────────────────────────
	dataCwords := byteCompact(data)

	// ── Auto-select ECC level if requested ────────────────────────────────
	if eccLevel == ECCLevelAuto {
		eccLevel = autoECCLevel(len(dataCwords) + 1)
	}
	eccCount := 1 << uint(eccLevel+1) // 2^(eccLevel+1)

	// ── Length descriptor ─────────────────────────────────────────────────
	// First codeword counts itself + all data codewords + all ECC codewords
	// (but NOT padding). This lets the decoder skip past padding tail.
	// Compute as int first to guard against uint16 truncation, then cast.
	rawDesc := 1 + len(dataCwords) + eccCount
	if rawDesc > 65535 {
		return nil, fmt.Errorf("%w: codeword count %d overflows uint16", ErrInputTooLong, rawDesc)
	}
	lengthDesc := uint16(rawDesc)

	// fullData = [lengthDesc, ...dataCwords] — fed to RS as the message.
	fullData := make([]uint16, 0, 1+len(dataCwords))
	fullData = append(fullData, lengthDesc)
	fullData = append(fullData, dataCwords...)

	// ── RS ECC ────────────────────────────────────────────────────────────
	eccCwords := rsEncode(fullData, eccLevel)

	// ── Choose dimensions ─────────────────────────────────────────────────
	totalCwords := len(fullData) + len(eccCwords)
	var cols, rows int

	if opts.Columns != 0 {
		if opts.Columns < minCols || opts.Columns > maxCols {
			return nil, fmt.Errorf("%w: columns must be 1–30, got %d", ErrInvalidDimensions, opts.Columns)
		}
		cols = opts.Columns
		rows = (totalCwords + cols - 1) / cols
		if rows < minRows {
			rows = minRows
		}
		if rows > maxRows {
			return nil, fmt.Errorf("%w: needs %d rows (max %d) at %d columns", ErrInputTooLong, rows, maxRows, cols)
		}
	} else {
		cols, rows = chooseDimensions(totalCwords)
	}

	// Verify capacity (defensive — auto-selection should always succeed).
	if cols*rows < totalCwords {
		return nil, fmt.Errorf("%w: cannot fit %d codewords in %d×%d grid", ErrInputTooLong, totalCwords, rows, cols)
	}

	// ── Pad to fill the data area exactly ─────────────────────────────────
	paddingCount := cols*rows - totalCwords
	paddedData := make([]uint16, 0, cols*rows)
	paddedData = append(paddedData, fullData...)
	for i := 0; i < paddingCount; i++ {
		paddedData = append(paddedData, paddingCW)
	}

	// fullSequence = [data + padding, ecc] — what gets rasterised, in order.
	fullSequence := make([]uint16, 0, cols*rows+len(eccCwords))
	fullSequence = append(fullSequence, paddedData...)
	fullSequence = append(fullSequence, eccCwords...)

	// ── Rasterize ─────────────────────────────────────────────────────────
	rowHeight := opts.RowHeight
	if rowHeight <= 0 {
		rowHeight = 3
	}
	grid := rasterize(fullSequence, rows, cols, eccLevel, rowHeight)
	return &grid, nil
}

// ============================================================================
// Rasterization — codewords → ModuleGrid
// ============================================================================

// rasterize converts the flat codeword sequence into a ModuleGrid.
//
// Each PDF417 row has the layout:
//
//	[start 17] [LRI 17] [data×cols 17 each] [RRI 17] [stop 18]
//
// Total module width per row: 17 (start) + 17 (LRI) + 17·cols (data) + 17 (RRI) + 18 (stop)
//                           = 69 + 17·cols
//
// Each logical row is repeated rowHeight times vertically to give scanners
// a tall enough strip to integrate over (PDF417 was designed for one-shot
// laser scanning where vertical redundancy compensates for laser jitter).
func rasterize(sequence []uint16, rows, cols, eccLevel, rowHeight int) barcode2d.ModuleGrid {
	moduleWidth := 69 + 17*cols
	moduleHeight := rows * rowHeight

	grid := barcode2d.MakeModuleGrid(uint32(moduleHeight), uint32(moduleWidth), barcode2d.ModuleShapeSquare)

	// Precompute start and stop module sequences (identical for every row).
	startModules := make([]bool, 0, 17)
	expandWidths(startPattern[:], &startModules)
	stopModules := make([]bool, 0, 18)
	expandWidths(stopPattern[:], &stopModules)

	// rowModules is reused to avoid per-row allocation churn.
	rowModules := make([]bool, 0, moduleWidth)

	for r := 0; r < rows; r++ {
		cluster := r % 3
		clusterTable := &clusterTables[cluster]

		rowModules = rowModules[:0]

		// 1. Start pattern (17 modules, identical every row).
		rowModules = append(rowModules, startModules...)

		// 2. Left Row Indicator (17 modules).
		lri := computeLRI(r, rows, cols, eccLevel)
		expandPattern(clusterTable[lri], &rowModules)

		// 3. Data codewords (17 modules each).
		for j := 0; j < cols; j++ {
			cw := sequence[r*cols+j]
			expandPattern(clusterTable[cw], &rowModules)
		}

		// 4. Right Row Indicator (17 modules).
		rri := computeRRI(r, rows, cols, eccLevel)
		expandPattern(clusterTable[rri], &rowModules)

		// 5. Stop pattern (18 modules, identical every row).
		rowModules = append(rowModules, stopModules...)

		// Sanity check — should never trigger if cluster tables are correct.
		if len(rowModules) != moduleWidth {
			panic(fmt.Sprintf("pdf417: row %d produced %d modules, expected %d", r, len(rowModules), moduleWidth))
		}

		// Repeat this 1D row pattern rowHeight times into the 2D grid.
		// Direct mutation of grid.Modules is safe because MakeModuleGrid
		// allocates fresh slices we own.
		moduleRowBase := r * rowHeight
		for h := 0; h < rowHeight; h++ {
			rowSlice := grid.Modules[moduleRowBase+h]
			for col, dark := range rowModules {
				if dark {
					rowSlice[col] = true
				}
			}
		}
	}
	return grid
}

// ============================================================================
// EncodeToScene — encode + layout in one step
// ============================================================================

// EncodeToScene encodes data and runs the result through Layout() to produce
// a pixel-resolved PaintScene ready for a render backend.
//
// If cfg is nil, defaults are used with quietZoneModules = 2 (PDF417's
// recommended minimum from ISO/IEC 15438:2015 §5.8).
func EncodeToScene(data []byte, opts Options, cfg *barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error) {
	grid, err := EncodeBytes(data, opts)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	if cfg == nil {
		c := barcode2d.DefaultBarcode2DLayoutConfig
		c.QuietZoneModules = 2
		cfg = &c
	}
	return barcode2d.Layout(*grid, cfg)
}
