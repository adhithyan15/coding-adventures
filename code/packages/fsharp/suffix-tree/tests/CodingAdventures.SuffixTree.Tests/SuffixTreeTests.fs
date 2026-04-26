namespace CodingAdventures.SuffixTree.Tests

open System
open CodingAdventures.SuffixTree
open Xunit

type SuffixTreeTests() =
    [<Fact>]
    member _.BuildSearchAndCountWork() =
        let tree = SuffixTree.Build("banana")

        Assert.Equal("banana", tree.Text)
        Assert.Equal<int>([ 1; 3 ], tree.Search "ana")
        Assert.Equal<int>([ 0..6 ], tree.Search "")
        Assert.Empty(tree.Search "band")
        Assert.Equal(2, tree.CountOccurrences "ana")
        Assert.Equal(7, tree.NodeCount())

    [<Fact>]
    member _.StaticHelpersDelegateToTree() =
        let tree = SuffixTree.BuildUkkonen("banana")

        Assert.Equal<int>([ 2; 4 ], SuffixTree.Search(tree, "n"))
        Assert.Equal(1, SuffixTree.CountOccurrences(tree, "nan"))
        Assert.Equal(7, SuffixTree.NodeCount tree)
        Assert.Throws<ArgumentNullException>(fun () -> SuffixTree.Build(null) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> tree.Search null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> SuffixTree.Search(null, "a") |> ignore) |> ignore

    [<Fact>]
    member _.SuffixAndRepeatedSubstringHelpersWork() =
        let tree = SuffixTree.Build("banana")

        Assert.Equal("ana", tree.LongestRepeatedSubstring())
        Assert.Equal("ana", SuffixTree.LongestRepeatedSubstring tree)
        Assert.Equal("banana", tree.AllSuffixes()[0])
        Assert.Equal("a", SuffixTree.AllSuffixes tree |> List.last)
        Assert.Throws<ArgumentNullException>(fun () -> SuffixTree.AllSuffixes null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> SuffixTree.LongestRepeatedSubstring null |> ignore) |> ignore

    [<Fact>]
    member _.LongestCommonSubstringHandlesEdges() =
        Assert.Equal("abxa", SuffixTree.LongestCommonSubstring("xabxac", "abcabxabcd"))
        Assert.Equal(String.Empty, SuffixTree.LongestCommonSubstring("", "abc"))
        Assert.Equal(String.Empty, SuffixTree.LongestCommonSubstring("abc", ""))
        Assert.Throws<ArgumentNullException>(fun () -> SuffixTree.LongestCommonSubstring(null, "abc") |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> SuffixTree.LongestCommonSubstring("abc", null) |> ignore) |> ignore
