# @coding-adventures/hyperloglog

HyperLogLog implementation for approximate cardinality estimation.

The package is dependency-free and intentionally mirrors the Rust DT21
surface closely enough for the datastore engine:

- configurable precision
- add / merge operations
- cardinality estimation
- helper functions for memory sizing and error-rate planning

## Example

```ts
import { HyperLogLog } from "@coding-adventures/hyperloglog";

const hll = new HyperLogLog();
hll.add("alice");
hll.add("bob");
hll.add("alice");

console.log(hll.count());
```
