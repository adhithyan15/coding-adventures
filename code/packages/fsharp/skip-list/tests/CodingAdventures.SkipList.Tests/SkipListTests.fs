namespace CodingAdventures.SkipList.FSharp.Tests

open CodingAdventures.SkipList.FSharp
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
