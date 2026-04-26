namespace CodingAdventures.TreeSet.Tests

open System
open CodingAdventures.TreeSet
open Xunit

type TreeSetTests() =
    [<Fact>]
    member _.ConstructionAndMutationCollapseDuplicates() =
        let set = TreeSet<int>([ 3; 1; 4; 1; 5 ])

        set.Add(9).Add(2).Add(6).Add(5) |> ignore

        Assert.Equal(7, set.Count)
        Assert.Equal(7, set.Size)
        Assert.False(set.IsEmpty)
        Assert.True(set.Contains 9)
        Assert.True(set.Has 3)
        Assert.False(set.Contains 8)
        Assert.True(set.Remove 4)
        Assert.True(set.Delete 9)
        Assert.False(set.Discard 99)
        Assert.Equal<int>([ 1; 2; 3; 5; 6 ], set.ToList())

    [<Fact>]
    member _.MinMaxPredecessorSuccessorAndRankWork() =
        let set = TreeSet<int>([ 10; 20; 30; 40; 50 ])

        Assert.Equal(Some 10, set.Min())
        Assert.Equal(Some 50, set.Max())
        Assert.Equal(Some 10, set.First())
        Assert.Equal(Some 50, set.Last())
        Assert.Equal(None, TreeSet<int>().Min())
        Assert.Equal(None, set.Predecessor 10)
        Assert.Equal(Some 20, set.Predecessor 25)
        Assert.Equal(Some 30, set.Successor 25)
        Assert.Equal(None, set.Successor 50)
        Assert.Equal(2, set.Rank 30)
        Assert.Equal(1, set.Rank 15)

    [<Fact>]
    member _.ByRankKthAndRangeReturnSortedResults() =
        let set = TreeSet<int>([ 1..10 ])

        Assert.Equal(Some 1, set.ByRank 0)
        Assert.Equal(Some 10, set.ByRank 9)
        Assert.Equal(None, set.ByRank -1)
        Assert.Equal(None, set.ByRank 10)
        Assert.Equal(Some 3, set.KthSmallest 3)
        Assert.Equal(None, set.KthSmallest 0)
        Assert.Equal<int>([ 3; 4; 5; 6; 7 ], set.Range(3, 7))
        Assert.Equal<int>([ 4; 5; 6 ], set.Range(3, 7, inclusive = false))
        Assert.Empty(set.Range(7, 3))
        Assert.Equal<int>(set.ToList(), set.ToSortedArray())

    [<Fact>]
    member _.SetAlgebraDoesNotMutateInputs() =
        let a = TreeSet<int>([ 1; 2; 3; 4 ])
        let b = TreeSet<int>([ 3; 4; 5; 6 ])

        Assert.Equal<int>([ 1; 2; 3; 4; 5; 6 ], a.Union(b).ToList())
        Assert.Equal<int>([ 3; 4 ], a.Intersection(b).ToList())
        Assert.Equal<int>([ 1; 2 ], a.Difference(b).ToList())
        Assert.Equal<int>([ 1; 2; 5; 6 ], a.SymmetricDifference(b).ToList())
        Assert.Equal<int>([ 1; 2; 3; 4 ], a.ToList())
        Assert.Equal<int>([ 3; 4; 5; 6 ], b.ToList())

    [<Fact>]
    member _.PredicatesAndEqualityCompareSetContents() =
        let small = TreeSet<int>([ 2; 3 ])
        let large = TreeSet<int>([ 1; 2; 3; 4 ])
        let disjoint = TreeSet<int>([ 8; 9 ])
        let same = TreeSet<int>([ 3; 2 ])

        Assert.True(small.IsSubset large)
        Assert.True(large.IsSuperset small)
        Assert.True(small.IsDisjoint disjoint)
        Assert.False(small.IsDisjoint large)
        Assert.True(small.Equals same)
        Assert.Equal(small.GetHashCode(), same.GetHashCode())
        Assert.False(small.Equals large)

    [<Fact>]
    member _.IterationAndToStringUseSortedOrder() =
        let set = TreeSet<int>([ 5; 2; 8; 1; 9; 3 ])

        Assert.Equal<int>([ 1; 2; 3; 5; 8; 9 ], set |> Seq.toList)
        Assert.Equal("TreeSet([1, 2, 3, 5, 8, 9])", set.ToString())

    [<Fact>]
    member _.StressMatchesFrameworkSet() =
        let ours = TreeSet<int>()
        let mutable reference = Set.empty<int>
        let random = Random(314)

        for _ in 1 .. 500 do
            let key = random.Next 300
            ours.Add key |> ignore
            reference <- reference.Add key

        for key in reference |> Seq.take 200 |> Seq.toList do
            Assert.Equal(reference.Contains key, ours.Remove key)
            reference <- reference.Remove key

        for _ in 1 .. 300 do
            let key = random.Next 600

            if random.Next(2) = 0 then
                ours.Add key |> ignore
                reference <- reference.Add key
            else
                Assert.Equal(reference.Contains key, ours.Remove key)
                reference <- reference.Remove key

        Assert.Equal<int>(Set.toList reference, ours.ToList())
        Assert.Equal(Some reference.MinimumElement, ours.Min())
        Assert.Equal(Some reference.MaximumElement, ours.Max())
