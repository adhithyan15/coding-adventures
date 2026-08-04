import XCTest
@testable import Sha256Native

/// Exercises the Rust SHA-256 through the C ABI, asserting FIPS 180-4 answers
/// and streaming parity with the one-shot path.
final class Sha256NativeTests: XCTestCase {

    private func bytes(_ s: String) -> [UInt8] { Array(s.utf8) }

    func testFipsVectors() {
        XCTAssertEqual(Sha256Native.hexString(bytes("")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        XCTAssertEqual(Sha256Native.hexString(bytes("abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        XCTAssertEqual(
            Sha256Native.hexString(bytes("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
    }

    func testMillionA() {
        let data = [UInt8](repeating: 0x61, count: 1_000_000)
        XCTAssertEqual(Sha256Native.hexString(data),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0")
    }

    func testDigestIs32Bytes() {
        XCTAssertEqual(Sha256Native.digest(bytes("")).count, 32)
        XCTAssertEqual(Sha256Native.digest(bytes("hello world")).count, 32)
    }

    func testBlockBoundariesDistinct() {
        var seen = Set<String>()
        for n in [55, 56, 63, 64, 127, 128] {
            let d = Sha256Native.digest([UInt8](repeating: 0, count: n))
            XCTAssertEqual(d.count, 32)
            seen.insert(Sha256Native.hex(d))
        }
        XCTAssertEqual(seen.count, 6)
    }

    func testStreamingMatchesOneShot() {
        let h = Sha256Native.Hasher()
        h.update(bytes("ab"))
        h.update(bytes("c"))
        XCTAssertEqual(h.digest(), Sha256Native.digest(bytes("abc")))
    }

    func testStreamingByteAtATime() {
        let data = (0..<100).map { UInt8($0) }
        let h = Sha256Native.Hasher()
        for b in data { h.update([b]) }
        XCTAssertEqual(h.digest(), Sha256Native.digest(data))
    }

    func testStreamingEmptyAndNonDestructive() {
        XCTAssertEqual(Sha256Native.Hasher().digest(), Sha256Native.digest(bytes("")))
        let h = Sha256Native.Hasher()
        h.update(bytes("abc"))
        XCTAssertEqual(h.digest(), h.digest())
        h.update(bytes("d"))
        XCTAssertEqual(h.digest(), Sha256Native.digest(bytes("abcd")))
    }

    func testCopyIsIndependent() {
        let h = Sha256Native.Hasher()
        h.update(bytes("ab"))
        let h2 = h.copy()
        h2.update(bytes("c"))
        h.update(bytes("x"))
        XCTAssertEqual(h2.digest(), Sha256Native.digest(bytes("abc")))
        XCTAssertEqual(h.digest(), Sha256Native.digest(bytes("abx")))
    }

    func testHexDigestMatches() {
        let h = Sha256Native.Hasher()
        h.update(bytes("abc"))
        XCTAssertEqual(h.hexDigest(), Sha256Native.hexString(bytes("abc")))
    }
}
