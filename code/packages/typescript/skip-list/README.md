# @coding-adventures/skip-list

Ordered collection with skip-list-style semantics.

The implementation is deterministic and dependency-free. It keeps the public
API close to the Rust version we use in the datastore stack:

- ordered inserts and updates
- point lookups and deletes
- rank queries
- range scans

## Example

```ts
import { SkipList } from "@coding-adventures/skip-list";

const scores = new SkipList<number, string>();
scores.insert(10, "alice");
scores.insert(5, "bob");

console.log(scores.toList()); // [5, 10]
```
