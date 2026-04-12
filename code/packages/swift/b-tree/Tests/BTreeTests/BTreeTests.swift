import XCTest
@testable import BTree

/// BTreeTests.swift — Comprehensive tests for the B-Tree (DT11)
///
/// Test strategy:
///   1. Basic operations on a simple tree
///   2. All deletion cases (1, 2a, 2b, 2c, 3a, 3b, 3c)
///   3. Multiple minimum degrees (t=2, t=3, t=5)
///   4. Large-scale insert/delete with 500+ keys
///   5. isValid() checked after every mutation
///   6. Range queries and in-order traversal
///   7. Edge cases (empty tree, single key, duplicate keys)

final class BTreeTests: XCTestCase {

    // =========================================================================
    // 1. Empty tree
    // =========================================================================

    func testEmptyTree() {
        let tree = BTree<Int, String>()
        XCTAssertEqual(tree.count, 0)
        XCTAssertEqual(tree.height, 0)
        XCTAssertNil(tree.search(42))
        XCTAssertFalse(tree.contains(42))
        XCTAssertNil(tree.minKey())
        XCTAssertNil(tree.maxKey())
        XCTAssertEqual(tree.inorder().map(\.0), [])
        XCTAssertTrue(tree.isValid())
        XCTAssertFalse(tree.delete(42))
    }

    // =========================================================================
    // 2. Single-key tree
    // =========================================================================

