// ============================================================================
// CasTests.swift — Unit Tests for Content-Addressable Storage
// ============================================================================
//
// These tests cover every public API surface of the `Cas` package:
//
//   · Hex utility functions (keyToHex, hexToKey, decodeHexPrefix)
//   · CasError description strings
//   · BlobStore protocol conformance using an in-memory store
//   · ContentAddressableStore round-trips, integrity, prefix lookup
//   · LocalDiskStore path layout, atomic writes, prefix scanning
//
// Coverage target: > 90 %.
//
// Why XCTest?
// ───────────
// XCTest is Swift's standard test framework, available on all Apple platforms
// and on Linux via swift-corelibs-xctest. No extra dependencies needed.
// ============================================================================

import XCTest
import Foundation
@testable import ContentAddressableStorage

// ============================================================================
// MARK: - Helpers
// ============================================================================

/// Create a fresh temporary directory for each test.
///
/// Each call generates a unique subdirectory under the system temp dir using
/// a UUID, so parallel tests never collide.
private func tempDir() throws -> URL {
    let base = FileManager.default.temporaryDirectory
    let dir  = base.appendingPathComponent("cas-test-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    return dir
}

/// Remove a temporary directory after a test, ignoring errors.
private func cleanUp(_ url: URL) {
    try? FileManager.default.removeItem(at: url)
}

// ============================================================================
// MARK: - InMemoryStore
// ============================================================================
//
// A trivial `BlobStore` conformance backed by a dictionary. Useful for:
//   1. Testing the protocol contract without filesystem I/O.
//   2. Demonstrating how to implement a custom backend.

/// Simple in-memory `BlobStore` using a `[String: [UInt8]]` dictionary.
///
/// Keys are stored as their 40-char hex representation to allow easy prefix
/// scanning.
struct InMemoryStore: BlobStore {
    // The backing dictionary. Keys are 40-char hex strings.
    private var storage: [String: [UInt8]] = [:]

    mutating func put(key: [UInt8], data: [UInt8]) throws {
        let hexKey = keyToHex(key)
        storage[hexKey] = data
    }

    func get(key: [UInt8]) throws -> [UInt8] {
        let hexKey = keyToHex(key)
        guard let data = storage[hexKey] else {
            // Mimic the NSError that LocalDiskStore throws on missing file.
            throw NSError(domain: NSCocoaErrorDomain, code: NSFileReadNoSuchFileError,
                          userInfo: [NSLocalizedDescriptionKey: "not found"])
        }
        return data
    }

    func exists(key: [UInt8]) throws -> Bool {
        storage[keyToHex(key)] != nil
    }

    func keysWithPrefix(_ prefix: [UInt8]) throws -> [[UInt8]] {
        guard !prefix.isEmpty else { return [] }
        // Build the hex prefix string to compare against stored keys.
        let hexPrefix = keyToHex(prefix)
        return storage.keys
            .filter { $0.hasPrefix(hexPrefix) }
            .compactMap { hexToKey($0) }
    }
}

// ============================================================================
// MARK: - Hex Utility Tests
// ============================================================================

final class HexUtilityTests: XCTestCase {

    // keyToHex: known 20-byte value → expected 40-char string
    func testKeyToHexKnownValue() {
        let key: [UInt8] = [
            0xa3, 0xf4, 0xb2, 0xc1, 0xd0,
            0xe9, 0xf8, 0xa7, 0xb6, 0xc5,
            0xd4, 0xe3, 0xf2, 0xa1, 0xb0,
            0xc9, 0xd8, 0xe7, 0xf6, 0xa5,
        ]
        XCTAssertEqual(keyToHex(key), "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5")
    }

    // keyToHex must always produce exactly 40 characters.
    func testKeyToHexLength() {
        let key = [UInt8](repeating: 0xff, count: 20)
        XCTAssertEqual(keyToHex(key).count, 40)
    }

    // hexToKey → keyToHex round-trip must recover the original value.
    func testHexToKeyRoundTrip() {
        let hex = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
        let key = hexToKey(hex)!
        XCTAssertEqual(keyToHex(key), hex)
    }

    // hexToKey must return nil for a string that is too short.
    func testHexToKeyRejectsShort() {
        XCTAssertNil(hexToKey("a3f4"))
    }

    // hexToKey must return nil for a string that is too long.
    func testHexToKeyRejectsLong() {
        XCTAssertNil(hexToKey(String(repeating: "a", count: 42)))
    }

    // hexToKey must return nil for non-hex characters.
    func testHexToKeyRejectsNonHex() {
        let bad = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7fgzz"
        XCTAssertNil(hexToKey(bad))
    }

    // hexToKey must accept uppercase hex.
    func testHexToKeyAcceptsUppercase() {
        let lower = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
        let upper = "A3F4B2C1D0E9F8A7B6C5D4E3F2A1B0C9D8E7F6A5"
        XCTAssertEqual(hexToKey(lower), hexToKey(upper))
    }

    // decodeHexPrefix: empty string must return nil.
    func testDecodeHexPrefixRejectsEmpty() {
        XCTAssertNil(decodeHexPrefix(""))
    }

    // decodeHexPrefix: odd-length string is right-padded with '0'.
    // "a3f" → [0xa3, 0xf0]
    func testDecodeHexPrefixOddLength() {
        let result = decodeHexPrefix("a3f")
        XCTAssertEqual(result, [0xa3, 0xf0])
    }

    // decodeHexPrefix: even-length string.
    func testDecodeHexPrefixEvenLength() {
        XCTAssertEqual(decodeHexPrefix("a3f4"), [0xa3, 0xf4])
    }

    // decodeHexPrefix: non-hex char must return nil.
    func testDecodeHexPrefixRejectsNonHex() {
        XCTAssertNil(decodeHexPrefix("a3gz"))
    }

    // decodeHexPrefix: single nibble "a" → [0xa0]
    func testDecodeHexPrefixSingleNibble() {
        XCTAssertEqual(decodeHexPrefix("a"), [0xa0])
    }
}

// ============================================================================
// MARK: - CasError Tests
// ============================================================================

final class CasErrorTests: XCTestCase {

    func testNotFoundDescription() {
        let key = [UInt8](repeating: 0, count: 20)
        let err = CasError.notFound(key)
        XCTAssert(err.description.contains("not found"))
    }

    func testCorruptedDescription() {
        let key = [UInt8](repeating: 0xab, count: 20)
        let err = CasError.corrupted(key)
        XCTAssert(err.description.contains("corrupted"))
    }

    func testAmbiguousPrefixDescription() {
        let err = CasError.ambiguousPrefix("a3f")
        XCTAssert(err.description.contains("ambiguous"))
        XCTAssert(err.description.contains("a3f"))
    }

    func testPrefixNotFoundDescription() {
        let err = CasError.prefixNotFound("deadbeef")
        XCTAssert(err.description.contains("not found"))
        XCTAssert(err.description.contains("deadbeef"))
    }

    func testInvalidPrefixDescription() {
        let err = CasError.invalidPrefix("xyz!")
        XCTAssert(err.description.contains("invalid"))
    }

    func testStoreErrorDescription() {
        let inner = NSError(domain: "test", code: 42,
                            userInfo: [NSLocalizedDescriptionKey: "disk full"])
        let err = CasError.storeError(inner)
        XCTAssert(err.description.contains("store error"))
    }
}

// ============================================================================
// MARK: - InMemoryStore Tests (protocol conformance)
// ============================================================================
//
// These tests exercise the CAS layer against the in-memory backend. They prove
// that any correct `BlobStore` implementation enables full CAS semantics.

final class InMemoryStoreTests: XCTestCase {

    // put → get round-trip for a small blob.
    func testRoundTripSmall() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let data: [UInt8] = Array("hello, world".utf8)
        let key = try cas.put(data: data)
        let got = try cas.get(key: key)
        XCTAssertEqual(got, data)
    }

    // put → get round-trip for the empty blob.
    //
    // SHA-1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
    // This is a well-known value; verifying it confirms we use the right hash.
    func testRoundTripEmpty() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let key = try cas.put(data: [])
        let got = try cas.get(key: key)
        XCTAssertEqual(got, [])
        // Check the known SHA-1 of the empty string.
        XCTAssertEqual(keyToHex(key), "da39a3ee5e6b4b0d3255bfef95601890afd80709")
    }

    // put → get round-trip for a large blob (1 MiB).
    func testRoundTripLarge() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let data = [UInt8](repeating: 0x42, count: 1024 * 1024) // 1 MiB of 'B'
        let key = try cas.put(data: data)
        let got = try cas.get(key: key)
        XCTAssertEqual(got, data)
    }

    // Calling put twice with the same data returns the same key; no error.
    func testIdempotentPut() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let data: [UInt8] = Array("idempotent".utf8)
        let key1 = try cas.put(data: data)
        let key2 = try cas.put(data: data)
        XCTAssertEqual(key1, key2)
    }

    // Requesting a key that was never stored → CasError.notFound.
    func testGetUnknownKeyThrowsNotFound() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let key = [UInt8](repeating: 0xde, count: 20)
        do {
            _ = try cas.get(key: key)
            XCTFail("expected CasError.notFound")
        } catch CasError.notFound(let k) {
            XCTAssertEqual(k, key)
        }
    }

    // exists returns false before a put, true after.
    func testExistsBeforeAndAfterPut() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let data: [UInt8] = Array("exists-test".utf8)

        // SHA-1 of "exists-test" so we can check before writing.
        // We compute the key via put, but check exists after the fact.
        let key = try cas.put(data: data)
        XCTAssertTrue(try cas.exists(key: key))

        // A random key that was never stored.
        let phantom = [UInt8](repeating: 0xcc, count: 20)
        XCTAssertFalse(try cas.exists(key: phantom))
    }

    // findByPrefix: unique match returns the key.
    func testFindByPrefixUniqueMatch() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        let key = try cas.put(data: Array("prefix-unique".utf8))
        let hex = keyToHex(key)
        // Use the first 8 characters (4 bytes) as the prefix.
        let found = try cas.findByPrefix(String(hex.prefix(8)))
        XCTAssertEqual(found, key)
    }

    // findByPrefix: ambiguous prefix → CasError.ambiguousPrefix.
    func testFindByPrefixAmbiguous() throws {
        // Craft two keys that share the prefix "a3f4" (first two bytes = 0xa3, 0xf4).
        // We insert them directly into an InMemoryStore so we control the keys
        // precisely, then wrap the store in a CAS for the prefix lookup.
        let key1: [UInt8] = [0xa3, 0xf4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
        let key2: [UInt8] = [0xa3, 0xf4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]

        var store = InMemoryStore()
        try store.put(key: key1, data: [0x01])
        try store.put(key: key2, data: [0x02])
        let cas = ContentAddressableStore(store: store)

        do {
            _ = try cas.findByPrefix("a3f4")
            XCTFail("expected CasError.ambiguousPrefix")
        } catch CasError.ambiguousPrefix(let p) {
            XCTAssertEqual(p, "a3f4")
        }
    }

    // findByPrefix: no match → CasError.prefixNotFound.
    func testFindByPrefixNotFound() throws {
        let cas = ContentAddressableStore(store: InMemoryStore())
        do {
            _ = try cas.findByPrefix("deadbeef")
            XCTFail("expected CasError.prefixNotFound")
        } catch CasError.prefixNotFound(let p) {
            XCTAssertEqual(p, "deadbeef")
        }
    }

    // findByPrefix: empty string → CasError.invalidPrefix.
    func testFindByPrefixEmptyString() throws {
        let cas = ContentAddressableStore(store: InMemoryStore())
        do {
            _ = try cas.findByPrefix("")
            XCTFail("expected CasError.invalidPrefix")
        } catch CasError.invalidPrefix(let p) {
            XCTAssertEqual(p, "")
        }
    }

    // findByPrefix: invalid hex characters → CasError.invalidPrefix.
    func testFindByPrefixInvalidHex() throws {
        let cas = ContentAddressableStore(store: InMemoryStore())
        do {
            _ = try cas.findByPrefix("xyz!")
            XCTFail("expected CasError.invalidPrefix")
        } catch CasError.invalidPrefix(let p) {
            XCTAssertEqual(p, "xyz!")
        }
    }

    // inner property gives access to the underlying store.
    func testInnerAccess() throws {
        var cas = ContentAddressableStore(store: InMemoryStore())
        _ = try cas.put(data: Array("inner".utf8))
        // We just verify that `inner` is accessible (type check is enough).
        let _: InMemoryStore = cas.inner
    }
}

