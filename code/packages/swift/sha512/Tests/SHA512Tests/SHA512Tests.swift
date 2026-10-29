// SHA512Tests.swift
// Tests for the SHA-512 hash function implementation (FIPS 180-4).

import XCTest
import Foundation
@testable import SHA512

final class SHA512Tests: XCTestCase {

    private enum FixtureError: Error { case invalid }

    private func isLowerHex(_ value: String, count: Int? = nil) -> Bool {
        (count == nil || value.utf8.count == count!) && value.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
    }

    private func isID(_ value: String) -> Bool {
        !value.isEmpty && value.split(separator: "-", omittingEmptySubsequences: false).allSatisfy { part in
            !part.isEmpty && part.utf8.allSatisfy { (48...57).contains($0) || (97...122).contains($0) }
        }
    }

    private func parseFixture(_ data: Data) throws -> [String] {
        guard (1...4096).contains(data.count), let text = String(data: data, encoding: .utf8) else { throw FixtureError.invalid }
        let lines = text.split(separator: "\n", omittingEmptySubsequences: true).map {
            let line = String($0)
            return line.hasSuffix("\r") ? String(line.dropLast()) : line
        }
            .filter { !$0.hasPrefix("#") }
        guard (1...64).contains(lines.count) else { throw FixtureError.invalid }
        for line in lines {
            let fields = line.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
            guard fields.count == 5, isID(fields[1]) else { throw FixtureError.invalid }
            switch fields[0] {
            case "V":
                guard fields[2].utf8.count <= 8192, fields[2].utf8.count % 2 == 0, isLowerHex(fields[2]), isLowerHex(fields[4], count: 128) else { throw FixtureError.invalid }
                let byteCount = fields[2].utf8.count / 2
                guard byteCount <= 4096, (byteCount == 0) == fields[3].isEmpty else { throw FixtureError.invalid }
                let chunks: [Int]
                if fields[3].isEmpty { chunks = [] }
                else {
                    guard fields[3].utf8.allSatisfy({ (48...57).contains($0) || $0 == 44 }) else { throw FixtureError.invalid }
                    chunks = try fields[3].split(separator: ",", omittingEmptySubsequences: false).map {
                        guard !$0.isEmpty, $0.first != "0", let size = Int($0), (1...4096).contains(size) else { throw FixtureError.invalid }
                        return size
                    }
                }
                guard chunks.count <= 64, chunks.reduce(0, +) == byteCount else { throw FixtureError.invalid }
            case "L":
                guard isLowerHex(fields[2], count: 16), isLowerHex(fields[3], count: 16),
                    fields[4] == "ERROR" || isLowerHex(fields[4], count: 32) else { throw FixtureError.invalid }
            default: throw FixtureError.invalid
            }
        }
        return lines
    }

    private func fixtureLines(at suppliedURL: URL? = nil) throws -> [String] {
        let url = suppliedURL ?? URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .appendingPathComponent("../../../../../specs/fixtures/sha512-v1/cases.txt")
            .standardizedFileURL
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        let data = try handle.read(upToCount: 4097) ?? Data()
        let lines = try parseFixture(data)
        XCTAssertEqual(lines.filter { $0.hasPrefix("V|") }.count, 9)
        XCTAssertEqual(lines.filter { $0.hasPrefix("L|") }.count, 6)
        return lines
    }

    func testFixtureReaderRejectsMalformedRows() {
        let digest = String(repeating: "0", count: 128)
        for row in ["X|id|||", "V|UPPER|00|1|\(digest)", "V|bad|0|1|\(digest)", "V|bad|00|2|\(digest)",
                    "L|bad|000000000000000G|0000000000000000|ERROR", "L|bad|0000000000000000|0000000000000000|ABC"] {
            XCTAssertThrowsError(try parseFixture(Data(row.utf8)))
        }
    }

    func testFixtureReaderRejectsOversizedFile() throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("sha512-fixture-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: url) }
        try Data(repeating: 0, count: 4097).write(to: url)
        XCTAssertThrowsError(try fixtureLines(at: url))
    }

    private func decodeHex(_ value: String) -> Data {
        precondition(value.count % 2 == 0)
        var result = Data()
        var index = value.startIndex
        while index < value.endIndex {
            let end = value.index(index, offsetBy: 2)
            result.append(UInt8(value[index..<end], radix: 16)!)
            index = end
        }
        return result
    }

