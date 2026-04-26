# CodingAdventures.Rope.CSharp

Pure C# immutable rope for text concat, split, insert, delete, indexing, substring, depth, and rebalance operations.

```csharp
var rope = Rope.FromString("hello").Concat(Rope.FromString(" world"));
var edited = rope.Insert(5, ",");
var text = edited.ToString();
```
