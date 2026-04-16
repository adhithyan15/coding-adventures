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
