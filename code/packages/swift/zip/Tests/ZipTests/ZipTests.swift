// ZipTests.swift — CMP09 ZIP package tests (TC-1 through TC-12).
//
// Each test case exercises a distinct capability of the ZIP writer/reader.

import XCTest
@testable import Zip

final class ZipTests: XCTestCase {

    // ── CRC-32 known vectors ─────────────────────────────────────────────────

    func testCRC32Empty() {
        XCTAssertEqual(crc32([]), 0x0000_0000)
    }

    func testCRC32HelloWorld() {
        let data = Array("hello world".utf8)
        XCTAssertEqual(crc32(data), 0x0D4A_1185)
    }

    func testCRC32Incremental() {
        let data = Array("hello world".utf8)
        let half = data.count / 2
        let c1 = crc32(Array(data[..<half]))
        let c2 = crc32(Array(data[half...]), initial: c1)
        XCTAssertEqual(c2, crc32(data))
    }

    // ── DOS datetime ─────────────────────────────────────────────────────────

    func testDosEpochConstant() {
        XCTAssertEqual(dosEpoch, 0x0021_0000)
    }

    func testDosDatetimeMidnight() {
        XCTAssertEqual(dosDatetime(year: 1980, month: 1, day: 1) & 0xFFFF, 0)
    }

    func testDosDatetimeDateField() {
        XCTAssertEqual((dosDatetime(year: 1980, month: 1, day: 1) >> 16) & 0xFFFF, 33)
    }

    // ── TC-1: Single file, Stored (no compression) ───────────────────────────

    func testTC1RoundTripStored() throws {
        let data = Array("Hello, ZIP!".utf8)
        var w = ZipWriter()
        w.addFile("hello.txt", data: data, compress: false)
        let archive = w.finish()
        let files = try unzip(archive)
        XCTAssertEqual(files["hello.txt"], data)
    }

    func testTC1StoredMethod() throws {
        var w = ZipWriter()
        w.addFile("a.txt", data: Array("abc".utf8), compress: false)
        let archive = w.finish()
        let reader = try ZipReader(archive)
        XCTAssertEqual(reader.entries().first?.method, 0)
    }

    // ── TC-2: Single file, DEFLATE ────────────────────────────────────────────

    func testTC2RoundTripDeflate() throws {
        let text = String(repeating: "ABCABCABCABCABC", count: 100)
        let data = Array(text.utf8)
        let archive = zip([("rep.txt", data)], compress: true)
        let files = try unzip(archive)
        XCTAssertEqual(files["rep.txt"], data)
    }

    func testTC2DeflateShrinksRepetitiveData() throws {
        let data = Array(String(repeating: "x", count: 1000).utf8)
        let archive = zip([("x.txt", data)], compress: true)
        let reader = try ZipReader(archive)
        let entry = reader.entries().first!
        XCTAssertLessThan(entry.compressedSize, entry.size)
        XCTAssertEqual(entry.method, 8)
    }

    // ── TC-3: Multiple files ──────────────────────────────────────────────────

    func testTC3MultipleFilesRoundTrip() throws {
        let entries: [(name: String, data: [UInt8])] = [
            ("a.txt", Array("alpha".utf8)),
            ("b.txt", Array("beta".utf8)),
            ("c.txt", Array("gamma".utf8)),
        ]
        let archive = zip(entries)
        let files = try unzip(archive)
        XCTAssertEqual(files["a.txt"], Array("alpha".utf8))
        XCTAssertEqual(files["b.txt"], Array("beta".utf8))
        XCTAssertEqual(files["c.txt"], Array("gamma".utf8))
    }

    func testTC3EntryCount() throws {
        let archive = zip([("one.txt", Array("1".utf8)), ("two.txt", Array("2".utf8))])
        let reader = try ZipReader(archive)
        XCTAssertEqual(reader.entries().count, 2)
    }

    // ── TC-4: Directory entry ─────────────────────────────────────────────────

