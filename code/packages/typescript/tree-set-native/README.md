# @coding-adventures/tree-set-native

Native Node.js addon that wraps the Rust `tree-set` crate.

This package is Node-only. Use `@coding-adventures/tree-set` for the browser-safe
pure TypeScript implementation.

## Usage

```typescript
import { TreeSet } from "@coding-adventures/tree-set-native";

const set = new TreeSet([5, 1, 3, 9]);
set.add(7);
console.log(set.toSortedArray()); // [1, 3, 5, 7, 9]
```

