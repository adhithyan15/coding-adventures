namespace CodingAdventures.BPlusTree.Tests

open System
open System.Collections.Generic
open CodingAdventures.BPlusTree
open Xunit

type BPlusTreeTests() =
    [<Fact>]
    member _.ConstructorRejectsInvalidMinimumDegree() =
        Assert.Throws<ArgumentException>(fun () -> BPlusTree<int, string>(1) |> ignore)
        |> ignore

        let tree = BPlusTree<int, string>()
        Assert.Equal(2, tree.MinimumDegree)
        Assert.Equal(0, tree.Count)
        Assert.Equal(0, tree.Size)
        Assert.True(tree.IsEmpty)
        Assert.Equal(0, tree.Height())
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.InsertSearchAndUpdateKeepSizeStable() =
        let tree = BPlusTree<int, string>(3)

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
    member _.ContainsHandlesNullValues() =
        let tree = BPlusTree<int, string>()
        tree.Insert(42, null)

        Assert.True(tree.Contains 42)
        Assert.Equal(Some null, tree.Search 42)
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.NullKeysAreRejected() =
        let tree = BPlusTree<string, int>()
        let nullKey = Unchecked.defaultof<string>

        Assert.Throws<ArgumentNullException>(fun () -> tree.Insert(nullKey, 1)) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> tree.Search nullKey |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> tree.Contains nullKey |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> tree.Delete nullKey) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> tree.RangeScan(nullKey, "z") |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> tree.RangeScan("a", nullKey) |> ignore) |> ignore

    [<Fact>]
    member _.SequentialInsertsSplitLeavesAndTraverseSorted() =
        let tree = BPlusTree<int, string>(2)

        for key in 1 .. 3 do
            tree.Insert(key, $"v{key}")

        Assert.Equal(0, tree.Height())

        for key in 4 .. 50 do
            tree.Insert(key, $"v{key}")
            Assert.True(tree.IsValid())

        Assert.True(tree.Height() >= 2)
        Assert.Equal<int>([ 1..50 ], tree.FullScan() |> List.map _.Key)
        Assert.Equal<int>([ 1..50 ], tree |> Seq.map _.Key |> Seq.toList)

    [<Fact>]
    member _.RangeScanIsInclusiveAndRejectsInvertedBounds() =
        let tree = BPlusTree<int, string>(2)

        for key in [ 9; 3; 7; 1; 5; 2; 8; 4; 6; 10 ] do
            tree.Insert(key, $"v{key}")

        Assert.Equal<int>([ 3..7 ], tree.RangeScan(3, 7) |> List.map _.Key)
        Assert.Equal<int>([ 3..7 ], tree.RangeQuery(3, 7) |> List.map _.Key)
        Assert.Empty(tree.RangeScan(11, 20))
        Assert.Throws<ArgumentException>(fun () -> tree.RangeScan(7, 3) |> ignore) |> ignore

    [<Fact>]
    member _.MinMaxAndEmptyEdgesBehavePredictably() =
        let tree = BPlusTree<int, string>()

        Assert.Throws<InvalidOperationException>(fun () -> tree.MinKey() |> ignore) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> tree.MaxKey() |> ignore) |> ignore
        Assert.Empty(tree.FullScan())
        Assert.Empty(tree.InOrder())
        Assert.Empty(tree.RangeScan(1, 10))
        Assert.Equal("BPlusTree(t=2, size=0, height=0)", tree.ToString())

        tree.Insert(20, "twenty")
        tree.Insert(10, "ten")
        tree.Insert(30, "thirty")

        Assert.Equal(10, tree.MinKey())
        Assert.Equal(30, tree.MaxKey())
        Assert.Equal("BPlusTree(t=2, size=3, height=0)", tree.ToString())

    [<Fact>]
    member _.DeleteRemovesKeysAndMissingDeleteIsNoOp() =
        let tree = BPlusTree<int, string>(2)

        for key in 1 .. 25 do
            tree.Insert(key, $"v{key}")

        for key in [ 7; 12; 1; 25; 13; 14; 15; 16; 17; 18 ] do
            tree.Delete key
            Assert.False(tree.Contains key)
            Assert.True(tree.IsValid())

        tree.Delete 99

        Assert.Equal(15, tree.Count)
        Assert.Equal(2, tree.MinKey())
        Assert.Equal(24, tree.MaxKey())
        Assert.True(tree.Height() > 0)

    [<Fact>]
    member _.MultipleMinimumDegreesStayValid() =
        for degree in [ 2; 3; 5; 8 ] do
            let tree = BPlusTree<int, string>(degree)

            for key in 100 .. -1 .. 1 do
                tree.Insert(key, $"v{key}")

            Assert.True(tree.IsValid())
            Assert.Equal<int>([ 1..100 ], tree.FullScan() |> List.map _.Key)

    [<Fact>]
    member _.StressMatchesSortedDictionary() =
        let tree = BPlusTree<int, string>(3)
        let reference = SortedDictionary<int, string>()
        let random = Random(1234)

        for step in 0 .. 499 do
            let key = random.Next 200

            if random.Next(4) = 0 then
                tree.Delete key
                reference.Remove key |> ignore
            else
                let value = $"v{step}"
                tree.Insert(key, value)
                reference[key] <- value

            Assert.Equal(reference.Count, tree.Count)
            Assert.True(tree.IsValid())
            Assert.Equal<int>(Seq.toList reference.Keys, tree.FullScan() |> List.map _.Key)

            for entry in reference do
                Assert.True(tree.Contains entry.Key)
                Assert.Equal(Some entry.Value, tree.Search entry.Key)
