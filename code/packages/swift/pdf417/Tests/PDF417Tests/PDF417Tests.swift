// PDF417Tests.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - PDF417 Test Suite
// ============================================================================
//
// These tests verify every major component of the PDF417 encoder:
//
//   1. Cluster tables    — 3 clusters × 929 entries, all decode to 17 modules.
//   2. Start/stop bits   — exact module patterns match the spec.
//   3. GF(929) tables    — α^0, α^1, α^928 (Fermat), GF_LOG round-trip.
//   4. GF(929) arithmetic — gfAdd, gfMul (including 3 × 310 ≡ 1).
//   5. Byte compaction   — empty, single byte, full 6-byte group, mixed.
//   6. Row indicators    — LRI/RRI follow the cluster-aware formulas.
//   7. Dimension chooser — minimum 3 rows, minimum 1 col, capacity invariant.
//   8. Auto-ECC selector — thresholds at 40, 160, 320, 863 codewords.
//   9. Rasterization     — module-width formula, row repetition, start/stop.
//  10. Integration       — encode strings, all-256-bytes, empty input.
//  11. Determinism       — same input → identical grids.
//  12. Error cases       — invalid ECC, invalid cols, input too long.
//  13. encodeAndLayout   — produces valid PaintScene with > 1 instructions.
//
// Tests are organised into Swift Testing `@Suite`s; each `@Test` has a brief
// comment explaining what property it verifies and why.

import Testing
import Barcode2D
@testable import PDF417

// ============================================================================
// MARK: - Helpers
// ============================================================================

/// Convert one row of a `ModuleGrid` into a "01"-string for easy comparison.
private func rowBits(_ grid: ModuleGrid, row: Int) -> String {
    grid.modules[row].map { $0 ? "1" : "0" }.joined()
}

/// Expected start pattern bits — 17 modules.
private let START_BITS = "11111111010101000"

/// Expected stop pattern bits — 18 modules.
private let STOP_BITS  = "111111101000101001"

// ============================================================================
// MARK: - Cluster tables
// ============================================================================

@Suite("Cluster tables")
struct ClusterTableTests {

    /// PDF417 always has exactly 3 clusters (rows mod 3).
    @Test func threeClusters() {
        #expect(PDF417_CLUSTER_TABLES.count == 3)
    }

    /// Each cluster has 929 entries (one per codeword in GF(929)).
    @Test func eachClusterHas929Entries() {
        for cluster in PDF417_CLUSTER_TABLES {
            #expect(cluster.count == 929)
        }
    }

    /// Every packed entry is a non-zero UInt32 — zero would mean "no widths"
    /// which is never a valid PDF417 codeword pattern.
    @Test func allEntriesNonZero() {
        for cluster in PDF417_CLUSTER_TABLES {
            for entry in cluster {
                #expect(entry > 0)
            }
        }
    }

    /// Spot-check 10 entries from each cluster; each must expand to 17 modules.
    /// Stride 93 covers a representative subset without being exhaustive.
    @Test func clusterEntriesExpandTo17Modules() {
        for ci in 0..<3 {
            for cw in stride(from: 0, to: 929, by: 93) {
                var modules: [Bool] = []
                pdf417ExpandPattern(packed: PDF417_CLUSTER_TABLES[ci][cw], into: &modules)
                #expect(modules.count == 17)
            }
        }
    }

    /// Every codeword in every cluster expands to exactly 17 modules.
    /// Important — symbol width depends on this invariant.
    @Test func everyClusterEntryExpandsTo17Modules() {
        for cluster in PDF417_CLUSTER_TABLES {
            for entry in cluster {
                var modules: [Bool] = []
                pdf417ExpandPattern(packed: entry, into: &modules)
                #expect(modules.count == 17)
            }
        }
    }

    /// Start pattern has 17 modules and matches the canonical bit string.
    @Test func startPatternMatchesSpec() {
        var modules: [Bool] = []
        pdf417ExpandWidths(widths: PDF417_START_PATTERN, into: &modules)
        #expect(modules.count == 17)
        let bits = modules.map { $0 ? "1" : "0" }.joined()
        #expect(bits == START_BITS)
    }

    /// Stop pattern has 18 modules and matches the canonical bit string.
    @Test func stopPatternMatchesSpec() {
        var modules: [Bool] = []
        pdf417ExpandWidths(widths: PDF417_STOP_PATTERN, into: &modules)
        #expect(modules.count == 18)
        let bits = modules.map { $0 ? "1" : "0" }.joined()
        #expect(bits == STOP_BITS)
    }
}

