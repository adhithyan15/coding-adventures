# C# rank statistics

`CodingAdventures.RankStatistics.CSharp` provides a sample median and exact
distribution-free confidence interval for a population median. It uses
`System.Numerics.BigInteger` for the binomial cutoff and has no third-party
runtime dependency.

```csharp
using CodingAdventures.RankStatistics;

double estimate = RankStatistics.Median(new[] { 4.0, 1.0, 9.0, 3.0 });
MedianInterval? interval = RankStatistics.MedianInterval(
    new[] { 0.8, 0.8, 0.8, 0.8, 0.8, 0.8 }, 95);
```

Samples must be nonempty and finite; the input is not modified. Confidence is
an integer percentage from 1 through 99. If the exact cutoff admits no rank,
`MedianInterval` returns `null`.