    func testPortableCorpus() throws {
        for line in try fixtureLines().filter({ $0.hasPrefix("V|") }) {
            let fields = line.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
            let data = decodeHex(fields[2])
            XCTAssertEqual(sha512Hex(data), fields[4], fields[1])
            var hasher = SHA512Hasher()
            var offset = 0
            let chunks = fields[3].isEmpty ? [] : fields[3].split(separator: ",").map { Int($0)! }
            XCTAssertLessThanOrEqual(chunks.count, 64)
            for size in chunks {
                XCTAssertGreaterThan(size, 0)
                XCTAssertLessThanOrEqual(offset + size, data.count)
                hasher.update(data[offset..<offset + size])
                offset += size
            }
            XCTAssertEqual(offset, data.count)
            XCTAssertEqual(hasher.hexDigest(), fields[4], fields[1])
        }
    }

    func testPortableLengthCorpus() throws {
        for line in try fixtureLines().filter({ $0.hasPrefix("L|") }) {
            let fields = line.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
            let high = UInt64(fields[2], radix: 16)!
            let low = UInt64(fields[3], radix: 16)!
            if fields[4] == "ERROR" {
                XCTAssertThrowsError(try encodeBitLength(byteCountHigh: high, byteCountLow: low))
            } else {
                let encoded = try encodeBitLength(byteCountHigh: high, byteCountLow: low)
                    .map { String(format: "%02x", $0) }.joined()
                XCTAssertEqual(encoded, fields[4])
            }
        }
    }

    func testUpdateConsumesInputWithoutAliasing() {
        var input = Data("abc".utf8)
        var hasher = SHA512Hasher()
        hasher.update(input)
        input[0] = 0x7a
        XCTAssertEqual(hasher.digest(), sha512(Data("abc".utf8)))
    }

    // ========================================================================
    // One-Shot sha512() -- FIPS 180-4 Test Vectors
    // ========================================================================

