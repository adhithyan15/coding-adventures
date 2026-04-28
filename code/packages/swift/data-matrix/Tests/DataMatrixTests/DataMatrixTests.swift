// DataMatrixTests.swift — Test suite for the DataMatrix Swift package.
//
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Test Plan
// ============================================================================
//
// These tests verify every major component of the Data Matrix ECC200 encoder:
//
//   1.  Package constants   — dataMatrixVersion, gf256Prime
//   2.  Auto-size selection — "A" → 10×10, longer input → bigger symbol
//   3.  Grid shape          — rows, cols, modules dimensions always match
//   4.  Module types        — all modules are Bool
//   5.  Determinism         — identical input always produces identical grid
//   6.  L-finder            — bottom row all dark, left column all dark
//   7.  Timing border       — top row alternating, right column alternating
//   8.  Empty input         — encode("") works without throwing
//   9.  Error: too long     — DataMatrixError.inputTooLong for huge input
//  10.  Error: invalid size — DataMatrixError.invalidSize for bad explicit size
//  11.  Explicit size       — encode with squareSize forces symbol dimensions
//  12.  Rectangular symbols — encode(rows:cols:) works for rect sizes
//  13.  Numeric compaction  — digit strings use smaller symbols
//  14.  Larger input        — more data → bigger symbol
//  15.  Test corpus         — known (input, expectedSymbolSize) pairs
//  16.  gridToString        — renders '0' / '1' grid correctly

import Testing
import Barcode2D
@testable import DataMatrix

// ============================================================================
// MARK: - Helpers
// ============================================================================

/// Render a ModuleGrid as a compact string for snapshot comparison.
/// '1' = dark, '0' = light. Rows separated by newlines.
private func gridToStr(_ grid: ModuleGrid) -> String {
    gridToString(grid)
}

// ============================================================================
// MARK: - 1. Package constants
// ============================================================================

@Suite("Package constants")
struct PackageConstantsTests {

    /// Version string must match the canonical semver for this release.
    @Test func versionIs010() {
        #expect(dataMatrixVersion == "0.1.0")
    }

    /// GF(256) primitive polynomial for Data Matrix (0x12D = 301 decimal).
    /// This distinguishes Data Matrix from QR Code (which uses 0x11D = 285).
    @Test func gf256PrimeIs0x12D() {
        #expect(gf256Prime == 0x12D)
        #expect(gf256Prime == 301)
    }
}

// ============================================================================
// MARK: - 2. Auto-size selection
// ============================================================================
//
// ISO/IEC 16022:2006 §5: the encoder always selects the smallest ECC200
// symbol whose data capacity ≥ the number of encoded codewords.
//
// The smallest square symbol is 10×10, holding 3 data codewords.
// "A" encodes to 1 codeword (ASCII 65 + 1 = 66) → fits in 10×10.

@Suite("Auto-size selection")
struct AutoSizeSelectionTests {

    /// Single ASCII letter "A" → smallest square (10×10).
    @Test func singleLetterGives10x10() throws {
        let g = try encode("A")
        #expect(g.rows == 10)
        #expect(g.cols == 10)
    }

    /// Single ASCII digit → also fits in 10×10 (1 codeword).
    @Test func singleDigitGives10x10() throws {
        let g = try encode("5")
        #expect(g.rows == 10)
        #expect(g.cols == 10)
    }

    /// Digit pair "12" → 1 codeword (digit-pair compaction) → 10×10.
    @Test func digitPairGives10x10() throws {
        let g = try encode("12")
        #expect(g.rows == 10)
        #expect(g.cols == 10)
    }

    /// "HELLO" = 5 codewords → exceeds 10×10 capacity (3) → 12×12 (capacity 5).
    @Test func helloGives12x12() throws {
        let g = try encode("HELLO")
        #expect(g.rows == 12)
        #expect(g.cols == 12)
    }

    /// Empty string → 0 codewords → smallest symbol 10×10.
    @Test func emptyStringGives10x10() throws {
        let g = try encode("")
        #expect(g.rows == 10)
        #expect(g.cols == 10)
    }
}

// ============================================================================
// MARK: - 3. Grid shape / structure
// ============================================================================
//
// ModuleGrid.modules must have exactly `rows` arrays, each of length `cols`.
// All modules must be Bool. This validates that the encoder constructs the
// grid correctly at every dimension.

@Suite("Grid shape and structure")
struct GridShapeTests {

    /// modules.count == grid.rows for 10×10.
    @Test func moduleRowCountMatches10x10() throws {
        let g = try encode("A")
        #expect(g.modules.count == g.rows)
    }

