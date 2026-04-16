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

    [<Fact>]
    member _.``supports clone membership and relations``() =
        let left = HashSet([ "a"; "b" ])
        let right = HashSet([ "b"; "c" ])
        let clone = left.Clone()
        let intersection = left.Intersection(right)
        let removed = left.Remove("a")

        Assert.True(left.Contains "a")
        Assert.False(HashSet<string>().IsEmpty() |> not)
        Assert.True((clone.ToList() |> List.sort) = [ "a"; "b" ])
        Assert.True(intersection.ToList() = [ "b" ])
        Assert.True(removed.ToList() = [ "b" ])
