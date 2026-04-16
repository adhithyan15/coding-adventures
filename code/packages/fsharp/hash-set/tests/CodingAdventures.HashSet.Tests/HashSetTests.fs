namespace CodingAdventures.HashSet.FSharp.Tests

open CodingAdventures.HashSet.FSharp
open Xunit

type HashSetTests() =
    [<Fact>]
    member _.``supports basic set operations``() =
        let set = HashSet<string>().Add("a").Add("b").Add("a")
        Assert.Equal(2, set.Size)
        Assert.True(set.Has "a")

        let unioned = set.Union(HashSet([ "c" ]))
        Assert.True(unioned.Has "c")

        let diff = unioned.Difference(HashSet([ "a" ]))
        Assert.False(diff.Has "a")
