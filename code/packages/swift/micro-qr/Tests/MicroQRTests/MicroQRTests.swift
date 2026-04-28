// MicroQRTests.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Micro QR Code Test Suite
// ============================================================================
//
// These tests verify every major component of the Micro QR Code encoder:
//
//   1. Symbol dimensions    — correct size for each version
//   2. Auto-version selection — smallest symbol that fits the input
//   3. Structural modules   — finder pattern, separator, timing placement
//   4. Determinism          — same input always produces same grid
//   5. ECC level constraints— valid and invalid (version, ecc) combinations
//   6. Error handling       — InputTooLong, ECCNotAvailable, etc.
//   7. Capacity boundaries  — at, one-above max for each symbol
//   8. Format information   — non-zero modules in the format strip
//   9. Grid completeness    — square, correct dimensions
//  10. Cross-language corpus— reference inputs and expected symbol sizes
//
// Tests are structured to give isolated, descriptive failure messages.
// Each test has a short comment explaining what property is being verified
// and why it matters.

import Testing
import Barcode2D
@testable import MicroQR

// ============================================================================
// MARK: - Helpers
// ============================================================================

/// Serialize a ModuleGrid to a compact string for comparison across test runs
/// and across language implementations.
///
/// Format: each row is a string of '0' and '1', rows joined by newlines.
/// '1' = dark module, '0' = light module.
private func gridToString(_ grid: ModuleGrid) -> String {
    grid.modules.map { row in
        row.map { $0 ? "1" : "0" }.joined()
    }.joined(separator: "\n")
}

// ============================================================================
// MARK: - Symbol dimensions
// ============================================================================
//
// ISO/IEC 18004:2015 Annex E specifies: size = 2 × version_number + 9
// M1 → 11×11, M2 → 13×13, M3 → 15×15, M4 → 17×17

@Suite("Symbol dimensions")
struct SymbolDimensionTests {

    /// M1 encodes a single digit: verify 11×11 output.
    @Test func m1Is11x11() throws {
        let g = try encode("1")
        #expect(g.rows == 11)
        #expect(g.cols == 11)
    }

    /// M2 is selected for "HELLO" (5 alphanumeric chars): verify 13×13.
    @Test func m2Is13x13ForHello() throws {
        let g = try encode("HELLO")
        #expect(g.rows == 13)
        #expect(g.cols == 13)
    }

    /// M4 is selected for a URL: verify 17×17.
    @Test func m4Is17x17ForUrl() throws {
        let g = try encode("https://a.b")
        #expect(g.rows == 17)
        #expect(g.cols == 17)
    }

    /// Every grid must be square: rows == cols for all valid inputs.
    @Test func moduleShapeIsSquare() throws {
        let g = try encode("1")
        #expect(g.moduleShape == .square)
    }
}

// ============================================================================
// MARK: - Auto-version selection
// ============================================================================
//
// The encoder always selects the smallest symbol that can hold the input.
// Auto-selection order: M1 → M2-L → M2-M → M3-L → M3-M → M4-L → M4-M → M4-Q

@Suite("Auto-version selection")
struct AutoVersionSelectionTests {

    /// Single digit → M1 (smallest possible).
    @Test func autoSelectsM1ForSingleDigit() throws {
        #expect(try encode("1").rows == 11)
    }

    /// Five digits fill M1's numeric capacity exactly.
    @Test func autoSelectsM1For12345() throws {
        #expect(try encode("12345").rows == 11)
    }

    /// Six digits exceed M1's capacity (5 max) → M2.
    @Test func autoSelectsM2For6Digits() throws {
        #expect(try encode("123456").rows == 13)
    }

    /// "HELLO" = 5 alphanumeric chars → M2-L (alphaCap = 6).
    @Test func autoSelectsM2ForHello() throws {
        #expect(try encode("HELLO").rows == 13)
    }

    /// "hello" = 5 bytes (lowercase is byte mode, not alphanumeric).
    /// M3-L has byteCap = 9, M2-L has byteCap = 4 → M3-L selected.
    @Test func autoSelectsM3ForHelloLowercase() throws {
        #expect(try encode("hello").rows >= 15)
    }

