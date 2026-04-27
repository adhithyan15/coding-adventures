# Heap (Swift)

Comparator-based binary min-heap and max-heap package for Swift.

## Usage

```swift
import Heap

var minHeap = MinHeap<Int>()
minHeap.push(5)
minHeap.push(1)
minHeap.push(3)
print(minHeap.pop()!) // 1

var maxHeap = MaxHeap<Int>()
maxHeap.push(5)
maxHeap.push(1)
maxHeap.push(3)
print(maxHeap.pop()!) // 5

var tupleHeap = MinHeap<(priority: Int, label: String)> {
    if $0.priority != $1.priority {
        return $0.priority < $1.priority
    }
    return $0.label < $1.label
}
tupleHeap.push((priority: 1, label: "b"))
tupleHeap.push((priority: 1, label: "a"))
print(tupleHeap.pop()!.label) // a
```

## API

- `MinHeap<Element>()` and `MaxHeap<Element>()` for `Element: Comparable`
- `MinHeap<Element> { ... }` and `MaxHeap<Element> { ... }` for custom ordering
- `init(_ elements: Sequence)` and `init(_ elements: Sequence, _ comparator)`
- `push`, `pop`, `peek`, `count`, `isEmpty`, and `toArray`

## Running Tests

```bash
swift test
```
