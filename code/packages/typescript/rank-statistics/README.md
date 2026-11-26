# @coding-adventures/rank-statistics

Reusable finite-sample median and exact distribution-free confidence interval
for the population median. The exact interval uses integer arithmetic to decide
which binomial order-statistic rank qualifies; floating point is used only for
the sample values and returned endpoints.

```ts
import { median, medianInterval } from "@coding-adventures/rank-statistics";
median([1, 2, 8]); // 2
medianInterval([1, 2, 3, 4, 5, 6], 95); // null: no rank reaches 95% coverage
```

Inputs must be non-empty arrays of finite numbers. The input array is not
modified. Confidence is an integer percentage from 1 through 99. An interval
is `null` when no order-statistic rank reaches the requested confidence. The
portable paired-ratio fixture is included under `testdata/` for source and
package-artifact conformance.
