import XCTest
@testable import Md5Native

final class Md5NativeTests: XCTestCase {
    private func b(_ s: String) -> [UInt8] { Array(s.utf8) }

    func testRfcVectors() {
        XCTAssertEqual(Md5Native.hexString(b("")), "d41d8cd98f00b204e9800998ecf8427e")
        XCTAssertEqual(Md5Native.hexString(b("a")), "0cc175b9c0f1b6a831c399e269772661")
        XCTAssertEqual(Md5Native.hexString(b("abc")), "900150983cd24fb0d6963f7d28e17f72")
        XCTAssertEqual(Md5Native.hexString(b("message digest")), "f96b697d7cb7938d525a2f31aaf161d0")
    }
    func testMillionA() {
        let data = [UInt8](repeating: 0x61, count: 1_000_000)
        XCTAssertEqual(Md5Native.hexString(data), "7707d6ae4e027c70eea2a935c2296f21")
    }
    func testBytes0To255() {
        let data = (0...255).map { UInt8($0) }
        XCTAssertEqual(Md5Native.hexString(data), "e2c865db4162bed963bfaa9ef6ac18f0")
    }
    func testDigestIs16Bytes() {
        XCTAssertEqual(Md5Native.digest(b("")).count, 16)
        XCTAssertEqual(Md5Native.digest(b("hello world")).count, 16)
    }
    func testStreamingMatchesOneShot() {
        let h = Md5Native.Hasher(); h.update(b("ab")); h.update(b("c"))
        XCTAssertEqual(h.digest(), Md5Native.digest(b("abc")))
    }
    func testStreamingByteAtATime() {
        let data = (0..<100).map { UInt8($0) }
        let h = Md5Native.Hasher(); for x in data { h.update([x]) }
        XCTAssertEqual(h.digest(), Md5Native.digest(data))
    }
    func testStreamingEmptyAndNonDestructive() {
        XCTAssertEqual(Md5Native.Hasher().digest(), Md5Native.digest(b("")))
        let h = Md5Native.Hasher(); h.update(b("abc"))
        XCTAssertEqual(h.digest(), h.digest())
        h.update(b("d"))
        XCTAssertEqual(h.digest(), Md5Native.digest(b("abcd")))
    }
    func testCopyIsIndependent() {
        let h = Md5Native.Hasher(); h.update(b("ab"))
        let h2 = h.copy(); h2.update(b("c")); h.update(b("x"))
        XCTAssertEqual(h2.digest(), Md5Native.digest(b("abc")))
        XCTAssertEqual(h.digest(), Md5Native.digest(b("abx")))
    }
}