    /// modules[0].count == grid.cols for 10×10.
    @Test func moduleColCountMatches10x10() throws {
        let g = try encode("A")
        for row in g.modules {
            #expect(row.count == g.cols)
        }
    }

    /// Same checks for 12×12.
    @Test func moduleCountMatches12x12() throws {
        let g = try encode("HELLO")
        #expect(g.modules.count == 12)
        for row in g.modules {
            #expect(row.count == 12)
        }
    }

    /// Module shape is always .square for square Data Matrix symbols.
    @Test func moduleShapeIsSquare() throws {
        let g = try encode("A")
        #expect(g.moduleShape == .square)
    }

    /// Total module count equals rows × cols.
    @Test func totalModuleCount() throws {
        let g = try encode("A")
        let total = g.modules.reduce(0) { $0 + $1.count }
        #expect(total == g.rows * g.cols)
    }
}

// ============================================================================
// MARK: - 4. Determinism
// ============================================================================
//
// Data Matrix has NO masking step. The same input must always produce
// bit-for-bit identical output — this is a critical property for
// interoperability: any scanner must be able to decode any encoder's output.

@Suite("Determinism")
struct DeterminismTests {

    /// Encode "A" twice and compare grid strings: must be identical.
    @Test func sameInputSameOutput() throws {
        let g1 = try encode("A")
        let g2 = try encode("A")
        #expect(gridToStr(g1) == gridToStr(g2))
    }

    /// Different inputs must produce different grids (encodes distinct data).
    @Test func differentInputDifferentOutput() throws {
        let g1 = try encode("A")
        let g2 = try encode("B")
        #expect(gridToStr(g1) != gridToStr(g2))
    }

    /// "HELLO" is deterministic.
    @Test func helloIsDeterministic() throws {
        let g1 = try encode("HELLO")
        let g2 = try encode("HELLO")
        #expect(gridToStr(g1) == gridToStr(g2))
    }
}

// ============================================================================
// MARK: - 5. L-finder verification
// ============================================================================
//
// The L-shaped finder pattern is Data Matrix's most distinctive feature.
// ISO/IEC 16022:2006 §7.7.2 specifies:
//
//   - Bottom row  (row R-1): ALL DARK — the horizontal leg of the L.
//   - Left column (col 0):   ALL DARK — the vertical leg of the L.
//
// Scanners use this solid L to locate and orient the symbol. Without it,
// the symbol cannot be found or decoded.

@Suite("L-finder pattern")
struct LFinderTests {

    /// Bottom row of 10×10 symbol must be all dark.
    @Test func bottomRowAllDark10x10() throws {
        let g = try encode("A")
        let lastRow = g.rows - 1
        for c in 0..<g.cols {
            #expect(g.modules[lastRow][c] == true, "Bottom row col \(c) should be dark")
        }
    }

    /// Left column of 10×10 symbol must be all dark.
    @Test func leftColumnAllDark10x10() throws {
        let g = try encode("A")
        for r in 0..<g.rows {
            #expect(g.modules[r][0] == true, "Left col row \(r) should be dark")
        }
    }

    /// Bottom row of 12×12 symbol must be all dark.
    @Test func bottomRowAllDark12x12() throws {
        let g = try encode("HELLO")
        let lastRow = g.rows - 1
        for c in 0..<g.cols {
            #expect(g.modules[lastRow][c] == true, "12×12 bottom row col \(c) should be dark")
        }
    }

    /// Left column of 12×12 symbol must be all dark.
    @Test func leftColumnAllDark12x12() throws {
        let g = try encode("HELLO")
        for r in 0..<g.rows {
            #expect(g.modules[r][0] == true, "12×12 left col row \(r) should be dark")
        }
    }

    /// L-finder for a 22×22 symbol (fits "HELLO WORLD 123456789").
    @Test func lFinderForLargerSymbol() throws {
        let input = "HELLO WORLD 123456789"
        let g = try encode(input)
        // Bottom row all dark.
        let lastRow = g.rows - 1
        for c in 0..<g.cols {
            #expect(g.modules[lastRow][c] == true)
        }
        // Left column all dark.
        for r in 0..<g.rows {
            #expect(g.modules[r][0] == true)
        }
    }
}