    func testTC4DirectoryEntry() throws {
        var w = ZipWriter()
        w.addDirectory("mydir/")
        let archive = w.finish()
        let reader = try ZipReader(archive)
        let dirEntry = reader.entries().first(where: { $0.name == "mydir/" })
        XCTAssertNotNil(dirEntry)
        XCTAssertTrue(dirEntry!.isDirectory)
    }

    func testTC4ReadDirectoryReturnsEmpty() throws {
        var w = ZipWriter()
        w.addDirectory("dir/")
        let archive = w.finish()
        let reader = try ZipReader(archive)
        let entry = reader.entries().first(where: { $0.name == "dir/" })!
        let result = try reader.read(entry)
        XCTAssertEqual(result, [])
    }

    // ── TC-5: CRC-32 mismatch ─────────────────────────────────────────────────

    func testTC5CRCMismatchThrows() throws {
        let data = Array("important data".utf8)
        var w = ZipWriter()
        w.addFile("file.txt", data: data, compress: false)
        var archive = w.finish()

        // Corrupt a byte in the file data (after the 30 + name_len local header).
        let lhNameLen = Int(archive[26]) | (Int(archive[27]) << 8)
        let dataStart = 30 + lhNameLen
        archive[dataStart] ^= 0xFF

        let reader = try ZipReader(archive)
        let entry = reader.entries()[0]
        XCTAssertThrowsError(try reader.read(entry)) { error in
            if case ZipError.crcMismatch(let msg) = error {
                XCTAssertTrue(msg.contains("CRC-32 mismatch"))
            } else {
                XCTFail("Expected CRC mismatch error, got \(error)")
            }
        }
    }

    // ── TC-6: Random-access read ──────────────────────────────────────────────

    func testTC6RandomAccessRead() throws {
        var entries = [(name: String, data: [UInt8])]()
        for i in 0..<10 {
            entries.append(("f\(i).txt", Array("content of f\(i)".utf8)))
        }
        let archive = zip(entries)
        let reader = try ZipReader(archive)
        let result = try reader.readByName("f5.txt")
        XCTAssertEqual(String(bytes: result, encoding: .utf8), "content of f5")
    }

    // ── TC-7: Incompressible data → Stored ────────────────────────────────────

    func testTC7IncompressibleStoredAsMethod0() throws {
        // 256 distinct bytes — DEFLATE will expand, so zip falls back to Stored.
        var data = [UInt8](repeating: 0, count: 256)
        for i in 0..<256 { data[i] = UInt8(i) }
        let archive = zip([("rand.bin", data)], compress: true)
        let reader = try ZipReader(archive)
        let entry = reader.entries()[0]
        XCTAssertEqual(entry.method, 0)
        let result = try reader.read(entry)
        XCTAssertEqual(result, data)
    }

    // ── TC-8: Empty file ──────────────────────────────────────────────────────

    func testTC8EmptyFileRoundTrip() throws {
        let archive = zip([("empty.txt", [])])
        let files = try unzip(archive)
        XCTAssertEqual(files["empty.txt"], [])
    }

    func testTC8EmptyFileSize() throws {
        let archive = zip([("e.txt", [])])
        let reader = try ZipReader(archive)
        let entry = reader.entries()[0]
        XCTAssertEqual(entry.size, 0)
        XCTAssertEqual(entry.compressedSize, 0)
    }

    // ── TC-9: Large file ──────────────────────────────────────────────────────

    func testTC9LargeFileRoundTrip() throws {
        var data = [UInt8](repeating: 0, count: 100_000)
        for i in 0..<data.count { data[i] = UInt8(i % 26 + 65) } // A-Z repeating
        let archive = zip([("big.bin", data)], compress: true)
        let files = try unzip(archive)
        XCTAssertEqual(files["big.bin"], data)
    }

    func testTC9LargeFileCompressesSignificantly() throws {
        let data = [UInt8](repeating: 65, count: 10_000)
        let archive = zip([("same.bin", data)], compress: true)
        let reader = try ZipReader(archive)
        let entry = reader.entries()[0]
        XCTAssertLessThan(Int(entry.compressedSize), data.count / 4)
    }

