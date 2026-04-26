namespace CodingAdventures.Treap.Tests

open System
open CodingAdventures.Treap
open Xunit

type TreapTests() =
    let build keys =
        keys |> Seq.fold (fun (treap: Treap) key -> treap.Insert key) (Treap.WithSeed 42)

    [<Fact>]
    member _.EmptyAndSingleTreapsExposeMetrics() =
        let empty = Treap.WithSeed 1

        Assert.True(empty.IsValidTreap())
        Assert.True(empty.IsEmpty)
        Assert.Equal(0, empty.Size)
        Assert.Equal(0, empty.Height)
        Assert.Equal(None, empty.Min())
        Assert.Equal(None, empty.Max())
        Assert.Throws<ArgumentException>(fun () -> empty.KthSmallest 1 |> ignore) |> ignore

        let single = empty.Insert 10
        Assert.False(single.IsEmpty)
        Assert.Equal(1, single.Size)
        Assert.Equal(1, single.Height)
        Assert.Equal(Some 10, single.Min())
        Assert.Equal(Some 10, single.Max())
        Assert.True(single.IsValidTreap())

    [<Fact>]
    member _.ExplicitPrioritiesShapeTreapDeterministically() =
        let treap =
            (Treap.WithSeed 0)
                .InsertWithPriority(5, 0.91)
                .InsertWithPriority(3, 0.53)
                .InsertWithPriority(7, 0.75)
                .InsertWithPriority(1, 0.22)
                .InsertWithPriority(4, 0.68)

        Assert.True(treap.IsValidTreap())
        Assert.Equal(Some 5, treap.Root |> Option.map _.Key)
        Assert.Equal<int>([ 1; 3; 4; 5; 7 ], treap.ToSortedList())

    [<Fact>]
    member _.InsertIgnoresDuplicatesAndPreservesOriginal() =
        let original = build [ 5; 3; 7 ]
        let modified = original.Insert(1).Insert(9).Insert(5)

        Assert.Equal<int>([ 3; 5; 7 ], original.ToSortedList())
        Assert.Equal<int>([ 1; 3; 5; 7; 9 ], modified.ToSortedList())
        Assert.Equal(5, modified.Size)
        Assert.True(modified.IsValidTreap())

    [<Fact>]
    member _.ContainsMinMaxPredecessorSuccessorAndKthWork() =
        let treap = build [ 10; 5; 15; 3; 7; 12; 20 ]

        Assert.True(treap.Contains 10)
        Assert.False(treap.Contains 11)
        Assert.Equal(Some 3, treap.Min())
        Assert.Equal(Some 20, treap.Max())
        Assert.Equal(None, treap.Predecessor 3)
        Assert.Equal(Some 7, treap.Predecessor 10)
        Assert.Equal(Some 12, treap.Successor 10)
        Assert.Equal(None, treap.Successor 20)
        Assert.Equal(3, treap.KthSmallest 1)
        Assert.Equal(10, treap.KthSmallest 4)
        Assert.Throws<ArgumentException>(fun () -> treap.KthSmallest 0 |> ignore) |> ignore

    [<Fact>]
    member _.SplitAndMergePartitionAndReconstruct() =
        let original = build [ 1..10 ]
        let parts = original.Split 5
        let left = Treap.FromRoot parts.Left
        let right = Treap.FromRoot parts.Right

        Assert.Equal<int>([ 1; 2; 3; 4; 5 ], left.ToSortedList())
        Assert.Equal<int>([ 6; 7; 8; 9; 10 ], right.ToSortedList())

        let merged = Treap.Merge(left, right)
        Assert.True(merged.IsValidTreap())
        Assert.Equal<int>(original.ToSortedList(), merged.ToSortedList())

    [<Fact>]
    member _.DeleteHandlesAbsentRootAndAllKeys() =
        let treap =
            (Treap.WithSeed 0)
                .InsertWithPriority(5, 0.9)
                .InsertWithPriority(3, 0.5)
                .InsertWithPriority(7, 0.6)

        Assert.Same(treap, treap.Delete 99)
        let withoutRoot = treap.Delete 5
        Assert.True(withoutRoot.IsValidTreap())
        Assert.Equal<int>([ 3; 7 ], withoutRoot.ToSortedList())

        let mutable all = build [ 10; 5; 15; 3; 7; 12; 20 ]
        for key in all.ToSortedList() do
            all <- all.Delete key
            Assert.True(all.IsValidTreap())
            Assert.False(all.Contains key)

        Assert.True(all.IsEmpty)

    [<Fact>]
    member _.RandomStressPreservesSortedSetContents() =
        let random = Random 7
        let mutable treap = Treap.WithSeed 55
        let mutable reference = Set.empty<int>

        for _ in 1 .. 200 do
            let key = random.Next 100
            treap <- treap.Insert key
            reference <- reference.Add key
            Assert.True(treap.IsValidTreap())

        for key in reference |> Seq.toList |> List.sortBy (fun _ -> random.Next()) do
            treap <- treap.Delete key
            Assert.True(treap.IsValidTreap())

        Assert.True(treap.IsEmpty)