// ============================================================================
// MARK: - LocalDiskStore Tests
// ============================================================================

final class LocalDiskStoreTests: XCTestCase {

    // put → get round-trip using the real filesystem.
    func testRoundTrip() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let data: [UInt8] = Array("disk round-trip".utf8)
        let key = try cas.put(data: data)
        XCTAssertEqual(try cas.get(key: key), data)
    }

    // put → get for the empty blob.
    func testRoundTripEmpty() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let key = try cas.put(data: [])
        XCTAssertEqual(try cas.get(key: key), [])
        XCTAssertEqual(keyToHex(key), "da39a3ee5e6b4b0d3255bfef95601890afd80709")
    }

    // put → get for a large blob (1 MiB).
    func testRoundTripLarge() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let data = [UInt8](repeating: 0x55, count: 1024 * 1024)
        let key  = try cas.put(data: data)
        XCTAssertEqual(try cas.get(key: key), data)
    }

    // Calling put twice returns the same key; no error.
    func testIdempotentPut() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let data: [UInt8] = Array("idempotent-disk".utf8)
        let k1 = try cas.put(data: data)
        let k2 = try cas.put(data: data)
        XCTAssertEqual(k1, k2)
    }

    // Requesting an unknown key → CasError.notFound.
    func testGetUnknownKeyThrowsNotFound() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        let cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let key = [UInt8](repeating: 0xab, count: 20)
        do {
            _ = try cas.get(key: key)
            XCTFail("expected CasError.notFound")
        } catch CasError.notFound(let k) {
            XCTAssertEqual(k, key)
        }
    }

    // exists returns false before a put, true after.
    func testExistsBeforeAndAfterPut() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let data: [UInt8] = Array("exists-disk".utf8)
        let key = try cas.put(data: data)
        XCTAssertTrue(try cas.exists(key: key))

        let phantom = [UInt8](repeating: 0x77, count: 20)
        XCTAssertFalse(try cas.exists(key: phantom))
    }

    // Verify the 2/38 fanout path layout.
    //
    // Given a key whose first byte is 0xda (SHA-1 of ""), we expect:
    //   dir  = <root>/da/
    //   file = <root>/da/39a3ee5e6b4b0d3255bfef95601890afd80709
    func testPathLayout() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        // SHA-1 of "" = da39a3ee5e6b4b0d3255bfef95601890afd80709
        let key = try cas.put(data: [])

        let hex      = keyToHex(key)   // "da39a3ee…"
        let bucketDir = hex.prefix(2)  // "da"
        let fileName  = String(hex.dropFirst(2))  // "39a3ee…" (38 chars)

        let expectedPath = dir
            .appendingPathComponent(String(bucketDir))
            .appendingPathComponent(fileName)

        XCTAssertTrue(FileManager.default.fileExists(atPath: expectedPath.path),
                      "Expected file at \(expectedPath.path)")
        XCTAssertEqual(fileName.count, 38)
    }

    // Mutating a stored file should cause get() to throw CasError.corrupted.
    func testCorruptedFileThrowsCorrupted() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let data: [UInt8] = Array("corrupt me".utf8)
        let key = try cas.put(data: data)

        // Locate the file and overwrite it with different bytes.
        let hex  = keyToHex(key)
        let path = dir
            .appendingPathComponent(String(hex.prefix(2)))
            .appendingPathComponent(String(hex.dropFirst(2)))

        try Data([0xff, 0xfe, 0xfd]).write(to: path)

        do {
            _ = try cas.get(key: key)
            XCTFail("expected CasError.corrupted")
        } catch CasError.corrupted(let k) {
            XCTAssertEqual(k, key)
        }
    }

    // findByPrefix: unique match on disk.
    func testFindByPrefixUniqueMatchDisk() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let key = try cas.put(data: Array("prefix-disk-unique".utf8))
        let hex = keyToHex(key)
        let found = try cas.findByPrefix(String(hex.prefix(10)))
        XCTAssertEqual(found, key)
    }

    // findByPrefix with the full 40-char hex also works.
    func testFindByPrefixFullHex() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        var cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        let key = try cas.put(data: Array("full-hex".utf8))
        let found = try cas.findByPrefix(keyToHex(key))
        XCTAssertEqual(found, key)
    }

    // findByPrefix: not found on disk.
    func testFindByPrefixNotFoundDisk() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        let cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        do {
            _ = try cas.findByPrefix("aabbccdd")
            XCTFail("expected CasError.prefixNotFound")
        } catch CasError.prefixNotFound(_) {
            // expected
        }
    }

    // findByPrefix: ambiguous on disk.
    //
    // We store two objects whose first two bytes (bucket) are identical by
    // directly writing crafted files into the store's directory layout, then
    // verify that CAS reports an ambiguous match.
    func testFindByPrefixAmbiguousDisk() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        // Create the "a3" bucket directory and plant two 38-char filename stubs.
        let bucketDir = dir.appendingPathComponent("a3")
        try FileManager.default.createDirectory(at: bucketDir, withIntermediateDirectories: true)

        let file1 = bucketDir.appendingPathComponent(
            "f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
        )
        let file2 = bucketDir.appendingPathComponent(
            "f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a6"
        )
        try Data([0x01]).write(to: file1)
        try Data([0x02]).write(to: file2)

        let cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        do {
            _ = try cas.findByPrefix("a3f4b2")
            XCTFail("expected CasError.ambiguousPrefix")
        } catch CasError.ambiguousPrefix(let p) {
            XCTAssertEqual(p, "a3f4b2")
        }
    }

    // findByPrefix: invalid hex string.
    func testFindByPrefixInvalidHexDisk() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        let cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        do {
            _ = try cas.findByPrefix("xyz!")
            XCTFail("expected CasError.invalidPrefix")
        } catch CasError.invalidPrefix(_) {
            // expected
        }
    }

    // findByPrefix: empty string.
    func testFindByPrefixEmptyStringDisk() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        let cas = ContentAddressableStore(store: try LocalDiskStore(root: dir))
        do {
            _ = try cas.findByPrefix("")
            XCTFail("expected CasError.invalidPrefix")
        } catch CasError.invalidPrefix(_) {
            // expected
        }
    }

    // LocalDiskStore.init creates the root directory when it doesn't exist.
    func testInitCreatesRootDirectory() throws {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("cas-init-test-\(UUID().uuidString)")
        defer { cleanUp(dir) }

        // The directory must NOT exist before init.
        XCTAssertFalse(FileManager.default.fileExists(atPath: dir.path))
        _ = try LocalDiskStore(root: dir)
        XCTAssertTrue(FileManager.default.fileExists(atPath: dir.path))
    }

    // LocalDiskStore.exists (non-throwing overload) returns false gracefully.
    func testExistsNonThrowingVariant() throws {
        let dir = try tempDir()
        defer { cleanUp(dir) }

        let store = try LocalDiskStore(root: dir)
        let phantom = [UInt8](repeating: 0x11, count: 20)
        XCTAssertFalse(store.exists(key: phantom))
    }
}
