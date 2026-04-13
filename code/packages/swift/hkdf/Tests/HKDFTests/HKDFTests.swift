// ============================================================================
// HKDF Tests — RFC 5869 Test Vectors + Edge Cases
// ============================================================================
//
// These tests verify the HKDF implementation against all three SHA-256 test
// cases from RFC 5869 Appendix A, plus edge cases for error handling.
//
// ============================================================================

import XCTest
import Foundation
@testable import HKDF

final class HKDFTests: XCTestCase {

    // ========================================================================
    // Helper: Convert hex string to Data
    // ========================================================================

    func hexToData(_ hex: String) -> Data {
        var data = Data()
        var index = hex.startIndex
        while index < hex.endIndex {
            let nextIndex = hex.index(index, offsetBy: 2)
            let byteString = hex[index..<nextIndex]
            if let byte = UInt8(byteString, radix: 16) {
                data.append(byte)
            }
            index = nextIndex
        }
        return data
    }

    func dataToHex(_ data: Data) -> String {
        return data.map { String(format: "%02x", $0) }.joined()
    }

    // ========================================================================
    // RFC 5869 Test Case 1: Basic SHA-256
    // ========================================================================

    func testCase1Extract() {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let salt = hexToData("000102030405060708090a0b0c")
        let expectedPRK = "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"

        let prk = hkdfExtract(salt: salt, ikm: ikm, hash: .sha256)
        XCTAssertEqual(dataToHex(prk), expectedPRK)
    }

    func testCase1Expand() throws {
        let prk = hexToData("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
        let info = hexToData("f0f1f2f3f4f5f6f7f8f9")
        let expectedOKM = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

        let okm = try hkdfExpand(prk: prk, info: info, length: 42, hash: .sha256)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }

    func testCase1Combined() throws {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let salt = hexToData("000102030405060708090a0b0c")
        let info = hexToData("f0f1f2f3f4f5f6f7f8f9")
        let expectedOKM = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

        let okm = try hkdf(salt: salt, ikm: ikm, info: info, length: 42, hash: .sha256)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }

    // ========================================================================
    // RFC 5869 Test Case 2: SHA-256 with longer inputs/outputs
    // ========================================================================

    func testCase2Extract() {
        let ikm = hexToData("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f")
        let salt = hexToData("606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf")
        let expectedPRK = "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244"

        let prk = hkdfExtract(salt: salt, ikm: ikm, hash: .sha256)
        XCTAssertEqual(dataToHex(prk), expectedPRK)
    }

    func testCase2Expand() throws {
        let prk = hexToData("06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244")
        let info = hexToData("b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
        let expectedOKM = "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"

        let okm = try hkdfExpand(prk: prk, info: info, length: 82, hash: .sha256)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }

    func testCase2Combined() throws {
        let ikm = hexToData("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f")
        let salt = hexToData("606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf")
        let info = hexToData("b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
        let expectedOKM = "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"

        let okm = try hkdf(salt: salt, ikm: ikm, info: info, length: 82, hash: .sha256)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }

    // ========================================================================
    // RFC 5869 Test Case 3: SHA-256 empty salt and info
    // ========================================================================

    func testCase3Extract() {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let salt = Data()
        let expectedPRK = "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"

        let prk = hkdfExtract(salt: salt, ikm: ikm, hash: .sha256)
        XCTAssertEqual(dataToHex(prk), expectedPRK)
    }

    func testCase3Expand() throws {
        let prk = hexToData("19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04")
        let info = Data()
        let expectedOKM = "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"

        let okm = try hkdfExpand(prk: prk, info: info, length: 42, hash: .sha256)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }

    func testCase3Combined() throws {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let salt = Data()
        let info = Data()
        let expectedOKM = "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"

        let okm = try hkdf(salt: salt, ikm: ikm, info: info, length: 42, hash: .sha256)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    func testExpandRejectsZeroLength() {
        let prk = Data(count: 32)
        XCTAssertThrowsError(try hkdfExpand(prk: prk, info: Data(), length: 0, hash: .sha256))
    }

    func testExpandRejectsNegativeLength() {
        let prk = Data(count: 32)
        XCTAssertThrowsError(try hkdfExpand(prk: prk, info: Data(), length: -1, hash: .sha256))
    }

    func testExpandRejectsLengthExceedingMaxSHA256() {
        let prk = Data(count: 32)
        // Maximum for SHA-256: 255 * 32 = 8160 bytes
        XCTAssertThrowsError(try hkdfExpand(prk: prk, info: Data(), length: 8161, hash: .sha256))
    }

    func testExpandRejectsLengthExceedingMaxSHA512() {
        let prk = Data(count: 64)
        // Maximum for SHA-512: 255 * 64 = 16320 bytes
        XCTAssertThrowsError(try hkdfExpand(prk: prk, info: Data(), length: 16321, hash: .sha512))
    }

    func testSingleByteOutput() throws {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let okm = try hkdf(salt: Data(), ikm: ikm, info: Data(), length: 1, hash: .sha256)
        XCTAssertEqual(okm.count, 1)
        // First byte of test case 3 OKM
        XCTAssertEqual(okm[0], 0x8d)
    }

    func testSHA512ExtractProduces64BytePRK() {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let prk = hkdfExtract(salt: Data(), ikm: ikm, hash: .sha512)
        XCTAssertEqual(prk.count, 64)
    }

    func testDifferentInfoProducesDifferentOutput() throws {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let info1 = "encryption".data(using: .utf8)!
        let info2 = "authentication".data(using: .utf8)!
        let okm1 = try hkdf(salt: Data(), ikm: ikm, info: info1, length: 32, hash: .sha256)
        let okm2 = try hkdf(salt: Data(), ikm: ikm, info: info2, length: 32, hash: .sha256)
        XCTAssertNotEqual(dataToHex(okm1), dataToHex(okm2))
    }

    func testDefaultsToSHA256() throws {
        let ikm = hexToData("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        let salt = hexToData("000102030405060708090a0b0c")
        let info = hexToData("f0f1f2f3f4f5f6f7f8f9")
        let expectedOKM = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

        // Omit hash parameter — should default to sha256
        let okm = try hkdf(salt: salt, ikm: ikm, info: info, length: 42)
        XCTAssertEqual(dataToHex(okm), expectedOKM)
    }
}
