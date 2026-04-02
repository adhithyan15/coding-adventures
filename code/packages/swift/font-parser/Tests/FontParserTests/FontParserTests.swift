import XCTest
@testable import FontParser

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the font fixture path relative to the test file's source location.
/// #filePath gives the compile-time path of this file, so this works for both
/// `swift test` and Xcode test runs.
private func interFontURL() -> URL {
    // This file lives at:
    //   code/packages/swift/font-parser/Tests/FontParserTests/FontParserTests.swift
    // We need to climb up to the package root and then find the fixture:
    //   code/fixtures/fonts/Inter-Regular.ttf
    let thisFile = URL(fileURLWithPath: #filePath)
    return thisFile
        .deletingLastPathComponent()  // FontParserTests/
        .deletingLastPathComponent()  // Tests/
        .deletingLastPathComponent()  // font-parser/ (package root)
        .deletingLastPathComponent()  // swift/
        .deletingLastPathComponent()  // packages/
        .deletingLastPathComponent()  // code/ ← now we're in the code/ dir
        .appendingPathComponent("fixtures/fonts/Inter-Regular.ttf")
}

private func interBytes() throws -> Data {
    try Data(contentsOf: interFontURL())
}

/// Build a minimal valid OpenType binary with a kern Format 0 table.
/// `pairs` is an array of `(left, right, value)` tuples.
private func buildSyntheticFont(pairs: [(UInt16, UInt16, Int16)] = []) -> Data {
    func w16(_ v: UInt16) -> [UInt8] { [UInt8(v >> 8), UInt8(v & 0xFF)] }
    func wi16(_ v: Int16) -> [UInt8] { w16(UInt16(bitPattern: v)) }
    func w32(_ v: UInt32) -> [UInt8] {
        [UInt8(v >> 24), UInt8((v >> 16) & 0xFF), UInt8((v >> 8) & 0xFF), UInt8(v & 0xFF)]
    }
    func tag(_ s: String) -> [UInt8] {
        let b = Array(s.utf8)
        return b + Array(repeating: 0, count: max(0, 4 - b.count))
    }

    let numTables = 6
    let dirSize   = 12 + numTables * 16

    let headLen = 54; let hheaLen = 36; let maxpLen = 6
    let cmapLen = 36; let hmtxLen = 5 * 4
    let nPairs  = pairs.count
    let kernLen = 4 + 6 + 8 + nPairs * 6

    let headOff = dirSize
    let hheaOff = headOff + headLen
    let maxpOff = hheaOff + hheaLen
    let cmapOff = maxpOff + maxpLen
    let hmtxOff = cmapOff + cmapLen
    let kernOff = hmtxOff + hmtxLen

    var buf = [UInt8]()

    // Offset table
    buf += w32(0x00010000) + w16(UInt16(numTables)) + w16(64) + w16(2) + w16(32)

    // Table records (sorted: cmap < head < hhea < hmtx < kern < maxp)
    for (t, off, len) in [
        (tag("cmap"), cmapOff, cmapLen), (tag("head"), headOff, headLen),
        (tag("hhea"), hheaOff, hheaLen), (tag("hmtx"), hmtxOff, hmtxLen),
        (tag("kern"), kernOff, kernLen), (tag("maxp"), maxpOff, maxpLen)
    ] {
        buf += t + w32(0) + w32(UInt32(off)) + w32(UInt32(len))
    }

    // head (54 bytes)
    buf += w32(0x00010000) + w32(0x00010000) + w32(0) + w32(0x5F0F3CF5)
    buf += w16(0) + w16(1000) + Array(repeating: 0, count: 16)
    buf += wi16(0) + wi16(0) + wi16(0) + wi16(0)  // xMin yMin xMax yMax
    buf += w16(0) + w16(8) + wi16(2) + wi16(0) + wi16(0)

    // hhea (36 bytes)
    buf += w32(0x00010000) + wi16(800) + wi16(-200) + wi16(0)
    buf += w16(1000) + wi16(0) + wi16(0) + wi16(0)
    buf += wi16(1) + wi16(0) + wi16(0) + Array(repeating: 0, count: 8)
    buf += wi16(0) + w16(5)

    // maxp (6 bytes)
    buf += w32(0x00005000) + w16(5)

    // cmap (36 bytes)
    buf += w16(0) + w16(1) + w16(3) + w16(1) + w32(12)
    buf += w16(4) + w16(24) + w16(0) + w16(2) + w16(2) + w16(0) + w16(0)
    buf += w16(0xFFFF) + w16(0) + w16(0xFFFF) + wi16(1) + w16(0)

    // hmtx: 5 records {600, 50}
    for _ in 0 ..< 5 { buf += w16(600) + wi16(50) }

    // kern table
    let subLen = 6 + 8 + nPairs * 6
    let sorted = pairs.sorted { Int($0.0) * 65536 + Int($0.1) < Int($1.0) * 65536 + Int($1.1) }
    buf += w16(0) + w16(1)
    buf += w16(0) + w16(UInt16(subLen)) + w16(0x0001)
    buf += w16(UInt16(nPairs)) + w16(0) + w16(0) + w16(0)
    for (l, r, v) in sorted { buf += w16(l) + w16(r) + wi16(v) }

    return Data(buf)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

final class FontParserTests: XCTestCase {

    // MARK: load

    func testEmptyDataRaisesBufferTooShort() {
        XCTAssertThrowsError(try FontParser.load(Data())) { err in
            XCTAssertEqual(err as? FontError, FontError.bufferTooShort)
        }
    }

    func testWrongMagicRaisesInvalidMagic() {
        var buf = Data(repeating: 0, count: 256)
        buf[0] = 0xDE; buf[1] = 0xAD; buf[2] = 0xBE; buf[3] = 0xEF
        XCTAssertThrowsError(try FontParser.load(buf)) { err in
            XCTAssertEqual(err as? FontError, FontError.invalidMagic)
        }
    }

    func testNoTablesRaisesTableNotFound() {
        // sfntVersion=0x00010000, numTables=0 → head table will be missing.
        var buf = Data(count: 12)
        buf[0] = 0x00; buf[1] = 0x01; buf[2] = 0x00; buf[3] = 0x00
        XCTAssertThrowsError(try FontParser.load(buf)) { err in
            if case FontError.tableNotFound = err { } else {
                XCTFail("Expected tableNotFound, got \(err)")
            }
        }
    }

    func testLoadInterRegularSucceeds() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNotNil(font)
    }

    func testLoadSyntheticFontSucceeds() throws {
        let font = try FontParser.load(buildSyntheticFont(pairs: [(1, 2, -140)]))
        XCTAssertNotNil(font)
    }

    // MARK: fontMetrics

    func testUnitsPerEmIs2048() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertEqual(fontMetrics(font).unitsPerEm, 2048)
    }

    func testFamilyNameIsInter() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertEqual(fontMetrics(font).familyName, "Inter")
    }

    func testSubfamilyNameIsRegular() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertEqual(fontMetrics(font).subfamilyName, "Regular")
    }

    func testAscenderIsPositive() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertGreaterThan(fontMetrics(font).ascender, 0)
    }

    func testDescenderIsNonPositive() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertLessThanOrEqual(fontMetrics(font).descender, 0)
    }

    func testNumGlyphsIsLarge() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertGreaterThan(fontMetrics(font).numGlyphs, 100)
    }

    func testXHeightIsPositive() throws {
        let font = try FontParser.load(try interBytes())
        let m = fontMetrics(font)
        XCTAssertNotNil(m.xHeight)
        XCTAssertGreaterThan(m.xHeight!, 0)
    }

    func testCapHeightIsPositive() throws {
        let font = try FontParser.load(try interBytes())
        let m = fontMetrics(font)
        XCTAssertNotNil(m.capHeight)
        XCTAssertGreaterThan(m.capHeight!, 0)
    }

    func testSyntheticFontUnitsPerEm1000() throws {
        let font = try FontParser.load(buildSyntheticFont())
        XCTAssertEqual(fontMetrics(font).unitsPerEm, 1000)
    }

    func testSyntheticFontFamilyNameUnknown() throws {
        let font = try FontParser.load(buildSyntheticFont())
        XCTAssertEqual(fontMetrics(font).familyName, "(unknown)")
    }

    // MARK: glyphId

    func testGlyphIdForAIsNotNil() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNotNil(glyphId(font, codepoint: 0x0041))
    }

    func testGlyphIdForVIsNotNil() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNotNil(glyphId(font, codepoint: 0x0056))
    }

    func testGlyphIdForSpaceIsNotNil() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNotNil(glyphId(font, codepoint: 0x0020))
    }

    func testGlyphIdsForAAndVDiffer() throws {
        let font = try FontParser.load(try interBytes())
        let gidA = glyphId(font, codepoint: 0x0041)
        let gidV = glyphId(font, codepoint: 0x0056)
        XCTAssertNotNil(gidA); XCTAssertNotNil(gidV)
        XCTAssertNotEqual(gidA, gidV)
    }

    func testCodepointAboveBMPReturnsNil() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNil(glyphId(font, codepoint: 0x10000))
    }

    func testNegativeCodepointReturnsNil() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNil(glyphId(font, codepoint: -1))
    }

    // MARK: glyphMetrics

    func testAdvanceWidthForAIsPositive() throws {
        let font = try FontParser.load(try interBytes())
        let gid  = glyphId(font, codepoint: 0x0041)!
        let gm   = glyphMetrics(font, glyphId: Int(gid))
        XCTAssertNotNil(gm)
        XCTAssertGreaterThan(gm!.advanceWidth, 0)
    }

    func testAdvanceWidthInReasonableRange() throws {
        let font = try FontParser.load(try interBytes())
        let gid  = glyphId(font, codepoint: 0x0041)!
        let gm   = glyphMetrics(font, glyphId: Int(gid))!
        XCTAssertTrue(gm.advanceWidth >= 100 && gm.advanceWidth <= 2400)
    }

    func testOutOfRangeGlyphReturnsNil() throws {
        let font = try FontParser.load(try interBytes())
        let ng   = Int(fontMetrics(font).numGlyphs)
        XCTAssertNil(glyphMetrics(font, glyphId: ng))
    }

    func testNegativeGlyphIdReturnsNil() throws {
        let font = try FontParser.load(try interBytes())
        XCTAssertNil(glyphMetrics(font, glyphId: -1))
    }

    // MARK: kerning

    func testInterAVReturnsZeroGPOSFont() throws {
        let font = try FontParser.load(try interBytes())
        let gidA = Int(glyphId(font, codepoint: 0x0041)!)
        let gidV = Int(glyphId(font, codepoint: 0x0056)!)
        XCTAssertEqual(kerning(font, left: gidA, right: gidV), 0)
    }

    func testSyntheticPair12ReturnsNeg140() throws {
        let font = try FontParser.load(buildSyntheticFont(pairs: [(1, 2, -140), (3, 4, 80)]))
        XCTAssertEqual(kerning(font, left: 1, right: 2), -140)
    }

    func testSyntheticPair34Returns80() throws {
        let font = try FontParser.load(buildSyntheticFont(pairs: [(1, 2, -140), (3, 4, 80)]))
        XCTAssertEqual(kerning(font, left: 3, right: 4), 80)
    }

    func testAbsentPairReturnsZero() throws {
        let font = try FontParser.load(buildSyntheticFont(pairs: [(1, 2, -140), (3, 4, 80)]))
        XCTAssertEqual(kerning(font, left: 1, right: 4), 0)
    }

    func testReversedPairReturnsZero() throws {
        let font = try FontParser.load(buildSyntheticFont(pairs: [(1, 2, -140)]))
        XCTAssertEqual(kerning(font, left: 2, right: 1), 0)
    }
}