    func testEmptyString() {
        XCTAssertEqual(
            sha512Hex(Data()),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce"
            + "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        )
    }

    func testABC() {
        XCTAssertEqual(
            sha512Hex(Data("abc".utf8)),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
            + "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        )
    }

    func testTwoBlockVector() {
        // FIPS 180-4 two-block test vector
        let input = "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
        XCTAssertEqual(
            sha512Hex(Data(input.utf8)),
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018"
            + "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
        )
    }

    // ========================================================================
    // One-Shot sha512() -- Additional Vectors
    // ========================================================================

    func testSingleA() {
        XCTAssertEqual(
            sha512Hex(Data("a".utf8)),
            "1f40fc92da241694750979ee6cf582f2d5d7d28e18335de05abc54d0560e0f53"
            + "02860c652bf08d560252aa5e74210546f369fbbbce8c12cfc7957b2652fe9a75"
        )
    }

    func testHello() {
        XCTAssertEqual(
            sha512Hex(Data("hello".utf8)),
            "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7"
            + "2323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043"
        )
    }

    func testPangram() {
        XCTAssertEqual(
            sha512Hex(Data("The quick brown fox jumps over the lazy dog".utf8)),
            "07e547d9586f6a73f73fbac0435ed76951218fb7d0c8d788a309d785436bbb64"
            + "2e93a252a954f23912547d1e8a3b5ed6e1bfd7097821233fa0538f3db854fee6"
        )
    }

    func testSingleZeroByte() {
        XCTAssertEqual(sha512(Data([0x00])).count, 64)
    }

    // ========================================================================
    // Edge Cases: Block Boundary Tests
    // ========================================================================

    func testExact111Bytes() {
        // 111 bytes + 1 (0x80) + 0 padding + 16 (length) = 128 bytes = 1 block
        let data = Data(repeating: 0x61, count: 111)
        let hex = sha512Hex(data)
        XCTAssertEqual(hex.count, 128)
    }

    func testExact112Bytes() {
        // 112 bytes + 1 (0x80) = 113 > 112, so padding spills to next block
        let data = Data(repeating: 0x61, count: 112)
        let hex = sha512Hex(data)
        XCTAssertEqual(hex.count, 128)
    }

    func testExact128Bytes() {
        let data = Data(repeating: 0x61, count: 128)
        let hex = sha512Hex(data)
        XCTAssertEqual(hex.count, 128)
    }

    func test127Bytes() {
        let data = Data(repeating: 0x61, count: 127)
        let hex = sha512Hex(data)
        XCTAssertEqual(hex.count, 128)
    }

    func testBinaryData() {
        let data = Data((0..<256).map { UInt8($0) })
        let hex = sha512Hex(data)
        XCTAssertEqual(hex.count, 128)
    }

    // ========================================================================
    // Return Type and Property Tests
    // ========================================================================

    func testReturnTypeIs64Bytes() {
        let digest = sha512(Data("abc".utf8))
        XCTAssertEqual(digest.count, 64)
    }

    func testDeterministic() {
        let data = Data("hello world".utf8)
        XCTAssertEqual(sha512(data), sha512(data))
    }

    func testAvalanche() {
        XCTAssertNotEqual(sha512(Data("a".utf8)), sha512(Data("b".utf8)))
    }

    // ========================================================================
    // sha512Hex() Tests
    // ========================================================================

    func testHexLength128() {
        let hex = sha512Hex(Data("abc".utf8))
        XCTAssertEqual(hex.count, 128)
    }

    func testHexLowercase() {
        let hex = sha512Hex(Data("abc".utf8))
        XCTAssertEqual(hex, hex.lowercased())
    }

    func testHexMatchesDigest() {
        let data = Data("test".utf8)
        let digest = sha512(data)
        let hex = sha512Hex(data)
        let expected = digest.map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(hex, expected)
    }

    // ========================================================================
    // Streaming SHA512Hasher Tests
    // ========================================================================

    func testStreamingSingleUpdateEqualsOneShot() {
        var hasher = SHA512Hasher()
        hasher.update(Data("abc".utf8))
        XCTAssertEqual(hasher.hexDigest(), sha512Hex(Data("abc".utf8)))
    }

    func testStreamingSplitAtByteBoundary() {
        var hasher = SHA512Hasher()
        hasher.update(Data("ab".utf8))
        hasher.update(Data("c".utf8))
        XCTAssertEqual(
            hasher.hexDigest(),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
            + "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        )
    }

    func testStreamingSplitAtBlockBoundary() {
        let data = Data(repeating: 0x61, count: 256)
        let chunk1 = data.prefix(128)
        let chunk2 = data.suffix(128)

        var hasher = SHA512Hasher()
        hasher.update(chunk1)
        hasher.update(chunk2)
        XCTAssertEqual(hasher.hexDigest(), sha512Hex(data))
    }

    func testStreamingManyTinyUpdates() {
        let message = "abcdefghijklmnopqrstuvwxyz"
        var hasher = SHA512Hasher()
        for byte in message.utf8 {
            hasher.update(Data([byte]))
        }
        XCTAssertEqual(hasher.hexDigest(), sha512Hex(Data(message.utf8)))
    }

    func testDigestNonDestructive() {
        var hasher = SHA512Hasher()
        hasher.update(Data("abc".utf8))
        let digest1 = hasher.hexDigest()
        let digest2 = hasher.hexDigest()
        XCTAssertEqual(digest1, digest2)
    }

    func testContinueAfterDigest() {
        var hasher = SHA512Hasher()
        hasher.update(Data("abc".utf8))
        _ = hasher.hexDigest()
        hasher.update(Data("def".utf8))
        XCTAssertEqual(hasher.hexDigest(), sha512Hex(Data("abcdef".utf8)))
    }

    func testCopyIsIndependent() {
        var original = SHA512Hasher()
        original.update(Data("abc".utf8))
        var copied = original.copy()

        copied.update(Data("def".utf8))

        XCTAssertEqual(original.hexDigest(), sha512Hex(Data("abc".utf8)))
        XCTAssertEqual(copied.hexDigest(), sha512Hex(Data("abcdef".utf8)))
    }

    func testEmptyStreaming() {
        let hasher = SHA512Hasher()
        XCTAssertEqual(hasher.hexDigest(), sha512Hex(Data()))
    }

    func testStreamingLargeInput() {
        let fullData = Data(repeating: 0x42, count: 1000)
        var hasher = SHA512Hasher()
        var offset = 0
        while offset < fullData.count {
            let end = min(offset + 100, fullData.count)
            hasher.update(fullData[offset..<end])
            offset = end
        }
        XCTAssertEqual(hasher.hexDigest(), sha512Hex(fullData))
    }
}
