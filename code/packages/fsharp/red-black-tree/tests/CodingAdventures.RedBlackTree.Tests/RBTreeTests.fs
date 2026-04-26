namespace CodingAdventures.RedBlackTree.Tests

open System
open System.Collections.Generic
open CodingAdventures.RedBlackTree
open Xunit

type RBTreeTests() =
    let build values =
        values |> Seq.fold (fun (tree: RBTree) value -> tree.Insert value) (RBTree.Empty())

    [<Fact>]
    member _.EmptyTreeReportsMetadataAndMissingValues() =
        let tree = RBTree.Empty()

        Assert.True(tree.IsValidRB())
        Assert.True(tree.IsEmpty)
        Assert.Equal(0, tree.Size())
        Assert.Equal(0, tree.Height())
        Assert.Equal(0, tree.BlackHeight())
        Assert.False(tree.Contains 42)
        Assert.Equal(None, tree.Min())
        Assert.Equal(None, tree.Max())
        Assert.Empty(tree.ToSortedList())
        Assert.Throws<InvalidOperationException>(fun () -> tree.KthSmallest 1 |> ignore) |> ignore

    [<Fact>]
    member _.SingleInsertCreatesBlackRoot() =
        let tree = build [ 10 ]

        Assert.True(tree.IsValidRB())
        Assert.Equal(Some Black, tree.Root |> Option.map _.Color)
        Assert.Equal(1, tree.Size())
        Assert.True(tree.Contains 10)
        Assert.False(tree.Contains 11)
        Assert.Equal(Some 10, tree.Min())
        Assert.Equal(Some 10, tree.Max())

    [<Fact>]
    member _.InsertSequencesStayValidAndSorted() =
        let mutable ascending = RBTree.Empty()

        for value in 1 .. 40 do
            ascending <- ascending.Insert value
            Assert.True(ascending.IsValidRB())

        Assert.Equal<int list>([ 1..40 ], ascending.ToSortedList())

        let mutable descending = RBTree.Empty()

        for value in 40 .. -1 .. 1 do
            descending <- descending.Insert value
            Assert.True(descending.IsValidRB())

        Assert.Equal<int list>([ 1..40 ], descending.ToSortedList())

    [<Fact>]
    member _.DuplicatesAreIgnored() =
        let tree = build [ 5; 5; 5; 3; 3; 7 ]

        Assert.True(tree.IsValidRB())
        Assert.Equal(3, tree.Size())
        Assert.Equal<int list>([ 3; 5; 7 ], tree.ToSortedList())

    [<Fact>]
    member _.SearchMinMaxPredecessorSuccessorWork() =
        let tree = build [ 10; 5; 15; 3; 7; 12; 20 ]

        Assert.True(tree.Contains 10)
        Assert.True(tree.Contains 20)
        Assert.False(tree.Contains 11)
        Assert.Equal(Some 3, tree.Min())
        Assert.Equal(Some 20, tree.Max())
        Assert.Equal(Some 10, tree.Predecessor 12)
        Assert.Equal(Some 7, tree.Predecessor 10)
        Assert.Equal(None, tree.Predecessor 3)
        Assert.Equal(Some 12, tree.Successor 10)
        Assert.Equal(Some 10, tree.Successor 7)
        Assert.Equal(None, tree.Successor 20)

    [<Fact>]
    member _.KthSmallestUsesSortedOrder() =
        let tree = build [ 5; 3; 8; 1; 9; 4 ]

        Assert.Equal(1, tree.KthSmallest 1)
        Assert.Equal(3, tree.KthSmallest 2)
        Assert.Equal(4, tree.KthSmallest 3)
        Assert.Equal(5, tree.KthSmallest 4)
        Assert.Equal(8, tree.KthSmallest 5)
        Assert.Equal(9, tree.KthSmallest 6)
        Assert.Throws<InvalidOperationException>(fun () -> tree.KthSmallest 0 |> ignore) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> tree.KthSmallest 7 |> ignore) |> ignore

    [<Fact>]
    member _.DeleteCasesPreserveInvariants() =
        let absent = (build [ 5; 3; 7 ]).Delete 99
        Assert.True(absent.IsValidRB())
        Assert.Equal<int list>([ 3; 5; 7 ], absent.ToSortedList())

        Assert.True((build [ 42 ]).Delete(42).IsEmpty)
        Assert.Equal<int list>([ 3 ], (build [ 5; 3 ]).Delete(5).ToSortedList())
        Assert.Equal<int list>([ 5; 7 ], (build [ 5; 3; 7 ]).Delete(3).ToSortedList())

        let internalDeleted = (build [ 10; 5; 15; 3; 7; 12; 20 ]).Delete 5
        Assert.True(internalDeleted.IsValidRB())
        Assert.Equal<int list>([ 3; 7; 10; 12; 15; 20 ], internalDeleted.ToSortedList())

    [<Fact>]
    member _.DeleteAllElementsInSeveralOrders() =
        let values = [ 10; 5; 15; 3; 7; 12; 20 ]
        let mutable tree = build values

        for value in values do
            tree <- tree.Delete value
            Assert.True(tree.IsValidRB())
            Assert.False(tree.Contains value)

        Assert.True(tree.IsEmpty)

        let mutable minOrder = build [ 1; 2; 3; 4; 5 ]

        for value in 1 .. 5 do
            minOrder <- minOrder.Delete value
            Assert.True(minOrder.IsValidRB())

        let mutable maxOrder = build [ 1; 2; 3; 4; 5 ]

        for value in 5 .. -1 .. 1 do
            maxOrder <- maxOrder.Delete value
            Assert.True(maxOrder.IsValidRB())

    [<Fact>]
    member _.ImmutabilityKeepsOldTreeUnchanged() =
        let original = build [ 5; 3; 7 ]
        let modified = original.Insert(1).Insert(9).Delete 3

        Assert.Equal<int list>([ 3; 5; 7 ], original.ToSortedList())
        Assert.Equal<int list>([ 1; 5; 7; 9 ], modified.ToSortedList())
        Assert.True(modified.IsValidRB())

    [<Fact>]
    member _.HeightBlackHeightAndToStringAreConsistent() =
        let tree = build [ 1..100 ]
        let maxAllowedHeight = int (2.0 * Math.Ceiling(Math.Log2 101.0))

        Assert.True(tree.IsValidRB())
        Assert.True(tree.Height() <= maxAllowedHeight)
        Assert.True(tree.BlackHeight() > 0)
        Assert.Equal($"RBTree{{size={tree.Size()}, height={tree.Height()}, blackHeight={tree.BlackHeight()}}}", tree.ToString())

    [<Fact>]
    member _.RandomStressMatchesSortedSet() =
        let random = Random 42
        let reference = SortedSet<int>()
        let mutable tree = RBTree.Empty()

        for _ in 1 .. 200 do
            let value = random.Next 100
            tree <- tree.Insert value
            reference.Add value |> ignore
            Assert.True(tree.IsValidRB())
            Assert.Equal<int list>(reference |> Seq.toList, tree.ToSortedList())

        for value in reference |> Seq.sortBy (fun _ -> random.Next()) |> Seq.toArray do
            tree <- tree.Delete value
            reference.Remove value |> ignore
            Assert.True(tree.IsValidRB())
            Assert.Equal<int list>(reference |> Seq.toList, tree.ToSortedList())

        Assert.True(tree.IsEmpty)

    [<Fact>]
    member _.NodeRecordsExposeColorAndIsRed() =
        let red =
            { Value = 5
              Color = Red
              Left = None
              Right = None }

        let black = { red with Color = Black }

        Assert.True(red.IsRed)
        Assert.False(black.IsRed)
        Assert.Equal(5, red.Value)