    /// URL exceeds all M1–M3 byte capacities → M4.
    @Test func autoSelectsM4ForUrl() throws {
        #expect(try encode("https://a.b").rows == 17)
    }

    /// Forcing version M4 for a single digit still produces 17×17.
    @Test func forcedVersionM4() throws {
        #expect(try encode("1", version: .M4).rows == 17)
    }

    /// Same content in M2-L vs M2-M must produce different grids (different ECC).
    @Test func forcedEccMProducesDifferentGrid() throws {
        let gL = try encode("HELLO", ecc: .L)
        let gM = try encode("HELLO", ecc: .M)
        #expect(gridToString(gL) != gridToString(gM))
    }
}

// ============================================================================
// MARK: - Test corpus
// ============================================================================
//
// A set of canonical inputs whose expected symbol sizes are fixed across all
// language implementations. Cross-language verification compares these values.

@Suite("Test corpus")
struct TestCorpusTests {

    @Test func corpus1() throws { #expect(try encode("1").rows == 11) }
    @Test func corpus12345() throws { #expect(try encode("12345").rows == 11) }
    @Test func corpusHello() throws { #expect(try encode("HELLO").rows == 13) }

    /// 6 alphanumeric chars fit in M2-L (alphaCap = 6).
    @Test func corpusA1B2C3M2() throws {
        #expect(try encode("A1B2C3").rows == 13)
    }

    /// "hello" = 5 bytes → M3-L (byteCap = 9, M2-L only has 4).
    @Test func corpusHelloByteM3() throws {
        #expect(try encode("hello").rows >= 15)
    }

    /// 8 numeric digits → M2-L (numericCap = 10).
    @Test func corpus8DigitNumeric() throws {
        #expect(try encode("01234567").rows == 13)
    }

    /// 13 alphanumeric chars → M3-L (alphaCap = 14).
    @Test func corpusMicroQRTestM3L() throws {
        #expect(try encode("MICRO QR TEST").rows == 15)
    }
}

// ============================================================================
// MARK: - Structural modules
// ============================================================================
//
// Every Micro QR symbol has the same fixed structural elements regardless of
// the input data. Testing these independently verifies that buildGrid() is
// correct before the data-placement tests run.

@Suite("Structural modules")
struct StructuralModuleTests {

    /// Finder pattern: outer ring (rows/cols 0 and 6) must be all dark.
    @Test func finderPatternOuterRingM1() throws {
        let g = try encode("1")
        let m = g.modules
        // Top row of finder (row 0): all 7 modules dark
        for c in 0..<7 {
            #expect(m[0][c] == true, "finder top row col \(c) should be dark")
        }
        // Bottom row of finder (row 6): all 7 modules dark
        for c in 0..<7 {
            #expect(m[6][c] == true, "finder bottom row col \(c) should be dark")
        }
        // Left col of finder (col 0): all 7 modules dark
        for r in 0..<7 {
            #expect(m[r][0] == true, "finder left col row \(r) should be dark")
        }
        // Right col of finder (col 6): all 7 modules dark
        for r in 0..<7 {
            #expect(m[r][6] == true, "finder right col row \(r) should be dark")
        }
    }

    /// Finder pattern inner ring: row 1 cols 1–5 must be light.
    @Test func finderPatternInnerRingLightM1() throws {
        let g = try encode("1")
        let m = g.modules
        for c in 1...5 {
            #expect(m[1][c] == false, "finder inner ring row 1 col \(c) should be light")
        }
    }

    /// Finder pattern core: rows 2–4, cols 2–4 must be dark (3×3 center).
    @Test func finderPatternCoreM1() throws {
        let g = try encode("1")
        let m = g.modules
        for r in 2...4 {
            for c in 2...4 {
                #expect(m[r][c] == true, "finder core (\(r),\(c)) should be dark")
            }
        }
    }

    /// L-shaped separator: row 7 cols 0–7 must all be light.
    @Test func separatorBottomRowM2() throws {
        let g = try encode("HELLO")
        let m = g.modules
        for c in 0...7 {
            #expect(m[7][c] == false, "separator bottom row col \(c) should be light")
        }
    }

    /// L-shaped separator: col 7 rows 0–7 must all be light.
    @Test func separatorRightColM2() throws {
        let g = try encode("HELLO")
        let m = g.modules
        for r in 0...7 {
            #expect(m[r][7] == false, "separator right col row \(r) should be light")
        }
    }

    /// Timing row: row 0 cols 8+ must alternate dark/light (even=dark, odd=light).
    ///
    /// Unlike regular QR (timing at row 6), Micro QR runs timing along row 0 and
    /// col 0 from position 8 to the symbol edge.
    @Test func timingRowM4() throws {
        let g = try encode("https://a.b")
        let m = g.modules
        for c in 8..<17 {
            #expect(m[0][c] == (c % 2 == 0), "timing row 0 col \(c)")
        }
    }

    /// Timing col: col 0 rows 8+ must alternate dark/light (even=dark, odd=light).
    @Test func timingColM4() throws {
        let g = try encode("https://a.b")
        let m = g.modules
        for r in 8..<17 {
            #expect(m[r][0] == (r % 2 == 0), "timing col 0 row \(r)")
        }
    }
}

// ============================================================================
// MARK: - Determinism
// ============================================================================
//
// The encoder must be deterministic: same input always produces identical output.
// This is critical for cross-language corpus verification.

@Suite("Determinism")
struct DeterminismTests {

    /// Encoding the same string twice must yield identical grids.
    @Test func deterministicAcrossMultipleInputs() throws {
        for input in ["1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b"] {
            let g1 = try encode(input)
            let g2 = try encode(input)
            #expect(
                gridToString(g1) == gridToString(g2),
                "non-deterministic for '\(input)'"
            )
        }
    }

    /// Different inputs must produce different grids.
    @Test func differentInputsProduceDifferentGrids() throws {
        let g1 = try encode("1")
        let g2 = try encode("2")
        #expect(gridToString(g1) != gridToString(g2))
    }
}

// ============================================================================
// MARK: - ECC level constraints
// ============================================================================
//
// Not all (version, ECC) combinations are valid in Micro QR:
//
//   M1: detection only
//   M2: L or M
//   M3: L or M
//   M4: L, M, or Q
//
// Valid combinations must succeed; invalid ones must throw eccNotAvailable.

@Suite("ECC level constraints")
struct EccLevelConstraintTests {

    /// M1/detection is valid: must succeed.
    @Test func m1Detection() throws {
        let g = try encode("1", version: .M1, ecc: .detection)
        #expect(g.rows == 11)
    }

    /// M4/Q is the highest valid ECC level.
    @Test func m4Q() throws {
        let g = try encode("HELLO", version: .M4, ecc: .Q)
        #expect(g.rows == 17)
    }

    /// All three ECC levels for M4 must produce distinct grids for the same input.
    @Test func m4AllEccDiffer() throws {
        let gL = try encode("HELLO", version: .M4, ecc: .L)
        let gM = try encode("HELLO", version: .M4, ecc: .M)
        let gQ = try encode("HELLO", version: .M4, ecc: .Q)
        #expect(gridToString(gL) != gridToString(gM))
        #expect(gridToString(gM) != gridToString(gQ))
        #expect(gridToString(gL) != gridToString(gQ))
    }

    /// M1 only supports .detection — requesting .L must fail.
    @Test func m1RejectsEccL() {
        #expect(throws: MicroQRError.self) {
            try encode("1", version: .M1, ecc: .L)
        }
    }

    /// M2 does not support Q — must fail.
    @Test func m2RejectsEccQ() {
        #expect(throws: MicroQRError.self) {
            try encode("1", version: .M2, ecc: .Q)
        }
    }

