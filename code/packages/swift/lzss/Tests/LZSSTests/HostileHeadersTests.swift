// HostileHeadersTests.swift
// Security regression tests: hostile headers must not drive work or memory.
//
// Each case is a few bytes whose header claims an enormous count or length.  An
// implementation that sized an allocation or loop from the claim (or trapped on
// an invalid index) would exhaust memory, run for minutes or crash the process.
// Also round-trips every input of length <= 2 and a seeded random sample.

import Foundation
import XCTest
@testable import LZSS

final class HostileHeadersTests: XCTestCase {

    func u32(_ v: UInt32) -> [UInt8] {
        [UInt8(v >> 24 & 255), UInt8(v >> 16 & 255), UInt8(v >> 8 & 255), UInt8(v & 255)]
    }

    func str(_ s: String) -> [UInt8] { Array(s.utf8) }

    func quick<T>(_ body: () -> T) -> T {
        let start = Date()
        let result = body()
        XCTAssertLessThan(Date().timeIntervalSince(start), 10.0, "hostile input took too long")
        return result
    }

    func hdr(_ len: UInt32, _ blocks: UInt32) -> [UInt8] { u32(len) + u32(blocks) }

    func testMaxHeaderOnly() {
        XCTAssertEqual(quick { decompress([UInt8](repeating: 255, count: 8)) }, [])
    }

    func testMaxLengthOneLiteral() {
        XCTAssertEqual(quick { decompress(hdr(0xFFFF_FFFF, 1) + [0, 120]) }, str("x"))
    }

    func testBlockCountLargerThanRemainingBytes() {
        XCTAssertEqual(quick { decompress(hdr(2, 0xFFFF_FFFF) + [0, 104, 105]) }, str("hi"))
    }

    func testLengthInconsistentWithTokens() {
        XCTAssertEqual(decompress(hdr(1, 1) + [0, 104, 105]), str("h"))
    }

    func testInvalidMatchOffsetsAreSkipped() {
        XCTAssertEqual(decompress(hdr(100, 1) + [1, 0, 0, 5]), [])
        XCTAssertEqual(decompress(hdr(100, 1) + [1, 255, 255, 255]), [])
    }

    func testRoundTripEveryInputUpToLength2() {
        var inputs: [[UInt8]] = [[]]
        for a in 0..<256 {
            inputs.append([UInt8(a)])
            for b in 0..<256 { inputs.append([UInt8(a), UInt8(b)]) }
        }
        for d in inputs {
            XCTAssertEqual(decompress(compress(d)), d)
        }
    }

    func testRoundTripSeededRandomInputs() {
        // xorshift64* so the sample is deterministic across platforms.
        var state: UInt64 = 0x5EED
        func next() -> UInt64 {
            state ^= state >> 12
            state ^= state << 25
            state ^= state >> 27
            return state &* 0x2545F4914F6CDD1D
        }
        let alphabets: [UInt64] = [2, 4, 16, 256]
        for i in 0..<300 {
            let n = 3 + Int(next() % 597)
            let d = (0..<n).map { _ in UInt8(next() % alphabets[i % 4]) }
            XCTAssertEqual(decompress(compress(d)), d)
        }
    }
}
