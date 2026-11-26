# RankStatistics (Swift)

Exact median and distribution-free median intervals for finite `Double` data.

## Usage

```swift
import RankStatistics

let center = try median([9.0, 1.0, 4.0, 7.0])
let interval = try medianInterval([0.8, 0.8, 0.9, 1.0, 1.1, 1.2], confidencePercent: 95)
print(center) // 5.5
print(interval as Any)
```

`median` sorts a copy and handles large finite values and subnormal midpoint
rounding. `medianInterval` uses an exact multi-limb binomial cutoff and returns
`nil` when the requested confidence has no qualifying order-statistic rank.
The standalone exact interval API accepts at most 10,000 observations.

## Build and tests

```bash
bash BUILD
```

On Windows, `BUILD_windows` delegates to `tools/run-windows.cmd`. It skips the
package test only when Swift is unavailable; a test failure is returned to the
build tool.
