namespace CodingAdventures.HashMap.FSharp.Tests

open CodingAdventures.HashMap.FSharp
open Xunit

type HashMapTests() =
    [<Fact>]
    member _.``stores and deletes values``() =
        let map = HashMap<int, string>().Set(1, "one").Set(2, "two")
        Assert.Equal(2, map.Size)
        Assert.Equal(Some "one", map.Get 1)
        Assert.True(map.Has 2)

        let next = map.Delete 1
        Assert.Equal(None, next.Get 1)
        Assert.Equal(Some "two", next.Get 2)

    [<Fact>]
    member _.``supports clone clearing and collection helpers``() =
        let map = HashMap.fromEntries [ "a", 1; "b", 2 ]
        let clone = map.Clone()
        let cleared = map.Clear()
        let dict = map.ToDictionary()

        Assert.Equal(2, map.Count)
        Assert.True((map.Keys() |> List.sort) = [ "a"; "b" ])
        Assert.True((map.Values() |> List.sort) = [ 1; 2 ])
        Assert.True((map.Entries() |> List.sortBy fst) = [ ("a", 1); ("b", 2) ])
        Assert.Equal(Some 1, clone.Get "a")
        Assert.False(cleared.Has "a")
        Assert.Equal(2, dict.Count)
