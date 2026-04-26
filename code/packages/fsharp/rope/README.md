# CodingAdventures.Rope

Pure F# immutable rope for text concat, split, insert, delete, indexing, substring, depth, and rebalance operations.

```fsharp
let rope = Rope.FromString("hello").Concat(Rope.FromString(" world"))
let edited = rope.Insert(5, ",")
let text = edited.ToString()
```
