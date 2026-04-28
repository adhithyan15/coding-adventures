namespace CodingAdventures.Rope.Tests

open System
open CodingAdventures.Rope
open Xunit

type RopeTests() =
    [<Fact>]
    member _.EmptyAndFromStringExposeLength() =
        let empty = Rope.Empty()
        let rope = Rope.FromString("hello")

        Assert.True(empty.IsEmpty)
        Assert.Equal(0, empty.Length)
        Assert.Equal(0, empty.Count)
        Assert.Equal(String.Empty, empty.ToString())
        Assert.True(empty.IsBalanced())
        Assert.Equal(0, empty.Depth())
        Assert.Equal("hello", Rope.RopeFromString("hello").ToString())
        Assert.Equal(5, rope.Length)
        Assert.Throws<ArgumentNullException>(fun () -> Rope.FromString(null) |> ignore) |> ignore

    [<Fact>]
    member _.ConcatSplitAndIndexWork() =
        let rope = Rope.Concat(Rope.FromString("hello"), Rope.FromString(" world"))

        Assert.Equal(11, rope.Length)
        Assert.Equal("hello world", rope.ToString())
        Assert.Equal(Some 'e', rope.Index 1)
        Assert.Equal(None, rope.Index -1)
        Assert.Equal(None, rope.Index 11)

        let left, right = rope.Split 5
        Assert.Equal("hello", left.ToString())
        Assert.Equal(" world", right.ToString())

    [<Fact>]
    member _.InstanceConcatPreservesEmptyIdentities() =
        let left = Rope.Empty().Concat(Rope.FromString("a"))
        let right = Rope.FromString("b").Concat(Rope.Empty())

        Assert.Equal("a", left.ToString())
        Assert.Equal("b", right.ToString())
        Assert.Throws<ArgumentNullException>(fun () -> Rope.Concat(null, Rope.Empty()) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Rope.Concat(Rope.Empty(), null) |> ignore) |> ignore

    [<Fact>]
    member _.InsertDeleteAndSubstringClampLikeRust() =
        let rope = Rope.FromString("ace").Insert(1, "b").Insert(3, "d")

        Assert.Equal("abcde", rope.ToString())
        Assert.Equal("bcd", rope.Substring(1, 4))
        Assert.Equal(String.Empty, rope.Substring(4, 1))
        Assert.Equal("abcde", rope.Substring(-20, 20))
        Assert.Equal("ade", rope.Delete(1, 2).ToString())
        Assert.Equal("abcde!", rope.Insert(99, "!").ToString())
        Assert.Equal("bcde", rope.Delete(-10, 1).ToString())
        Assert.Throws<ArgumentNullException>(fun () -> rope.Insert(0, null) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> rope.Delete(0, -1) |> ignore) |> ignore

    [<Fact>]
    member _.RebalanceProducesBalancedTreeWithSameText() =
        let rope =
            Rope.FromString("a")
                .Concat(Rope.FromString("b"))
                .Concat(Rope.FromString("c"))
                .Concat(Rope.FromString("d"))
                .Concat(Rope.FromString("e"))
                .Concat(Rope.FromString("f"))

        Assert.False(rope.IsBalanced())

        let balanced = rope.Rebalance()

        Assert.Equal("abcdef", balanced.ToString())
        Assert.True(balanced.IsBalanced())
        Assert.True(balanced.Depth() <= 3)
