# Go trie

A generic trie for string keys and arbitrary Go values. It supports exact lookup,
deletion, prefix scans, sorted key enumeration, and longest-prefix matching.

```go
trie := trie.New[int]()
trie.Insert("app", 1)
trie.Insert("apple", 2)

value, ok := trie.Search("app")
matches := trie.WordsWithPrefix("app")
```
