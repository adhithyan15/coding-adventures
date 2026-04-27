# CodingAdventures.Trie

An immutable trie for binary string keys. It supports exact lookup, deletion,
prefix enumeration, sorted keys, and longest-prefix matching.

```elixir
trie =
  CodingAdventures.Trie.new()
  |> CodingAdventures.Trie.insert("app", 1)
  |> CodingAdventures.Trie.insert("apple", 2)

CodingAdventures.Trie.search(trie, "app")
# {:ok, 1}
```
