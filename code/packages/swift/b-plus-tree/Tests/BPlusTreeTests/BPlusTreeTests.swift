import XCTest
@testable import BPlusTree

/// BPlusTreeTests.swift — Comprehensive tests for the B+ Tree (DT12)
///
/// Test strategy:
///   1. Basic operations (insert, search, delete)
///   2. Leaf linked-list integrity verified after every mutation
///   3. Range scan (the B+ Tree's key differentiator)
///   4. Full scan via linked list
///   5. Multiple minimum degrees (t=2, t=3, t=5)
///   6. 500+ key scale tests
///   7. isValid() checked after every mutation
///   8. Edge cases (empty, single key, upsert)

final class BPlusTreeTests: XCTestCase {

    // =========================================================================
    // Helper — verify linked-list integrity manually
    // =========================================================================

    /// Walk the full-scan result and verify keys are strictly increasing.
    private func assertLinkedListSorted<K: Comparable, V>(_ tree: BPlusTree<K, V>, file: StaticString = #filePath, line: UInt = #line) {
        let items = tree.fullScan()
        guard items.count > 1 else { return }
        for i in 1..<items.count {
            XCTAssertLessThan(items[i-1].0, items[i].0,
                "Linked list out of order at index \(i): \(items[i-1].0) >= \(items[i].0)",
                file: file, line: line)
        }
    }

    // =========================================================================
    // 1. Empty tree
    // =========================================================================

    func testEmptyTree() {
        let tree = BPlusTree<Int, String>()
        XCTAssertEqual(tree.count, 0)
        XCTAssertEqual(tree.height, 0)
        XCTAssertNil(tree.search(1))
        XCTAssertNil(tree.minKey())
        XCTAssertNil(tree.maxKey())
        XCTAssertEqual(tree.fullScan().count, 0)
        XCTAssertTrue(tree.isValid())
        XCTAssertFalse(tree.delete(1))
    }

    // =========================================================================
    // 2. Single key
    // =========================================================================

    func testSingleKey() {
        let tree = BPlusTree<Int, String>()
        tree.insert(42, "forty-two")
        XCTAssertEqual(tree.count, 1)
        XCTAssertEqual(tree.search(42), "forty-two")
        XCTAssertEqual(tree.minKey(), 42)
        XCTAssertEqual(tree.maxKey(), 42)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)

        tree.delete(42)
        XCTAssertEqual(tree.count, 0)
        XCTAssertNil(tree.search(42))
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 3. Basic insert/search/delete (t=2)
    // =========================================================================

    func testBasicT2() {
        let tree = BPlusTree<Int, String>(t: 2)
        for i in [5, 3, 7, 1, 4, 6, 8] { tree.insert(i, "v\(i)") }
        XCTAssertEqual(tree.count, 7)
        for i in [5, 3, 7, 1, 4, 6, 8] { XCTAssertEqual(tree.search(i), "v\(i)") }
        XCTAssertNil(tree.search(99))
        XCTAssertEqual(tree.minKey(), 1)
        XCTAssertEqual(tree.maxKey(), 8)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
    }

    // =========================================================================
    // 4. Upsert
    // =========================================================================

    func testUpsert() {
        let tree = BPlusTree<Int, String>(t: 2)
        tree.insert(10, "ten")
        tree.insert(10, "TEN")
        XCTAssertEqual(tree.count, 1)
        XCTAssertEqual(tree.search(10), "TEN")
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
    }

    // =========================================================================
    // 5. Full scan returns all keys sorted (uses linked list)
    // =========================================================================

    func testFullScan() {
        let tree = BPlusTree<Int, Int>(t: 2)
        let keys = [30, 10, 50, 20, 40]
        for k in keys { tree.insert(k, k) }
        let all = tree.fullScan()
        XCTAssertEqual(all.map(\.0), [10, 20, 30, 40, 50])
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 6. Range scan (core B+ Tree feature)
    // =========================================================================

    func testRangeScan() {
        let tree = BPlusTree<Int, String>(t: 2)
        for i in 1...20 { tree.insert(i, "v\(i)") }
        XCTAssertTrue(tree.isValid())

        let r = tree.rangeScan(from: 5, to: 10)
        XCTAssertEqual(r.map(\.0), [5, 6, 7, 8, 9, 10])

        // Range covering nothing.
        XCTAssertEqual(tree.rangeScan(from: 100, to: 200).count, 0)

        // Single-element range.
        XCTAssertEqual(tree.rangeScan(from: 7, to: 7).map(\.0), [7])

        // Full range.
        XCTAssertEqual(tree.rangeScan(from: 1, to: 20).count, 20)
        assertLinkedListSorted(tree)
    }

    // =========================================================================
    // 7. Delete operations
    // =========================================================================

    func testDeleteLeaf() {
        let tree = BPlusTree<Int, String>(t: 2)
        for i in 1...5 { tree.insert(i, "v\(i)") }
        XCTAssertTrue(tree.delete(3))
        XCTAssertNil(tree.search(3))
        XCTAssertEqual(tree.count, 4)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
    }

    func testDeleteAll() {
        let tree = BPlusTree<Int, Int>(t: 2)
        for i in 1...10 { tree.insert(i, i) }
        for i in 1...10 {
            XCTAssertTrue(tree.delete(i), "Failed to delete \(i)")
            XCTAssertTrue(tree.isValid(), "Invalid after deleting \(i)")
            assertLinkedListSorted(tree)
        }
        XCTAssertEqual(tree.count, 0)
    }

    func testDeleteMissingKey() {
        let tree = BPlusTree<Int, Int>(t: 2)
        tree.insert(1, 1)
        XCTAssertFalse(tree.delete(99))
        XCTAssertEqual(tree.count, 1)
        XCTAssertTrue(tree.isValid())
    }

    // =========================================================================
    // 8. Leaf linked list integrity after splits
    // =========================================================================

    func testLinkedListAfterSplits() {
        // Insert enough keys to force multiple leaf splits.
        let tree = BPlusTree<Int, Int>(t: 2)
        for i in 1...20 { tree.insert(i, i) }
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)

        // Delete some keys and verify list remains intact.
        for i in [3, 7, 11, 15, 19] { tree.delete(i) }
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
    }