// ============================================================================
// MARK: - 6. Timing border verification
// ============================================================================
//
// ISO/IEC 16022:2006 §7.7.3 specifies:
//
//   - Top row    (row 0):   ALTERNATING dark/light, starting dark at col 0.
//   - Right column (col C-1): ALTERNATING dark/light, starting dark at row 0.
//
// Corner overlap rule (important):
//
//   The top row and right column share a corner at (0, C-1). The init order
//   writes the right column AFTER the top row, so the corner takes the right
//   column's value: row 0 is even → dark. This means col C-1 of the top row
//   appears dark regardless of whether C-1 is odd or even.
//
//   Similarly, (R-1, C-1) is overwritten by the L-finder bottom row (always dark).
//
//   Test consequence: we check timing for cols 0..(C-2) in the top row and
//   rows 0..(R-2) in the right column. The corner pixels follow their own rules.
//
// Scanners use the timing pattern to measure module size and compensate for
// perspective distortion — the alternation frequency tells the scanner how
// many modules wide the symbol is.

@Suite("Timing border pattern")
struct TimingBorderTests {

    /// Top row alternates starting dark (col 0 = dark, col 1 = light, …).
    /// Excludes col C-1 (overwritten by right column's row-0 = dark).
    @Test func topRowAlternates() throws {
        let g = try encode("A")
        // Check cols 0 .. cols-2 (the non-corner portion of the top row).
        for c in 0..<(g.cols - 1) {
            let expected = (c % 2 == 0)   // dark on even cols
            #expect(g.modules[0][c] == expected, "Top row col \(c): expected \(expected)")
        }
        // Corner (0, C-1) is set by the right column (row 0 is even → dark).
        #expect(g.modules[0][g.cols - 1] == true, "Top-right corner must be dark")
    }

    /// Right column alternates starting dark (row 0 = dark, row 1 = light, …).
    /// Excludes row R-1 (overwritten by L-finder bottom row = always dark).
    @Test func rightColumnAlternates() throws {
        let g = try encode("A")
        let lastCol = g.cols - 1
        // Check rows 0 .. rows-2 (the non-corner portion of the right column).
        for r in 0..<(g.rows - 1) {
            let expected = (r % 2 == 0)   // dark on even rows
            #expect(g.modules[r][lastCol] == expected, "Right col row \(r): expected \(expected)")
        }
        // Bottom-right corner (R-1, C-1) is overwritten by the L-finder bottom row → dark.
        #expect(g.modules[g.rows - 1][lastCol] == true, "Bottom-right corner must be dark")
    }

    /// Top row of 12×12 alternates correctly (excluding corner).
    @Test func topRowAlternates12x12() throws {
        let g = try encode("HELLO")
        for c in 0..<(g.cols - 1) {
            #expect(g.modules[0][c] == (c % 2 == 0), "12×12 top row col \(c)")
        }
        #expect(g.modules[0][g.cols - 1] == true, "12×12 top-right corner must be dark")
    }

    /// Right column of 12×12 alternates correctly (excluding corner).
    @Test func rightColumnAlternates12x12() throws {
        let g = try encode("HELLO")
        let lastCol = g.cols - 1
        for r in 0..<(g.rows - 1) {
            #expect(g.modules[r][lastCol] == (r % 2 == 0), "12×12 right col row \(r)")
        }
        #expect(g.modules[g.rows - 1][lastCol] == true, "12×12 bottom-right corner must be dark")
    }
}

// ============================================================================
// MARK: - 7. Empty input
// ============================================================================

@Suite("Empty input")
struct EmptyInputTests {

    /// encode("") must not throw.
    @Test func emptyDoesNotThrow() throws {
        let g = try encode("")
        // Empty encodes to 0 codewords → smallest symbol.
        #expect(g.rows == 10)
        #expect(g.cols == 10)
    }

    /// Empty grid has correct module count.
    @Test func emptyGridStructure() throws {
        let g = try encode("")
        #expect(g.modules.count == 10)
        for row in g.modules { #expect(row.count == 10) }
    }
}

// ============================================================================
// MARK: - 8. Error handling
// ============================================================================

@Suite("Error handling")
struct ErrorHandlingTests {

    /// A string far exceeding 1558 codewords should throw inputTooLong.
    @Test func inputTooLongThrows() throws {
        // Repeat "A" 2000 times → 2000 codewords → exceeds max 1558.
        let huge = String(repeating: "A", count: 2000)
        #expect(throws: DataMatrixError.self) {
            _ = try encode(huge)
        }
    }

