namespace CodingAdventures.BTree.Tests

open System
open System.Collections.Generic
open CodingAdventures.BTree
open Xunit

type BTreeTests() =
    [<Fact>]
    member _.ConstructorRejectsInvalidMinimumDegree() =
        Assert.Throws<ArgumentException>(fun () -> BTree<int, string>(1) |> ignore)
        |> ignore

        let tree = BTree<int, string>()
        Assert.True(tree.IsEmpty)
        Assert.Equal(0, tree.Height())
        Assert.True(tree.IsValid())
        Assert.Equal(2, tree.MinimumDegree)

    [<Fact>]
    member _.InsertSearchAndUpdateKeepSizeStable() =
        let tree = BTree<int, string>(3)

        for key in [ 10; 5; 20; 1; 15; 30 ] do
            tree.Insert(key, $"v{key}")

        tree.Insert(15, "updated")

        Assert.Equal(6, tree.Count)
        Assert.True(tree.Contains 20)
        Assert.False(tree.Contains 99)
        Assert.Equal(Some "updated", tree.Search 15)
        Assert.Equal(None, tree.Search 99)
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.SequentialInsertsSplitRootAndTraverseSorted() =
        let tree = BTree<int, string>(2)

        for key in 0 .. 99 do
            tree.Insert(key, $"v{key}")
            Assert.True(tree.IsValid())

        Assert.Equal(100, tree.Count)
        Assert.True(tree.Height() > 0)
        Assert.Equal<int>([ 0..99 ], tree.InOrder() |> List.map _.Key)

    [<Fact>]
    member _.DeleteHandlesLeafBorrowMergeAndRootShrink() =
        let tree = BTree<int, string>(2)

        for key in 1 .. 25 do
            tree.Insert(key, $"v{key}")

        let deletionOrder = [ 7; 12; 1; 25; 13; 14; 15; 16; 17; 18; 19; 20; 21; 22; 23; 24; 2; 3; 4; 5; 6; 8; 9; 10 ]

        for key in deletionOrder do
            tree.Delete key
            Assert.False(tree.Contains key)
            Assert.True(tree.IsValid())

        Assert.Equal(1, tree.Count)
        Assert.Equal(11, tree.MinKey())
        Assert.Equal(11, tree.MaxKey())
        Assert.Equal(0, tree.Height())

    [<Fact>]
    member _.DeleteMissingKeyThrows() =
        let tree = BTree<int, string>()
        tree.Insert(10, "ten")

        Assert.Throws<KeyNotFoundException>(fun () -> tree.Delete 99) |> ignore
        Assert.True(tree.Contains 10)
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.MinMaxAndRangeQueryWorkAcrossLevels() =
        let tree = BTree<int, string>(3)

        for key in 1 .. 50 do
            tree.Insert(key, $"v{key}")

        Assert.Equal(1, tree.MinKey())
        Assert.Equal(50, tree.MaxKey())
        Assert.Equal<int>([ 10..20 ], tree.RangeQuery(10, 20) |> List.map _.Key)
        Assert.Empty(tree.RangeQuery(60, 70))
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.EmptyMinMaxThrowAndRangeIsEmpty() =
        let tree = BTree<int, string>()

        Assert.Throws<InvalidOperationException>(fun () -> tree.MinKey() |> ignore) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> tree.MaxKey() |> ignore) |> ignore
        Assert.Empty(tree.RangeQuery(1, 10))
        Assert.Empty(tree.InOrder())
        Assert.Equal("BTree(t=2, size=0, height=0)", tree.ToString())

    [<Fact>]
    member _.StressMatchesSortedDictionary() =
        let tree = BTree<int, string>(3)
        let reference = SortedDictionary<int, string>()
        let random = Random(1234)
        let keys = [ 0..399 ] |> List.sortBy (fun _ -> random.Next())

        for key in keys do
            let value = $"v{key}"
            tree.Insert(key, value)
            reference[key] <- value

        for key in keys do
            Assert.Equal(Some reference[key], tree.Search key)

        for key in keys |> List.take 175 do
            tree.Delete key
            reference.Remove key |> ignore

        for _ in 1 .. 100 do
            let key = random.Next 600

            if random.Next(2) = 0 then
                tree.Insert(key, $"v{key}")
                reference[key] <- $"v{key}"
            elif reference.Remove key then
                tree.Delete key

        Assert.Equal(reference.Count, tree.Count)
        Assert.True(tree.IsValid())
        Assert.Equal<int>(Seq.toList reference.Keys, tree.InOrder() |> List.map _.Key)