// ============================================================================
// MARK: - GF(929) tables
// ============================================================================

@Suite("GF(929) tables")
struct GF929TableTests {

    /// α^0 = 1 by definition.
    @Test func gfExpZeroIsOne() { #expect(PDF417_GF_EXP[0] == 1) }
    /// α = 3 (the chosen primitive root mod 929).
    @Test func gfExpOneIsAlpha() { #expect(PDF417_GF_EXP[1] == 3) }
    /// α^2 = 9.
    @Test func gfExpTwoIsNine() { #expect(PDF417_GF_EXP[2] == 9) }
    /// α^3 = 27.
    @Test func gfExpThreeIs27() { #expect(PDF417_GF_EXP[3] == 27) }
    /// Fermat's little theorem: α^(p-1) ≡ 1 mod p (since 929 is prime).
    @Test func gfExp928IsOneByFermat() { #expect(PDF417_GF_EXP[928] == 1) }
    /// log_α(1) = 0.
    @Test func gfLogOneIsZero() { #expect(PDF417_GF_LOG[1] == 0) }
    /// log_α(3) = 1.
    @Test func gfLogThreeIsOne() { #expect(PDF417_GF_LOG[3] == 1) }
}

// ============================================================================
// MARK: - GF(929) arithmetic
// ============================================================================

@Suite("GF(929) arithmetic")
struct GF929ArithmeticTests {

    /// Addition wraps at 929: (100 + 900) mod 929 = 71.
    @Test func gfAddWrapsAt929() { #expect(pdf417GFAdd(100, 900) == 71) }
    /// 928 + 1 ≡ 0 (mod 929).
    @Test func gfAddBoundary() { #expect(pdf417GFAdd(928, 1) == 0) }
    /// Identity element: 0 + x = x.
    @Test func gfAddIdentity() { #expect(pdf417GFAdd(0, 500) == 500) }

    /// 3 × 3 = 9 (basic).
    @Test func gfMulThreeTimesThree() { #expect(pdf417GFMul(3, 3) == 9) }
    /// 3 × 310 = 930 ≡ 1 mod 929 — 310 is the multiplicative inverse of 3.
    @Test func gfMulInverseOfThree() { #expect(pdf417GFMul(3, 310) == 1) }
    /// Absorbing element: 0 × anything = 0.
    @Test func gfMulZero() {
        #expect(pdf417GFMul(0, 500) == 0)
        #expect(pdf417GFMul(500, 0) == 0)
    }
    /// Identity element: 1 × x = x.
    @Test func gfMulIdentity() { #expect(pdf417GFMul(1, 928) == 928) }
}

// ============================================================================
// MARK: - Byte compaction
// ============================================================================

@Suite("Byte compaction")
struct ByteCompactionTests {

    /// Empty input still emits the latch codeword 924 (so a scanner knows
    /// what mode the data was in — even if there are no data codewords).
    @Test func emptyInputJustLatch() {
        #expect(pdf417ByteCompact(bytes: []) == [924])
    }

    /// A single byte goes through the "alternate" sub-mode: latch + raw byte.
    @Test func singleByte() {
        let r = pdf417ByteCompact(bytes: [65])
        #expect(r.count == 2)
        #expect(r[0] == 924)
        #expect(r[1] == 65)
    }

    /// 6 bytes form one full group → 5 base-900 codewords.
    /// Computing the round-trip manually against the encoder's output.
    @Test func sixBytesProduceFiveCodewords() {
        let bs: [UInt8] = [0x41, 0x42, 0x43, 0x44, 0x45, 0x46]
        let r = pdf417ByteCompact(bytes: bs)
        #expect(r[0] == 924)
        #expect(r.count == 6)

        // Compute expected via the same algorithm directly.
        var n: UInt64 = 0
        for b in bs { n = n * 256 + UInt64(b) }
        var expected = [Int](repeating: 0, count: 5)
        for j in stride(from: 4, through: 0, by: -1) {
            expected[j] = Int(n % 900)
            n /= 900
        }
        for i in 0..<5 {
            #expect(r[i + 1] == expected[i])
        }
    }

    /// 7 bytes = 1 full group + 1 trailing byte → 5 codewords + 1 raw byte.
    @Test func sevenBytesGroupPlusOne() {
        let r = pdf417ByteCompact(bytes: [65, 66, 67, 68, 69, 70, 71])
        #expect(r[0] == 924)
        #expect(r.count == 7)
        #expect(r[6] == 71)  // last byte encoded directly
    }

    /// 12 bytes = 2 full groups → 10 codewords + latch = 11 total.
    @Test func twelveBytesTwoGroups() {
        let r = pdf417ByteCompact(bytes: [UInt8](repeating: 65, count: 12))
        #expect(r[0] == 924)
        #expect(r.count == 11)
    }
}

// ============================================================================
// MARK: - Row indicators
// ============================================================================

@Suite("Row indicators (LRI / RRI)")
struct RowIndicatorTests {

    // Test parameters: R = 4, C = 3, L = 2.
    // R_info = (4 - 1) / 3 = 1
    // C_info = 3 - 1 = 2
    // L_info = 3 * 2 + (4 - 1) % 3 = 6

    /// Cluster 0 (row 0): LRI = R_info = 1, RRI = C_info = 2.
    @Test func cluster0Row0() {
        #expect(pdf417ComputeLRI(r: 0, rows: 4, cols: 3, eccLevel: 2) == 1)
        #expect(pdf417ComputeRRI(r: 0, rows: 4, cols: 3, eccLevel: 2) == 2)
    }

    /// Cluster 1 (row 1): LRI = L_info = 6, RRI = R_info = 1.
    @Test func cluster1Row1() {
        #expect(pdf417ComputeLRI(r: 1, rows: 4, cols: 3, eccLevel: 2) == 6)
        #expect(pdf417ComputeRRI(r: 1, rows: 4, cols: 3, eccLevel: 2) == 1)
    }

    /// Cluster 2 (row 2): LRI = C_info = 2, RRI = L_info = 6.
    @Test func cluster2Row2() {
        #expect(pdf417ComputeLRI(r: 2, rows: 4, cols: 3, eccLevel: 2) == 2)
        #expect(pdf417ComputeRRI(r: 2, rows: 4, cols: 3, eccLevel: 2) == 6)
    }

    /// Cluster 0 (row 3) row_group = 1 → bias by 30 → LRI = 31, RRI = 32.
    @Test func cluster0Row3WithGroupOffset() {
        #expect(pdf417ComputeLRI(r: 3, rows: 4, cols: 3, eccLevel: 2) == 31)
        #expect(pdf417ComputeRRI(r: 3, rows: 4, cols: 3, eccLevel: 2) == 32)
    }
}

// ============================================================================
// MARK: - Dimension heuristic
// ============================================================================

@Suite("chooseDimensions")
struct DimensionTests {

    /// Always produces ≥ 3 rows even for tiny inputs.
    @Test func minimumRowsIsThree() {
        let (_, rows) = pdf417ChooseDimensions(total: 1)
        #expect(rows >= 3)
    }

    /// Always produces ≥ 1 column.
    @Test func minimumColsIsOne() {
        let (cols, _) = pdf417ChooseDimensions(total: 1)
        #expect(cols >= 1)
    }

    /// Capacity invariant: cols × rows ≥ total for reasonable inputs.
    @Test func capacityInvariantForTypicalInputs() {
        for total in [1, 10, 50, 100, 500] {
            let (cols, rows) = pdf417ChooseDimensions(total: total)
            #expect(cols * rows >= total)
        }
    }
}

// ============================================================================
// MARK: - Auto-ECC selector
// ============================================================================

@Suite("autoEccLevel")
struct AutoEccLevelTests {

    /// ≤ 40 data codewords → level 2 (smallest practical level).
    @Test func smallInputsGetLevel2() {
        #expect(pdf417AutoEccLevel(dataCount: 10) == 2)
        #expect(pdf417AutoEccLevel(dataCount: 40) == 2)
    }

    /// 41–160 → level 3.
    @Test func mediumInputsGetLevel3() {
        #expect(pdf417AutoEccLevel(dataCount: 41) == 3)
        #expect(pdf417AutoEccLevel(dataCount: 160) == 3)
    }

    /// 161–320 → level 4.
    @Test func largeInputsGetLevel4() {
        #expect(pdf417AutoEccLevel(dataCount: 161) == 4)
        #expect(pdf417AutoEccLevel(dataCount: 320) == 4)
    }

    /// 321–863 → level 5; > 863 → level 6.
    @Test func veryLargeInputs() {
        #expect(pdf417AutoEccLevel(dataCount: 500) == 5)
        #expect(pdf417AutoEccLevel(dataCount: 1000) == 6)
    }
}

// ============================================================================
// MARK: - Module width formula
// ============================================================================

@Suite("Symbol dimensions")
struct SymbolDimensionTests {

    /// Module width formula: width = 69 + 17 × cols.
    /// Verified across a broad range of column counts.
    @Test func moduleWidthFormulaHolds() throws {
        for c in [1, 3, 5, 10, 30] {
            let g = try encode(
                "HELLO WORLD HELLO WORLD",
                options: PDF417Options(columns: c, rowHeight: 1)
            )
            #expect(g.cols == 69 + 17 * c)
        }
    }

    /// Even tiny inputs get the spec-mandated minimum of 3 logical rows.
    @Test func minimumLogicalRowsIsThree() throws {
        let g = try encode("A", options: PDF417Options(rowHeight: 1))
        #expect(g.rows >= 3)
    }

    /// Doubling rowHeight doubles the pixel-grid height.
    @Test func rowHeightScalesGridHeight() throws {
        let g3 = try encode("A", options: PDF417Options(rowHeight: 3))
        let g6 = try encode("A", options: PDF417Options(rowHeight: 6))
        #expect(g6.rows == g3.rows * 2)
    }
}

// ============================================================================
// MARK: - Start / stop pattern in every row
// ============================================================================

@Suite("Start / stop patterns")
struct StartStopRowTests {

    /// Every module row begins with the canonical start pattern (17 modules).
    @Test func everyRowStartsWithStartPattern() throws {
        let g = try encode("TEST", options: PDF417Options(columns: 3, rowHeight: 1))
        for r in 0..<g.rows {
            let bits = rowBits(g, row: r)
            #expect(String(bits.prefix(17)) == START_BITS)
        }
    }

    /// Every module row ends with the canonical stop pattern (18 modules).
    @Test func everyRowEndsWithStopPattern() throws {
        let g = try encode("TEST", options: PDF417Options(columns: 3, rowHeight: 1))
        for r in 0..<g.rows {
            let bits = rowBits(g, row: r)
            #expect(String(bits.suffix(18)) == STOP_BITS)
        }
    }
}

// ============================================================================
// MARK: - Integration tests
// ============================================================================

@Suite("encode() integration")
struct EncodeIntegrationTests {

    /// A single byte produces a valid grid with at least the minimum dimensions.
    @Test func singleByteEncodesSuccessfully() throws {
        let g = try encode("A")
        #expect(g.rows >= 3)
        #expect(g.cols >= 69 + 17)
    }

    /// "HELLO WORLD" produces correct start/stop in every row.
    @Test func helloWorldRowStructure() throws {
        let g = try encode("HELLO WORLD", options: PDF417Options(rowHeight: 1))
        for r in 0..<g.rows {
            let bits = rowBits(g, row: r)
            #expect(String(bits.prefix(17)) == START_BITS)
            #expect(String(bits.suffix(18)) == STOP_BITS)
        }
    }

    /// All 256 byte values encode without error — exercises every byte value
    /// through the byte-compaction path.
    @Test func all256ByteValuesEncode() throws {
        let bytes = (0..<256).map { UInt8($0) }
        let g = try encode(bytes: bytes)
        #expect(g.rows >= 3)
    }

    /// Pathological: 256 copies of 0xFF — exercises the maximum-byte path
    /// through the base-900 conversion (UInt64 must hold 0xFFFFFFFFFFFF).
    @Test func repeated0xFFEncodes() throws {
        let bytes = [UInt8](repeating: 0xff, count: 256)
        let g = try encode(bytes: bytes)
        #expect(g.rows >= 3)
    }

    /// Empty input still encodes (just the latch codeword fills one slot).
    @Test func emptyInputEncodes() throws {
        let g = try encode(bytes: [])
        #expect(g.rows >= 3)
    }

    /// Determinism: encoding the same input twice yields identical grids.
    /// Important for cross-language comparison and reproducible builds.
    @Test func deterministicSameInputSameGrid() throws {
        let s = "PDF417 TEST"
        let g1 = try encode(s)
        let g2 = try encode(s)
        #expect(g1.rows == g2.rows)
        #expect(g1.cols == g2.cols)
        for r in 0..<g1.rows {
            for c in 0..<g1.cols {
                #expect(g1.modules[r][c] == g2.modules[r][c])
            }
        }
    }

    /// Different inputs produce DIFFERENT grids — not a true correctness
    /// check but catches mass data-bus disconnections.
    @Test func differentInputsDifferentGrids() throws {
        let g1 = try encode("AAA", options: PDF417Options(rowHeight: 1))
        let g2 = try encode("BBB", options: PDF417Options(rowHeight: 1))
        var differ = false
        for r in 0..<min(g1.rows, g2.rows) where !differ {
            for c in 0..<min(g1.cols, g2.cols) where !differ {
                if g1.modules[r][c] != g2.modules[r][c] {
                    differ = true
                }
            }
        }
        #expect(differ)
    }

    /// Each logical row repeats `rowHeight` times in the rasterized grid.
    @Test func rowsRepeatRowHeightTimes() throws {
        let rowHeight = 4
        let g = try encode(
            "HELLO",
            options: PDF417Options(columns: 3, rowHeight: rowHeight)
        )
        let logicalRows = g.rows / rowHeight
        for lr in 0..<logicalRows {
            for h in 1..<rowHeight {
                for c in 0..<g.cols {
                    #expect(g.modules[lr * rowHeight][c] == g.modules[lr * rowHeight + h][c])
                }
            }
        }
    }

    /// Higher ECC level yields an equal-or-larger symbol (more codewords).
    @Test func higherEccProducesLargerSymbol() throws {
        let g2 = try encode("HELLO WORLD", options: PDF417Options(eccLevel: 2))
        let g4 = try encode("HELLO WORLD", options: PDF417Options(eccLevel: 4))
        #expect(g4.rows * g4.cols >= g2.rows * g2.cols)
    }

    /// ECC level 0 (the smallest valid) is accepted.
    @Test func eccLevel0Accepted() throws {
        _ = try encode("A", options: PDF417Options(eccLevel: 0))
    }

    /// ECC level 8 (the largest valid) is accepted.
    @Test func eccLevel8Accepted() throws {
        _ = try encode("A", options: PDF417Options(eccLevel: 8))
    }

    /// Output module shape is `.square` — PDF417 never uses hex modules.
    @Test func outputShapeIsSquare() throws {
        let g = try encode("A")
        #expect(g.moduleShape == .square)
    }
}

// ============================================================================
// MARK: - Error cases
// ============================================================================

@Suite("Error cases")
struct ErrorTests {

    /// ECC level 9 exceeds the spec maximum.
    @Test func eccLevel9Throws() {
        #expect(throws: PDF417Error.self) {
            _ = try encode("A", options: PDF417Options(eccLevel: 9))
        }
    }

    /// Negative ECC level is invalid.
    @Test func eccLevelNegativeThrows() {
        #expect(throws: PDF417Error.self) {
            _ = try encode("A", options: PDF417Options(eccLevel: -1))
        }
    }

    /// columns = 0 violates the 1–30 range.
    @Test func columnsZeroThrows() {
        #expect(throws: PDF417Error.self) {
            _ = try encode("A", options: PDF417Options(columns: 0))
        }
    }

    /// columns = 31 exceeds the 1–30 range.
    @Test func columnsThirtyOneThrows() {
        #expect(throws: PDF417Error.self) {
            _ = try encode("A", options: PDF417Options(columns: 31))
        }
    }

    /// 3000 bytes squeezed into 1 column requires more than 90 rows.
    @Test func inputTooLongForOneColumnThrows() {
        let huge = [UInt8](repeating: 65, count: 3000)
        #expect(throws: PDF417Error.self) {
            _ = try encode(bytes: huge, options: PDF417Options(columns: 1))
        }
    }
}

// ============================================================================
// MARK: - encodeAndLayout
// ============================================================================

@Suite("encodeAndLayout")
struct LayoutTests {

    /// Returns a PaintScene with positive dimensions.
    @Test func returnsPositiveDimensions() throws {
        let scene = try encodeAndLayout("HELLO")
        #expect(scene.width > 0)
        #expect(scene.height > 0)
    }

    /// Scene contains at least the background rect plus one dark module rect.
    @Test func hasMultipleInstructions() throws {
        let scene = try encodeAndLayout("HELLO")
        #expect(scene.instructions.count > 1)
    }
}
