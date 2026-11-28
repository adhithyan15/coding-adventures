namespace CodingAdventures.RankStatistics

open System
open System.Numerics

/// A closed interval with endpoints selected from the sample.
[<Struct>]
type MedianInterval =
    { Low: float
      High: float }

/// Finite-sample median and exact distribution-free interval operations.
module RankStatistics =
    let private copyFinite (values: seq<float>) =
        if isNull (box values) then nullArg (nameof values)
        let sample = values |> Seq.toArray
        if sample.Length = 0 then invalidArg (nameof values) "values must not be empty"
        if sample |> Array.exists (Double.IsFinite >> not) then
            invalidArg (nameof values) "all values must be finite"
        sample

    /// Returns the binary64 median without mutating or retaining the input.
    let median (values: seq<float>) =
        let ordered = copyFinite values
        Array.sortInPlace ordered
        let middle = ordered.Length / 2
        if ordered.Length % 2 = 1 then ordered[middle]
        else
            let low = ordered[middle - 1]
            let high = ordered[middle]
            let sum = low + high
            if Double.IsFinite sum then sum / 2.0 else low / 2.0 + high / 2.0

    /// Returns the largest exact order-statistic confidence interval, if one qualifies.
    let medianInterval (values: seq<float>) (confidencePercent: int) =
        if confidencePercent < 1 || confidencePercent > 99 then
            invalidArg (nameof confidencePercent) "confidence must be from 1 through 99"
        let ordered = copyFinite values
        Array.sortInPlace ordered
        let n = ordered.Length
        let denominator = BigInteger.One <<< n
        let mutable choose = BigInteger.One
        let mutable cumulative = BigInteger.Zero
        let mutable largestRank = 0
        let mutable keepGoing = true
        for j in 0 .. n / 2 do
            if keepGoing then
                cumulative <- cumulative + choose
                if 200I * cumulative > bigint (100 - confidencePercent) * denominator then
                    keepGoing <- false
                else
                    largestRank <- j + 1
                    choose <- choose * bigint (n - j) / bigint (j + 1)
        if largestRank = 0 then None
        else
            Some
                { Low = ordered[largestRank - 1]
                  High = ordered[n - largestRank] }
