namespace CodingAdventures.SkipList.FSharp.Tests

open CodingAdventures.SkipList.FSharp
open System.Collections.Generic
open Xunit

type SkipListTests() =
    [<Fact>]
    member _.``keeps items ordered``() =
        let skip = SkipList<int, string>()
        skip.Insert(2, "b")
        skip.Insert(1, "a")
        skip.Insert(3, "c")

        Assert.Equal(Some "a", skip.Search 1)
        Assert.Equal(Some 1, skip.Rank 2)
        Assert.Equal(3, skip.Length)

    [<Fact>]
    member _.``supports replace delete and entry helpers``() =
        let skip = SkipList<int, string>()
        Assert.True(skip.IsEmpty())

        skip.Insert(2, "b")
        skip.Insert(1, "a")
        skip.Insert(2, "updated")

        Assert.True(skip.Contains 1)
        Assert.Equal(Some "updated", skip.Search 2)
        Assert.True((skip.EntriesList() |> List.map (fun pair -> pair.Key, pair.Value)) = [ (1, "a"); (2, "updated") ])
        Assert.True(skip.Delete 1)
        Assert.False(skip.Delete 99)
        Assert.Equal(None, skip.Rank 99)
