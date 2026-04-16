namespace CodingAdventures.HyperLogLog.FSharp.Tests

open CodingAdventures.HyperLogLog.FSharp
open Xunit

type HyperLogLogTests() =
    [<Fact>]
    member _.``approximates cardinality``() =
        let hll = HyperLogLog()
        [ "a"; "b"; "c"; "d" ] |> List.iter (box >> hll.Add)
        Assert.InRange(hll.Count(), 3, 5)

    [<Fact>]
    member _.``supports clone merge and precision helpers``() =
        let left = HyperLogLog(8)
        let right = HyperLogLog(8)
        [ "a"; "b"; "c" ] |> List.iter (box >> left.Add)
        [ "c"; "d"; "e" ] |> List.iter (box >> right.Add)

        let clone = left.Clone()
        let merged = left.Merge(right)

        Assert.Equal(8, left.Precision)
        Assert.Equal(256, left.Registers.Length)
        Assert.Equal(left.Count(), clone.Count())
        Assert.True(merged.Count() >= left.Count())
        Assert.ThrowsAny<System.ArgumentException>(fun () -> HyperLogLog(2) |> ignore) |> ignore
