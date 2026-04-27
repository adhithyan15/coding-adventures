private struct BinaryHeap<Element> {
    private var storage: [Element] = []
    private let higherPriority: (Element, Element) -> Bool

    init(_ higherPriority: @escaping (Element, Element) -> Bool) {
        self.higherPriority = higherPriority
    }

    init<S: Sequence>(_ elements: S, _ higherPriority: @escaping (Element, Element) -> Bool)
    where S.Element == Element {
        self.init(higherPriority)
        self.storage = Array(elements)
        self.buildHeap()
    }

    var count: Int {
        storage.count
    }

    var isEmpty: Bool {
        storage.isEmpty
    }

    func peek() -> Element? {
        storage.first
    }

    func toArray() -> [Element] {
        storage
    }

    mutating func push(_ element: Element) {
        storage.append(element)
        siftUp(from: storage.count - 1)
    }

    mutating func pop() -> Element? {
        guard !storage.isEmpty else {
            return nil
        }

        if storage.count == 1 {
            return storage.removeLast()
        }

        let top = storage[0]
        storage[0] = storage.removeLast()
        siftDown(from: 0)
        return top
    }

    private mutating func buildHeap() {
        guard storage.count > 1 else {
            return
        }

        for index in stride(from: (storage.count - 2) / 2, through: 0, by: -1) {
            siftDown(from: index)
        }
    }

    private mutating func siftUp(from index: Int) {
        var current = index
        while current > 0 {
            let parent = (current - 1) / 2
            if higherPriority(storage[current], storage[parent]) {
                storage.swapAt(current, parent)
                current = parent
            } else {
                break
            }
        }
    }

    private mutating func siftDown(from index: Int) {
        var current = index

        while true {
            let left = current * 2 + 1
            let right = left + 1
            var best = current

            if left < storage.count && higherPriority(storage[left], storage[best]) {
                best = left
            }
            if right < storage.count && higherPriority(storage[right], storage[best]) {
                best = right
            }

            if best == current {
                break
            }

            storage.swapAt(current, best)
            current = best
        }
    }
}

public struct MinHeap<Element> {
    private var heap: BinaryHeap<Element>

    public init(_ areInIncreasingOrder: @escaping (Element, Element) -> Bool) {
        self.heap = BinaryHeap(areInIncreasingOrder)
    }

    public init<S: Sequence>(_ elements: S, _ areInIncreasingOrder: @escaping (Element, Element) -> Bool)
    where S.Element == Element {
        self.heap = BinaryHeap(elements, areInIncreasingOrder)
    }

    public init() where Element: Comparable {
        self.init(<)
    }

    public init<S: Sequence>(_ elements: S) where S.Element == Element, Element: Comparable {
        self.init(elements, <)
    }

    public var count: Int {
        heap.count
    }

    public var isEmpty: Bool {
        heap.isEmpty
    }

    public func peek() -> Element? {
        heap.peek()
    }

    public func toArray() -> [Element] {
        heap.toArray()
    }

    public mutating func push(_ element: Element) {
        heap.push(element)
    }

    public mutating func pop() -> Element? {
        heap.pop()
    }
}

public struct MaxHeap<Element> {
    private var heap: BinaryHeap<Element>

    public init(_ areInIncreasingOrder: @escaping (Element, Element) -> Bool) {
        self.heap = BinaryHeap { left, right in
            areInIncreasingOrder(right, left)
        }
    }

    public init<S: Sequence>(_ elements: S, _ areInIncreasingOrder: @escaping (Element, Element) -> Bool)
    where S.Element == Element {
        self.heap = BinaryHeap(elements) { left, right in
            areInIncreasingOrder(right, left)
        }
    }

    public init() where Element: Comparable {
        self.init(<)
    }

    public init<S: Sequence>(_ elements: S) where S.Element == Element, Element: Comparable {
        self.init(elements, <)
    }

    public var count: Int {
        heap.count
    }

    public var isEmpty: Bool {
        heap.isEmpty
    }

    public func peek() -> Element? {
        heap.peek()
    }

    public func toArray() -> [Element] {
        heap.toArray()
    }

    public mutating func push(_ element: Element) {
        heap.push(element)
    }

    public mutating func pop() -> Element? {
        heap.pop()
    }
}
