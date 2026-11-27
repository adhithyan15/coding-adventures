# CodingAdventures::RankStatistics

Pure Perl reusable statistics for finite binary64 samples.

```perl
use CodingAdventures::RankStatistics qw(median median_interval);

my $center = median([8, 1, 4, 2]);
my $interval = median_interval([8, 1, 4, 2, 3, 5, 6, 7], 95);
```

`median` sorts a copy, rejects empty/non-finite input, and rounds an even
sample midpoint to nearest binary64. `median_interval` uses an exact
`Math::BigInt` cutoff and returns `[low, high]` or `undef` when no rank
qualifies. The package is independent of the microbenchmark collector.

Run `perl -Ilib t/rank_statistics.t` to exercise bounded independent sign
enumeration, frozen wide cutoffs, finite-value boundaries, and seeded generated
arrays. The package depends only on core Perl modules plus `Test2::V0` for its
test suite.
`tests/installed-consumer/Consumer.pl` checks the installed package surface.

`BUILD` measures Devel::Cover 1.52 statement coverage for
`lib/CodingAdventures/RankStatistics.pm` only and requires more than 80% with a
nonzero denominator. The test harness and sibling modules are outside that
production-library metric; branch, condition, and subroutine figures are
reported separately.
