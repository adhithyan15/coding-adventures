namespace CodingAdventures.HyperLogLog.FSharp.Tests

open CodingAdventures.HyperLogLog.FSharp
open Xunit

type HyperLogLogTests() =
    [<Fact>]
    member _.``approximates cardinality``() =
        let hll = HyperLogLog()
        [ "a"; "b"; "c"; "d" ] |> List.iter (box >> hll.Add)
        Assert.InRange(hll.Count(), 3, 5)
