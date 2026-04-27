namespace CodingAdventures.SegmentTree.Tests

open System
open CodingAdventures.SegmentTree
open Xunit

type SegmentTreeTests() =
    let bruteSum (values: int array) left right =
        values[left..right] |> Array.sum

    let bruteMin (values: int array) left right =
        values[left..right] |> Array.min

    let bruteMax (values: int array) left right =
        values[left..right] |> Array.max

    let gcd (left: int) (right: int) =
        let mutable a = Math.Abs left
        let mutable b = Math.Abs right

        while b <> 0 do
            let next = a % b
            a <- b
            b <- next

        a

    let bruteGcd (values: int array) left right =
        values[left..right] |> Array.reduce gcd

    let assertAllRanges (values: int array) (tree: SegmentTree<int>) (reference: int array -> int -> int -> int) =
        for left in 0 .. values.Length - 1 do
            for right in left .. values.Length - 1 do
                Assert.Equal(reference values left right, tree.Query(left, right))

    [<Fact>]
    member _.EmptyTreeHasMetadataAndRejectsOperations() =
        let tree = SegmentTree.sumTree [||]

        Assert.Equal(0, tree.Size)
        Assert.Equal(0, tree.Count)
        Assert.True(tree.IsEmpty)
        Assert.Empty(tree.ToList())
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Query(0, 0) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Update(0, 1)) |> ignore

    [<Fact>]
    member _.SingleElementTreesQueryAndUpdate() =
        let sum = SegmentTree.sumTree [| 42 |]
        let minimum = SegmentTree.minTree [| -7 |]
        let maximum = SegmentTree.maxTree [| 99 |]

        Assert.Equal(42, sum.Query(0, 0))
        Assert.Equal(-7, minimum.Query(0, 0))
        Assert.Equal(99, maximum.Query(0, 0))
        sum.Update(0, 55)
        Assert.Equal(55, sum.Query(0, 0))

    [<Fact>]
    member _.SumTreeMatchesSpecExampleAndUpdates() =
        let tree = SegmentTree.sumTree [| 2; 1; 5; 3; 4 |]

        Assert.Equal(15, tree.Query(0, 4))
        Assert.Equal(9, tree.Query(1, 3))
        Assert.Equal(3, tree.Query(0, 1))
        Assert.Equal(7, tree.Query(3, 4))
        Assert.Equal(5, tree.Query(2, 2))
        tree.Update(2, 7)
        Assert.Equal(11, tree.Query(1, 3))
        Assert.Equal(17, tree.Query(0, 4))
        Assert.Equal<int list>([ 2; 1; 7; 3; 4 ], tree.ToList())

    [<Fact>]
    member _.SumTreeAllRangesMatchBruteForce() =
        let values = [| -3; 1; -4; 1; 5; -9; 2; 6 |]
        let tree = SegmentTree.sumTree values

        assertAllRanges values tree bruteSum

    [<Fact>]
    member _.MinTreeAllRangesAndUpdatesMatchBruteForce() =
        let values = [| 5; 3; 7; 1; 9; 2 |]
        let tree = SegmentTree.minTree values

        assertAllRanges values tree bruteMin
        values[3] <- 10
        tree.Update(3, 10)
        Assert.Equal(2, tree.Query(0, 5))
        assertAllRanges values tree bruteMin

    [<Fact>]
    member _.MaxTreeAllRangesAndUpdatesMatchBruteForce() =
        let values = [| 3; -1; 4; 1; 5; 9; 2; 6 |]
        let tree = SegmentTree.maxTree values

        assertAllRanges values tree bruteMax
        values[2] <- 100
        tree.Update(2, 100)
        Assert.Equal(100, tree.Query(0, 7))
        Assert.Equal(100, tree.Query(1, 3))
        assertAllRanges values tree bruteMax

    [<Fact>]
    member _.GcdTreeAllRangesMatchBruteForce() =
        let values = [| 12; 8; 6; 4; 9 |]
        let tree = SegmentTree.gcdTree values

        Assert.Equal(2, tree.Query(0, 2))
        Assert.Equal(1, tree.Query(1, 4))
        Assert.Equal(4, tree.Query(0, 1))
        assertAllRanges values tree bruteGcd

    [<Fact>]
    member _.NonPowerOfTwoAndMultipleUpdatesStayConsistent() =
        let values = [| 1; 2; 3; 4; 5; 6; 7 |]
        let tree = SegmentTree.sumTree values

        Assert.Equal(28, tree.Query(0, 6))
        Assert.Equal(9, tree.Query(1, 3))
        values[0] <- 10
        values[6] <- 20
        values[2] <- 0
        tree.Update(0, 10)
        tree.Update(6, 20)
        tree.Update(2, 0)
        Assert.Equal<int list>(values |> Array.toList, tree.ToList())
        assertAllRanges values tree bruteSum

    [<Fact>]
    member _.InvalidRangesAndIndicesThrow() =
        let tree = SegmentTree.sumTree [| 1; 2; 3 |]

        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Query(2, 1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Query(-1, 2) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Query(0, 3) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Update(-1, 5)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> tree.Update(3, 5)) |> ignore

    [<Fact>]
    member _.CustomCombineSupportsProductAndBitwiseOr() =
        let product = SegmentTree<int>([| 2; 3; 4; 5 |], (*), 1)
        Assert.Equal(120, product.Query(0, 3))
        Assert.Equal(24, product.Query(0, 2))
        Assert.Equal(20, product.Query(2, 3))

        let bitwiseOr = SegmentTree<int>([| 0b0001; 0b0010; 0b0100; 0b1000 |], (|||), 0)
        Assert.Equal(0b1111, bitwiseOr.Query(0, 3))
        Assert.Equal(0b0011, bitwiseOr.Query(0, 1))
        Assert.Equal(0b0110, bitwiseOr.Query(1, 2))

    [<Fact>]
    member _.RandomStressMatchesReferenceArray() =
        let random = Random 12345
        let values = Array.init 200 (fun _ -> random.Next(1000) - 500)
        let sum = SegmentTree.sumTree values
        let minimum = SegmentTree.minTree values
        let maximum = SegmentTree.maxTree values

        for _ in 1 .. 50 do
            let left = random.Next values.Length
            let right = left + random.Next(values.Length - left)
            Assert.Equal(bruteSum values left right, sum.Query(left, right))
            Assert.Equal(bruteMin values left right, minimum.Query(left, right))
            Assert.Equal(bruteMax values left right, maximum.Query(left, right))

        for _ in 1 .. 200 do
            let index = random.Next values.Length
            let value = random.Next(1000) - 500
            values[index] <- value
            sum.Update(index, value)
            minimum.Update(index, value)
            maximum.Update(index, value)

            let left = random.Next values.Length
            let right = left + random.Next(values.Length - left)
            Assert.Equal(bruteSum values left right, sum.Query(left, right))
            Assert.Equal(bruteMin values left right, minimum.Query(left, right))
            Assert.Equal(bruteMax values left right, maximum.Query(left, right))

    [<Fact>]
    member _.ConstructorsValidateNullArgumentsAndToStringReportsMetadata() =
        Assert.Throws<ArgumentNullException>(fun () -> SegmentTree<int>(null, (+), 0) |> ignore) |> ignore
        let missingCombine = Unchecked.defaultof<int -> int -> int>
        Assert.Throws<ArgumentNullException>(fun () -> SegmentTree<int>([| 1 |], missingCombine, 0) |> ignore) |> ignore
        Assert.Equal("SegmentTree{n=3, identity=0}", (SegmentTree.sumTree [| 1; 2; 3 |]).ToString())
