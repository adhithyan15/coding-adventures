# F# rank statistics

`CodingAdventures.RankStatistics.FSharp` exposes idiomatic F# functions for a
sample median and its exact distribution-free population-median interval. It
uses .NET `BigInteger` for exact binomial cutoffs and has no third-party runtime
dependency.

```fsharp
open CodingAdventures.RankStatistics

let estimate = RankStatistics.median [ 4.0; 1.0; 9.0; 3.0 ]
let interval = RankStatistics.medianInterval [ 0.8; 0.8; 0.8; 0.8; 0.8; 0.8 ] 95
```

Samples must be nonempty and finite; the sequence is enumerated once and not
mutated. Confidence is an integer percentage from 1 through 99. No qualifying
rank is represented by `None`.
