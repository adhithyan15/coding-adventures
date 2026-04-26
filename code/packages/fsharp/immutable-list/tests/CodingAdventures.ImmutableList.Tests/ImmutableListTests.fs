namespace CodingAdventures.ImmutableList.Tests

open System
open CodingAdventures.ImmutableList
open Xunit

type ImmutableListTests() =
    [<Fact>]
    member _.EmptyListHasExpectedShape() =
        let list = ImmutableList<string>.Empty
        let mutable value = ""

        Assert.True(list.IsEmpty)
        Assert.Equal(0, list.Count)
        Assert.Equal(0, list.Length)
        Assert.Equal(None, list.Get 0)
        Assert.False(list.TryGet(0, &value))
        Assert.Null(value)
        Assert.Empty(list)
        Assert.Equal("ImmutableList(count=0)", list.ToString())

    [<Fact>]
    member _.PushReturnsNewListAndLeavesOriginalUnchanged() =
        let empty = ImmutableList<string>.Empty
        let one = empty.Push "hello"
        let two = one.Push "world"

        Assert.Empty(empty)
        Assert.Equal<string>([ "hello" ], one.ToArray())
        Assert.Equal<string>([ "hello"; "world" ], two.ToList())
        Assert.Equal("world", two[1])

    [<Fact>]
    member _.FromSeqAndFromSlicePreserveOrder() =
        let list = ImmutableList<int>.FromSeq([ 3; 1; 4 ])
        let slice = ImmutableList<int>.FromSlice([ 1; 5; 9 ])

        Assert.Equal<int>([ 3; 1; 4 ], list.ToArray())
        Assert.Equal<int>([ 1; 5; 9 ], slice.ToArray())
        Assert.Same(ImmutableList<int>.Empty, ImmutableList<int>.FromSeq([]))
        Assert.Throws<ArgumentNullException>(fun () -> ImmutableList<int>.FromSeq(Unchecked.defaultof<seq<int>>) |> ignore) |> ignore

    [<Fact>]
    member _.SetReturnsChangedCopy() =
        let list = ImmutableList<string>.FromSlice([ "a"; "b"; "c" ])
        let updated = list.Set(1, "B")

        Assert.Equal<string>([ "a"; "b"; "c" ], list.ToArray())
        Assert.Equal<string>([ "a"; "B"; "c" ], updated.ToArray())
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> list.Set(-1, "x") |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> list.Set(3, "x") |> ignore) |> ignore

    [<Fact>]
    member _.PopReturnsRemainderAndRemovedValue() =
        let list = ImmutableList<int>.FromSlice([ 1; 2; 3 ])
        let two, removed = list.Pop()
        let one, secondRemoved = two.Pop()
        let empty, firstRemoved = one.Pop()

        Assert.Equal(3, removed)
        Assert.Equal(2, secondRemoved)
        Assert.Equal(1, firstRemoved)
        Assert.Equal<int>([ 1; 2 ], two.ToArray())
        Assert.Same(ImmutableList<int>.Empty, empty)
        Assert.Throws<InvalidOperationException>(fun () -> empty.Pop() |> ignore) |> ignore

    [<Fact>]
    member _.GetTryGetAndIndexerHandleBounds() =
        let list = ImmutableList<int>.FromSlice([ 10; 20 ])
        let mutable value = 0

        Assert.Equal(Some 10, list.Get 0)
        Assert.Equal(None, list.Get 20)
        Assert.True(list.TryGet(1, &value))
        Assert.Equal(20, value)
        Assert.False(list.TryGet(-1, &value))
        Assert.Equal(0, value)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> list[-1] |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> list[2] |> ignore) |> ignore

    [<Fact>]
    member _.EnumerationUsesSnapshotOrder() =
        let list = ImmutableList<string>().Push("a").Push("b").Push("c")

        Assert.Equal<string>([ "a"; "b"; "c" ], list |> Seq.toList)
        Assert.Equal("ImmutableList(count=3)", list.ToString())