    // =========================================================================
    // 9. Min / max keys
    // =========================================================================

    func testMinMax() {
        let tree = BPlusTree<Int, Int>(t: 3)
        for k in [7, 3, 11, 1, 5, 9, 13] { tree.insert(k, k) }
        XCTAssertEqual(tree.minKey(), 1)
        XCTAssertEqual(tree.maxKey(), 13)
        XCTAssertTrue(tree.isValid())
        tree.delete(1)
        XCTAssertEqual(tree.minKey(), 3)
        tree.delete(13)
        XCTAssertEqual(tree.maxKey(), 11)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
    }

    // =========================================================================
    // 10. Large-scale test — 500 keys, t=2
    // =========================================================================

    func testLargeScaleT2() {
        let tree = BPlusTree<Int, Int>(t: 2)
        var keys = Array(1...500)
        keys.shuffle()
        for k in keys { tree.insert(k, k * 2) }
        XCTAssertEqual(tree.count, 500)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)

        // Full scan must have 500 entries in order.
        let all = tree.fullScan()
        XCTAssertEqual(all.count, 500)
        XCTAssertEqual(all.map(\.0), Array(1...500))

        // Range scan for 200..300.
        let range = tree.rangeScan(from: 200, to: 300)
        XCTAssertEqual(range.count, 101)
        XCTAssertEqual(range.first?.0, 200)
        XCTAssertEqual(range.last?.0, 300)

        // Delete half the keys.
        for k in stride(from: 1, through: 500, by: 2) { tree.delete(k) }
        XCTAssertEqual(tree.count, 250)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
    }

    // =========================================================================
    // 11. Large-scale test — 600 keys, t=3
    // =========================================================================

    func testLargeScaleT3() {
        let tree = BPlusTree<Int, Int>(t: 3)
        for k in 1...600 { tree.insert(k, k) }
        XCTAssertEqual(tree.count, 600)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)

        let all = tree.fullScan().map(\.0)
        XCTAssertEqual(all, Array(1...600))
    }

    // =========================================================================
    // 12. Large-scale test — 750 keys, t=5
    // =========================================================================

    func testLargeScaleT5() {
        let tree = BPlusTree<Int, String>(t: 5)
        for k in 1...750 { tree.insert(k, "v\(k)") }
        XCTAssertEqual(tree.count, 750)
        XCTAssertTrue(tree.isValid())
        XCTAssertLessThanOrEqual(tree.height, 5)
        assertLinkedListSorted(tree)

        let range = tree.rangeScan(from: 500, to: 600)
        XCTAssertEqual(range.count, 101)
    }

    // =========================================================================
    // 13. Internal nodes have no values
    // =========================================================================

    func testInorderEqualsFullScan() {
        let tree = BPlusTree<Int, Int>(t: 2)
        for k in [10, 5, 20, 15, 25, 30, 1] { tree.insert(k, k) }
        XCTAssertTrue(tree.isValid())
        // inorder() and fullScan() must return identical results.
        let inord = tree.inorder()
        let full = tree.fullScan()
        XCTAssertEqual(inord.map(\.0), full.map(\.0))
    }

    // =========================================================================
    // 14. Reverse-order insert
    // =========================================================================

    func testReverseOrderInsert() {
        let tree = BPlusTree<Int, Int>(t: 2)
        for k in stride(from: 200, through: 1, by: -1) { tree.insert(k, k) }
        XCTAssertEqual(tree.count, 200)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
        XCTAssertEqual(tree.fullScan().map(\.0), Array(1...200))
    }

    // =========================================================================
    // 15. Insert → delete → re-insert
    // =========================================================================

    func testInsertDeleteReinsert() {
        let tree = BPlusTree<Int, Int>(t: 2)
        for i in 1...50 { tree.insert(i, i) }
        for i in 1...50 { tree.delete(i) }
        XCTAssertEqual(tree.count, 0)
        XCTAssertTrue(tree.isValid())
        for i in 51...100 { tree.insert(i, i) }
        XCTAssertEqual(tree.count, 50)
        XCTAssertTrue(tree.isValid())
        assertLinkedListSorted(tree)
        XCTAssertEqual(tree.minKey(), 51)
        XCTAssertEqual(tree.maxKey(), 100)
    }

    // =========================================================================
    // 16. String keys
    // =========================================================================

    func testStringKeys() {
        let tree = BPlusTree<String, Int>(t: 2)
        let words = ["zebra", "ant", "mango", "banana", "cherry"]
        for (i, w) in words.enumerated() { tree.insert(w, i) }
        XCTAssertEqual(tree.count, 5)
        XCTAssertTrue(tree.isValid())
        let all = tree.fullScan()
        XCTAssertEqual(all.map(\.0), all.map(\.0).sorted())
        XCTAssertEqual(tree.minKey(), "ant")
        XCTAssertEqual(tree.maxKey(), "zebra")
        assertLinkedListSorted(tree)
    }
}
