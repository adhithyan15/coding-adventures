# Go radix tree

A compressed trie for string-keyed prefix lookup.

```go
tree := radixtree.New[int]()
tree.Insert("search", 1)
tree.Insert("searcher", 2)
matches := tree.WordsWithPrefix("search")
```