    /// M3 does not support Q — must fail.
    @Test func m3RejectsEccQ() {
        #expect(throws: MicroQRError.self) {
            try encode("1", version: .M3, ecc: .Q)
        }
    }
}

// ============================================================================
// MARK: - Error handling
// ============================================================================

@Suite("Error handling")
struct ErrorHandlingTests {

    /// 36 numeric chars exceed M4-L's capacity (35 max) → inputTooLong.
    @Test func inputTooLong() {
        let long = String(repeating: "1", count: 36)
        #expect(throws: MicroQRError.self) {
            try encode(long)
        }
    }

    /// Empty string is valid — selects M1 (numeric mode, 0 chars).
    @Test func emptyStringEncodesToM1() throws {
        let g = try encode("")
        #expect(g.rows == 11)
    }

    /// M1/Q is a completely invalid combination.
    @Test func eccNotAvailableForNonexistentCombo() {
        #expect(throws: MicroQRError.self) {
            try encode("1", version: .M1, ecc: .Q)
        }
    }
}

// ============================================================================
// MARK: - Capacity boundaries
// ============================================================================
//
// Verify at-capacity and overflow-by-one for key (version, mode) combinations.

@Suite("Capacity boundaries")
struct CapacityBoundaryTests {

    /// M1 max: exactly 5 numeric chars → stays in M1.
    @Test func m1Max5Digits() throws {
        #expect(try encode("12345").rows == 11)
    }