    // ── TC-10: Unicode filename ───────────────────────────────────────────────

    func testTC10UnicodeFilename() throws {
        let name = "日本語/résumé.txt"
        let data = Array("unicode content".utf8)
        let archive = zip([(name, data)])
        let reader = try ZipReader(archive)
        let entry = reader.entries().first(where: { $0.name == name })
        XCTAssertNotNil(entry)
        let result = try reader.readByName(name)
        XCTAssertEqual(result, data)
    }

    // ── TC-11: Nested paths ───────────────────────────────────────────────────

    func testTC11NestedPaths() throws {
        let entries: [(name: String, data: [UInt8])] = [
            ("a/b/c.txt", Array("deep".utf8)),
            ("a/b/d.txt", Array("also deep".utf8)),
        ]
        let archive = zip(entries)
        let files = try unzip(archive)
        XCTAssertEqual(files["a/b/c.txt"], Array("deep".utf8))
        XCTAssertEqual(files["a/b/d.txt"], Array("also deep".utf8))
    }

    // ── TC-12: Empty archive ──────────────────────────────────────────────────

    func testTC12EmptyArchive() throws {
        var w = ZipWriter()
        let archive = w.finish()
        let reader = try ZipReader(archive)
        XCTAssertEqual(reader.entries().count, 0)
    }

    func testTC12EmptyUnzip() throws {
        var w = ZipWriter()
        let archive = w.finish()
        let files = try unzip(archive)
        XCTAssertTrue(files.isEmpty)
    }

    // ── readByName ────────────────────────────────────────────────────────────

    func testReadByNameFound() throws {
        let archive = zip([("x.txt", Array("xray".utf8))])
        let reader = try ZipReader(archive)
        let result = try reader.readByName("x.txt")
        XCTAssertEqual(String(bytes: result, encoding: .utf8), "xray")
    }

    func testReadByNameNotFound() throws {
        let archive = zip([("x.txt", Array("xray".utf8))])
        let reader = try ZipReader(archive)
        XCTAssertThrowsError(try reader.readByName("missing.txt")) { error in
            if case ZipError.notFound = error { } else {
                XCTFail("Expected notFound, got \(error)")
            }
        }
    }

    // ── Error paths ───────────────────────────────────────────────────────────

    func testMalformedNotAZip() {
        XCTAssertThrowsError(try ZipReader(Array("not a zip file".utf8))) { error in
            if case ZipError.malformed = error { } else {
                XCTFail("Expected malformed, got \(error)")
            }
        }
    }

    func testMalformedTooShort() {
        XCTAssertThrowsError(try ZipReader([])) { error in
            if case ZipError.malformed = error { } else {
                XCTFail("Expected malformed, got \(error)")
            }
        }
    }

    // ── Security: path traversal rejection ───────────────────────────────────