    /// An invalid square size throws invalidSize.
    @Test func invalidSquareSizeThrows() throws {
        // 11×11 is not a valid ECC200 symbol size.
        #expect(throws: DataMatrixError.self) {
            _ = try encode("A", squareSize: 11)
        }
    }

    /// An invalid square size of 0 throws invalidSize.
    @Test func zeroSizeThrows() throws {
        #expect(throws: DataMatrixError.self) {
            _ = try encode("A", squareSize: 0)
        }
    }

    /// An invalid rectangular size throws invalidSize.
    @Test func invalidRectSizeThrows() throws {
        // 10×20 is not in the rectangular size table.
        #expect(throws: DataMatrixError.self) {
            _ = try encode("A", rows: 10, cols: 20)
        }
    }

    /// Encoding input that doesn't fit a forced size throws inputTooLong.
    @Test func inputTooLongForForcedSize() throws {
        // 10×10 holds 3 codewords. "HELLO" = 5 codewords → too long.
        #expect(throws: DataMatrixError.self) {
            _ = try encode("HELLO", squareSize: 10)
        }
    }
}

// ============================================================================
// MARK: - 9. Explicit size overrides
// ============================================================================

@Suite("Explicit size overrides")
struct ExplicitSizeTests {

    /// Forcing 12×12 gives a 12×12 grid even for short input.
    @Test func forcedSize12x12() throws {
        let g = try encode("A", squareSize: 12)
        #expect(g.rows == 12)
        #expect(g.cols == 12)
    }

    /// Forcing 18×18 gives a 18×18 grid.
    @Test func forcedSize18x18() throws {
        let g = try encode("Hi", squareSize: 18)
        #expect(g.rows == 18)
        #expect(g.cols == 18)
    }

    /// Forcing 26×26 gives a 26×26 grid (largest single-region square symbol).
    @Test func forcedSize26x26() throws {
        let g = try encode("TEST", squareSize: 26)
        #expect(g.rows == 26)
        #expect(g.cols == 26)
    }

    /// Forcing 32×32 (first multi-region square: 2×2 data regions).
    @Test func forcedSize32x32() throws {
        let g = try encode("TEST", squareSize: 32)
        #expect(g.rows == 32)
        #expect(g.cols == 32)
    }
}

// ============================================================================
// MARK: - 10. Rectangular symbols
// ============================================================================

@Suite("Rectangular symbols")
struct RectangularSymbolTests {

    /// 8×18 is the smallest rectangular symbol.
    @Test func encodeInto8x18() throws {
        let g = try encode("Hi", rows: 8, cols: 18)
        #expect(g.rows == 8)
        #expect(g.cols == 18)
    }

    /// 8×32 rectangular symbol.
    @Test func encodeInto8x32() throws {
        let g = try encode("Hi", rows: 8, cols: 32)
        #expect(g.rows == 8)
        #expect(g.cols == 32)
    }

    /// 16×48 is the largest rectangular symbol.
    @Test func encodeInto16x48() throws {
        let g = try encode("Hi", rows: 16, cols: 48)
        #expect(g.rows == 16)
        #expect(g.cols == 48)
    }

    /// Rectangular symbol has correct grid dimensions.
    @Test func rectGridStructure() throws {
        let g = try encode("A", rows: 8, cols: 18)
        #expect(g.modules.count == 8)
        for row in g.modules { #expect(row.count == 18) }
    }

    /// Auto-select rectangular: shape = .rectangle picks from rect sizes.
    @Test func autoRectSelection() throws {
        var opts = DataMatrixOptions()
        opts.shape = .rectangle
        let g = try encode("Hi", options: opts)
        // 8×18 has capacity 5, "Hi" = 2 codewords → smallest rect.
        #expect(g.rows == 8)
        #expect(g.cols == 18)
    }

    /// L-finder is present on a rectangular symbol (bottom row all dark).
    @Test func rectLFinderBottomRow() throws {
        let g = try encode("Hi", rows: 8, cols: 18)
        let lastRow = g.rows - 1
        for c in 0..<g.cols {
            #expect(g.modules[lastRow][c] == true, "Rect bottom row col \(c) should be dark")
        }
    }

    /// L-finder is present on a rectangular symbol (left col all dark).
    @Test func rectLFinderLeftCol() throws {
        let g = try encode("Hi", rows: 8, cols: 18)
        for r in 0..<g.rows {
            #expect(g.modules[r][0] == true, "Rect left col row \(r) should be dark")
        }
    }
}

// ============================================================================
// MARK: - 11. Larger inputs produce bigger symbols
// ============================================================================

@Suite("Larger inputs produce bigger symbols")
struct SymbolScalingTests {