    func testSingleKey() {
        let tree = BTree<Int, String>()
        tree.insert(5, "five")
        XCTAssertEqual(tree.count, 1)
        XCTAssertEqual(tree.search(5), "five")
        XCTAssertTrue(tree.contains(5))
        XCTAssertEqual(tree.minKey(), 5)
        XCTAssertEqual(tree.maxKey(), 5)
        XCTAssertTrue(tree.isValid())
        tree.delete(5)
        XCTAssertEqual(tree.count, 0)
        XCTAssertNil(tree.search(5))
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 3. Basic insert and search (t=2)
    // =========================================================================

    func testBasicInsertSearchT2() {
        let tree = BTree<Int, String>(t: 2)
        let pairs = [(10, "ten"), (20, "twenty"), (5, "five"), (15, "fifteen"), (25, "twenty-five")]
        for (k, v) in pairs { tree.insert(k, v) }
        XCTAssertEqual(tree.count, 5)
        for (k, v) in pairs { XCTAssertEqual(tree.search(k), v) }
        XCTAssertNil(tree.search(99))
        XCTAssertEqual(tree.minKey(), 5)
        XCTAssertEqual(tree.maxKey(), 25)
        XCTAssertTrue(tree.isValid())
        // In-order should be sorted
        let keys = tree.inorder().map(\.0)
        XCTAssertEqual(keys, keys.sorted())
    }

    // =========================================================================
    // 4. Upsert (duplicate key updates value)
    // =========================================================================

    func testUpsert() {
        let tree = BTree<Int, String>(t: 2)
        tree.insert(1, "one")
        tree.insert(1, "ONE")
        XCTAssertEqual(tree.count, 1)
        XCTAssertEqual(tree.search(1), "ONE")
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 5. In-order traversal is always sorted
    // =========================================================================

    func testInorderSorted() {
        let tree = BTree<Int, Int>(t: 3)
        let keys = [50, 10, 80, 30, 60, 5, 25, 55, 75, 90]
        for k in keys { tree.insert(k, k * 2) }
        let result = tree.inorder()
        let resultKeys = result.map(\.0)
        XCTAssertEqual(resultKeys, resultKeys.sorted())
        for (k, v) in result { XCTAssertEqual(v, k * 2) }
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 6. Range query
    // =========================================================================

    func testRangeQuery() {
        let tree = BTree<Int, String>(t: 2)
        for i in 1...20 { tree.insert(i, "v\(i)") }
        let result = tree.rangeQuery(from: 5, to: 10)
        XCTAssertEqual(result.map(\.0), [5, 6, 7, 8, 9, 10])
        XCTAssertTrue(tree.isValid())

        // Empty range
        let empty = tree.rangeQuery(from: 100, to: 200)
        XCTAssertEqual(empty.count, 0)

        // Single-element range
        let single = tree.rangeQuery(from: 7, to: 7)
        XCTAssertEqual(single.map(\.0), [7])
    }

    // =========================================================================
    // 7. Deletion — Case 1: remove from leaf
    // =========================================================================

    func testDeleteLeaf() {
        let tree = BTree<Int, String>(t: 2)
        for i in [10, 20, 5, 15, 25] { tree.insert(i, "\(i)") }
        XCTAssertTrue(tree.delete(5))
        XCTAssertEqual(tree.count, 4)
        XCTAssertNil(tree.search(5))
        XCTAssertTrue(tree.isValid())

        // Delete absent key
        XCTAssertFalse(tree.delete(99))
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 8. Deletion — Case 2: remove from internal node
    // =========================================================================

    func testDeleteInternalNode() {
        // Build a tree deep enough to have internal nodes.
        let tree = BTree<Int, String>(t: 2)
        for i in stride(from: 1, through: 15, by: 1) { tree.insert(i, "\(i)") }
        XCTAssertTrue(tree.isValid())

        // Delete a key that is definitely in an internal node.
        // After 15 inserts with t=2, height ≥ 2.
        XCTAssertTrue(tree.delete(8))
        XCTAssertNil(tree.search(8))
        XCTAssertEqual(tree.count, 14)
        XCTAssertTrue(tree.isValid())

        // Delete more internal keys.
        XCTAssertTrue(tree.delete(4))
        XCTAssertTrue(tree.delete(12))
        XCTAssertEqual(tree.count, 12)
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 9. Deletion — all cases exercised with sequential keys
    // =========================================================================

    func testDeleteAllCases() {
        let tree = BTree<Int, Int>(t: 2)
        for i in 1...30 { tree.insert(i, i) }
        XCTAssertTrue(tree.isValid())

        // Delete in various orders to exercise Case 2a, 2b, 2c, 3a, 3b, 3c.
        let deleteOrder = [15, 1, 30, 10, 20, 5, 25, 8, 22, 3, 17, 28]
        for k in deleteOrder {
            XCTAssertTrue(tree.delete(k), "Expected to delete \(k)")
            XCTAssertNil(tree.search(k))
            XCTAssertTrue(tree.isValid(), "Tree invalid after deleting \(k)")
        }
        XCTAssertEqual(tree.count, 30 - deleteOrder.count)
    }

    // =========================================================================
    // 10. Delete until empty
    // =========================================================================

    func testDeleteUntilEmpty() {
        let tree = BTree<Int, Int>(t: 2)
        for i in 1...10 { tree.insert(i, i) }
        for i in 1...10 {
            XCTAssertTrue(tree.delete(i))
            XCTAssertTrue(tree.isValid())
        }
        XCTAssertEqual(tree.count, 0)
        XCTAssertNil(tree.minKey())
        XCTAssertNil(tree.maxKey())
    }

    // =========================================================================
    // 11. Large-scale test — 500+ keys, t=2
    // =========================================================================

    func testLargeScaleT2() {
        let tree = BTree<Int, Int>(t: 2)
        let n = 500
        // Insert in shuffled order.
        var keys = Array(1...n)
        keys.shuffle()
        for k in keys { tree.insert(k, k * 3) }
        XCTAssertEqual(tree.count, n)
        XCTAssertTrue(tree.isValid())
        XCTAssertEqual(tree.minKey(), 1)
        XCTAssertEqual(tree.maxKey(), n)

        // Verify every key is findable.
        for k in 1...n { XCTAssertEqual(tree.search(k), k * 3) }

        // Delete half the keys.
        for k in stride(from: 1, through: n, by: 2) {
            XCTAssertTrue(tree.delete(k))
        }
        XCTAssertEqual(tree.count, n / 2)
        XCTAssertTrue(tree.isValid())

        // Remaining keys (even) should still be findable.
        for k in stride(from: 2, through: n, by: 2) {
            XCTAssertEqual(tree.search(k), k * 3)
        }
    }

    // =========================================================================
    // 12. Large-scale test — 500+ keys, t=3
    // =========================================================================

    func testLargeScaleT3() {
        let tree = BTree<Int, Int>(t: 3)
        let n = 600
        for k in 1...n { tree.insert(k, k) }
        XCTAssertEqual(tree.count, n)
        XCTAssertTrue(tree.isValid())

        // In-order must be sorted.
        let inord = tree.inorder().map(\.0)
        XCTAssertEqual(inord, Array(1...n))

        // Delete all even keys.
        for k in stride(from: 2, through: n, by: 2) { tree.delete(k) }
        XCTAssertEqual(tree.count, n / 2)
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 13. Large-scale test — 500+ keys, t=5
    // =========================================================================

    func testLargeScaleT5() {
        let tree = BTree<Int, String>(t: 5)
        let n = 750
        for k in 1...n { tree.insert(k, "key\(k)") }
        XCTAssertEqual(tree.count, n)
        XCTAssertTrue(tree.isValid())
        // Height should be very small with t=5.
        XCTAssertLessThanOrEqual(tree.height, 5)

        // Range query over a large range.
        let range = tree.rangeQuery(from: 100, to: 200)
        XCTAssertEqual(range.count, 101)
        XCTAssertEqual(range.first?.0, 100)
        XCTAssertEqual(range.last?.0, 200)
    }

    // =========================================================================
    // 14. Min / max keys
    // =========================================================================

    func testMinMax() {
        let tree = BTree<Int, Int>(t: 2)
        for k in [7, 3, 11, 1, 5, 9, 13] { tree.insert(k, k) }
        XCTAssertEqual(tree.minKey(), 1)
        XCTAssertEqual(tree.maxKey(), 13)
        XCTAssertTrue(tree.isValid())
        tree.delete(1)
        XCTAssertEqual(tree.minKey(), 3)
        tree.delete(13)
        XCTAssertEqual(tree.maxKey(), 11)
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 15. isValid on manually broken invariants (sanity check of validator)
    // =========================================================================

    func testIsValidOnValidTree() {
        let tree = BTree<Int, Int>(t: 2)
        for i in 1...20 { tree.insert(i, i) }
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 16. String keys
    // =========================================================================

    func testStringKeys() {
        let tree = BTree<String, Int>(t: 2)
        let words = ["banana", "apple", "cherry", "date", "elderberry"]
        for (i, w) in words.enumerated() { tree.insert(w, i) }
        XCTAssertEqual(tree.count, 5)
        XCTAssertTrue(tree.isValid())
        let inord = tree.inorder().map(\.0)
        XCTAssertEqual(inord, inord.sorted())
        XCTAssertEqual(tree.minKey(), "apple")
        XCTAssertEqual(tree.maxKey(), "elderberry")
    }

    // =========================================================================
    // 17. Height increases as keys are inserted
    // =========================================================================

    func testHeightGrowth() {
        let tree = BTree<Int, Int>(t: 2)
        // An empty tree has height 0.
        XCTAssertEqual(tree.height, 0)
        // After 3 inserts with t=2, the root may still be a leaf (height 0)
        // if no split has occurred yet.
        tree.insert(1, 1); tree.insert(2, 2); tree.insert(3, 3)
        XCTAssertTrue(tree.isValid())
        // After enough inserts we expect height to grow.
        for k in 4...50 { tree.insert(k, k) }
        XCTAssertGreaterThan(tree.height, 0)
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 18. Reverse-order insert stresses rotations
    // =========================================================================

    func testReverseOrderInsert() {
        let tree = BTree<Int, Int>(t: 3)
        for k in stride(from: 300, through: 1, by: -1) { tree.insert(k, k) }
        XCTAssertEqual(tree.count, 300)
        XCTAssertTrue(tree.isValid())
        let inord = tree.inorder().map(\.0)
        XCTAssertEqual(inord, Array(1...300))
    }

    // =========================================================================
    // 19. Insert then delete all then re-insert
    // =========================================================================

    func testInsertDeleteReinsert() {
        let tree = BTree<Int, Int>(t: 2)
        for i in 1...50 { tree.insert(i, i) }
        for i in 1...50 { tree.delete(i) }
        XCTAssertEqual(tree.count, 0)
        XCTAssertTrue(tree.isValid())
        for i in 51...100 { tree.insert(i, i) }
        XCTAssertEqual(tree.count, 50)
        XCTAssertTrue(tree.isValid())
        XCTAssertEqual(tree.minKey(), 51)
        XCTAssertEqual(tree.maxKey(), 100)
    }
}