    /// M1 overflow: 6 numeric chars → bumps to M2.
    @Test func m1Overflow6Digits() throws {
        #expect(try encode("123456").rows == 13)
    }

    /// M4 max: exactly 35 numeric chars → stays in M4.
    @Test func m4Max35Digits() throws {
        let g = try encode(String(repeating: "1", count: 35))
        #expect(g.rows == 17)
    }

    /// M4 overflow: 36 numeric chars → throws inputTooLong.
    @Test func m4Overflow36Digits() {
        #expect(throws: MicroQRError.self) {
            try encode(String(repeating: "1", count: 36))
        }
    }

    /// M4-L byte capacity: exactly 15 lowercase chars → stays in M4.
    @Test func m4MaxByte15Chars() throws {
        let g = try encode(String(repeating: "a", count: 15))
        #expect(g.rows == 17)
    }

    /// M4-Q has 21 numeric capacity with highest ECC.
    @Test func m4QMax21Numeric() throws {
        let g = try encode(String(repeating: "1", count: 21), ecc: .Q)
        #expect(g.rows == 17)
    }
}

// ============================================================================
// MARK: - Format information
// ============================================================================
//
// The format info strip occupies:
//   Row 8, cols 1–8 (8 modules)
//   Col 8, rows 1–7 (7 modules)
//
// For any valid encoding the XOR-masked format word is non-zero, so at least
// one module in the strip must be dark.

@Suite("Format information")
struct FormatInformationTests {

    /// M4-L format info strip must contain at least one dark module.
    @Test func formatInfoNonZeroM4() throws {
        let g = try encode("HELLO", version: .M4, ecc: .L)
        let m = g.modules
        let anyDarkRow = (1...8).contains { m[8][$0] }
        let anyDarkCol = (1...7).contains { m[$0][8] }
        #expect(anyDarkRow || anyDarkCol, "format info should have some dark modules")
    }

    /// M1 format info strip must contain at least one dark module.
    @Test func formatInfoNonZeroM1() throws {
        let g = try encode("1")
        let m = g.modules
        let count = (1...8).filter { m[8][$0] }.count
                  + (1...7).filter { m[$0][8] }.count
        #expect(count > 0, "M1 format info should have some dark modules")
    }

    /// Different mask patterns produce different format info bits.
    /// We verify this indirectly: two distinct (symbol, mask) combos must
    /// differ in format positions (verified by whole-grid comparison).
    @Test func differentVersionsDifferentFormat() throws {
        let gM1 = try encode("1")
        let gM2 = try encode("HELLO")
        // The format strips encode different symbol indicators, so they must differ
        let m1fmt = (1...8).map { gM1.modules[8][$0] }
        let m2fmt = (1...8).map { gM2.modules[8][$0] }
        #expect(m1fmt != m2fmt, "M1 and M2 should have different format info")
    }
}

// ============================================================================
// MARK: - Grid completeness
// ============================================================================
//
// Every output grid must:
//   - Be exactly square (rows == cols)
//   - Have the correct number of rows
//   - Have every row the correct length

