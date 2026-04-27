import XCTest
@testable import Heap

final class HeapTests: XCTestCase {
    func testMinHeapOrdersAscending() {
        var heap = MinHeap<Int>()
        [5, 3, 8, 1, 4].forEach { heap.push($0) }

        XCTAssertEqual(heap.peek(), 1)
        XCTAssertEqual(heap.pop(), 1)
        XCTAssertEqual(heap.pop(), 3)
        XCTAssertEqual(heap.pop(), 4)
        XCTAssertEqual(heap.pop(), 5)
        XCTAssertEqual(heap.pop(), 8)
        XCTAssertNil(heap.pop())
    }

    func testMaxHeapOrdersDescending() {
        var heap = MaxHeap<Int>()
        [5, 3, 8, 1, 4].forEach { heap.push($0) }

        XCTAssertEqual(heap.peek(), 8)
        XCTAssertEqual(heap.pop(), 8)
        XCTAssertEqual(heap.pop(), 5)
        XCTAssertEqual(heap.pop(), 4)
        XCTAssertEqual(heap.pop(), 3)
        XCTAssertEqual(heap.pop(), 1)
        XCTAssertNil(heap.pop())
    }

    func testSequenceInitializerHeapifies() {
        var heap = MinHeap([9, 2, 7, 1, 5])
        XCTAssertEqual(heap.count, 5)
        XCTAssertEqual(heap.peek(), 1)
        XCTAssertEqual(heap.toArray().count, 5)
        XCTAssertEqual(heap.pop(), 1)
        XCTAssertEqual(heap.pop(), 2)
        XCTAssertEqual(heap.pop(), 5)
        XCTAssertEqual(heap.pop(), 7)
        XCTAssertEqual(heap.pop(), 9)
    }

    func testCustomComparatorSupportsTuples() {
        typealias Pair = (priority: Int, label: String)
        var heap = MinHeap<Pair> {
            if $0.priority != $1.priority {
                return $0.priority < $1.priority
            }
            return $0.label < $1.label
        }

        heap.push((priority: 1, label: "b"))
        heap.push((priority: 1, label: "a"))
        heap.push((priority: 0, label: "z"))

        XCTAssertEqual(heap.pop()?.priority, 0)
        XCTAssertEqual(heap.pop()?.label, "a")
        XCTAssertEqual(heap.pop()?.label, "b")
    }
}
