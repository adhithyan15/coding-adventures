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

## Checked bounded queues

`BoundedMinHeap<T>` and `BoundedMaxHeap<T>` are additive checked queues. Pass
an explicit comparator and a logical capacity; construction does not preallocate
that capacity. The existing unbounded classes and helpers retain their behavior.

```ts
import {
  BoundedMinHeap,
  type BoundedPriorityQueue,
} from "@coding-adventures/heap";

const queue: BoundedPriorityQueue<number> = new BoundedMinHeap(2, (a, b) => a - b);
queue.tryPush(8); // { status: "accepted" }
queue.tryPush(3); // { status: "accepted" }
queue.tryPush(1); // { status: "full", rejected: 1 }
queue.tryPeek();  // { status: "ok", value: 3 }
queue.tryPop();   // { status: "ok", value: 3 }
```

The comparator must be deterministic, non-reentrant, and define a strict weak
order for values while they are queued. It may update observational counters;
any external effects are the caller's responsibility. Capacity is a safe
integer from zero through `2^32 - 1`. Result discriminants keep an actual
`undefined` value distinct from an empty queue.

`fromIterable(capacity, items, comparator)` consumes one pass and retains no
more than `capacity` items. Its `capacity-exceeded` result includes both the
accepted-prefix heap and the exact first rejected item. The iterator is closed
on overflow using JavaScript iterator semantics. Construction uses repeated
insertion, with O(k log k) comparisons for k accepted items.

The package root resolves to emitted ESM JavaScript and `.d.ts` files. Build
the package before installing a local `file:../heap` consumer so the package's
`dist` entrypoints exist.