@Suite("Grid completeness")
struct GridCompletenessTests {

    @Test func allModulesAreBool() throws {
        for input in ["1", "HELLO", "hello", "https://a.b"] {
            let g = try encode(input)
            #expect(g.rows == g.cols, "grid should be square for '\(input)'")
            #expect(g.modules.count == g.rows)
            for row in g.modules {
                #expect(row.count == g.cols)
            }
        }
    }
}

// ============================================================================
// MARK: - Cross-language corpus
// ============================================================================
//
// Reference inputs paired with expected symbol sizes. All language
// implementations (Swift, Rust, Python, Ruby, …) must agree on these values.
// If the sizes differ, there is a bug in one implementation.

@Suite("Cross-language corpus")
struct CrossLanguageCorpusTests {

    @Test func crossLanguageCorpus() throws {
        let cases: [(String, Int)] = [
            ("1",             11),  // M1:   single numeric
            ("12345",         11),  // M1:   at max numeric capacity
            ("HELLO",         13),  // M2-L: 5 alphanumeric chars
            ("01234567",      13),  // M2-L: 8 numeric chars
            ("https://a.b",   17),  // M4-L: byte mode URL
            ("MICRO QR TEST", 15),  // M3-L: 13 alphanumeric chars (with space)
        ]
        for (input, expectedSize) in cases {
            let g = try encode(input)
            #expect(
                g.rows == expectedSize,
                "input '\(input)': expected \(expectedSize)×\(expectedSize) but got \(g.rows)×\(g.cols)"
            )
        }
    }
}

// ============================================================================
// MARK: - Module values (spot checks)
// ============================================================================
//
// Spot-check specific module positions that are deterministic regardless of
// the input data. The top-left corner (0,0) is always dark (start of finder).

@Suite("Module value spot checks")
struct ModuleSpotCheckTests {

    /// Module (0,0) is always dark: it is the corner of the finder pattern.
    @Test func topLeftCornerAlwaysDark() throws {
        for input in ["1", "12345", "HELLO", "hello"] {
            let g = try encode(input)
            #expect(g.modules[0][0] == true, "top-left module must be dark for '\(input)'")
        }
    }

    /// Module (7,7) is always light: it is the corner of the separator.
    @Test func separatorCornerAlwaysLight() throws {
        for input in ["HELLO", "12345", "hello"] {
            let g = try encode(input)
            #expect(g.modules[7][7] == false, "separator corner (7,7) must be light for '\(input)'")
        }
    }

    /// The grid for the same M2-L input is fully deterministic pixel-by-pixel.
    @Test func m2LGridDeterministic() throws {
        let g1 = try encode("HELLO", version: .M2, ecc: .L)
        let g2 = try encode("HELLO", version: .M2, ecc: .L)
        #expect(gridToString(g1) == gridToString(g2))
    }
}

// ============================================================================
// MARK: - encodeAt convenience API
// ============================================================================

@Suite("encodeAt API")
struct EncodeAtAPITests {

    /// encodeAt is sugar for encode(_:version:ecc:) — must produce identical output.
    @Test func encodeAtMatchesEncode() throws {
        let g1 = try encodeAt("HELLO", version: .M2, ecc: .L)
        let g2 = try encode("HELLO", version: .M2, ecc: .L)
        #expect(gridToString(g1) == gridToString(g2))
    }

    @Test func encodeAtM1Detection() throws {
        let g = try encodeAt("123", version: .M1, ecc: .detection)
        #expect(g.rows == 11)
    }

    @Test func encodeAtM4Q() throws {
        let g = try encodeAt("HELLO WORLD", version: .M4, ecc: .Q)
        #expect(g.rows == 17)
    }
}

// ============================================================================
// MARK: - Numeric encoding edge cases
// ============================================================================
//
// Numeric encoding groups digits in triples (10 bits), pairs (7 bits), or
// singles (4 bits). These tests exercise all three code paths.

@Suite("Numeric encoding edge cases")
struct NumericEncodingTests {

