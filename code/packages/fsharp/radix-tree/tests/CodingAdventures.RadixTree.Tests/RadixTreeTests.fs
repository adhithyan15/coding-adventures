namespace CodingAdventures.RadixTree.Tests

open CodingAdventures.RadixTree
open Xunit

type RadixTreeTests() =
    [<Fact>]
    member _.InsertAndSearchCoverSplitCases() =
        let tree = RadixTree<int>()

        tree.Insert("application", 1)
        tree.Insert("apple", 2)
        tree.Insert("app", 3)
        tree.Insert("apt", 4)

        Assert.Equal(Some 1, tree.Search "application")
        Assert.Equal(Some 2, tree.Search "apple")
        Assert.Equal(Some 3, tree.Search "app")
        Assert.Equal(Some 4, tree.Search "apt")
        Assert.Equal(None, tree.Search "appl")
        Assert.Equal(4, tree.Count)

    [<Fact>]
    member _.UpdatingExistingKeyDoesNotChangeCount() =
        let tree = RadixTree<string>()

        tree.Insert("alpha", "first")
        tree.Put("alpha", "second")

        Assert.True(tree.ContainsKey "alpha")
        Assert.Equal(Some "second", tree.Get "alpha")
        Assert.Equal(1, tree.Count)

    [<Fact>]
    member _.DeletePrunesAndMerges() =
        let tree = RadixTree<int>()
        tree.Insert("app", 1)
        tree.Insert("apple", 2)

        Assert.Equal(3, tree.NodeCount())
        Assert.True(tree.Delete "app")
        Assert.False(tree.ContainsKey "app")
        Assert.Equal(Some 2, tree.Search "apple")
        Assert.Equal(2, tree.NodeCount())
        Assert.False(tree.Delete "app")

    [<Fact>]
    member _.SupportsPrefixQueriesAndMatches() =
        let tree = RadixTree<int>()
        tree.Insert("search", 1)
        tree.Insert("searcher", 2)
        tree.Insert("searching", 3)
        tree.Insert("banana", 4)

        Assert.True(tree.StartsWith "sear")
        Assert.False(tree.StartsWith "seek")
        Assert.Equal<string>([ "search"; "searcher"; "searching" ], tree.WordsWithPrefix "search")
        Assert.Equal(Some "search", tree.LongestPrefixMatch "search-party")
        Assert.Equal(None, tree.LongestPrefixMatch "xyz")

    [<Fact>]
    member _.SupportsEmptyStringAndSortedKeys() =
        let tree = RadixTree<int>()
        tree.Insert("", 1)
        tree.Insert("banana", 2)
        tree.Insert("apple", 3)
        tree.Insert("apricot", 4)
        tree.Insert("app", 5)

        Assert.Equal(Some 1, tree.Search "")
        Assert.Equal(Some "", tree.LongestPrefixMatch "xyz")
        Assert.True(tree.Delete "")
        Assert.Equal(None, tree.Search "")
        Assert.Equal<string>([ "app"; "apple"; "apricot"; "banana" ], tree.Keys())
        Assert.Equal<int>([ 5; 3; 4; 2 ], tree.Values())
        Assert.Equal(4, tree.Size)
        Assert.False(tree.IsEmpty)

    [<Fact>]
    member _.ConstructorAndMapExportKeepSortedKeys() =
        let tree =
            RadixTree<int>(
                [ "delta", 4
                  "alpha", 1
                  "charlie", 3 ]
            )

        Assert.Equal<string>([ "alpha"; "charlie"; "delta" ], tree.ToMap() |> Map.toList |> List.map fst)
