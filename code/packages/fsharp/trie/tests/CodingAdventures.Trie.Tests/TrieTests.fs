namespace CodingAdventures.Trie.Tests

open System
open Xunit
open CodingAdventures.Trie

type TrieTests() =
    [<Fact>]
    member _.``Insert get and overwrite``() =
        let trie = Trie<int>()
        trie.Insert("apple", 1)
        trie.Insert("app", 2)

        let mutable apple = 0
        Assert.True(trie.TryGetValue("apple", &apple))
        Assert.Equal(1, apple)
        Assert.Equal(Some 2, trie.Get("app"))
        Assert.Equal(2, trie.Count)

        trie.Insert("apple", 42)
        Assert.Equal(Some 42, trie.Get("apple"))
        Assert.Equal(2, trie.Size)

    [<Fact>]
    member _.``Missing keys and prefixes are distinct``() =
        let trie = Trie<int>()
        trie.Insert("apple", 1)
        Assert.False(trie.Contains "app")
        Assert.True(trie.StartsWith "app")
        Assert.Equal(None, trie.Get "banana")
        Assert.False(trie.StartsWith "banana")

    [<Fact>]
    member _.``Empty key is supported``() =
        let trie = Trie<string>()
        trie.Insert("", "empty")
        Assert.True(trie.Contains "")
        Assert.Equal(Some "empty", trie.Get "")
        Assert.Equal(1, trie.Count)

    [<Fact>]
    member _.``Delete unmarks only the requested key``() =
        let trie = Trie<int>()
        trie.Insert("app", 1)
        trie.Insert("apple", 2)
        trie.Insert("apply", 3)

        Assert.True(trie.Delete "apple")
        Assert.False(trie.Contains "apple")
        Assert.True(trie.Contains "app")
        Assert.True(trie.Contains "apply")
        Assert.False(trie.Delete "apple")
        Assert.False(trie.Delete "banana")
        Assert.Equal(2, trie.Count)

    [<Fact>]
    member _.``Keys with prefix returns matches``() =
        let trie = Trie<int>()
        trie.Insert("app", 1)
        trie.Insert("apple", 2)
        trie.Insert("apply", 3)
        trie.Insert("apt", 4)
        trie.Insert("banana", 5)

        let appKeys = trie.KeysWithPrefix "app"
        Assert.Equal(3, List.length appKeys)
        Assert.Contains("app", appKeys)
        Assert.Contains("apple", appKeys)
        Assert.Contains("apply", appKeys)
        Assert.DoesNotContain("apt", appKeys)
        Assert.Empty(trie.KeysWithPrefix "z")

        let allKeys = trie.Keys()
        Assert.Equal(5, List.length allKeys)
        Assert.Contains("banana", allKeys)

    [<Fact>]
    member _.``Size and is empty track mutations``() =
        let trie = Trie<int>()
        Assert.True(trie.IsEmpty)
        trie.Insert("a", 1)
        trie.Insert("b", 2)
        Assert.False(trie.IsEmpty)
        trie.Delete("a") |> ignore
        trie.Delete("b") |> ignore
        Assert.True(trie.IsEmpty)
        Assert.Equal(0, trie.Count)

    [<Fact>]
    member _.``Unicode and large datasets work``() =
        let trie = Trie<int>()
        trie.Insert("cafe\u0301", 1)
        trie.Insert("cafe\u0301s", 2)
        Assert.True(trie.Contains "cafe\u0301")
        Assert.Equal(2, trie.KeysWithPrefix("caf").Length)

        for i in 0 .. 249 do
            trie.Insert($"key{i}", i)

        Assert.Equal(252, trie.Count)
        Assert.Equal(250, trie.KeysWithPrefix("key").Length)

    [<Fact>]
    member _.``Null keys are rejected or treated as absent``() =
        let trie = Trie<int>()
        Assert.Throws<ArgumentNullException>(fun () -> trie.Insert(null, 1)) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> trie.KeysWithPrefix(null) |> ignore) |> ignore
        Assert.False(trie.Contains null)
        Assert.False(trie.StartsWith null)
        Assert.False(trie.Delete null)
