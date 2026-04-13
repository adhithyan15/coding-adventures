import XCTest
@testable import Code128

final class Code128Tests: XCTestCase {
    func testComputeCode128ChecksumMatchesReference() {
        let values = "Code 128".map { valueForCode128BCharacter(String($0)) }
        XCTAssertEqual(computeCode128Checksum(values), 64)
    }

    func testNormalizeRejectsNonPrintableCharacters() {
        XCTAssertThrowsError(try normalizeCode128B("HELLO\n"))
    }

    func testEncodeCode128IncludesStartChecksumAndStop() throws {
        let encoded = try encodeCode128B("Code 128")
        XCTAssertEqual(encoded.first?.role, "start")
        XCTAssertEqual(encoded.dropLast().last?.role, "check")
        XCTAssertEqual(encoded.last?.role, "stop")
    }

    func testLayoutCode128IncludesMetadata() throws {
        let scene = try layoutCode128("Code 128")
        XCTAssertEqual(scene.metadata["symbology"], "code128")
        XCTAssertEqual(scene.metadata["code_set"], "B")
        XCTAssertEqual(scene.height, 120)
        XCTAssertFalse(scene.instructions.isEmpty)
    }
}
