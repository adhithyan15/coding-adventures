// ============================================================================
// FenwickTreeTests.swift — Unit tests for FenwickTree
// ============================================================================

import XCTest
@testable import FenwickTree

final class FenwickTreeTests: XCTestCase {

    private func assertClose(_ a: Double, _ b: Double, _ msg: String = "",
                             file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertLessThan(abs(a - b), 1e-9, msg, file: file, line: line)
    }

    // ── Construction / prefix sums ───────────────────────────────────────────

    func testFromValuesBuildsExpectedPrefixSums() throws {
        let tree = FenwickTree(values: [3, 2, 1, 7, 4])
        assertClose(try tree.prefixSum(1), 3)
        assertClose(try tree.prefixSum(2), 5)
        assertClose(try tree.prefixSum(3), 6)
        assertClose(try tree.prefixSum(4), 13)
        assertClose(try tree.prefixSum(5), 17)
    }

    func testPrefixSumAcceptsZero() throws {
        let tree = FenwickTree(values: [1, 2, 3])
        assertClose(try tree.prefixSum(0), 0)
    }

    // ── Range sum / point query ──────────────────────────────────────────────

    func testRangeSumAndPointQuery() throws {
        let tree = FenwickTree(values: [3, 2, 1, 7, 4])
        assertClose(try tree.rangeSum(2, 4), 10)
        assertClose(try tree.pointQuery(4), 7)
        assertClose(try tree.rangeSum(1, 5), 17)
        assertClose(try tree.rangeSum(3, 3), 1)
    }

    // ── Updates ──────────────────────────────────────────────────────────────

    func testUpdateChangesFutureQueries() throws {
        var tree = FenwickTree(values: [3, 2, 1, 7, 4])
        try tree.update(3, delta: 5)
        assertClose(try tree.prefixSum(3), 11)
        assertClose(try tree.pointQuery(3), 6)
        assertClose(try tree.prefixSum(5), 22)
    }

    func testNegativeDelta() throws {
        var tree = FenwickTree(values: [10, 10, 10])
        try tree.update(2, delta: -4)
        assertClose(try tree.pointQuery(2), 6)
        assertClose(try tree.rangeSum(1, 3), 26)
    }

    // ── find_kth (order statistic) ───────────────────────────────────────────

    func testFindKthMatchesOrderStatistic() throws {
        let tree = FenwickTree(values: [1, 2, 3, 4, 5])
        XCTAssertEqual(try tree.findKth(1), 1)
        XCTAssertEqual(try tree.findKth(2), 2)
        XCTAssertEqual(try tree.findKth(3), 2)
        XCTAssertEqual(try tree.findKth(4), 3)
        XCTAssertEqual(try tree.findKth(10), 4)
    }

    func testFindKthErrors() {
        let empty = FenwickTree(size: 0)
        XCTAssertThrowsError(try empty.findKth(1)) { XCTAssertEqual($0 as? FenwickError, .emptyTree) }

        let tree = FenwickTree(values: [1, 2, 3])
        XCTAssertThrowsError(try tree.findKth(0)) {
            XCTAssertEqual($0 as? FenwickError, .nonPositiveTarget(target: 0))
        }
        XCTAssertThrowsError(try tree.findKth(100)) {
            guard case .targetExceedsTotal = ($0 as? FenwickError) else {
                return XCTFail("expected targetExceedsTotal")
            }
        }
    }

    // ── Error cases ──────────────────────────────────────────────────────────

    func testInvalidIndicesThrow() {
        let tree = FenwickTree(values: [1, 2, 3])
        XCTAssertThrowsError(try tree.prefixSum(4)) {
            guard case .indexOutOfRange = ($0 as? FenwickError) else {
                return XCTFail("expected indexOutOfRange")
            }
        }
        XCTAssertThrowsError(try tree.rangeSum(0, 3)) {
            guard case .indexOutOfRange = ($0 as? FenwickError) else {
                return XCTFail("expected indexOutOfRange")
            }
        }
        XCTAssertThrowsError(try tree.rangeSum(3, 1)) {
            XCTAssertEqual($0 as? FenwickError, .invalidRange(left: 3, right: 1))
        }
    }

    // ── Introspection ────────────────────────────────────────────────────────

    func testBitArrayAndDescription() {
        let tree = FenwickTree(values: [1, 2])
        XCTAssertEqual(tree.bitArray, [1, 3])
        XCTAssertTrue(tree.description.contains("FenwickTree"))
    }

    func testCountAndIsEmpty() {
        XCTAssertTrue(FenwickTree(size: 0).isEmpty)
        let tree = FenwickTree(values: [1, 2, 3])
        XCTAssertEqual(tree.count, 3)
        XCTAssertFalse(tree.isEmpty)
    }

    // ── Brute-force cross-check ──────────────────────────────────────────────

    func testBruteForcePrefixAndRange() throws {
        let values: [Double] = [5, -2, 7, 1.5, 4.5]
        let tree = FenwickTree(values: values)
        for index in 1...values.count {
            assertClose(try tree.prefixSum(index), values.prefix(index).reduce(0, +))
        }
        for left in 1...values.count {
            for right in left...values.count {
                let expected = values[(left - 1)..<right].reduce(0, +)
                assertClose(try tree.rangeSum(left, right), expected)
            }
        }
    }
}