    /// Single digit: 4-bit encoding path.
    @Test func singleDigitEncodes() throws {
        for d in ["0", "1", "5", "9"] {
            let g = try encode(d)
            #expect(g.rows == 11, "single digit '\(d)' should fit in M1")
        }
    }

    /// Two digits: 7-bit pair encoding path.
    @Test func twoDigitsEncode() throws {
        let g = try encode("42")
        #expect(g.rows == 11)
    }

    /// Three digits: 10-bit triple encoding path.
    @Test func threeDigitsEncode() throws {
        let g = try encode("999")
        #expect(g.rows == 11)
    }

    /// Five digits (M1 max): triple + pair path.
    @Test func fiveDigitsEncodeInM1() throws {
        let g = try encode("99999")
        #expect(g.rows == 11)
    }
}

// ============================================================================
// MARK: - Alphanumeric encoding edge cases
// ============================================================================

@Suite("Alphanumeric encoding edge cases")
struct AlphanumericEncodingTests {

    /// Single alphanumeric char: 6-bit single path.
    @Test func singleAlphaChar() throws {
        let g = try encode("A")
        #expect(g.rows == 13)  // M2-L: alphaCap=6
    }

    /// Odd-length alphanumeric: last char uses 6-bit path.
    @Test func oddLengthAlphanumeric() throws {
        let g = try encode("ABC")
        #expect(g.rows == 13)
    }

    /// Even-length alphanumeric: all pairs, no single.
    @Test func evenLengthAlphanumeric() throws {
        let g = try encode("ABCD")
        #expect(g.rows == 13)
    }

    /// Space is in the 45-char set (index 36).
    @Test func spaceIsAlphanumeric() throws {
        let g = try encode("MICRO QR TEST")
        #expect(g.rows == 15)
    }

    /// Special alphanumeric chars: $ % * + - . / :
    /// 8 chars exceed M2-L alphaCap (6) and M2-M alphaCap (5) → M3-L (alphaCap=14).
    @Test func specialAlphanumericChars() throws {
        let g = try encode("$%*+-./:") // 8 special chars → M3-L
        #expect(g.rows == 15)
    }
}

// ============================================================================
// MARK: - Byte mode encoding
// ============================================================================

@Suite("Byte mode encoding")
struct ByteModeEncodingTests {

    /// Lowercase letters require byte mode (not in alphanumeric set).
    @Test func lowercaseRequiresByteMode() throws {
        let g = try encode("hello")
        // "hello" = 5 bytes, M3-L has byteCap=9 → M3
        #expect(g.rows >= 15)
    }

    /// Mixed case forces byte mode.
    @Test func mixedCaseByteMode() throws {
        let g = try encode("Hello")
        #expect(g.rows >= 15)
    }

    /// A single non-alphanumeric character (e.g. lowercase 'a') forces byte mode.
    @Test func singleLowercaseIsValidByteMode() throws {
        let g = try encode("a", version: .M3, ecc: .L)
        #expect(g.rows == 15)
    }
}

// ============================================================================
// MARK: - Mask selection
// ============================================================================
//
// The encoder evaluates all 4 masks and picks the one with the lowest penalty.
// We don't test which mask is chosen (that would be too brittle), but we do
// verify that the choice produces a valid grid.

@Suite("Mask selection")
struct MaskSelectionTests {

    /// Encoding with explicit version/ecc produces a valid-sized grid for
    /// each of the 8 valid symbol configurations.
    @Test func allEightConfigurations() throws {
        let configs: [(MicroQRVersion, MicroQREccLevel, String, Int)] = [
            (.M1, .detection, "12345", 11),
            (.M2, .L, "HELLO", 13),
            (.M2, .M, "HI", 13),
            (.M3, .L, "HELLO WORLD", 15),
            (.M3, .M, "HELLO", 15),
            (.M4, .L, "https://a.b", 17),
            (.M4, .M, "HELLO WORLD", 17),
            (.M4, .Q, "HELLO", 17),
        ]
        for (v, e, input, expectedSize) in configs {
            let g = try encode(input, version: v, ecc: e)
            #expect(g.rows == expectedSize, "config \(v)/\(e) for '\(input)'")
        }
    }
}
