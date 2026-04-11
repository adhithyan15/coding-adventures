# @coding-adventures/hash-set

Immutable hash set built on top of the TypeScript `hash-map` package.

The API mirrors the DT19 set semantics closely enough for the datastore
engine and the future native-addon layer:

- insertion and deletion return new sets
- membership checks are O(1) average via the backing hash map
- set algebra helpers are available for union/intersection/difference

## Example

```ts
import { HashSet } from "@coding-adventures/hash-set";

const base = HashSet.fromList(["alpha", "beta"]);
const next = base.add("gamma");

console.log(base.has("gamma")); // false
console.log(next.has("gamma")); // true
```
