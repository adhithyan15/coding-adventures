namespace CodingAdventures.FenwickTree.Tests

open System
open Xunit
open CodingAdventures.FenwickTree

type FenwickTreeTests() =
    let epsilon = 1e-9

    let assertClose actual expected =
        Assert.True(
            abs (actual - expected) < epsilon,
            sprintf "Expected %f, got %f" expected actual
        )

    let brutePrefix (values: float array) index =
        values[.. index - 1] |> Array.sum

    let bruteRange (values: float array) left right =
        values[left - 1 .. right - 1] |> Array.sum

    [<Fact>]
    member _.``construction validates size and tracks length``() =
        let empty = FenwickTree(0)
        Assert.Equal(0, empty.Length)
        Assert.True(empty.IsEmpty)
        Assert.Equal(0.0, empty.PrefixSum(0))

        let sized = FenwickTree(5)
        Assert.Equal(5, sized.Length)
        Assert.False(sized.IsEmpty)

        Assert.Throws<FenwickError>(fun () -> FenwickTree(-1) |> ignore)
        |> ignore

    [<Fact>]
    member _.``fromList builds expected prefix totals``() =
        let tree = FenwickTree.FromList([ 3.0; 2.0; 1.0; 7.0; 4.0 ])

        assertClose (tree.PrefixSum(1)) 3.0
        assertClose (tree.PrefixSum(2)) 5.0
        assertClose (tree.PrefixSum(3)) 6.0
        assertClose (tree.PrefixSum(4)) 13.0
        assertClose (tree.PrefixSum(5)) 17.0

    [<Fact>]
    member _.``prefix range and point queries match expected values``() =
        let tree = FenwickTree.FromList([ 3.0; 2.0; 1.0; 7.0; 4.0 ])

        assertClose (tree.PrefixSum(0)) 0.0
        assertClose (tree.RangeSum(1, 5)) 17.0
        assertClose (tree.RangeSum(2, 4)) 10.0
        assertClose (tree.PointQuery(4)) 7.0

    [<Fact>]
    member _.``update applies positive and negative deltas``() =
        let tree = FenwickTree.FromList([ 3.0; 2.0; 1.0; 7.0; 4.0 ])

        tree.Update(3, 5.0)
        assertClose (tree.PointQuery(3)) 6.0
        assertClose (tree.PrefixSum(3)) 11.0

        tree.Update(2, -1.0)
        assertClose (tree.PointQuery(2)) 1.0
        assertClose (tree.PrefixSum(3)) 10.0

    [<Fact>]
    member _.``update from index one propagates across power-of-two parents``() =
        let tree = FenwickTree.FromList(Array.zeroCreate<float> 8)

        tree.Update(1, 10.0)

        for index in 1 .. 8 do
            assertClose (tree.PrefixSum(index)) 10.0

    [<Fact>]
    member _.``query operations validate bounds``() =
        let tree = FenwickTree.FromList([ 1.0; 2.0; 3.0 ])

        Assert.Throws<FenwickIndexOutOfRangeError>(fun () -> tree.PrefixSum(-1) |> ignore)
        |> ignore
        Assert.Throws<FenwickIndexOutOfRangeError>(fun () -> tree.PrefixSum(4) |> ignore)
        |> ignore
        Assert.Throws<FenwickIndexOutOfRangeError>(fun () -> tree.Update(0, 1.0))
        |> ignore
        Assert.Throws<FenwickIndexOutOfRangeError>(fun () -> tree.RangeSum(0, 2) |> ignore)
        |> ignore
        Assert.Throws<FenwickIndexOutOfRangeError>(fun () -> tree.PointQuery(4) |> ignore)
        |> ignore
        Assert.Throws<FenwickError>(fun () -> tree.RangeSum(3, 1) |> ignore)
        |> ignore

    [<Fact>]
    member _.``findKth matches documented examples and validation rules``() =
        let tree = FenwickTree.FromList([ 1.0; 2.0; 3.0; 4.0; 5.0 ])

        Assert.Equal(1, tree.FindKth(1.0))
        Assert.Equal(2, tree.FindKth(2.0))
        Assert.Equal(2, tree.FindKth(3.0))
        Assert.Equal(3, tree.FindKth(4.0))
        Assert.Equal(4, tree.FindKth(10.0))
        Assert.Equal(5, tree.FindKth(11.0))

        Assert.Throws<FenwickError>(fun () -> tree.FindKth(0.0) |> ignore)
        |> ignore
        Assert.Throws<FenwickError>(fun () -> tree.FindKth(100.0) |> ignore)
        |> ignore
        Assert.Throws<FenwickEmptyTreeError>(fun () -> FenwickTree(0).FindKth(1.0) |> ignore)
        |> ignore

    [<Fact>]
    member _.``prefix and range queries match brute force across random arrays``() =
        let mutable seed = 1337

        let next () =
            seed <- int (((int64 seed * 1103515245L) + 12345L) &&& 0x7fffffffL)
            seed

        for _ in 0 .. 119 do
            let n = (next () % 30) + 1
            let values = Array.init n (fun _ -> float ((next () % 101) - 50))
            let tree = FenwickTree.FromList(values)

            for index in 1 .. n do
                assertClose (tree.PrefixSum(index)) (brutePrefix values index)

            for left in 1 .. n do
                for right in left .. n do
                    assertClose (tree.RangeSum(left, right)) (bruteRange values left right)

    [<Fact>]
    member _.``tree stays consistent under interleaved updates and queries``() =
        let mutable seed = 99u

        let next () =
            seed <- seed * 1664525u + 1013904223u
            seed

        let size = 60
        let values = Array.init size (fun _ -> float ((int (next () % 20u)) + 1))
        let tree = FenwickTree.FromList(values)

        for _ in 0 .. 1199 do
            if next () % 10u < 4u then
                let left = (int (next () % uint32 size)) + 1
                let right = left + int (next () % uint32 (size - left + 1))
                assertClose (tree.RangeSum(left, right)) (bruteRange values left right)
            else
                let index = (int (next () % uint32 size)) + 1
                let delta = float ((int (next () % 41u)) - 20)
                values[index - 1] <- values[index - 1] + delta
                tree.Update(index, delta)

    [<Fact>]
    member _.``toString renders logical shape``() =
        let tree = FenwickTree.FromList([ 1.0; 2.0; 3.0 ])
        Assert.Equal("FenwickTree(n=3, bit=[1, 3, 3])", tree.ToString())
