# @coding-adventures/heap

Min-heap and max-heap implementations backed by a flat array, plus pure helper
functions:

- `heapify`
- `heapSort`
- `nlargest`
- `nsmallest`

## Quick Start

```ts
import { MinHeap, heapSort, nlargest } from "@coding-adventures/heap";

const heap = new MinHeap<number>();
for (const value of [5, 3, 8, 1, 4]) {
  heap.push(value);
}

console.log(heap.peek()); // 1
console.log(heap.pop());  // 1

console.log(heapSort([3, 1, 4, 1, 5, 9])); // [1, 1, 3, 4, 5, 9]
console.log(nlargest([3, 1, 4, 1, 5, 9], 3)); // [9, 5, 4]
```