    func testPathTraversalDotDotRejected() throws {
        // Build an archive manually with a ".." segment in the entry name.
        var w = ZipWriter()
        w.addFile("safe.txt", data: Array("ok".utf8))
        let archive = w.finish()

        // Patch the name "safe.txt" (8 bytes) to "../../x\0" style won't work
        // through ZipWriter since we control names.  Instead, craft raw bytes:
        // Build a minimal archive with a traversal name directly.
        let maliciousName = Array("../../evil.txt".utf8)
        var crafted = [UInt8]()
        // Local File Header
        func le16(_ v: UInt16) -> [UInt8] { [UInt8(v & 0xFF), UInt8((v >> 8) & 0xFF)] }
        func le32(_ v: UInt32) -> [UInt8] {
            [UInt8(v & 0xFF), UInt8((v >> 8) & 0xFF),
             UInt8((v >> 16) & 0xFF), UInt8((v >> 24) & 0xFF)]
        }
        crafted += le32(0x04034B50)
        crafted += le16(10); crafted += le16(0x0800); crafted += le16(0)
        crafted += le16(0); crafted += le16(0x0021)
        crafted += le32(0); crafted += le32(0); crafted += le32(0)
        crafted += le16(UInt16(maliciousName.count)); crafted += le16(0)
        crafted += maliciousName
        // CD entry
        let cdOffset = UInt32(crafted.count)
        crafted += le32(0x02014B50); crafted += le16(0x031E); crafted += le16(10)
        crafted += le16(0x0800); crafted += le16(0)
        crafted += le16(0x0021); crafted += le16(0)
        crafted += le32(0); crafted += le32(0); crafted += le32(0)
        crafted += le16(UInt16(maliciousName.count)); crafted += le16(0); crafted += le16(0)
        crafted += le16(0); crafted += le16(0); crafted += le32(0); crafted += le32(0)
        crafted += maliciousName
        let cdSize = UInt32(crafted.count) - cdOffset
        // EOCD
        crafted += le32(0x06054B50); crafted += le16(0); crafted += le16(0)
        crafted += le16(1); crafted += le16(1)
        crafted += le32(cdSize); crafted += le32(cdOffset); crafted += le16(0)

        XCTAssertThrowsError(try ZipReader(crafted)) { error in
            if case ZipError.malformed(let msg) = error {
                XCTAssertTrue(msg.contains("traversal") || msg.contains(".."),
                              "Expected path traversal error, got: \(msg)")
            } else {
                XCTFail("Expected malformed error, got \(error)")
            }
        }
        _ = archive // suppress warning
    }

    func testAbsolutePathRejected() throws {
        let maliciousName = Array("/etc/passwd".utf8)
        var crafted = [UInt8]()
        func le16(_ v: UInt16) -> [UInt8] { [UInt8(v & 0xFF), UInt8((v >> 8) & 0xFF)] }
        func le32(_ v: UInt32) -> [UInt8] {
            [UInt8(v & 0xFF), UInt8((v >> 8) & 0xFF),
             UInt8((v >> 16) & 0xFF), UInt8((v >> 24) & 0xFF)]
        }
        crafted += le32(0x04034B50)
        crafted += le16(10); crafted += le16(0x0800); crafted += le16(0)
        crafted += le16(0); crafted += le16(0x0021)
        crafted += le32(0); crafted += le32(0); crafted += le32(0)
        crafted += le16(UInt16(maliciousName.count)); crafted += le16(0)
        crafted += maliciousName
        let cdOffset = UInt32(crafted.count)
        crafted += le32(0x02014B50); crafted += le16(0x031E); crafted += le16(10)
        crafted += le16(0x0800); crafted += le16(0)
        crafted += le16(0x0021); crafted += le16(0)
        crafted += le32(0); crafted += le32(0); crafted += le32(0)
        crafted += le16(UInt16(maliciousName.count)); crafted += le16(0); crafted += le16(0)
        crafted += le16(0); crafted += le16(0); crafted += le32(0); crafted += le32(0)
        crafted += maliciousName
        let cdSize = UInt32(crafted.count) - cdOffset
        crafted += le32(0x06054B50); crafted += le16(0); crafted += le16(0)
        crafted += le16(1); crafted += le16(1)
        crafted += le32(cdSize); crafted += le32(cdOffset); crafted += le16(0)

        XCTAssertThrowsError(try ZipReader(crafted)) { error in
            if case ZipError.malformed(let msg) = error {
                XCTAssertTrue(msg.contains("absolute"), "Expected absolute path error, got: \(msg)")
            } else {
                XCTFail("Expected malformed error, got \(error)")
            }
        }
    }

    func testDuplicateEntryNameRejected() throws {
        var w = ZipWriter()
        w.addFile("dup.txt", data: Array("first".utf8), compress: false)
        w.addFile("dup.txt", data: Array("second".utf8), compress: false)
        let archive = w.finish()
        XCTAssertThrowsError(try unzip(archive)) { error in
            if case ZipError.malformed(let msg) = error {
                XCTAssertTrue(msg.contains("duplicate"), "Expected duplicate error, got: \(msg)")
            } else {
                XCTFail("Expected malformed error, got \(error)")
            }
        }
    }
}
