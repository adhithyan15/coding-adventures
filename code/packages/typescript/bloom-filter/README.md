# @coding-adventures/bloom-filter

A probabilistic membership filter with zero false negatives and tunable false
positive probability.

```ts
import { BloomFilter } from "@coding-adventures/bloom-filter";

const filter = new BloomFilter({ expectedItems: 1_000, falsePositiveRate: 0.01 });
filter.add("hello");

filter.contains("hello"); // true
filter.contains("world"); // probably false
```
