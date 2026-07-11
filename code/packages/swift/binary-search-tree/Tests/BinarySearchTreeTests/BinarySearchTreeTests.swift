// ============================================================================
// BinarySearchTreeTests.swift — Unit tests for BST
// ============================================================================

import XCTest
@testable import BinarySearchTree

final class BinarySearchTreeTests: XCTestCase {

    private func sample() -> BST<Int> {
        // Mirrors the reference test tree.
        BST<Int>()
            .insert(8).insert(3).insert(10).insert(1)
            .insert(6).insert(14).insert(4).insert(7)
    }

    func testInsertSearchAndDelete() {
        let tree = sample()
        XCTAssertTrue(tree.contains(4))
        XCTAssertEqual(tree.search(4), 4)
        XCTAssertNil(tree.search(99))
        XCTAssertEqual(tree.minValue(), 1)
        XCTAssertEqual(tree.maxValue(), 14)
        XCTAssertEqual(tree.rank(6), 3)
        XCTAssertEqual(tree.kthSmallest(4), 6)
        XCTAssertEqual(tree.count, 8)

        let deleted = tree.delete(3)
        XCTAssertFalse(deleted.contains(3))
        XCTAssertTrue(deleted.isValid())
        XCTAssertEqual(deleted.count, 7)
        // Original is untouched (immutability).
        XCTAssertTrue(tree.contains(3))
        XCTAssertEqual(tree.count, 8)
    }

    func testFromSortedBuildsBalancedTree() {
        let tree = BST.fromSorted([1, 2, 3, 4, 5, 6, 7])
        XCTAssertEqual(tree.toSortedArray(), [1, 2, 3, 4, 5, 6, 7])
        XCTAssertLessThanOrEqual(tree.height, 2)
        XCTAssertTrue(tree.isValid())
    }

    func testEmptyTree() {
        let tree = BST<Int>()
        XCTAssertTrue(tree.isEmpty)
        XCTAssertEqual(tree.count, 0)
        XCTAssertEqual(tree.height, -1)
        XCTAssertNil(tree.minValue())
        XCTAssertNil(tree.maxValue())
        XCTAssertNil(tree.search(1))
        XCTAssertNil(tree.kthSmallest(1))
        XCTAssertEqual(tree.rank(5), 0)
        XCTAssertTrue(tree.isValid())
        XCTAssertEqual(tree.toSortedArray(), [])
    }

    func testInsertDuplicateIsNoOp() {
        let tree = BST<Int>().insert(5).insert(5).insert(5)
        XCTAssertEqual(tree.count, 1)
        XCTAssertEqual(tree.toSortedArray(), [5])
    }

    func testToSortedArrayIsInOrder() {
        let tree = BST<Int>().insert(5).insert(2).insert(8).insert(1).insert(9).insert(3)
        XCTAssertEqual(tree.toSortedArray(), [1, 2, 3, 5, 8, 9])
    }

    func testDeleteAllNodeShapes() {
        // Delete a leaf, a one-child node, and a two-child node.
        var tree = BST.fromSorted([1, 2, 3, 4, 5, 6, 7])
        tree = tree.delete(1)          // leaf
        XCTAssertFalse(tree.contains(1))
        tree = tree.delete(2)          // node with one child (after leaf removal)
        XCTAssertFalse(tree.contains(2))
        tree = tree.delete(4)          // (likely) two-child node
        XCTAssertFalse(tree.contains(4))
        XCTAssertTrue(tree.isValid())
        XCTAssertEqual(tree.toSortedArray(), [3, 5, 6, 7])

        // Deleting an absent value is a no-op.
        let same = tree.delete(999)
        XCTAssertEqual(same.toSortedArray(), tree.toSortedArray())
    }

    func testDeleteRepeatedlyStaysValidAndSorted() {
        var tree = BST<Int>()
        for v in [50, 30, 70, 20, 40, 60, 80, 10, 25, 35, 45] { tree = tree.insert(v) }
        var expected = tree.toSortedArray()
        while !tree.isEmpty {
            // Repeatedly delete the median element — exercises many node shapes.
            let mid = tree.toSortedArray()[tree.count / 2]
            tree = tree.delete(mid)
            expected.removeAll { $0 == mid }
            XCTAssertTrue(tree.isValid())
            XCTAssertEqual(tree.toSortedArray(), expected)
        }
        XCTAssertTrue(tree.isEmpty)
    }

    func testPredecessorAndSuccessor() {
        let tree = BST<Int>().insert(20).insert(10).insert(30).insert(5).insert(15).insert(25).insert(35)
        XCTAssertEqual(tree.predecessor(20), 15)
        XCTAssertEqual(tree.successor(20), 25)
        XCTAssertEqual(tree.predecessor(15), 10)   // present value → strictly less
        XCTAssertEqual(tree.successor(15), 20)
        XCTAssertNil(tree.predecessor(5))          // nothing smaller than the min
        XCTAssertNil(tree.successor(35))           // nothing larger than the max
        XCTAssertEqual(tree.predecessor(12), 10)   // absent value between elements
        XCTAssertEqual(tree.successor(12), 15)
    }

    func testKthSmallestAndRankAcrossFullRange() {
        let values = [15, 3, 27, 9, 21, 1, 33, 6, 12, 18, 24, 30]
        var tree = BST<Int>()
        for v in values { tree = tree.insert(v) }
        let sorted = values.sorted()
        for i in 1...values.count {
            XCTAssertEqual(tree.kthSmallest(i), sorted[i - 1], "kthSmallest(\(i))")
        }
        XCTAssertNil(tree.kthSmallest(0))
        XCTAssertNil(tree.kthSmallest(values.count + 1))
        for v in sorted {
            XCTAssertEqual(tree.rank(v), sorted.firstIndex(of: v)!, "rank(\(v))")
        }
        // rank of an absent value == count of elements strictly less than it.
        XCTAssertEqual(tree.rank(10), sorted.filter { $0 < 10 }.count)
    }

    func testWorksWithStrings() {
        let tree = BST<String>().insert("pear").insert("apple").insert("fig").insert("date")
        XCTAssertEqual(tree.toSortedArray(), ["apple", "date", "fig", "pear"])
        XCTAssertEqual(tree.minValue(), "apple")
        XCTAssertEqual(tree.kthSmallest(2), "date")
        XCTAssertTrue(tree.isValid())
    }

    func testDescriptionMentionsCount() {
        XCTAssertTrue(sample().description.contains("count=8"))
    }
}
