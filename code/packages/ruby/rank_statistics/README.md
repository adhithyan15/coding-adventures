# coding_adventures_rank_statistics

Finite-sample order statistics for real-valued data. This standalone RubyGem
provides a sample median and an exact distribution-free confidence interval
for a population median; it does not depend on benchmark code.

```ruby
require "coding_adventures_rank_statistics"

CodingAdventures::RankStatistics.median([9, 1, 5, 3])
# => 4.0

CodingAdventures::RankStatistics.median_interval([1, 2, 3, 4, 5, 6], 95)
# => nil when no order-statistic rank meets the requested coverage
```

Samples must be non-empty arrays of real values representable as finite
binary64 numbers. Input arrays are left unchanged. The interval uses arbitrary
precision integer binomial cutoffs, retains tied values, and returns `nil` if
the sample cannot reach the requested confidence. Confidence is an integer
percentage from 1 through 99.

The package carries the shared `microbenchmark-pairs-v1.json` fixture under
`testdata/` for cross-language conformance. Its gem has no runtime dependencies.

## Development

```sh
bundle install
bundle exec rake test
```

The independent sign-assignment oracle, multi-limb Pascal-triangle differential,
and seeded property checks run in the regular suite. SimpleCov targets at least
95 percent line coverage.