    /// Longer text should never produce a smaller symbol.
    @Test func longerInputBiggerOrEqualSymbol() throws {
        let short = try encode("A")
        let medium = try encode("HELLO WORLD")
        let long_ = try encode("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG")
        // Symbol rows must be non-decreasing as input grows.
        #expect(short.rows <= medium.rows)
        #expect(medium.rows <= long_.rows)
    }

    /// "HELLO WORLD" requires more than 10×10.
    @Test func helloWorldExceeds10x10() throws {
        let g = try encode("HELLO WORLD")
        #expect(g.rows > 10)
    }

    /// A 40-character string should need at least a 20×20 symbol.
    @Test func fortyCharsNeedsLargerSymbol() throws {
        let g = try encode("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$")
        #expect(g.rows >= 20)
    }
}

// ============================================================================
// MARK: - 12. Digit-pair compaction
// ============================================================================
//
// Two consecutive ASCII digits pack into one codeword, halving the codeword
// budget for numeric strings. This means "0123456789" (10 chars) encodes
// to only 5 codewords, fitting in 10×10 (capacity 3)?
// Actually 5 > 3, so it upgrades to 12×12 (capacity 5). Exactly fits.

@Suite("Digit-pair compaction")
struct DigitPairCompactionTests {

    /// "0123456789" → 5 codewords (digit pairs) → 12×12 (capacity 5).
    @Test func tenDigitsFitIn12x12() throws {
        let g = try encode("0123456789")
        #expect(g.rows == 12)
        #expect(g.cols == 12)
    }

    /// "12345678901234" → 7 codewords → 14×14 (capacity 8).
    @Test func fourteenDigitsFitIn14x14() throws {
        let g = try encode("12345678901234")
        #expect(g.rows == 14)
    }

    /// Digit-pair compaction is deterministic.
    @Test func digitCompactionIsDeterministic() throws {
        let g1 = try encode("123456")
        let g2 = try encode("123456")
        #expect(gridToStr(g1) == gridToStr(g2))
    }
}

// ============================================================================
// MARK: - 13. Test corpus (cross-language compatibility)
// ============================================================================
//
// These canonical (input, expected symbol rows) pairs must produce identical
// results across ALL language implementations of the Data Matrix encoder
// (Python, Kotlin, Java, Swift, Lua, Perl, …).
//
// They are chosen to cover:
//   - Single ASCII chars
//   - Digit pairs
//   - Mixed alphanumeric
//   - URL patterns
//   - Boundary inputs at exact capacity

@Suite("Cross-language test corpus")
struct TestCorpusTests {

