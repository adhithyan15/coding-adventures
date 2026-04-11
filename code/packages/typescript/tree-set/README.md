# @coding-adventures/tree-set

Browser-safe ordered set with sorted iteration, rank / selection helpers,
range queries, and set algebra.

## Usage

```typescript
import { TreeSet } from "@coding-adventures/tree-set";

const set = new TreeSet([5, 1, 3, 9]);
set.add(7);

console.log(set.toSortedArray()); // [1, 3, 5, 7, 9]
console.log(set.rank(7)); // 3
console.log(set.range(3, 7)); // [3, 5, 7]
```

The pure package is browser-safe and depends on no Node-only APIs.