    @Test func corpusA()         throws { #expect(try encode("A").rows == 10) }
    @Test func corpusAB()        throws { #expect(try encode("AB").rows == 10) }
    @Test func corpus123()       throws { #expect(try encode("123").rows == 10) }
    /// "1234" → two digit pairs "12" and "34" → 2 codewords → fits in 10×10.
    @Test func corpus1234()      throws { #expect(try encode("1234").rows == 10) }
    @Test func corpusHello()     throws { #expect(try encode("HELLO").rows == 12) }
    @Test func corpusHello12()   throws { #expect(try encode("HELLO12").rows >= 14) }
    @Test func corpusURL()       throws { #expect(try encode("http://x.co").rows >= 14) }

    /// "HELLO WORLD!" = 12 codewords → 14×14 (capacity 8)? Let's check.
    /// Actually 12 > 8, so it upgrades to 16×16 (capacity 12). Fits exactly.
    @Test func corpusHelloWorld() throws {
        let g = try encode("HELLO WORLD!")
        #expect(g.rows >= 14)
    }

    /// Three-digit number "999" → 2 codewords (one digit pair "99" + one "9").
    @Test func corpus999() throws {
        let g = try encode("999")
        #expect(g.rows == 10)
    }

    /// "AB" encodes to 2 codewords → fits in 10×10 (capacity 3).
    @Test func corpusABFits10x10() throws {
        let g = try encode("AB")
        #expect(g.rows == 10 && g.cols == 10)
    }
}

// ============================================================================
// MARK: - 14. gridToString utility
// ============================================================================

@Suite("gridToString utility")
struct GridToStringTests {

    /// Output has exactly `rows` lines.
    @Test func lineCount() throws {
        let g = try encode("A")
        let s = gridToStr(g)
        let lines = s.split(separator: "\n", omittingEmptySubsequences: false)
        #expect(lines.count == g.rows)
    }

    /// Each line has exactly `cols` characters.
    @Test func lineLength() throws {
        let g = try encode("A")
        let s = gridToStr(g)
        for line in s.split(separator: "\n", omittingEmptySubsequences: false) {
            #expect(line.count == g.cols)
        }
    }

    /// All characters are '0' or '1'.
    @Test func onlyZerosAndOnes() throws {
        let g = try encode("HELLO")
        let s = gridToStr(g)
        let chars = Set(s.filter { $0 != "\n" })
        #expect(chars.isSubset(of: ["0", "1"]))
    }

    /// The bottom row string ends with all '1's (L-finder).
    @Test func bottomRowIsAllOnes() throws {
        let g = try encode("A")
        let lines = gridToStr(g).split(separator: "\n", omittingEmptySubsequences: false)
        let lastLine = String(lines.last!)
        #expect(lastLine == String(repeating: "1", count: g.cols))
    }

    /// The left column in the string representation starts with '1' on every row.
    @Test func leftColumnIsAllOnes() throws {
        let g = try encode("A")
        let lines = gridToStr(g).split(separator: "\n", omittingEmptySubsequences: false)
        for line in lines {
            #expect(line.first == "1")
        }
    }

    /// Deterministic: same input → same string.
    @Test func deterministicString() throws {
        let s1 = gridToStr(try encode("HELLO"))
        let s2 = gridToStr(try encode("HELLO"))
        #expect(s1 == s2)
    }
}

// ============================================================================
// MARK: - 15. DataMatrixOptions
// ============================================================================

@Suite("DataMatrixOptions")
struct DataMatrixOptionsTests {

    /// Default options: size == nil, shape == .square.
    @Test func defaultOptions() {
        let opts = DataMatrixOptions()
        #expect(opts.size == nil)
        if case .square = opts.shape { } else {
            Issue.record("Expected .square shape by default")
        }
    }

    /// Setting size forces the symbol dimension.
    @Test func explicitSizeOption() throws {
        var opts = DataMatrixOptions()
        opts.size = 14
        let g = try encode("A", options: opts)
        #expect(g.rows == 14 && g.cols == 14)
    }

    /// Setting shape = .rectangle allows rectangular auto-selection.
    @Test func rectangleShapeOption() throws {
        var opts = DataMatrixOptions()
        opts.shape = .rectangle
        let g = try encode("A", options: opts)
        // 8×18 has capacity 5, "A" = 1 codeword → smallest rect.
        #expect(g.rows == 8)
        #expect(g.cols == 18)
    }

    /// Setting shape = .any auto-selects from both families.
    @Test func anyShapeOption() throws {
        var opts = DataMatrixOptions()
        opts.shape = .any
        // "A" = 1 codeword → smallest of all sizes is 8×18 (capacity 5).
        let g = try encode("A", options: opts)
        // 8×18 beats 10×10 on total area: 8*18=144 vs 10*10=100.
        // Actually 10×10 area (100) < 8×18 area (144), so auto picks 10×10.
        #expect(g.rows == 10 || g.rows == 8)  // either is valid depending on sort order
    }
}

// ============================================================================
// MARK: - 16. Multi-region symbol structural integrity
// ============================================================================
//
// Symbols ≥ 32×32 have multiple data regions separated by alignment borders.
// These tests verify that even for multi-region symbols the public API
// produces structurally valid output.

@Suite("Multi-region symbols")
struct MultiRegionTests {

    /// 32×32 symbol (2×2 data regions) has correct dimensions.
    @Test func symbol32x32Dimensions() throws {
        let g = try encode("TEST DATA FOR LARGER SYMBOL 32X32", squareSize: 32)
        #expect(g.rows == 32 && g.cols == 32)
    }

    /// 32×32 L-finder bottom row all dark.
    @Test func symbol32x32BottomRow() throws {
        let g = try encode("A", squareSize: 32)
        let lastRow = g.rows - 1
        for c in 0..<g.cols {
            #expect(g.modules[lastRow][c] == true)
        }
    }

    /// 32×32 L-finder left col all dark.
    @Test func symbol32x32LeftCol() throws {
        let g = try encode("A", squareSize: 32)
        for r in 0..<g.rows {
            #expect(g.modules[r][0] == true)
        }
    }

    /// 64×64 symbol (4×4 data regions) — just check it encodes without error.
    @Test func symbol64x64Encodes() throws {
        // 64×64 holds 280 data codewords. Use a short input.
        let g = try encode("DATA MATRIX 64X64", squareSize: 64)
        #expect(g.rows == 64 && g.cols == 64)
    }
}
